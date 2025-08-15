use alloc::collections::VecDeque;
use alloc::vec::Vec;
use alloc::string::ToString;
use core::ops::DerefMut;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use spin::{Mutex, Once};
use x86_64::{VirtAddr, PhysAddr};
use crate::arch::{get_cpu_count, get_current_cpu_id};

static NEXT_PID: AtomicU64 = AtomicU64::new(1);
static SMP_SCHEDULER: Once<Mutex<SmpScheduler>> = Once::new();
static _LEGACY_SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
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
            let _current_pid = get_current_process_id();
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

/// Priority inheritance state for IPC operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityInheritanceState {
    None,
    Inherited { original_priority: Priority, inherited_from: u64 },
    Boosted { boost_level: u8 },
}

/// NUMA node information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumaNode {
    pub id: u8,
    pub cpu_mask: u64,
    pub memory_base: u64,
    pub memory_size: u64,
}

/// CBS (Constant Bandwidth Server) parameters
#[derive(Debug, Clone, Copy)]
pub struct CbsParams {
    pub server_budget_us: u64,
    pub server_period_us: u64,
    pub remaining_budget: u64,
    pub next_replenishment: u64,
    pub throttled: bool,
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
    pub cbs_params: Option<CbsParams>, // CBS server parameters
    pub priority_inheritance: PriorityInheritanceState, // Priority inheritance state
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
            cbs_params: None,
            priority_inheritance: PriorityInheritanceState::None,
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
    pub rt_params: RtParams,
    pub numa_node: Option<NumaNode>, // Real-time scheduling parameters
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
            cpu_affinity: CpuAffinity::ANY,
            rt_params: RtParams::default(),
            numa_node: None,
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
                    crate::vmm::VmPermissions::READ | crate::vmm::VmPermissions::WRITE | crate::vmm::VmPermissions::USER,
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
            cpu_affinity: CpuAffinity::ANY,
            rt_params: RtParams::default(),
            numa_node: None,
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
    _cpu_id: u32,
    ready_queues: [VecDeque<u64>; 4], // One queue per priority level
    rt_edf_queue: VecDeque<u64>,      // EDF real-time queue (sorted by deadline)
    rt_cbs_queue: VecDeque<u64>,      // CBS real-time queue
    current_process: Option<u64>,
    time_slice: u64,
    current_time_slice_remaining: u64,
    idle_thread_pid: Option<u64>,
    load: AtomicU32, // Current load (number of ready processes)
    _last_balance_time: u64,
    rt_isolated: bool, // Whether this CPU is isolated for RT tasks
    numa_node: Option<NumaNode>, // NUMA node this CPU belongs to
    cbs_budget_tracker: alloc::collections::BTreeMap<u64, u64>, // Track CBS budget usage
    priority_inheritance_chains: alloc::collections::BTreeMap<u64, Vec<u64>>, // PI chains
}

