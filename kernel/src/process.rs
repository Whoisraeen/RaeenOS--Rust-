use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::{VirtAddr, PhysAddr};

static NEXT_PID: AtomicU64 = AtomicU64::new(1);
static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
static IDLE_THREAD_PID: AtomicU64 = AtomicU64::new(0);

pub type ProcessId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    High = 0,
    Normal = 1,
    Low = 2,
    Gaming = 3, // Special priority for gaming mode
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

#[derive(Debug)]
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
    // Keep kernel stack backing alive for kernel threads
    pub kernel_stack_ptr: Option<usize>, // Store as usize to make it Send
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
            kernel_stack_ptr: None,
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
}

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
            if let Some(process) = self.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
                if process.state == ProcessState::Running {
                    process.state = ProcessState::Ready;
                }
            }
            self.current_process = None;
        }
    }
    
    pub fn block_current(&mut self) {
        if let Some(pid) = self.current_process {
            if let Some(process) = self.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
                process.state = ProcessState::Blocked;
                // Remove from ready queues
                for queue in &mut self.ready_queues {
                    queue.retain(|&p| p != pid);
                }
            }
            self.current_process = None;
        }
    }
    
    pub fn unblock_process(&mut self, pid: u64) {
        if let Some(process) = self.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            if process.state == ProcessState::Blocked {
                process.state = ProcessState::Ready;
                let priority = process.priority as usize;
                self.ready_queues[priority].push_back(pid);
            }
        }
    }
}

// Public API functions
pub fn init() {
    // Initialize the scheduler - already done with static initialization
}

pub fn create_process(name: alloc::string::String, entry_point: VirtAddr, priority: Priority) -> u64 {
    let process = Process::new(name, entry_point, priority);
    SCHEDULER.lock().add_process(process)
}

pub fn create_kernel_process(name: alloc::string::String, entry_point: VirtAddr) -> u64 {
    let process = Process::kernel_process(name, entry_point);
    SCHEDULER.lock().add_process(process)
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
    SCHEDULER.lock().add_process(proc)
}

pub fn schedule() -> Option<u64> {
    SCHEDULER.lock().schedule()
}

pub fn yield_current() {
    SCHEDULER.lock().yield_current();
}

pub fn block_current() {
    SCHEDULER.lock().block_current();
}

pub fn unblock_process(pid: u64) {
    SCHEDULER.lock().unblock_process(pid);
}

pub fn terminate_process(pid: u64) {
    SCHEDULER.lock().remove_process(pid);
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
    SCHEDULER.lock().set_idle_thread(idle_pid);
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
    SCHEDULER.lock().set_gaming_mode(enabled);
}

pub fn get_current_process_info() -> Option<(u64, alloc::string::String, ProcessState)> {
    let scheduler = SCHEDULER.lock();
    scheduler.get_current_process().map(|p| (p.pid, p.name.clone(), p.state))
}

pub fn get_current_process_id() -> u64 {
    let scheduler = SCHEDULER.lock();
    scheduler.current_process.unwrap_or(0)
}



