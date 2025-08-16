//! Per-CPU data structures for SMP systems
//! Provides CPU-local storage and management for RaeenOS

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;


use crate::arch;
use crate::process::ProcessId;

/// Maximum number of CPUs supported
const MAX_CPUS: usize = 256;

/// Per-CPU data structure
#[repr(C, align(64))] // Cache line aligned to prevent false sharing
#[derive(Debug)]
pub struct PerCpuData {
    /// CPU ID
    pub cpu_id: u32,
    
    /// APIC ID
    pub apic_id: u32,
    
    /// CPU online status
    pub online: AtomicBool,
    
    /// Current running process ID
    pub current_process: AtomicU64,
    
    /// Idle process ID for this CPU
    pub idle_process: AtomicU64,
    
    /// CPU utilization counter (ticks spent in user/kernel mode)
    pub user_ticks: AtomicU64,
    pub kernel_ticks: AtomicU64,
    pub idle_ticks: AtomicU64,
    
    /// Interrupt statistics
    pub interrupt_count: AtomicU64,
    pub timer_interrupts: AtomicU64,
    pub ipi_count: AtomicU64,
    
    /// Scheduler statistics
    pub context_switches: AtomicU64,
    pub preemptions: AtomicU64,
    
    /// Memory management per-CPU data
    pub tlb_flushes: AtomicU64,
    pub page_faults: AtomicU64,
    
    /// TSC frequency for this CPU (may vary on some systems)
    pub tsc_frequency: AtomicU64,
    
    /// Last TSC reading for deadline calculations
    pub last_tsc: AtomicU64,
    
    /// CPU temperature (if available)
    pub temperature: AtomicU32, // In degrees Celsius * 1000
    
    /// CPU frequency scaling
    pub current_frequency: AtomicU64, // In Hz
    pub max_frequency: AtomicU64,
    pub min_frequency: AtomicU64,
    
    /// Power management state
    pub c_state: AtomicU32,
    pub p_state: AtomicU32,
    
    /// Cache information
    pub l1_cache_misses: AtomicU64,
    pub l2_cache_misses: AtomicU64,
    pub l3_cache_misses: AtomicU64,
    
    /// NUMA node ID
    pub numa_node: AtomicU32,
    
    /// CPU flags and capabilities
    pub features: CpuFeatures,
    
    /// Kernel stack pointer for this CPU
    pub kernel_stack: AtomicU64,
    
    /// Exception stack pointer
    pub exception_stack: AtomicU64,
    
    /// IRQ stack pointer
    pub irq_stack: AtomicU64,
}

/// CPU feature flags
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub has_tsc: bool,
    pub has_invariant_tsc: bool,
    pub has_tsc_deadline: bool,
    pub has_x2apic: bool,
    pub has_smep: bool,
    pub has_smap: bool,
    pub has_umip: bool,
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_avx512: bool,
    pub has_aes: bool,
    pub has_rdrand: bool,
    pub has_rdseed: bool,
}

impl Default for CpuFeatures {
    fn default() -> Self {
        Self {
            has_tsc: false,
            has_invariant_tsc: false,
            has_tsc_deadline: false,
            has_x2apic: false,
            has_smep: false,
            has_smap: false,
            has_umip: false,
            has_avx: false,
            has_avx2: false,
            has_avx512: false,
            has_aes: false,
            has_rdrand: false,
            has_rdseed: false,
        }
    }
}

impl PerCpuData {
    /// Create new per-CPU data structure
    pub fn new(cpu_id: u32, apic_id: u32) -> Self {
        Self {
            cpu_id,
            apic_id,
            online: AtomicBool::new(false),
            current_process: AtomicU64::new(0),
            idle_process: AtomicU64::new(0),
            user_ticks: AtomicU64::new(0),
            kernel_ticks: AtomicU64::new(0),
            idle_ticks: AtomicU64::new(0),
            interrupt_count: AtomicU64::new(0),
            timer_interrupts: AtomicU64::new(0),
            ipi_count: AtomicU64::new(0),
            context_switches: AtomicU64::new(0),
            preemptions: AtomicU64::new(0),
            tlb_flushes: AtomicU64::new(0),
            page_faults: AtomicU64::new(0),
            tsc_frequency: AtomicU64::new(0),
            last_tsc: AtomicU64::new(0),
            temperature: AtomicU32::new(0),
            current_frequency: AtomicU64::new(0),
            max_frequency: AtomicU64::new(0),
            min_frequency: AtomicU64::new(0),
            c_state: AtomicU32::new(0),
            p_state: AtomicU32::new(0),
            l1_cache_misses: AtomicU64::new(0),
            l2_cache_misses: AtomicU64::new(0),
            l3_cache_misses: AtomicU64::new(0),
            numa_node: AtomicU32::new(0),
            features: CpuFeatures::default(),
            kernel_stack: AtomicU64::new(0),
            exception_stack: AtomicU64::new(0),
            irq_stack: AtomicU64::new(0),
        }
    }
    