impl CpuScheduler {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            _cpu_id: cpu_id,
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
            _last_balance_time: 0,
            rt_isolated: false,
            numa_node: None,
            cbs_budget_tracker: alloc::collections::BTreeMap::new(),
            priority_inheritance_chains: alloc::collections::BTreeMap::new(),
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
            } else if let Some(pos) = self.rt_cbs_queue.iter().position(|&p| p == pid) {
                self.rt_cbs_queue.remove(pos);
                self.load.fetch_sub(1, Ordering::Relaxed);
            }
        }
        
        if self.current_process == Some(pid) {
            self.current_process = None;
        }
    }
    
    pub fn schedule(&mut self, gaming_mode: bool, processes: &[Option<Process>]) -> Option<u64> {
        let current_time = crate::time::get_uptime_ms() * 1000; // Convert to microseconds
        
        // 1. Real-time EDF scheduling (highest priority)
        // Find the process with the earliest deadline that has remaining budget
        let mut earliest_deadline = u64::MAX;
        let mut selected_edf_pid = None;
        let mut edf_index = None;
        
        for (i, &pid) in self.rt_edf_queue.iter().enumerate() {
            if let Some(process) = processes.get(pid as usize).and_then(|p| p.as_ref()) {
                // Only schedule if process has remaining budget and hasn't missed deadline
                if process.rt_params.remaining_budget > 0 && 
                   process.rt_params.next_deadline > current_time &&
                   process.rt_params.next_deadline < earliest_deadline {
                    earliest_deadline = process.rt_params.next_deadline;
                    selected_edf_pid = Some(pid);
                    edf_index = Some(i);
                }
            }
        }
        
        if let (Some(pid), Some(index)) = (selected_edf_pid, edf_index) {
            self.rt_edf_queue.remove(index);
            self.rt_edf_queue.push_back(pid); // Move to end for fairness
            self.current_process = Some(pid);
            // Use remaining budget or 100µs, whichever is smaller
            if let Some(process) = processes.get(pid as usize).and_then(|p| p.as_ref()) {
                self.current_time_slice_remaining = core::cmp::min(process.rt_params.remaining_budget / 1000, 100);
            } else {
                self.current_time_slice_remaining = 1;
            }
            return Some(pid);
        }
        
        // 2. Real-time CBS scheduling
        // CBS processes get bandwidth-controlled execution
        let mut cbs_candidates = Vec::new();
        for (i, &pid) in self.rt_cbs_queue.iter().enumerate() {
            if let Some(process) = processes.get(pid as usize).and_then(|p| p.as_ref()) {
                // Check if CBS process has budget available
                if process.rt_params.remaining_budget > 0 {
                    cbs_candidates.push((i, pid, process.rt_params.remaining_budget));
                }
            }
        }
        
        if let Some((index, pid, remaining_budget)) = cbs_candidates.first() {
            // Move to end of CBS queue for round-robin
            self.rt_cbs_queue.remove(*index);
            self.rt_cbs_queue.push_back(*pid);
            self.current_process = Some(*pid);
            // CBS gets smaller time slices for bandwidth control
            self.current_time_slice_remaining = core::cmp::min(remaining_budget / 1000, 50);
            return Some(*pid);
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
        let mut deadline_missed_pids = Vec::new();
        let mut budget_exhausted_pids = Vec::new();
        
        for &pid in &self.rt_edf_queue {
            if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
                // Check for deadline miss
                if current_time > process.rt_params.next_deadline {
                    deadline_missed_pids.push(pid);
                    // Log deadline miss for debugging
                    crate::serial_println!("EDF deadline miss: PID {} missed deadline by {}µs", 
                                         pid, current_time - process.rt_params.next_deadline);
                    
                    // Update to next period and replenish budget
                    while process.rt_params.next_deadline <= current_time {
                        process.rt_params.next_deadline += process.rt_params.period_us;
                    }
                    process.rt_params.remaining_budget = process.rt_params.budget_us;
                }
                
                // Check for budget exhaustion
                if process.rt_params.remaining_budget == 0 {
                    budget_exhausted_pids.push(pid);
                }
            }
        }
        
        // Remove processes that missed deadlines or exhausted budget
        for pid in deadline_missed_pids.iter().chain(budget_exhausted_pids.iter()) {
            if let Some(pos) = self.rt_edf_queue.iter().position(|&p| p == *pid) {
                self.rt_edf_queue.remove(pos);
            }
        }
        
        // Re-add deadline-missed processes (they get new deadlines)
        for pid in deadline_missed_pids {
            if let Some(process) = processes.get(pid as usize).and_then(|p| p.as_ref()) {
                self.add_rt_process(pid, process.rt_params.class, processes);
            }
        }
        
        // Update CBS processes
        let mut cbs_replenish_pids = Vec::new();
        let mut cbs_exhausted_pids = Vec::new();
        
        for &pid in &self.rt_cbs_queue {
            if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
                // Replenish budget if period elapsed
                if current_time >= process.rt_params.next_deadline {
                    cbs_replenish_pids.push(pid);
                    process.rt_params.remaining_budget = process.rt_params.budget_us;
                    process.rt_params.next_deadline += process.rt_params.period_us;
                }
                
                // Mark for removal from CBS queue if budget exhausted
                if process.rt_params.remaining_budget == 0 {
                    cbs_exhausted_pids.push(pid);
                }
            }
        }
        
        // Remove budget-exhausted CBS processes
        for pid in cbs_exhausted_pids {
            if let Some(pos) = self.rt_cbs_queue.iter().position(|&p| p == pid) {
                self.rt_cbs_queue.remove(pos);
            }
        }
        
        // Re-add CBS processes that got budget replenished
        for pid in cbs_replenish_pids {
            if !self.rt_cbs_queue.contains(&pid) {
                self.rt_cbs_queue.push_back(pid);
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
    
    /// Set time slice for current process (for dynamic scheduling)
    pub fn set_time_slice(&mut self, time_slice_ms: u32) {
        self.time_slice = time_slice_ms as u64;
        self.current_time_slice_remaining = time_slice_ms as u64;
    }
    
    /// Get remaining time slice in milliseconds
    pub fn get_remaining_time_slice(&self) -> u32 {
        self.current_time_slice_remaining as u32
    }
    
    /// Update real-time process deadlines
    pub fn update_rt_deadlines(&mut self, processes: &mut [Option<Process>], current_time_us: u64) {
        // Update EDF queue deadlines
        for &pid in &self.rt_edf_queue {
            if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
                if current_time_us >= process.rt_params.next_deadline {
                    // Deadline missed, update to next period
                    process.rt_params.next_deadline += process.rt_params.period_us;
                    process.rt_params.remaining_budget = process.rt_params.budget_us;
                }
            }
        }
        
        // Update CBS queue budgets
        for &pid in &self.rt_cbs_queue {
            if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
                if current_time_us >= process.rt_params.next_deadline {
                    // Replenish budget for next period
                    process.rt_params.next_deadline += process.rt_params.period_us;
                    process.rt_params.remaining_budget = process.rt_params.budget_us;
                }
            }
        }
        
        // Re-sort EDF queue by deadline
        self.rt_edf_queue.make_contiguous().sort_by(|&a, &b| {
            let deadline_a = processes.get(a as usize)
                .and_then(|p| p.as_ref())
                .map(|p| p.rt_params.next_deadline)
                .unwrap_or(u64::MAX);
            let deadline_b = processes.get(b as usize)
                .and_then(|p| p.as_ref())
                .map(|p| p.rt_params.next_deadline)
                .unwrap_or(u64::MAX);
            deadline_a.cmp(&deadline_b)
        });
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

    // Priority inheritance methods
    pub fn inherit_priority(&mut self, pid: u64, from_pid: u64, processes: &mut [Option<Process>]) {
        // First, get the priority from the source process
        let from_priority = if let Some(from_process) = processes.get(from_pid as usize).and_then(|p| p.as_ref()) {
            from_process.priority
        } else {
            return;
        };
        
        // Then modify the target process
        if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            match process.rt_params.priority_inheritance {
                PriorityInheritanceState::None => {
                    process.rt_params.priority_inheritance = PriorityInheritanceState::Inherited {
                        original_priority: process.priority,
                        inherited_from: from_pid,
                    };
                    process.priority = from_priority;
                    
                    // Track inheritance chain
                    self.priority_inheritance_chains.entry(from_pid).or_insert_with(Vec::new).push(pid);
                }
                _ => {} // Already inheriting, don't override
            }
        }
    }

    pub fn restore_priority(&mut self, pid: u64, processes: &mut [Option<Process>]) {
        if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            if let PriorityInheritanceState::Inherited { original_priority, inherited_from } = process.rt_params.priority_inheritance {
                process.priority = original_priority;
                process.rt_params.priority_inheritance = PriorityInheritanceState::None;
                
                // Remove from inheritance chain
                if let Some(chain) = self.priority_inheritance_chains.get_mut(&inherited_from) {
                    chain.retain(|&p| p != pid);
                    if chain.is_empty() {
                        self.priority_inheritance_chains.remove(&inherited_from);
                    }
                }
            }
        }
    }

    // CBS throttling methods
    pub fn update_cbs_budget(&mut self, pid: u64, consumed_us: u64, processes: &mut [Option<Process>]) {
        if let Some(process) = processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            if let Some(ref mut cbs_params) = process.rt_params.cbs_params {
                if cbs_params.remaining_budget >= consumed_us {
                    cbs_params.remaining_budget -= consumed_us;
                } else {
                    cbs_params.remaining_budget = 0;
                    cbs_params.throttled = true;
                }
                
                // Track budget usage
                *self.cbs_budget_tracker.entry(pid).or_insert(0) += consumed_us;
            }
        }
    }

    pub fn replenish_cbs_budget(&mut self, current_time_us: u64, processes: &mut [Option<Process>]) {
        for process_opt in processes.iter_mut() {
            if let Some(process) = process_opt {
                if let Some(ref mut cbs_params) = process.rt_params.cbs_params {
                    if current_time_us >= cbs_params.next_replenishment {
                        cbs_params.remaining_budget = cbs_params.server_budget_us;
                        cbs_params.next_replenishment = current_time_us + cbs_params.server_period_us;
                        cbs_params.throttled = false;
                    }
                }
            }
        }
    }

    // NUMA-aware scheduling methods
    pub fn set_numa_node(&mut self, numa_node: NumaNode) {
        self.numa_node = Some(numa_node);
    }

    pub fn get_numa_node(&self) -> Option<NumaNode> {
        self.numa_node
    }

    pub fn is_numa_local(&self, process: &Process) -> bool {
        match (self.numa_node, process.numa_node) {
            (Some(cpu_node), Some(process_node)) => cpu_node.id == process_node.id,
            _ => true, // If NUMA info is not available, assume local
        }
    }
}

