use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use core::ops::DerefMut;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use spin::{Mutex, Once};
use x86_64::{VirtAddr, PhysAddr};
use crate::arch::{detect_cpu_info, get_cpu_count, get_current_cpu_id};

static NEXT_PID: AtomicU64 = AtomicU64::new(1);
static SMP_SCHEDULER: Once<Mutex<SmpScheduler>> = Once::new();
static LEGACY_SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
static IDLE_THREAD_PID: AtomicU64 = AtomicU64::new(0);

pub type ProcessId = u64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

/// Basic signal types for process management
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal {
    SIGTERM = 15,  // Termination request
    SIGKILL = 9,   // Force kill
    SIGSTOP = 19,  // Stop process
    SIGCONT = 18,  // Continue process
    SIGUSR1 = 10,  // User-defined signal 1
    SIGUSR2 = 12,  // User-defined signal 2
}

/// Signal handler function type
pub type SignalHandler = fn(Signal);

/// Default signal handler that terminates the process
fn default_signal_handler(signal: Signal) {
    match signal {
        Signal::SIGTERM | Signal::SIGKILL => {
            exit_process(-1); // Exit with error code
        }
        Signal::SIGSTOP => {
            // Block the current process
            let current_pid = get_current_process_id();
            let cpu_id = get_current_cpu_id();
            get_smp_scheduler().lock().block_current_on_cpu(cpu_id);
        }
        Signal::SIGCONT => {
            // Continue is handled by the scheduler
        }
        _ => {
            // Ignore other signals by default
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    High = 0,
    Normal = 1,
    Low = 2,
    Gaming = 3, // Special priority for gaming mode
}

/// Real-time scheduling classes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtClass {
    /// Earliest Deadline First - for hard real-time tasks
    Edf,
    /// Constant Bandwidth Server - for soft real-time with bandwidth guarantees
    Cbs,
    /// Best effort - normal scheduling
    BestEffort,
}

/// Real-time scheduling parameters
#[derive(Debug, Clone, Copy)]
pub struct RtParams {
    pub class: RtClass,
    pub deadline_us: u64,     // Deadline in microseconds
    pub period_us: u64,       // Period in microseconds
    pub budget_us: u64,       // CPU budget per period
    pub remaining_budget: u64, // Remaining budget in current period
    pub next_deadline: u64,   // Absolute deadline timestamp
    pub last_replenish: u64,  // Last budget replenishment time
}

impl Default for RtParams {
    fn default() -> Self {
        Self {
            class: RtClass::BestEffort,
            deadline_us: 0,
            period_us: 0,
            budget_us: 0,
            remaining_budget: 0,
            next_deadline: 0,
            last_replenish: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct ProcessContext {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cs: u64,
    pub ss: u64,
}

impl Default for ProcessContext {
    fn default() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: 0, rflags: 0x202, // Enable interrupts
            cs: 0x08, // Kernel code segment
            ss: 0x10, // Kernel data segment
        }
    }
}

impl ProcessContext {
    pub fn new_user_context(entry_point: VirtAddr, user_stack: VirtAddr) -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: user_stack.as_u64(),
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: entry_point.as_u64(),
            rflags: 0x202, // Enable interrupts
            cs: crate::gdt::get_user_code_selector().0 as u64,
            ss: crate::gdt::get_user_data_selector().0 as u64,
        }
    }
    
    pub fn new_kernel_context(entry_point: VirtAddr, kernel_stack: VirtAddr) -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0, rsp: kernel_stack.as_u64(),
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            rip: entry_point.as_u64(),
            rflags: 0x202, // Enable interrupts
            cs: crate::gdt::get_kernel_code_selector().0 as u64,
            ss: crate::gdt::get_kernel_data_selector().0 as u64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u64,
    pub parent_pid: Option<u64>,
    pub state: ProcessState,
    pub priority: Priority,
    pub context: ProcessContext,
    pub page_table: Option<PhysAddr>,
    pub address_space_id: Option<u64>,  // Address space ID for this process
    pub stack_base: VirtAddr,
    pub stack_size: usize,
    pub heap_base: VirtAddr,
    pub heap_size: usize,
    pub name: alloc::string::String,
    pub cpu_time: u64,
    pub memory_usage: usize,
    pub open_files: Vec<u64>, // File descriptor IDs
    pub permissions: ProcessPermissions,
    pub pending_signals: u64, // Bitmask of pending signals
    pub signal_handlers: [Option<SignalHandler>; 32], // Signal handler table
    // Keep kernel stack backing alive for kernel threads
    pub kernel_stack_ptr: Option<usize>, // Store as usize to make it Send
    pub cpu_affinity: CpuAffinity, // CPU affinity for SMP scheduling
    pub rt_params: RtParams, // Real-time scheduling parameters
}

#[derive(Debug, Clone)]
pub struct ProcessPermissions {
    pub can_access_network: bool,
    pub can_access_filesystem: bool,
    pub can_access_hardware: bool,
    pub can_create_processes: bool,
    pub sandbox_level: SandboxLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxLevel {
    None,
    Basic,
    Strict,
    Isolated,
}

impl Default for ProcessPermissions {
    fn default() -> Self {
        Self {
            can_access_network: false,
            can_access_filesystem: false,
            can_access_hardware: false,
            can_create_processes: false,
            sandbox_level: SandboxLevel::Basic,
        }
    }
}

impl Process {
    pub fn new(name: alloc::string::String, entry_point: VirtAddr, priority: Priority) -> Self {
        let pid = NEXT_PID.fetch_add(1, Ordering::SeqCst);
        let stack_size = 0x10000; // 64KB stack
        let heap_size = 0x100000; // 1MB heap
        
        let mut context = ProcessContext::default();
        context.rip = entry_point.as_u64();
        context.rsp = 0x7FFFFFFF0000; // User stack top
        
        // Create a new address space for this process
        let address_space_id = crate::vmm::create_address_space();
        
        Self {
            pid,
            parent_pid: None,
            state: ProcessState::Ready,
            priority,
            context,
            page_table: None,
            address_space_id: Some(address_space_id),
            stack_base: VirtAddr::new(0x7FFFFFFF0000 - stack_size as u64),
            stack_size,
            heap_base: VirtAddr::new(0x400000), // 4MB base
            heap_size,
            name,
            cpu_time: 0,
            memory_usage: 0,
            open_files: Vec::new(),
            permissions: ProcessPermissions::default(),
            pending_signals: 0,
            signal_handlers: [None; 32],
            kernel_stack_ptr: None,
            cpu_affinity: CpuAffinity::Any,
            rt_params: RtParams::default(),
        }
    }
    