    /// Mark CPU as online
    pub fn set_online(&self) {
        self.online.store(true, Ordering::SeqCst);
    }
    
    /// Mark CPU as offline
    pub fn set_offline(&self) {
        self.online.store(false, Ordering::SeqCst);
    }
    
    /// Check if CPU is online
    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::SeqCst)
    }
    
    /// Set current running process
    pub fn set_current_process(&self, pid: ProcessId) {
        self.current_process.store(pid, Ordering::SeqCst);
    }
    
    /// Get current running process
    pub fn get_current_process(&self) -> ProcessId {
        self.current_process.load(Ordering::SeqCst)
    }
    
    /// Increment context switch counter
    pub fn inc_context_switches(&self) {
        self.context_switches.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Increment interrupt counter
    pub fn inc_interrupts(&self) {
        self.interrupt_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Increment timer interrupt counter
    pub fn inc_timer_interrupts(&self) {
        self.timer_interrupts.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Increment IPI counter
    pub fn inc_ipi_count(&self) {
        self.ipi_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Add user mode ticks
    pub fn add_user_ticks(&self, ticks: u64) {
        self.user_ticks.fetch_add(ticks, Ordering::Relaxed);
    }
    
    /// Add kernel mode ticks
    pub fn add_kernel_ticks(&self, ticks: u64) {
        self.kernel_ticks.fetch_add(ticks, Ordering::Relaxed);
    }
    
    /// Add idle ticks
    pub fn add_idle_ticks(&self, ticks: u64) {
        self.idle_ticks.fetch_add(ticks, Ordering::Relaxed);
    }
    
    /// Get CPU utilization percentage (0-100)
    pub fn get_utilization(&self) -> u32 {
        let user = self.user_ticks.load(Ordering::Relaxed);
        let kernel = self.kernel_ticks.load(Ordering::Relaxed);
        let idle = self.idle_ticks.load(Ordering::Relaxed);
        
        let total = user + kernel + idle;
        if total == 0 {
            return 0;
        }
        
        let active = user + kernel;
        ((active * 100) / total) as u32
    }
    
    /// Update TSC frequency
    pub fn set_tsc_frequency(&self, freq: u64) {
        self.tsc_frequency.store(freq, Ordering::SeqCst);
    }
    
    /// Get TSC frequency
    pub fn get_tsc_frequency(&self) -> u64 {
        self.tsc_frequency.load(Ordering::SeqCst)
    }
    
    /// Update last TSC reading
    pub fn update_last_tsc(&self, tsc: u64) {
        self.last_tsc.store(tsc, Ordering::SeqCst);
    }
    
    /// Get statistics summary
    pub fn get_stats(&self) -> CpuStats {
        CpuStats {
            cpu_id: self.cpu_id,
            online: self.is_online(),
            current_process: self.get_current_process(),
            utilization: self.get_utilization(),
            context_switches: self.context_switches.load(Ordering::Relaxed),
            interrupts: self.interrupt_count.load(Ordering::Relaxed),
            timer_interrupts: self.timer_interrupts.load(Ordering::Relaxed),
            ipi_count: self.ipi_count.load(Ordering::Relaxed),
            user_ticks: self.user_ticks.load(Ordering::Relaxed),
            kernel_ticks: self.kernel_ticks.load(Ordering::Relaxed),
            idle_ticks: self.idle_ticks.load(Ordering::Relaxed),
            tsc_frequency: self.get_tsc_frequency(),
            current_frequency: self.current_frequency.load(Ordering::Relaxed),
            temperature: self.temperature.load(Ordering::Relaxed),
            numa_node: self.numa_node.load(Ordering::Relaxed),
        }
    }
}

/// CPU statistics snapshot
#[derive(Debug, Clone)]
pub struct CpuStats {
    pub cpu_id: u32,
    pub online: bool,
    pub current_process: ProcessId,
    pub utilization: u32,
    pub context_switches: u64,
    pub interrupts: u64,
    pub timer_interrupts: u64,
    pub ipi_count: u64,
    pub user_ticks: u64,
    pub kernel_ticks: u64,
    pub idle_ticks: u64,
    pub tsc_frequency: u64,
    pub current_frequency: u64,
    pub temperature: u32,
    pub numa_node: u32,
}

/// Global per-CPU data array
static mut PER_CPU_DATA: [Option<PerCpuData>; MAX_CPUS] = [const { None }; MAX_CPUS];
static CPU_COUNT: AtomicU32 = AtomicU32::new(0);
static PERCPU_INITIALIZED: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref PERCPU_LOCK: Mutex<()> = Mutex::new(());
}

/// Initialize per-CPU data structures
pub fn init() -> Result<(), &'static str> {
    let _lock = PERCPU_LOCK.lock();
    
    if PERCPU_INITIALIZED.load(Ordering::SeqCst) {
        return Ok(()); // Already initialized
    }
    
    crate::serial::_print(format_args!("[PerCPU] Initializing per-CPU data structures...\n"));
    
    // Initialize BSP (Bootstrap Processor) data
    let bsp_apic_id = crate::apic::get_apic_id();
    let bsp_data = PerCpuData::new(0, bsp_apic_id);
    
    // Detect CPU features for BSP
    let mut features = CpuFeatures::default();
    features.has_tsc = arch::has_cpu_feature(arch::CpuFeature::Tsc);
    features.has_invariant_tsc = arch::has_cpu_feature(arch::CpuFeature::InvariantTsc);
    features.has_tsc_deadline = arch::has_cpu_feature(arch::CpuFeature::TscDeadline);
    features.has_x2apic = arch::has_cpu_feature(arch::CpuFeature::X2apic);
    features.has_smep = arch::has_cpu_feature(arch::CpuFeature::Smep);
    features.has_smap = arch::has_cpu_feature(arch::CpuFeature::Smap);
    features.has_umip = arch::has_cpu_feature(arch::CpuFeature::Umip);
    features.has_avx = arch::has_cpu_feature(arch::CpuFeature::Avx);
    features.has_avx2 = arch::has_cpu_feature(arch::CpuFeature::Avx2);
    features.has_aes = arch::has_cpu_feature(arch::CpuFeature::Aes);
    features.has_rdrand = arch::has_cpu_feature(arch::CpuFeature::Rdrand);
    features.has_rdseed = arch::has_cpu_feature(arch::CpuFeature::Rdseed);
    
    let mut bsp_data = bsp_data;
    bsp_data.features = features;
    bsp_data.set_online();
    
    // Set TSC frequency if available
    if features.has_tsc {
        let freq = crate::arch::tsc::get_frequency();
        if freq > 0 {
            bsp_data.set_tsc_frequency(freq);
        }
    }
    
    unsafe {
        PER_CPU_DATA[0] = Some(bsp_data);
    }
    
    CPU_COUNT.store(1, Ordering::SeqCst);
    PERCPU_INITIALIZED.store(true, Ordering::SeqCst);
    
    crate::serial::_print(format_args!(
        "[PerCPU] Initialized BSP (CPU 0, APIC ID {}) with features: TSC={}, TSC-deadline={}, x2APIC={}\n",
        bsp_apic_id,
        features.has_tsc,
        features.has_tsc_deadline,
        features.has_x2apic
    ));
    
    Ok(())
}

/// Add a new CPU to the per-CPU data structures
pub fn add_cpu(cpu_id: u32, apic_id: u32) -> Result<(), &'static str> {
    let _lock = PERCPU_LOCK.lock();
    
    if cpu_id as usize >= MAX_CPUS {
        return Err("CPU ID exceeds maximum supported CPUs");
    }
    
    unsafe {
        if PER_CPU_DATA[cpu_id as usize].is_some() {
            return Err("CPU already exists");
        }
        
        let mut cpu_data = PerCpuData::new(cpu_id, apic_id);
        
        // Copy features from BSP (assume same for all CPUs)
        if let Some(ref bsp_data) = PER_CPU_DATA[0] {
            cpu_data.features = bsp_data.features;
            cpu_data.set_tsc_frequency(bsp_data.get_tsc_frequency());
        }
        
        PER_CPU_DATA[cpu_id as usize] = Some(cpu_data);
    }
    
    CPU_COUNT.fetch_add(1, Ordering::SeqCst);
    
    crate::serial::_print(format_args!(
        "[PerCPU] Added CPU {} (APIC ID {})\n",
        cpu_id, apic_id
    ));
    
    Ok(())
}