/// Global SMP-aware scheduler
pub struct SmpScheduler {
    cpu_schedulers: Vec<Mutex<CpuScheduler>>,
    processes: Vec<Option<Process>>,
    gaming_mode: bool,
    num_cpus: u32,
    _current_cpu: AtomicU32,
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
            _current_cpu: AtomicU32::new(0),
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
        
        self.cpu_schedulers[cpu_id as usize].lock().schedule(self.gaming_mode, &self.processes)
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
                let _src_cpu = loads[loads.len() - 1].0;
                let _dst_cpu = loads[0].0;
                
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
            let _old_affinity = process.cpu_affinity;
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
    
    /// Update real-time deadlines for all CPUs (called periodically)
    pub fn update_all_rt_deadlines(&mut self) {
        let current_time_us = crate::time::get_precise_time_ns() / 1000;
        
        for cpu_scheduler in &mut self.cpu_schedulers {
            cpu_scheduler.lock().update_rt_deadlines(&mut self.processes, current_time_us);
        }
    }
    
    /// Set dynamic time slice for better responsiveness
    pub fn set_dynamic_time_slice(&mut self, cpu_id: u32, time_slice_ms: u32) {
        if let Some(cpu_scheduler) = self.cpu_schedulers.get_mut(cpu_id as usize) {
            cpu_scheduler.lock().set_time_slice(time_slice_ms);
        }
    }
    