    pub fn kernel_process(name: alloc::string::String, entry_point: VirtAddr) -> Self {
        let mut process = Self::new(name, entry_point, Priority::High);
        process.permissions = ProcessPermissions {
            can_access_network: true,
            can_access_filesystem: true,
            can_access_hardware: true,
            can_create_processes: true,
            sandbox_level: SandboxLevel::None,
        };
        // Kernel processes share the kernel address space (no separate AS needed)
        process.address_space_id = None;
        process
    }

    pub fn with_kernel_stack(mut self, stack_ptr: *mut u8, stack_size: usize) -> Self {
        self.kernel_stack_ptr = Some(stack_ptr as usize);
        self.stack_base = VirtAddr::new(stack_ptr as u64);
        self.stack_size = stack_size;
        self.context.rsp = (stack_ptr as u64) + stack_size as u64;
        self
    }
    
    pub fn user_process(name: alloc::string::String, entry_point: VirtAddr) -> Result<Self, crate::vmm::VmError> {
        let pid = NEXT_PID.fetch_add(1, Ordering::SeqCst);
        let priority = Priority::Normal;
        let stack_size = 0x10000; // 64KB user stack
        let heap_size = 0x100000; // 1MB heap
        
        // Create a new address space for this process
        let address_space_id = crate::vmm::create_address_space();
        
        // Allocate user stack in the new address space
        let user_stack_top = VirtAddr::new(0x7FFFFFFF0000);
        let user_stack_base = user_stack_top - stack_size as u64;
        
        // Set up user context with Ring3 segments
        let context = ProcessContext::new_user_context(entry_point, user_stack_top);
        
        // Allocate and map user stack
        crate::vmm::with_vmm(|vmm| {
            if let Some(address_space) = vmm.get_address_space_mut(address_space_id) {
                let stack_area = crate::vmm::VmArea::new(
                    user_stack_base,
                    user_stack_top,
                    crate::vmm::VmAreaType::Stack,
                    crate::vmm::VmPermissions::Read | crate::vmm::VmPermissions::Write | crate::vmm::VmPermissions::User,
                );
                address_space.add_area(stack_area)?;
            }
            Ok(())
        })?;
        
        Ok(Self {
            pid,
            parent_pid: None,
            state: ProcessState::Ready,
            priority,
            context,
            page_table: None,
            address_space_id: Some(address_space_id),
            stack_base: user_stack_base,
            stack_size,
            heap_base: VirtAddr::new(0x400000), // 4MB base
            heap_size,
            name,
            cpu_time: 0,
            memory_usage: 0,
            open_files: Vec::new(),
            pending_signals: 0,
            signal_handlers: [None; 32],
            permissions: ProcessPermissions {
                can_access_network: false,
                can_access_filesystem: true,
                can_access_hardware: false,
                can_create_processes: false,
                sandbox_level: SandboxLevel::Strict,
            },
            kernel_stack_ptr: None,
            cpu_affinity: CpuAffinity::Any,
            rt_params: RtParams::default(),
        })
     }
}

/// CPU affinity mask for processes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CpuAffinity {
    mask: u64, // Bitmask of allowed CPUs (up to 64 CPUs)
}

impl CpuAffinity {
    pub const ANY: Self = Self { mask: u64::MAX };
    
    pub fn new(mask: u64) -> Self {
        Self { mask }
    }
    
    pub fn all_cpus() -> Self {
        Self { mask: u64::MAX }
    }
    
    pub fn single_cpu(cpu_id: u32) -> Self {
        Self { mask: 1u64 << cpu_id }
    }
    
    pub fn can_run_on(&self, cpu_id: u32) -> bool {
        (self.mask & (1u64 << cpu_id)) != 0
    }
    
    pub fn set_cpu(&mut self, cpu_id: u32, allowed: bool) {
        if allowed {
            self.mask |= 1u64 << cpu_id;
        } else {
            self.mask &= !(1u64 << cpu_id);
        }
    }
    
    pub fn first_allowed_cpu(&self) -> Option<u32> {
        if self.mask == 0 {
            return None;
        }
        Some(self.mask.trailing_zeros())
    }
}

/// Per-CPU scheduler data
pub struct CpuScheduler {
    cpu_id: u32,
    ready_queues: [VecDeque<u64>; 4], // One queue per priority level
    rt_edf_queue: VecDeque<u64>,      // EDF real-time queue (sorted by deadline)
    rt_cbs_queue: VecDeque<u64>,      // CBS real-time queue
    current_process: Option<u64>,
    time_slice: u64,
    current_time_slice_remaining: u64,
    idle_thread_pid: Option<u64>,
    load: AtomicU32, // Current load (number of ready processes)
    last_balance_time: u64,
    rt_isolated: bool, // Whether this CPU is isolated for RT tasks
}