// Context switching function using inline assembly
unsafe fn switch_context(old_context: *mut ProcessContext, new_context: *const ProcessContext) {
    if !old_context.is_null() {
        // Save current context
        core::arch::asm!(
            "mov {}, rax",
            "mov {}, rbx",
            "mov {}, rcx",
            "mov {}, rdx",
            "mov {}, rsi",
            "mov {}, rdi",
            "mov {}, rbp",
            "mov {}, rsp",
            "mov {}, r8",
            "mov {}, r9",
            "mov {}, r10",
            "mov {}, r11",
            "mov {}, r12",
            "mov {}, r13",
            "mov {}, r14",
            "mov {}, r15",
            "pushfq",
            "pop {}",
            out(reg) (*old_context).rax,
            out(reg) (*old_context).rbx,
            out(reg) (*old_context).rcx,
            out(reg) (*old_context).rdx,
            out(reg) (*old_context).rsi,
            out(reg) (*old_context).rdi,
            out(reg) (*old_context).rbp,
            out(reg) (*old_context).rsp,
            out(reg) (*old_context).r8,
            out(reg) (*old_context).r9,
            out(reg) (*old_context).r10,
            out(reg) (*old_context).r11,
            out(reg) (*old_context).r12,
            out(reg) (*old_context).r13,
            out(reg) (*old_context).r14,
            out(reg) (*old_context).r15,
            out(reg) (*old_context).rflags,
            options(nostack, preserves_flags)
        );
        
        // Save return address as RIP
        let return_addr: u64;
        core::arch::asm!("lea {}, [rip]", out(reg) return_addr, options(nostack, nomem));
        (*old_context).rip = return_addr;
        (*old_context).cs = 0x08;  // Kernel code segment
        (*old_context).ss = 0x10;  // Kernel data segment
    }
    
    // Load new context
    let new_ctx = &*new_context;
    core::arch::asm!(
        "mov rax, {}",
        "mov rbx, {}",
        "mov rcx, {}",
        "mov rdx, {}",
        "mov rsi, {}",
        "mov rdi, {}",
        "mov rbp, {}",
        "mov rsp, {}",
        "mov r8, {}",
        "mov r9, {}",
        "mov r10, {}",
        "mov r11, {}",
        "mov r12, {}",
        "mov r13, {}",
        "mov r14, {}",
        "mov r15, {}",
        "push {}",
        "popfq",
        "jmp {}",
        in(reg) new_ctx.rax,
        in(reg) new_ctx.rbx,
        in(reg) new_ctx.rcx,
        in(reg) new_ctx.rdx,
        in(reg) new_ctx.rsi,
        in(reg) new_ctx.rdi,
        in(reg) new_ctx.rbp,
        in(reg) new_ctx.rsp,
        in(reg) new_ctx.r8,
        in(reg) new_ctx.r9,
        in(reg) new_ctx.r10,
        in(reg) new_ctx.r11,
        in(reg) new_ctx.r12,
        in(reg) new_ctx.r13,
        in(reg) new_ctx.r14,
        in(reg) new_ctx.r15,
        in(reg) new_ctx.rflags,
        in(reg) new_ctx.rip,
        options(noreturn)
    );
}

pub fn context_switch(old_pid: Option<u64>, new_pid: u64) {
    let mut scheduler = SCHEDULER.lock();
    
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
                scheduler = SCHEDULER.lock();
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
    let mut scheduler = SCHEDULER.lock();
    let current = scheduler.current_process;
    
    // Check if current process time slice expired
    let time_slice_expired = scheduler.tick_time_slice();
    
    // Only preempt if time slice expired or current process is not running
    let idle_pid = scheduler.idle_thread_pid;
    let should_schedule = if let Some(pid) = current {
        if let Some(proc_ref) = scheduler.processes.get_mut(pid as usize).and_then(|p| p.as_mut()) {
            if proc_ref.state == ProcessState::Running {
                if time_slice_expired {
                    // Time slice expired, move back to ready queue (unless it's idle thread)
                    if Some(pid) != idle_pid {
                        proc_ref.state = ProcessState::Ready;
                        let prio = proc_ref.priority as usize;
                        scheduler.ready_queues[prio].push_back(pid);
                    }
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
        if let Some(next_pid) = scheduler.schedule() {
            drop(scheduler);
            context_switch(current, next_pid);
        }
    }
}

// Process management functions for syscalls
pub fn fork_process() -> Result<ProcessId, ()> {
    let mut scheduler = SCHEDULER.lock();
    let current_pid = scheduler.current_process.ok_or(())?;
    
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
    
    let mut scheduler = SCHEDULER.lock();
    let current_pid = scheduler.current_process.ok_or(())?;
    
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

pub fn wait_for_process(pid: ProcessId) -> Result<i32, ()> {
    let mut scheduler = SCHEDULER.lock();
    let current_pid = scheduler.current_process.ok_or(())?;
    
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
            // Process already terminated, return exit code (simplified)
            let exit_code = 0; // Simplified - no exit_code field in Process struct
            
            // Clean up the terminated process
            if let Some(slot) = scheduler.processes.get_mut(pid as usize) {
                *slot = None;
            }
            crate::security::cleanup_process_security(pid as u32);
            
            Ok(exit_code)
        }
        _ => {
            // Process still running, block current process until it terminates
            if let Some(current_process) = scheduler.processes.get_mut(current_pid as usize)
                .and_then(|p| p.as_mut()) {
                current_process.state = ProcessState::Blocked;
            }
            
            // Remove current process from ready queues
            for queue in &mut scheduler.ready_queues {
                queue.retain(|&p| p != current_pid);
            }
            
            // In a real implementation, this would yield to scheduler
            // For now, return a placeholder exit code
            Ok(0)
        }
    }
}

pub fn get_process_count() -> u64 {
    let scheduler = SCHEDULER.lock();
    scheduler.processes.iter().filter(|p| p.is_some()).count() as u64
}