    /// Get the earliest deadline across all CPUs for tickless scheduling
    pub fn get_earliest_deadline_us(&self) -> u64 {
        let current_time_us = crate::time::get_precise_time_ns() / 1000;
        let mut earliest_deadline = u64::MAX;
        
        for cpu_scheduler in &self.cpu_schedulers {
            let scheduler = cpu_scheduler.lock();
            
            // Check EDF queue for earliest deadline (only processes with budget)
            for &pid in &scheduler.rt_edf_queue {
                if let Some(process) = self.processes.get(pid as usize).and_then(|p| p.as_ref()) {
                    // Only consider processes with remaining budget
                    if process.rt_params.remaining_budget > 0 && 
                       process.rt_params.next_deadline < earliest_deadline {
                        earliest_deadline = process.rt_params.next_deadline;
                    }
                }
            }
            
            // Check current process time slice
            if scheduler.current_process.is_some() {
                let remaining_us = (scheduler.current_time_slice_remaining * 1000) as u64;
                let time_slice_deadline = current_time_us + remaining_us;
                if time_slice_deadline < earliest_deadline {
                    earliest_deadline = time_slice_deadline;
                }
            }
        }
        
        if earliest_deadline == u64::MAX {
            1000 // Default 1ms
        } else if earliest_deadline <= current_time_us {
            1 // Schedule immediately
        } else {
            earliest_deadline - current_time_us
        }
    }
    