impl CpuScheduler {
    pub const fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            ready_queues: [
                VecDeque::new(), VecDeque::new(),
                VecDeque::new(), VecDeque::new()
            ],
            rt_edf_queue: VecDeque::new(),
            rt_cbs_queue: VecDeque::new(),
            current_process: None,
            time_slice: 10, // 10ms default
            current_time_slice_remaining: 10,
            idle_thread_pid: None,
            load: AtomicU32::new(0),
            last_balance_time: 0,
            rt_isolated: false,
        }
    }
    
    pub fn add_process(&mut self, pid: u64, priority: Priority) {
        let priority_idx = priority as usize;
        self.ready_queues[priority_idx].push_back(pid);
        self.load.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Add a real-time process to the appropriate RT queue
    pub fn add_rt_process(&mut self, pid: u64, rt_class: RtClass, processes: &[Option<Process>]) {
        match rt_class {
            RtClass::Edf => {
                // Insert in deadline order (EDF)
                if let Some(process) = processes.get(pid as usize).and_then(|p| p.as_ref()) {
                    let deadline = process.rt_params.next_deadline;
                    let mut inserted = false;
                    
                    for (i, &existing_pid) in self.rt_edf_queue.iter().enumerate() {
                        if let Some(existing_process) = processes.get(existing_pid as usize).and_then(|p| p.as_ref()) {
                            if deadline < existing_process.rt_params.next_deadline {
                                self.rt_edf_queue.insert(i, pid);
                                inserted = true;
                                break;
                            }
                        }
                    }
                    
                    if !inserted {
                        self.rt_edf_queue.push_back(pid);
                    }
                }
            },
            RtClass::Cbs => {
                self.rt_cbs_queue.push_back(pid);
            },
            RtClass::BestEffort => {
                // Fall back to normal priority scheduling
                if let Some(process) = processes.get(pid as usize).and_then(|p| p.as_ref()) {
                    self.add_process(pid, process.priority);
                }
            }
        }
        self.load.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn remove_process(&mut self, pid: u64) {
        let mut found = false;
        
        // Remove from regular queues
        for queue in &mut self.ready_queues {
            if let Some(pos) = queue.iter().position(|&p| p == pid) {
                queue.remove(pos);
                self.load.fetch_sub(1, Ordering::Relaxed);
                found = true;
                break;
            }
        }
        
        // Remove from RT queues if not found in regular queues
        if !found {
            if let Some(pos) = self.rt_edf_queue.iter().position(|&p| p == pid) {
                self.rt_edf_queue.remove(pos);
                self.load.fetch_sub(1, Ordering::Relaxed);
                found = true;
            } else if let Some(pos) = self.rt_cbs_queue.iter().position(|&p| p == pid) {
                self.rt_cbs_queue.remove(pos);
                self.load.fetch_sub(1, Ordering::Relaxed);
                found = true;
            }
        }
        
        if self.current_process == Some(pid) {
            self.current_process = None;
        }
    }
    
    pub fn schedule(&mut self, gaming_mode: bool) -> Option<u64> {
        let current_time = crate::time::get_uptime_ms() * 1000; // Convert to microseconds
        
        // 1. Real-time EDF scheduling (highest priority)
        if let Some(pid) = self.rt_edf_queue.front().copied() {
            self.rt_edf_queue.pop_front();
            self.current_process = Some(pid);
            self.current_time_slice_remaining = 1; // Short slice for RT tasks
            return Some(pid);
        }
        
        // 2. Real-time CBS scheduling
        if let Some(pid) = self.rt_cbs_queue.pop_front() {
            // Re-add to end for round-robin within CBS
            self.rt_cbs_queue.push_back(pid);
            self.current_process = Some(pid);
            self.current_time_slice_remaining = 2; // Slightly longer for CBS
            return Some(pid);
        }
        
        // 3. Gaming mode prioritization (if no RT tasks)
        if gaming_mode {
            if let Some(pid) = self.ready_queues[Priority::Gaming as usize].pop_front() {
                self.current_process = Some(pid);
                self.current_time_slice_remaining = self.time_slice;
                return Some(pid);
            }
        }
        
        // 4. Regular priority-based round-robin scheduling
        for (_priority, queue) in self.ready_queues.iter_mut().enumerate() {
            if let Some(pid) = queue.pop_front() {
                // Re-add to end of queue for round-robin
                queue.push_back(pid);
                self.current_process = Some(pid);
                self.current_time_slice_remaining = self.time_slice;
                self.load.fetch_sub(1, Ordering::Relaxed);
                return Some(pid);
            }
        }
        
        // 5. If no processes are ready, run idle thread
        if let Some(idle_pid) = self.idle_thread_pid {
            self.current_process = Some(idle_pid);
            self.current_time_slice_remaining = self.time_slice;
            return Some(idle_pid);
        }
        
        None
    }
    
    /// Update RT process deadlines and budgets
    pub fn update_rt_timing(&mut self, processes: &mut [Option<Process>]) {
        let current_time = crate::time::get_uptime_ms() * 1000; // Convert to microseconds
        
        // Update EDF processes
        let mut expired_pids = Vec::new();
        for &pid in &self.rt_edf_queue {
            if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
                // Check for deadline miss
                if current_time > process.rt_params.next_deadline {
                    expired_pids.push(pid);
                    // Update to next period
                    process.rt_params.next_deadline += process.rt_params.period_us;
                    process.rt_params.remaining_budget = process.rt_params.budget_us;
                }
            }
        }
        
        // Remove expired processes and re-add them (they'll be re-sorted by deadline)
        for pid in expired_pids {
            self.remove_process(pid);
            if let Some(process) = processes.get(pid as usize).and_then(|p| p.as_ref()) {
                self.add_rt_process(pid, process.rt_params.class, processes);
            }
        }
        
        // Update CBS processes
        for &pid in &self.rt_cbs_queue {
            if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
                // Replenish budget if period elapsed
                if current_time >= process.rt_params.next_deadline {
                    process.rt_params.remaining_budget = process.rt_params.budget_us;
                    process.rt_params.next_deadline += process.rt_params.period_us;
                }
            }
        }
    }
    
    /// Check if a process has remaining RT budget
    pub fn has_rt_budget(&self, pid: u64, processes: &[Option<Process>]) -> bool {
        if let Some(process) = processes.get(pid as usize).and_then(|p| p.as_ref()) {
            process.rt_params.remaining_budget > 0
        } else {
            false
        }
    }
    
    /// Consume RT budget for a process
    pub fn consume_rt_budget(&self, pid: u64, amount: u64, processes: &mut [Option<Process>]) {
        if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            process.rt_params.remaining_budget = process.rt_params.remaining_budget.saturating_sub(amount);
        }
    }
    
    pub fn get_load(&self) -> u32 {
        self.load.load(Ordering::Relaxed)
    }
    
    pub fn tick_time_slice(&mut self) -> bool {
        if self.current_time_slice_remaining > 0 {
            self.current_time_slice_remaining -= 1;
        }
        self.current_time_slice_remaining == 0
    }
    
    pub fn set_idle_thread(&mut self, pid: u64) {
        self.idle_thread_pid = Some(pid);
    }
    
    pub fn set_gaming_mode(&mut self, enabled: bool) {
        if enabled {
            self.time_slice = 5; // Shorter time slices for gaming
        } else {
            self.time_slice = 10;
        }
        self.current_time_slice_remaining = self.time_slice;
    }
    
    pub fn yield_current(&mut self) {
        if let Some(pid) = self.current_process {
            // Add current process back to ready queue
            // We'll assume it was running at priority 1 (normal)
            self.ready_queues[1].push_back(pid);
            self.current_process = None;
        }
    }
    
    pub fn block_current(&mut self) {
        if let Some(pid) = self.current_process {
            // Remove from ready queues
            for queue in &mut self.ready_queues {
                queue.retain(|&p| p != pid);
            }
            self.current_process = None;
        }
    }
}

/// Global SMP-aware scheduler
pub struct SmpScheduler {
    cpu_schedulers: Vec<Mutex<CpuScheduler>>,
    processes: Vec<Option<Process>>,
    gaming_mode: bool,
    num_cpus: u32,
    current_cpu: AtomicU32,
}

impl SmpScheduler {
    pub fn new() -> Self {
        let num_cpus = get_cpu_count();
        let mut cpu_schedulers = Vec::with_capacity(num_cpus as usize);
        
        for cpu_id in 0..num_cpus {
            cpu_schedulers.push(Mutex::new(CpuScheduler::new(cpu_id)));
        }
        
        Self {
            cpu_schedulers,
            processes: Vec::new(),
            gaming_mode: false,
            num_cpus,
            current_cpu: AtomicU32::new(0),
        }
    }
    