/// Get per-CPU data for the current CPU
pub fn current_cpu_data() -> Option<&'static PerCpuData> {
    let cpu_id = arch::get_current_cpu_id();
    get_cpu_data(cpu_id)
}

/// Get per-CPU data for a specific CPU
pub fn get_cpu_data(cpu_id: u32) -> Option<&'static PerCpuData> {
    if cpu_id as usize >= MAX_CPUS {
        return None;
    }
    
    unsafe {
        PER_CPU_DATA[cpu_id as usize].as_ref()
    }
}

/// Get mutable per-CPU data for a specific CPU (unsafe)
pub unsafe fn get_cpu_data_mut(cpu_id: u32) -> Option<&'static mut PerCpuData> {
    if cpu_id as usize >= MAX_CPUS {
        return None;
    }
    
    PER_CPU_DATA[cpu_id as usize].as_mut()
}

/// Get the number of online CPUs
pub fn get_cpu_count() -> u32 {
    CPU_COUNT.load(Ordering::SeqCst)
}

/// Get statistics for all CPUs
pub fn get_all_cpu_stats() -> Vec<CpuStats> {
    let mut stats = Vec::new();
    let cpu_count = get_cpu_count();
    
    for cpu_id in 0..cpu_count {
        if let Some(cpu_data) = get_cpu_data(cpu_id) {
            stats.push(cpu_data.get_stats());
        }
    }
    
    stats
}