    /// Check if a CPU core should be isolated for RT threads
    pub fn is_rt_core(&self, cpu_id: u8) -> bool {
        // Cores 2 and 3 are reserved for RT threads (input, audio, compositor)
        cpu_id >= 2 && cpu_id <= 3
    }
    
    /// Get the preferred CPU core for an RT thread type
    pub fn get_rt_cpu_affinity(&self, rt_class: RtClass) -> Option<u8> {
        match rt_class {
            RtClass::Edf => Some(2), // Input thread on core 2
            RtClass::Cbs => Some(3), // Audio/compositor on core 3
            _ => None,
        }
    }
    
    /// Migrate RT process to its preferred core
    pub fn migrate_rt_process(&mut self, pid: u64, target_cpu: u8) {
        if target_cpu as usize >= self.cpu_schedulers.len() {
            return;
        }
        
        // Remove from current CPU scheduler
        for (cpu_id, cpu_scheduler) in self.cpu_schedulers.iter().enumerate() {
            let mut scheduler = cpu_scheduler.lock();
            
            // Remove from EDF queue
            if let Some(pos) = scheduler.rt_edf_queue.iter().position(|&p| p == pid) {
                scheduler.rt_edf_queue.remove(pos);
                
                // Add to target CPU
                if cpu_id != target_cpu as usize {
                    drop(scheduler);
                    let mut target_scheduler = self.cpu_schedulers[target_cpu as usize].lock();
                    target_scheduler.rt_edf_queue.push_back(pid);
                }
                return;
            }
            
            // Remove from CBS queue
            if let Some(pos) = scheduler.rt_cbs_queue.iter().position(|&p| p == pid) {
                scheduler.rt_cbs_queue.remove(pos);
                
                // Add to target CPU
                if cpu_id != target_cpu as usize {
                    drop(scheduler);
                    let mut target_scheduler = self.cpu_schedulers[target_cpu as usize].lock();
                    target_scheduler.rt_cbs_queue.push_back(pid);
                }
                return;
            }
        }
    }
    