    pub fn add_process(&mut self, mut process: Process) -> u64 {
        let pid = process.pid;
        
        // Extend processes vector if needed
        while self.processes.len() <= pid as usize {
            self.processes.push(None);
        }
        
        // Set default CPU affinity if not set
        if process.cpu_affinity.mask == 0 {
            process.cpu_affinity = CpuAffinity::all_cpus();
        }
        
        // Find the least loaded CPU that can run this process
        let target_cpu = self.find_best_cpu_for_process(&process);
        
        self.processes[pid as usize] = Some(process.clone());
        
        if let Some(cpu_id) = target_cpu {
            self.cpu_schedulers[cpu_id as usize].lock().add_process(pid, process.priority);
        }
        
        pid
    }
    
    pub fn remove_process(&mut self, pid: u64) {
        if let Some(process) = self.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            process.state = ProcessState::Terminated;
            
            // Remove from all CPU schedulers
            for cpu_scheduler in &self.cpu_schedulers {
                cpu_scheduler.lock().remove_process(pid);
            }
        }
    }
    
    pub fn schedule_on_cpu(&self, cpu_id: u32) -> Option<u64> {
        if cpu_id >= self.num_cpus {
            return None;
        }
        
        self.cpu_schedulers[cpu_id as usize].lock().schedule(self.gaming_mode)
    }
    
    pub fn find_best_cpu_for_process(&self, process: &Process) -> Option<u32> {
        let mut best_cpu = None;
        let mut min_load = u32::MAX;
        
        for cpu_id in 0..self.num_cpus {
            if process.cpu_affinity.can_run_on(cpu_id) {
                let load = self.cpu_schedulers[cpu_id as usize].lock().get_load();
                if load < min_load {
                    min_load = load;
                    best_cpu = Some(cpu_id);
                }
            }
        }
        
        best_cpu
    }
    
    pub fn balance_load(&mut self) {
        // Simple load balancing: move processes from heavily loaded CPUs to lightly loaded ones
        let mut loads: Vec<(u32, u32)> = Vec::new(); // (cpu_id, load)
        
        for cpu_id in 0..self.num_cpus {
            let load = self.cpu_schedulers[cpu_id as usize].lock().get_load();
            loads.push((cpu_id, load));
        }
        
        loads.sort_by_key(|&(_, load)| load);
        
        // If the difference between max and min load is significant, balance
        if loads.len() >= 2 {
            let min_load = loads[0].1;
            let max_load = loads[loads.len() - 1].1;
            
            if max_load > min_load + 2 {
                // Move one process from the most loaded CPU to the least loaded CPU
                let src_cpu = loads[loads.len() - 1].0;
                let dst_cpu = loads[0].0;
                
                // This is a simplified implementation - in practice, we'd need more
                // sophisticated logic to migrate processes safely
                // For now, we just note that load balancing is needed
            }
        }
    }
    
    pub fn set_gaming_mode(&mut self, enabled: bool) {
        self.gaming_mode = enabled;
        for cpu_scheduler in &self.cpu_schedulers {
            cpu_scheduler.lock().set_gaming_mode(enabled);
        }
    }
    
    pub fn get_current_process(&self, cpu_id: u32) -> Option<&Process> {
        if cpu_id >= self.num_cpus {
            return None;
        }
        
        let scheduler = self.cpu_schedulers[cpu_id as usize].lock();
        scheduler.current_process
            .and_then(|pid| self.processes.get(pid as usize))
            .and_then(|p| p.as_ref())
    }
    
    pub fn set_process_affinity(&mut self, pid: u64, affinity: CpuAffinity) -> bool {
        if let Some(process) = self.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            let old_affinity = process.cpu_affinity;
            process.cpu_affinity = affinity;
            
            // If the process can no longer run on its current CPU, migrate it
            // This is a simplified implementation
            true
        } else {
            false
        }
    }
    
    pub fn block_current_on_cpu(&mut self, cpu_id: u32) {
        if cpu_id >= self.num_cpus {
            return;
        }
        
        self.cpu_schedulers[cpu_id as usize].lock().deref_mut().block_current();
    }
    
    pub fn yield_current_on_cpu(&mut self, cpu_id: u32) {
        if cpu_id >= self.num_cpus {
            return;
        }
        
        self.cpu_schedulers[cpu_id as usize].lock().deref_mut().yield_current();
    }
    
    pub fn get_current_process_id(&self, cpu_id: u32) -> Option<u64> {
        if cpu_id >= self.num_cpus {
            return None;
        }
        
        self.cpu_schedulers[cpu_id as usize].lock().current_process
    }
    
    pub fn tick_time_slice_on_cpu(&mut self, cpu_id: u32) -> bool {
        if cpu_id >= self.num_cpus {
            return false;
        }
        
        self.cpu_schedulers[cpu_id as usize].lock().tick_time_slice()
    }
    
    pub fn unblock_process(&mut self, pid: u64) {
        // First, check if the process exists and is blocked, and get its priority
        let (should_unblock, priority) = if let Some(process) = self.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            if process.state == ProcessState::Blocked {
                process.state = ProcessState::Ready;
                (true, process.priority)
            } else {
                (false, Priority::Normal)
            }
        } else {
            (false, Priority::Normal)
        };
        
        if should_unblock {
            // Now find the best CPU for this process (without borrowing self.processes)
            if let Some(process) = self.processes.get(pid as usize).and_then(|p| p.as_ref()) {
                if let Some(cpu_id) = self.find_best_cpu_for_process(process) {
                    self.cpu_schedulers[cpu_id as usize].lock().add_process(pid, priority);
                }
            }
        }
    }
    
    pub fn set_idle_thread(&mut self, pid: u64) {
        // Set idle thread for all CPUs
        for cpu_scheduler in &self.cpu_schedulers {
            cpu_scheduler.lock().set_idle_thread(pid);
        }
    }
    
    /// Add a real-time process to the best available CPU
    pub fn add_rt_process(&mut self, pid: u64, rt_class: RtClass) {
        if let Some(process) = self.processes.get(pid as usize).and_then(|p| p.as_ref()) {
            let best_cpu = self.find_best_cpu_for_process(process).unwrap_or(0);
            
            if let Some(cpu_scheduler) = self.cpu_schedulers.get(best_cpu as usize) {
                cpu_scheduler.lock().add_rt_process(pid, rt_class, &self.processes);
            }
        }
    }
    
    /// Update RT timing for all CPUs
    pub fn update_rt_timing(&mut self) {
        for cpu_scheduler in &self.cpu_schedulers {
            cpu_scheduler.lock().update_rt_timing(&mut self.processes);
        }
    }
    
    /// Set CPU isolation for real-time tasks
    pub fn set_rt_isolation(&mut self, cpu_id: u32, isolated: bool) {
        if let Some(cpu_scheduler) = self.cpu_schedulers.get(cpu_id as usize) {
            cpu_scheduler.lock().rt_isolated = isolated;
        }
    }
    
    /// Get RT isolation status for a CPU
    pub fn is_rt_isolated(&self, cpu_id: u32) -> bool {
        if let Some(cpu_scheduler) = self.cpu_schedulers.get(cpu_id as usize) {
            cpu_scheduler.lock().rt_isolated
        } else {
            false
        }
    }
}