/// Mark a CPU as online
pub fn set_cpu_online(cpu_id: u32) {
    if let Some(cpu_data) = get_cpu_data(cpu_id) {
        cpu_data.set_online();
        crate::serial::_print(format_args!("[PerCPU] CPU {} is now online\n", cpu_id));
    }
}

/// Mark a CPU as offline
pub fn set_cpu_offline(cpu_id: u32) {
    if let Some(cpu_data) = get_cpu_data(cpu_id) {
        cpu_data.set_offline();
        crate::serial::_print(format_args!("[PerCPU] CPU {} is now offline\n", cpu_id));
    }
}

/// Get the current CPU ID
pub fn current_cpu_id() -> u32 {
    arch::get_current_cpu_id()
}

/// Update current process for the current CPU
pub fn set_current_process(pid: ProcessId) {
    if let Some(cpu_data) = current_cpu_data() {
        cpu_data.set_current_process(pid);
    }
}

/// Get current process for the current CPU
pub fn get_current_process() -> ProcessId {
    if let Some(cpu_data) = current_cpu_data() {
        cpu_data.get_current_process()
    } else {
        0 // Default to process 0 if no per-CPU data
    }
}

/// Increment context switch counter for current CPU
pub fn inc_context_switches() {
    if let Some(cpu_data) = current_cpu_data() {
        cpu_data.inc_context_switches();
    }
}

/// Increment interrupt counter for current CPU
pub fn inc_interrupts() {
    if let Some(cpu_data) = current_cpu_data() {
        cpu_data.inc_interrupts();
    }
}

/// Add CPU time accounting
pub fn add_cpu_time(user_ticks: u64, kernel_ticks: u64, idle_ticks: u64) {
    if let Some(cpu_data) = current_cpu_data() {
        cpu_data.add_user_ticks(user_ticks);
        cpu_data.add_kernel_ticks(kernel_ticks);
        cpu_data.add_idle_ticks(idle_ticks);
    }
}

/// Check if per-CPU subsystem is initialized
pub fn is_initialized() -> bool {
    PERCPU_INITIALIZED.load(Ordering::SeqCst)
}