    /// Enforce RT core isolation by moving non-RT processes away from RT cores
    pub fn enforce_rt_core_isolation(&mut self) {
        for (cpu_id, cpu_scheduler) in self.cpu_schedulers.iter().enumerate() {
            if self.is_rt_core(cpu_id as u8) {
                let mut scheduler = cpu_scheduler.lock();
                let mut non_rt_processes = Vec::new();
                
                // Find non-RT processes on RT cores
                for priority_queue in &mut scheduler.ready_queues {
                    while let Some(pid) = priority_queue.pop_front() {
                        if let Some(process) = self.processes.get(pid as usize).and_then(|p| p.as_ref()) {
                            if matches!(process.rt_params.class, RtClass::BestEffort) {
                                non_rt_processes.push(pid);
                            } else {
                                priority_queue.push_back(pid); // Keep RT processes
                            }
                        }
                    }
                }
                
                drop(scheduler);
                
                // Migrate non-RT processes to non-RT cores
                for pid in non_rt_processes {
                    // Find a non-RT core with least load
                    let mut target_cpu = 0;
                    let mut min_load = u32::MAX;
                    
                    for (other_cpu_id, other_scheduler) in self.cpu_schedulers.iter().enumerate() {
                        if !self.is_rt_core(other_cpu_id as u8) {
                            let load = other_scheduler.lock().get_load();
                            if load < min_load {
                                min_load = load;
                                target_cpu = other_cpu_id;
                            }
                        }
                    }
                    
                    // Add to target CPU's ready queue
                    if let Some(process) = self.processes.get(pid as usize).and_then(|p| p.as_ref()) {
                        let priority = process.priority as usize;
                        if priority < self.cpu_schedulers[target_cpu].lock().ready_queues.len() {
                            self.cpu_schedulers[target_cpu].lock().ready_queues[priority].push_back(pid);
                        }
                    }
                }
            }
        }
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

    // NUMA-aware scheduling methods
    pub fn set_numa_topology(&mut self, numa_nodes: &[NumaNode]) {
        for (cpu_id, scheduler_mutex) in self.cpu_schedulers.iter().enumerate() {
            let mut scheduler = scheduler_mutex.lock();
            
            // Find the NUMA node for this CPU
            for numa_node in numa_nodes {
                if (numa_node.cpu_mask & (1 << cpu_id)) != 0 {
                    scheduler.set_numa_node(*numa_node);
                    break;
                }
            }
        }
    }

    pub fn numa_aware_load_balance(&mut self) {
        // Group CPUs by NUMA node
        let mut numa_groups: alloc::collections::BTreeMap<u8, Vec<usize>> = alloc::collections::BTreeMap::new();
        
        for (cpu_id, scheduler_mutex) in self.cpu_schedulers.iter().enumerate() {
            let scheduler = scheduler_mutex.lock();
            if let Some(numa_node) = scheduler.get_numa_node() {
                numa_groups.entry(numa_node.id).or_insert_with(Vec::new).push(cpu_id);
            }
        }

        // Balance load within each NUMA node first
        for (_numa_id, cpu_list) in numa_groups.iter() {
            self.balance_load_within_numa_node(cpu_list);
        }

        // Then balance across NUMA nodes if necessary
        self.balance_load_across_numa_nodes(&numa_groups);
    }

    fn balance_load_within_numa_node(&mut self, cpu_list: &[usize]) {
        if cpu_list.len() < 2 {
            return;
        }

        // Find the most and least loaded CPUs within this NUMA node
        let mut max_load = 0;
        let mut min_load = u32::MAX;
        let mut max_cpu = 0;
        let mut min_cpu = 0;

        for &cpu_id in cpu_list {
            let load = self.cpu_schedulers[cpu_id].lock().get_load();
            if load > max_load {
                max_load = load;
                max_cpu = cpu_id;
            }
            if load < min_load {
                min_load = load;
                min_cpu = cpu_id;
            }
        }

        // Migrate processes if load imbalance is significant
        if max_load > min_load + 2 {
            self.migrate_process_between_cpus(max_cpu as u32, min_cpu as u32);
        }
    }

    fn balance_load_across_numa_nodes(&mut self, numa_groups: &alloc::collections::BTreeMap<u8, Vec<usize>>) {
        // Calculate average load per NUMA node
        let mut numa_loads: Vec<(u8, u32)> = Vec::new();
        
        for (&numa_id, cpu_list) in numa_groups {
            let total_load: u32 = cpu_list.iter()
                .map(|&cpu_id| self.cpu_schedulers[cpu_id].lock().get_load())
                .sum();
            let avg_load = if cpu_list.is_empty() { 0 } else { total_load / cpu_list.len() as u32 };
            numa_loads.push((numa_id, avg_load));
        }

        // Sort by load
        numa_loads.sort_by_key(|&(_, load)| load);

        // Migrate processes from high-load to low-load NUMA nodes if imbalance is severe
        if numa_loads.len() >= 2 {
            let (low_numa, low_load) = numa_loads[0];
            let (high_numa, high_load) = numa_loads[numa_loads.len() - 1];
            
            if high_load > low_load + 4 {
                // Find representative CPUs from each NUMA node
                if let (Some(low_cpus), Some(high_cpus)) = (numa_groups.get(&low_numa), numa_groups.get(&high_numa)) {
                    if let (Some(&low_cpu), Some(&high_cpu)) = (low_cpus.first(), high_cpus.first()) {
                        self.migrate_process_between_cpus(high_cpu as u32, low_cpu as u32);
                    }
                }
            }
        }
    }

    fn migrate_process_between_cpus(&mut self, from_cpu: u32, to_cpu: u32) {
        // Find a suitable process to migrate
        let mut process_to_migrate: Option<u64> = None;
        
        {
            let from_scheduler = self.cpu_schedulers[from_cpu as usize].lock();
            
            // Look for a non-RT process in the lowest priority queue
            for priority in (0..4).rev() {
                if let Some(&pid) = from_scheduler.ready_queues[priority].front() {
                    // Check if process can run on target CPU
                    if let Some(process) = self.processes.get(pid as usize).and_then(|p| p.as_ref()) {
                        if process.cpu_affinity.can_run_on(to_cpu) {
                            process_to_migrate = Some(pid);
                            break;
                        }
                    }
                }
            }
        }

        // Perform the migration
        if let Some(pid) = process_to_migrate {
            {
                let mut from_scheduler = self.cpu_schedulers[from_cpu as usize].lock();
                from_scheduler.remove_process(pid);
            }
            
            if let Some(process) = self.processes.get(pid as usize).and_then(|p| p.as_ref()) {
                let mut to_scheduler = self.cpu_schedulers[to_cpu as usize].lock();
                to_scheduler.add_process(pid, process.priority);
            }
        }
    }

    // Priority inheritance across CPUs
    pub fn inherit_priority_across_cpus(&mut self, pid: u64, from_pid: u64) {
        // Find which CPUs these processes are on
        let mut pid_cpu: Option<u32> = None;
        
        for (cpu_id, scheduler_mutex) in self.cpu_schedulers.iter().enumerate() {
            let scheduler = scheduler_mutex.lock();
            if scheduler.current_process == Some(pid) {
                pid_cpu = Some(cpu_id as u32);
                break;
            }
        }

        // Apply priority inheritance
        if let Some(cpu_id) = pid_cpu {
            let mut scheduler = self.cpu_schedulers[cpu_id as usize].lock();
            scheduler.inherit_priority(pid, from_pid, &mut self.processes);
        }
    }

    pub fn restore_priority_across_cpus(&mut self, pid: u64) {
        // Find which CPU this process is on
        for (cpu_id, scheduler_mutex) in self.cpu_schedulers.iter().enumerate() {
            let scheduler = scheduler_mutex.lock();
            if scheduler.current_process == Some(pid) {
                drop(scheduler);
                let mut scheduler = self.cpu_schedulers[cpu_id].lock();
                scheduler.restore_priority(pid, &mut self.processes);
                break;
            }
        }
    }

    // CBS throttling across all CPUs
    pub fn update_all_cbs_budgets(&mut self, current_time_us: u64) {
        for scheduler_mutex in &self.cpu_schedulers {
            let mut scheduler = scheduler_mutex.lock();
            scheduler.replenish_cbs_budget(current_time_us, &mut self.processes);
        }
    }

    pub fn get_numa_node_for_cpu(&self, cpu_id: u32) -> Option<NumaNode> {
        if let Some(scheduler_mutex) = self.cpu_schedulers.get(cpu_id as usize) {
            scheduler_mutex.lock().get_numa_node()
        } else {
            None
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

/// Spawn a real-time kernel thread with specific RT parameters
pub fn spawn_rt_kernel_thread(
    name: &str, 
    entry: extern "C" fn() -> !, 
    rt_class: RtClass,
    period_us: u64,
    budget_us: u64,
    cpu_affinity: Option<CpuAffinity>
) -> u64 {
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
    
    // Set RT parameters
    let current_time = crate::time::get_uptime_ms() * 1000; // Convert to microseconds
    proc.rt_params = RtParams {
        class: rt_class,
        deadline_us: period_us,
        period_us,
        budget_us,
        remaining_budget: budget_us,
        next_deadline: current_time + period_us,
        last_replenish: current_time,
        cbs_params: None,
        priority_inheritance: PriorityInheritanceState::None,
    };
    
    // Set CPU affinity if specified
    if let Some(affinity) = cpu_affinity {
        proc.cpu_affinity = affinity;
    }
    
    let pid = get_smp_scheduler().lock().add_process(proc);
    
    // Add to RT scheduler
    get_smp_scheduler().lock().add_rt_process(pid, rt_class);
    
    pid
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
    let scheduler = get_smp_scheduler().lock();
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

// Input processing RT thread - handles input events with low latency
extern "C" fn input_rt_thread() -> ! {
    loop {
        // Process input events from keyboard, mouse, touchpad
        crate::input::process_input_events();
        
        // Yield to allow other RT tasks to run
        yield_current();
    }
}

// Audio processing RT thread - handles audio with strict timing
extern "C" fn audio_rt_thread() -> ! {
    loop {
        // Process audio buffers and maintain low-latency audio pipeline
        crate::sound::process_audio_buffers();
        
        // Yield to maintain timing constraints
        yield_current();
    }
}

// Compositor RT thread - handles frame rendering with vsync timing
extern "C" fn compositor_rt_thread() -> ! {
    loop {
        // Render frames and handle compositor operations
        crate::graphics::process_compositor_frame();
        
        // Yield to maintain frame timing
        yield_current();
    }
}

/// Initialize real-time threads for input, audio, and compositor
pub fn init_rt_threads() -> Result<(), &'static str> {
    // Get CPU count for RT core isolation
    let num_cpus = get_cpu_count();
    
    if num_cpus < 4 {
        return Err("RT core isolation requires at least 4 CPU cores (cores 2-3 for RT)");
    }
    
    let mut scheduler = get_smp_scheduler().lock();
    
    // Enforce RT core isolation before creating RT threads
    scheduler.enforce_rt_core_isolation();
    
    // Input RT thread - EDF with 1ms period, 200μs budget on core 2
    let input_affinity = CpuAffinity::single_cpu(2);
    let input_pid = spawn_rt_kernel_thread(
        "input_rt",
        input_rt_thread,
        RtClass::Edf,
        1000,  // 1ms period
        200,   // 200μs budget
        Some(input_affinity)
    );
    
    // Migrate input thread to its dedicated core
    scheduler.migrate_rt_process(input_pid, 2);
    
    // Audio RT thread - CBS with 2.67ms period, 500μs budget on core 3
    let audio_affinity = CpuAffinity::single_cpu(3);
    let audio_pid = spawn_rt_kernel_thread(
        "audio_rt",
        audio_rt_thread,
        RtClass::Cbs,
        2670,  // ~2.67ms period (128 samples at 48kHz)
        500,   // 500μs budget
        Some(audio_affinity)
    );
    
    // Migrate audio thread to its dedicated core
    scheduler.migrate_rt_process(audio_pid, 3);
    
    // Compositor RT thread - CBS with 8.33ms period, 2ms budget on core 3 (shared with audio)
    let compositor_affinity = CpuAffinity::single_cpu(3);
    let compositor_pid = spawn_rt_kernel_thread(
        "compositor_rt",
        compositor_rt_thread,
        RtClass::Cbs,
        8333,  // 8.33ms period (120Hz)
        2000,  // 2ms budget
        Some(compositor_affinity)
    );
    
    // Migrate compositor thread to core 3 (shared with audio via CBS)
    scheduler.migrate_rt_process(compositor_pid, 3);
    
    drop(scheduler);
    
    crate::serial_println!("[RT] Initialized RT threads with core isolation:");
    crate::serial_println!("[RT] Input (PID {}): EDF 1ms/200μs on CPU 2", input_pid);
    crate::serial_println!("[RT] Audio (PID {}): CBS 2.67ms/500μs on CPU 3", audio_pid);
    crate::serial_println!("[RT] Compositor (PID {}): CBS 8.33ms/2ms on CPU 3", compositor_pid);
    crate::serial_println!("[RT] Cores 2-3 isolated for real-time processing");
    
    Ok(())
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

pub fn get_current_process_parent_id() -> Option<u64> {
    let cpu_id = get_current_cpu_id();
    let scheduler = get_smp_scheduler().lock();
    scheduler.get_current_process(cpu_id).and_then(|p| p.parent_pid)
}

/// Check if a process is alive (exists and not in a terminated state)
pub fn is_process_alive(process_id: u64) -> bool {
    let scheduler = get_smp_scheduler().lock();
    if let Some(Some(process)) = scheduler.processes.get(process_id as usize) {
        match process.state {
            ProcessState::Running | ProcessState::Ready | ProcessState::Blocked => true,
            ProcessState::Terminated => false,
        }
    } else {
        false
    }
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
                    switch_context(old_ctx_ptr, &new_process.context as *const ProcessContext);
                }
                return;
            }
        }
        
        // No address space switch needed, just switch context
        switch_context(old_ctx_ptr, &new_process.context as *const ProcessContext);
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

/// Get the next scheduler deadline in microseconds for tickless operation
pub fn get_next_scheduler_deadline_us() -> u64 {
    // Use SMP scheduler's optimized deadline calculation
    let smp_scheduler = get_smp_scheduler().lock();
    smp_scheduler.get_earliest_deadline_us()
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