/// Legacy single-CPU scheduler for compatibility
pub struct Scheduler {
    ready_queues: [VecDeque<u64>; 4], // One queue per priority level
    processes: Vec<Option<Process>>,
    current_process: Option<u64>,
    time_slice: u64,
    gaming_mode: bool,
    current_time_slice_remaining: u64,
    idle_thread_pid: Option<u64>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            ready_queues: [
                VecDeque::new(), VecDeque::new(),
                VecDeque::new(), VecDeque::new()
            ],
            processes: Vec::new(),
            current_process: None,
            time_slice: 10, // 10ms default
            gaming_mode: false,
            current_time_slice_remaining: 10,
            idle_thread_pid: None,
        }
    }
    
    pub fn add_process(&mut self, process: Process) -> u64 {
        let pid = process.pid;
        let priority = process.priority as usize;
        
        // Extend processes vector if needed
        while self.processes.len() <= pid as usize {
            self.processes.push(None);
        }
        
        self.processes[pid as usize] = Some(process);
        self.ready_queues[priority].push_back(pid);
        pid
    }
    
    pub fn remove_process(&mut self, pid: u64) {
        if let Some(process) = self.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            process.state = ProcessState::Terminated;
            
            // Remove from ready queues
            for queue in &mut self.ready_queues {
                queue.retain(|&p| p != pid);
            }
            
            if self.current_process == Some(pid) {
                self.current_process = None;
            }
        }
    }
    
    pub fn schedule(&mut self) -> Option<u64> {
        // Gaming mode prioritization
        if self.gaming_mode {
            if let Some(pid) = self.ready_queues[Priority::Gaming as usize].pop_front() {
                self.current_process = Some(pid);
                self.current_time_slice_remaining = self.time_slice;
                return Some(pid);
            }
        }
        
        // Round-robin within priority levels
        for (_priority, queue) in self.ready_queues.iter_mut().enumerate() {
            if let Some(pid) = queue.pop_front() {
                // Re-add to end of queue for round-robin
                queue.push_back(pid);
                self.current_process = Some(pid);
                self.current_time_slice_remaining = self.time_slice;
                return Some(pid);
            }
        }
        
        // If no processes are ready, run idle thread
        if let Some(idle_pid) = self.idle_thread_pid {
            self.current_process = Some(idle_pid);
            self.current_time_slice_remaining = self.time_slice;
            return Some(idle_pid);
        }
        
        None
    }
    
    pub fn get_current_process(&self) -> Option<&Process> {
        self.current_process
            .and_then(|pid| self.processes.get(pid as usize))
            .and_then(|p| p.as_ref())
    }
    
    pub fn get_current_process_mut(&mut self) -> Option<&mut Process> {
        self.current_process
            .and_then(|pid| self.processes.get_mut(pid as usize))
            .and_then(|p| p.as_mut())
    }
    
    pub fn set_gaming_mode(&mut self, enabled: bool) {
        self.gaming_mode = enabled;
        if enabled {
            self.time_slice = 5; // Shorter time slices for gaming
        } else {
            self.time_slice = 10;
        }
        self.current_time_slice_remaining = self.time_slice;
    }
    
    pub fn set_idle_thread(&mut self, pid: u64) {
        self.idle_thread_pid = Some(pid);
    }
    
    pub fn tick_time_slice(&mut self) -> bool {
        if self.current_time_slice_remaining > 0 {
            self.current_time_slice_remaining -= 1;
        }
        self.current_time_slice_remaining == 0
    }
    
    pub fn yield_current(&mut self) {
        if let Some(pid) = self.current_process {
            // Add current process back to ready queue
            // We'll assume it was running at priority 1 (normal)
            self.ready_queues[1].push_back(pid);
            self.current_process = None;
        }
    }
    
    pub fn block_current(&mut self) {
        if let Some(pid) = self.current_process {
            // Remove from ready queues
            for queue in &mut self.ready_queues {
                queue.retain(|&p| p != pid);
            }
            self.current_process = None;
        }
    }
    
    pub fn unblock_process(&mut self, pid: u64) {
        // Add process to ready queue at normal priority
        self.ready_queues[1].push_back(pid);
    }
}

// Helper function to get the SMP scheduler instance
fn get_smp_scheduler() -> &'static Mutex<SmpScheduler> {
    SMP_SCHEDULER.call_once(|| Mutex::new(SmpScheduler::new()))
}

// Public API functions
pub fn init() {
    // Initialize the scheduler
    let _ = get_smp_scheduler();
}

pub fn create_process(name: alloc::string::String, entry_point: VirtAddr, priority: Priority) -> u64 {
    let process = Process::new(name, entry_point, priority);
    get_smp_scheduler().lock().add_process(process)
}

pub fn create_kernel_process(name: alloc::string::String, entry_point: VirtAddr) -> u64 {
    let process = Process::kernel_process(name, entry_point);
    get_smp_scheduler().lock().add_process(process)
}

pub fn spawn_kernel_thread(name: &str, entry: extern "C" fn() -> !) -> u64 {
    let stack_size: usize = 64 * 1024;
    let mut stack = alloc::vec::Vec::<u8>::with_capacity(stack_size);
    unsafe { stack.set_len(stack_size); }
    let stack_ptr = stack.as_mut_ptr();
    core::mem::forget(stack); // leak to keep alive; tracked by Process

    let mut ctx = ProcessContext::default();
    ctx.rip = entry as usize as u64;
    ctx.rsp = (stack_ptr as u64) + stack_size as u64;

    let mut proc = Process::kernel_process(alloc::string::String::from(name), VirtAddr::new(ctx.rip));
    proc.context = ctx;
    proc = proc.with_kernel_stack(stack_ptr, stack_size);
    get_smp_scheduler().lock().add_process(proc)
}

pub fn schedule() -> Option<u64> {
    let cpu_id = get_current_cpu_id();
    get_smp_scheduler().lock().schedule_on_cpu(cpu_id)
}

pub fn yield_current() {
    let cpu_id = get_current_cpu_id();
    get_smp_scheduler().lock().yield_current_on_cpu(cpu_id);
}

pub fn block_current() {
    let cpu_id = get_current_cpu_id();
    get_smp_scheduler().lock().block_current_on_cpu(cpu_id);
}

pub fn unblock_process(pid: u64) {
    get_smp_scheduler().lock().unblock_process(pid);
}

pub fn terminate_process(pid: u64) {
    // Perform comprehensive cleanup before removing process
    cleanup_process_resources(pid as u32);
    get_smp_scheduler().lock().remove_process(pid);
}

/// Comprehensive cleanup of all process resources
fn cleanup_process_resources(process_id: u32) {
    // Clean up security context
    crate::security::cleanup_process_security(process_id);
    
    // Clean up IPC resources
    crate::ipc::cleanup_process_ipc(process_id);
    
    // Clean up network resources
    crate::network::cleanup_process_network(process_id);
    
    // Clean up RaeKit applications
    crate::raekit::cleanup_process_raekit(process_id);
    
    // Clean up assistant sessions
    crate::rae_assistant::cleanup_process_assistant(process_id);
    
    // Clean up shell sessions
    crate::raeshell::cleanup_process_shell(process_id);
    
    // Clean up RaeDE sessions
    crate::raede::cleanup_process_raede(process_id);
    
    // Clean up address space if it exists
    let mut scheduler = get_smp_scheduler().lock();
    if let Some(process) = scheduler.processes.get(process_id as usize).and_then(|p| p.as_ref()) {
        if let Some(address_space_id) = process.address_space_id {
            drop(scheduler); // Release lock before VMM operations
            let _ = crate::vmm::destroy_address_space(address_space_id);
        }
    }
}

// Idle thread function - runs when no other processes are ready
extern "C" fn idle_thread_main() -> ! {
    loop {
        // Halt CPU until next interrupt to save power
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

// Initialize idle thread - should be called during kernel initialization
pub fn init_idle_thread() -> u64 {
    let idle_pid = spawn_kernel_thread("idle", idle_thread_main);
    get_smp_scheduler().lock().set_idle_thread(idle_pid);
    IDLE_THREAD_PID.store(idle_pid, Ordering::SeqCst);
    idle_pid
}

// Demo kernel thread for testing
extern "C" fn demo_kernel_thread() -> ! {
    let mut counter = 0u64;
    loop {
        counter += 1;
        if counter % 1000000 == 0 {
            crate::serial_println!("Demo kernel thread tick: {}", counter / 1000000);
        }
        
        // Yield occasionally to test preemption
        if counter % 500000 == 0 {
            yield_current();
        }
    }
}

// Spawn demo kernel thread for testing
pub fn spawn_demo_thread() -> u64 {
    spawn_kernel_thread("demo", demo_kernel_thread)
}

pub fn set_gaming_mode(enabled: bool) {
    get_smp_scheduler().lock().set_gaming_mode(enabled);
}

pub fn get_current_process_info() -> Option<(u64, alloc::string::String, ProcessState)> {
    let cpu_id = get_current_cpu_id();
    let scheduler = get_smp_scheduler().lock();
    scheduler.get_current_process(cpu_id).map(|p| (p.pid, p.name.clone(), p.state))
}

pub fn get_current_process_id() -> u64 {
    let cpu_id = get_current_cpu_id();
    let scheduler = get_smp_scheduler().lock();
    scheduler.get_current_process_id(cpu_id).unwrap_or(0)
}



/// Switch between process contexts using inline assembly
pub fn switch_context(old_context: *mut ProcessContext, new_context: *const ProcessContext) {
    unsafe {
    core::arch::asm!(
        // Check if old_context is null
        "test {old}, {old}",
        "jz 2f",
        
        // Save current context
        "mov [{old} + 0], rax",
        "mov [{old} + 8], rbx",
        "mov [{old} + 16], rcx",
        "mov [{old} + 24], rdx",
        "mov [{old} + 32], rsi",
        "mov [{old} + 40], rdi",
        "mov [{old} + 48], rbp",
        "mov [{old} + 56], rsp",
        "mov [{old} + 64], r8",
        "mov [{old} + 72], r9",
        "mov [{old} + 80], r10",
        "mov [{old} + 88], r11",
        "mov [{old} + 96], r12",
        "mov [{old} + 104], r13",
        "mov [{old} + 112], r14",
        "mov [{old} + 120], r15",
        
        // Save return address as RIP
        "lea rax, [rip + 8]",
        "mov [{old} + 136], rax",
        
        // Save RFLAGS
        "pushfq",
        "pop rax",
        "mov [{old} + 144], rax",
        
        // Load new context
        "2:",
        "mov rax, [{new} + 0]",
        "mov rbx, [{new} + 8]",
        "mov rcx, [{new} + 16]",
        "mov rdx, [{new} + 24]",
        "mov rbp, [{new} + 48]",
        "mov rsp, [{new} + 56]",
        "mov r8, [{new} + 64]",
        "mov r9, [{new} + 72]",
        "mov r10, [{new} + 80]",
        "mov r11, [{new} + 88]",
        "mov r12, [{new} + 96]",
        "mov r13, [{new} + 104]",
        "mov r14, [{new} + 112]",
        "mov r15, [{new} + 120]",
        
        // Load new RFLAGS
        "push qword ptr [{new} + 144]",
        "popfq",
        
        // Load new RSI and RDI last
        "mov rsi, [{new} + 32]",
        "mov rdi, [{new} + 40]",
        
        // Jump to new RIP
        "jmp qword ptr [{new} + 136]",
        
        old = in(reg) old_context,
         new = in(reg) new_context,
         options(noreturn)
     );
    }
}

pub fn context_switch(old_pid: Option<u64>, new_pid: u64) {
    let mut scheduler = get_smp_scheduler().lock();
    
    // Save old context and get address space info
    let mut old_ctx_ptr: *mut ProcessContext = core::ptr::null_mut();
    let mut old_as_id: Option<u64> = None;
    if let Some(old_pid) = old_pid {
        if let Some(old_process) = scheduler.processes.get_mut(old_pid as usize).and_then(|p| p.as_mut()) {
            old_ctx_ptr = &mut old_process.context as *mut ProcessContext;
            old_as_id = old_process.address_space_id;
        }
    }
    
    // Load new context and switch address space if needed
    if let Some(new_process) = scheduler.processes.get_mut(new_pid as usize).and_then(|p| p.as_mut()) {
        new_process.state = ProcessState::Running;
        
        // Switch address space if the new process has a different one
        if let Some(new_as_id) = new_process.address_space_id {
            if old_as_id != Some(new_as_id) {
                drop(scheduler); // Release scheduler lock before VMM operations
                let _ = crate::vmm::switch_address_space(new_as_id);
                // Re-acquire scheduler lock
                scheduler = get_smp_scheduler().lock();
                // Re-get the process reference after re-acquiring lock
                if let Some(new_process) = scheduler.processes.get_mut(new_pid as usize).and_then(|p| p.as_mut()) {
                    // Context will be loaded by the assembly routine
                    unsafe {
                        switch_context(old_ctx_ptr, &new_process.context as *const ProcessContext);
                    }
                }
                return;
            }
        }
        
        // No address space switch needed, just switch context
        unsafe {
            switch_context(old_ctx_ptr, &new_process.context as *const ProcessContext);
        }
    }
}

pub fn schedule_tick() {
    // Process signals for current process first
    process_signals();
    
    let cpu_id = get_current_cpu_id();
    let mut smp_scheduler = get_smp_scheduler().lock();
    
    // Check if current process time slice expired on this CPU
    let time_slice_expired = smp_scheduler.tick_time_slice_on_cpu(cpu_id);
    let current = smp_scheduler.get_current_process_id(cpu_id);
    
    // Only preempt if time slice expired or current process is not running
    let should_schedule = if let Some(pid) = current {
        if let Some(proc_ref) = smp_scheduler.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            // Check if process was terminated by signal handling
            if proc_ref.state == ProcessState::Terminated {
                true
            } else if proc_ref.state == ProcessState::Running {
                if time_slice_expired {
                    // Time slice expired, yield current process
                    smp_scheduler.yield_current_on_cpu(cpu_id);
                    true
                } else {
                    // Time slice not expired, continue running current process
                    false
                }
            } else {
                // Process is not running (blocked/terminated), schedule next
                true
            }
        } else {
            // Invalid process, schedule next
            true
        }
    } else {
        // No current process, schedule next
        true
    };
    
    if should_schedule {
        if let Some(next_pid) = smp_scheduler.schedule_on_cpu(cpu_id) {
            drop(smp_scheduler);
            context_switch(current, next_pid);
        }
    }
}

// Process management functions for syscalls
pub fn fork_process() -> Result<ProcessId, ()> {
    let cpu_id = get_current_cpu_id();
    let mut scheduler = get_smp_scheduler().lock();
    let current_pid = scheduler.get_current_process_id(cpu_id).ok_or(())?;
    
    // Get the current process
    let parent_process = scheduler.processes.get(current_pid as usize)
        .and_then(|p| p.as_ref())
        .ok_or(())?;
    
    // Create a new process ID
    let child_pid = NEXT_PID.fetch_add(1, Ordering::SeqCst);
    
    // Clone the parent process (simplified - just create new with same properties)
    let mut child_process = Process::new(
        parent_process.name.clone(),
        VirtAddr::new(parent_process.context.rip),
        parent_process.priority
    );
    child_process.parent_pid = Some(current_pid);
    child_process.state = ProcessState::Ready;
    child_process.permissions = parent_process.permissions.clone();
    
    // Initialize security context for child process
    let _ = crate::security::init_process_security(child_pid as u32, Some(current_pid as u32));
    
    // Add the child process
    scheduler.add_process(child_process);
    
    Ok(child_pid)
}

pub fn exec_process(path: &str, args: &[&str]) -> Result<(), ()> {
    use alloc::string::String;
    use alloc::vec::Vec;
    
    let cpu_id = get_current_cpu_id();
    let mut scheduler = get_smp_scheduler().lock();
    let current_pid = scheduler.get_current_process_id(cpu_id).ok_or(())?;
    
    // Get the current process
    let process = scheduler.processes.get_mut(current_pid as usize)
        .and_then(|p| p.as_mut())
        .ok_or(())?;
    
    // Check if we have permission to execute files
    if !crate::security::request_permission(current_pid as u32, "file.execute").unwrap_or(false) {
        return Err(());
    }
    
    // Validate the executable path
    if !crate::security::check_path_access(current_pid as u32, path, "execute").unwrap_or(false) {
        return Err(());
    }
    
    // Try to load the executable from filesystem
    let file_data = crate::filesystem::read_file(path).map_err(|_| ())?;
    
    // Basic ELF header validation (simplified)
    if file_data.len() < 4 || &file_data[0..4] != b"\x7fELF" {
        return Err(()); // Not a valid ELF file
    }
    
    // Reset process memory (simplified - in reality would parse ELF and map sections)
    process.memory_usage = 0;
    process.cpu_time = 0;
    
    // Store command line arguments
    let mut argv: Vec<String> = Vec::new();
    argv.push(String::from(path));
    for arg in args {
        argv.push(String::from(*arg));
    }
    
    // In a real implementation, we would:
    // 1. Parse ELF headers and program headers
    // 2. Map executable sections into memory
    // 3. Set up initial stack with arguments
    // 4. Set instruction pointer to entry point
    // For now, we'll just mark the process as ready
    
    process.state = ProcessState::Ready;
    
    Ok(())
}

/// Transition to Ring3 userspace using iretq
pub fn transition_to_ring3(entry_point: VirtAddr, user_stack: VirtAddr) -> ! {
    // Set up the stack frame for iretq
    // iretq expects: SS, RSP, RFLAGS, CS, RIP on the stack
    let user_cs = crate::gdt::get_user_code_selector().0 as u64;
    let user_ss = crate::gdt::get_user_data_selector().0 as u64;
    let rflags = 0x202u64; // Enable interrupts
    
    let user_ds = crate::gdt::get_user_data_selector().0;
    
    unsafe {
        core::arch::asm!(
            // Set up data segments for userspace
            "mov ax, {user_ds:x}",
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            
            // Push iretq frame onto stack
            "push {user_ss}",      // SS
            "push {user_rsp}",     // RSP
            "push {rflags}",       // RFLAGS
            "push {user_cs}",      // CS
            "push {rip}",          // RIP
            
            // Clear registers for security
            "xor rax, rax",
            "xor rbx, rbx",
            "xor rcx, rcx",
            "xor rdx, rdx",
            "xor rsi, rsi",
            "xor rdi, rdi",
            "xor r8, r8",
            "xor r9, r9",
            "xor r10, r10",
            "xor r11, r11",
            "xor r12, r12",
            "xor r13, r13",
            "xor r14, r14",
            "xor r15, r15",
            
            // Transition to Ring3
            "iretq",
            
            user_ds = in(reg) user_ds,
            user_ss = in(reg) user_ss,
            user_rsp = in(reg) user_stack.as_u64(),
            rflags = in(reg) rflags,
            user_cs = in(reg) user_cs,
            rip = in(reg) entry_point.as_u64(),
            options(noreturn)
        );
    }
}

/// Create and start a user process
pub fn spawn_user_process(name: &str, entry_point: VirtAddr) -> Result<u32, crate::vmm::VmError> {
    let process = Process::user_process(name.to_string(), entry_point)?;
    let pid = process.pid;
    
    // Add to scheduler
    get_smp_scheduler().lock().add_process(process);
    
    Ok(pid as u32)
}

pub fn wait_for_process(pid: ProcessId) -> Result<i32, ()> {
    let cpu_id = get_current_cpu_id();
    let mut scheduler = get_smp_scheduler().lock();
    let current_pid = scheduler.get_current_process_id(cpu_id).ok_or(())?;
    
    // Check if the target process exists
    let target_exists = scheduler.processes.get(pid as usize)
        .and_then(|p| p.as_ref())
        .is_some();
    
    if !target_exists {
        return Err(()); // Process doesn't exist
    }
    
    // Check if current process is parent of target process
    let is_parent = scheduler.processes.get(pid as usize)
        .and_then(|p| p.as_ref())
        .map(|p| p.parent_pid == Some(current_pid))
        .unwrap_or(false);
    
    if !is_parent {
        return Err(()); // Can only wait for child processes
    }
    
    // Check if process is already terminated
    let target_state = scheduler.processes.get(pid as usize)
        .and_then(|p| p.as_ref())
        .map(|p| p.state)
        .unwrap_or(ProcessState::Terminated);
    
    match target_state {
        ProcessState::Terminated => {
            // Process already terminated, get the actual exit code
            let exit_code = scheduler.processes.get(pid as usize)
                .and_then(|p| p.as_ref())
                .map(|p| p.get_exit_code())
                .unwrap_or(0);
            
            // Clean up the terminated process
            if let Some(slot) = scheduler.processes.get_mut(pid as usize) {
                *slot = None;
            }
            drop(scheduler); // Release lock before cleanup
            crate::security::cleanup_process_security(pid as u32);
            
            Ok(exit_code)
        }
        _ => {
            // Process still running, block current process until it terminates
            if let Some(current_process) = scheduler.processes.get_mut(current_pid as usize)
                .and_then(|p| p.as_mut()) {
                current_process.state = ProcessState::Blocked;
            }
            
            // Remove current process from all CPU ready queues
            for cpu_scheduler in &scheduler.cpu_schedulers {
                cpu_scheduler.lock().remove_process(current_pid);
            }
            
            // In a real implementation, this would yield to scheduler
            // For now, return a placeholder exit code
            Ok(0)
        }
    }
}

pub fn get_process_count() -> u64 {
    let scheduler = get_smp_scheduler().lock();
    scheduler.processes.iter().filter(|p| p.is_some()).count() as u64
}

/// Add exit code support to Process struct
impl Process {
    pub fn set_exit_code(&mut self, code: i32) {
        // Store exit code in unused field or add new field
        // For now, we'll use a simple approach
        self.cpu_time = code as u64; // Repurpose this field temporarily
    }
    
    pub fn get_exit_code(&self) -> i32 {
        self.cpu_time as i32
    }
}

/// Send a signal to a process
pub fn send_signal(pid: ProcessId, signal: Signal) -> Result<(), &'static str> {
    let mut scheduler = get_smp_scheduler().lock();
    
    if let Some(process) = scheduler.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
        // Set the signal bit in pending_signals
        process.pending_signals |= 1 << (signal as u8);
        
        // If it's SIGKILL, force terminate immediately
        if signal == Signal::SIGKILL {
            process.state = ProcessState::Terminated;
            process.set_exit_code(-9); // SIGKILL exit code
        }
        
        Ok(())
    } else {
        Err("Process not found")
    }
}

/// Process pending signals for the current process
pub fn process_signals() {
    let current_pid = get_current_process_id();
    let mut scheduler = get_smp_scheduler().lock();
    
    if let Some(process) = scheduler.processes.get_mut(current_pid as usize).and_then(|p| p.as_mut()) {
        let pending = process.pending_signals;
        process.pending_signals = 0; // Clear pending signals
        
        // Release scheduler lock before handling signals
        let handlers = process.signal_handlers;
        drop(scheduler);
        
        // Process each pending signal
        for signal_num in 0..32 {
            if pending & (1 << signal_num) != 0 {
                if let Ok(signal) = Signal::try_from(signal_num) {
                    // Use custom handler if set, otherwise use default
                    if let Some(handler) = handlers[signal_num as usize] {
                        handler(signal);
                    } else {
                        default_signal_handler(signal);
                    }
                }
            }
        }
    }
}

/// Set a signal handler for the current process
pub fn set_signal_handler(signal: Signal, handler: Option<SignalHandler>) -> Result<(), &'static str> {
    let current_pid = get_current_process_id();
    let mut scheduler = get_smp_scheduler().lock();
    
    if let Some(process) = scheduler.processes.get_mut(current_pid as usize).and_then(|p| p.as_mut()) {
        process.signal_handlers[signal as usize] = handler;
        Ok(())
    } else {
        Err("Current process not found")
    }
}

/// Convert signal number to Signal enum
impl Signal {
    fn try_from(value: u8) -> Result<Self, &'static str> {
        match value {
            9 => Ok(Signal::SIGKILL),
            10 => Ok(Signal::SIGUSR1),
            12 => Ok(Signal::SIGUSR2),
            15 => Ok(Signal::SIGTERM),
            18 => Ok(Signal::SIGCONT),
            19 => Ok(Signal::SIGSTOP),
            _ => Err("Unknown signal"),
        }
    }
}

/// Enhanced process termination with exit code
pub fn exit_process(exit_code: i32) -> ! {
    let current_pid = get_current_process_id();
    
    // Set exit code before cleanup
    {
        let mut scheduler = get_smp_scheduler().lock();
        if let Some(process) = scheduler.processes.get_mut(current_pid as usize).and_then(|p| p.as_mut()) {
            process.set_exit_code(exit_code);
            process.state = ProcessState::Terminated;
        }
    }
    
    // Perform cleanup
    cleanup_process_resources(current_pid as u32);
    
    // Remove from scheduler
    get_smp_scheduler().lock().remove_process(current_pid);
    
    // Force context switch to next process
    if let Some(next_pid) = schedule() {
        context_switch(Some(current_pid), next_pid);
    }
    
    // Should never reach here
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}