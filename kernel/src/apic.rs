//! Advanced Programmable Interrupt Controller (APIC) support for RaeenOS
//! Implements Local APIC, x2APIC, and I/O APIC functionality for SMP systems

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::instructions::port::Port;
use x86_64::{PhysAddr, VirtAddr};
use x86_64::structures::paging::{Mapper, PhysFrame, Size4KiB};
use alloc::vec::Vec;

use crate::arch::{has_cpu_feature, detect_cpu_info, CpuFeature, tsc};

/// APIC register offsets
const APIC_ID: u32 = 0x020;
const APIC_VERSION: u32 = 0x030;
const _APIC_TPR: u32 = 0x080;
const _APIC_APR: u32 = 0x090;
const _APIC_PPR: u32 = 0x0A0;
const APIC_EOI: u32 = 0x0B0;
const _APIC_RRD: u32 = 0x0C0;
const _APIC_LDR: u32 = 0x0D0;
const _APIC_DFR: u32 = 0x0E0;
const APIC_SPURIOUS: u32 = 0x0F0;
const APIC_ESR: u32 = 0x280;
const APIC_ICRL: u32 = 0x300;
const APIC_ICRH: u32 = 0x310;
const APIC_LVT_TIMER: u32 = 0x320;
const APIC_LVT_THERMAL: u32 = 0x330;
const APIC_LVT_PERF: u32 = 0x340;
const APIC_LVT_LINT0: u32 = 0x350;
const APIC_LVT_LINT1: u32 = 0x360;
const APIC_LVT_ERROR: u32 = 0x370;
const APIC_TIMER_ICR: u32 = 0x380;
const APIC_TIMER_CCR: u32 = 0x390;
const APIC_TIMER_DCR: u32 = 0x3E0;

/// x2APIC MSR addresses
const X2APIC_APICID: u32 = 0x802;
const _X2APIC_VERSION: u32 = 0x803;
const _X2APIC_TPR: u32 = 0x808;
const _X2APIC_PPR: u32 = 0x80A;
const _X2APIC_EOI: u32 = 0x80B;
const _X2APIC_LDR: u32 = 0x80D;
const _X2APIC_SPURIOUS: u32 = 0x80F;
const _X2APIC_ESR: u32 = 0x828;
const X2APIC_ICR: u32 = 0x830;
const _X2APIC_LVT_TIMER: u32 = 0x832;
const _X2APIC_LVT_THERMAL: u32 = 0x833;
const _X2APIC_LVT_PERF: u32 = 0x834;
const _X2APIC_LVT_LINT0: u32 = 0x835;
const _X2APIC_LVT_LINT1: u32 = 0x836;
const _X2APIC_LVT_ERROR: u32 = 0x837;
const _X2APIC_TIMER_ICR: u32 = 0x838;
const _X2APIC_TIMER_CCR: u32 = 0x839;
const _X2APIC_TIMER_DCR: u32 = 0x83E;

/// APIC timer modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApicTimerMode {
    OneShot = 0,
    Periodic = 1,
    TscDeadline = 2,
}

/// APIC delivery modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeliveryMode {
    Fixed = 0,
    LowestPriority = 1,
    Smi = 2,
    Nmi = 4,
    Init = 5,
    StartUp = 6,
    ExtInt = 7,
}

/// APIC destination modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DestinationMode {
    Physical = 0,
    Logical = 1,
}

/// Local APIC controller
#[derive(Debug)]
pub struct LocalApic {
    base_addr: VirtAddr,
    x2apic_enabled: bool,
    apic_id: u32,
    version: u32,
    max_lvt: u32,
    tsc_deadline_supported: bool,
    timer_frequency: AtomicU64,
}

impl LocalApic {
    /// Create a new Local APIC instance
    pub fn new() -> Self {
        Self {
            base_addr: VirtAddr::new(0),
            x2apic_enabled: false,
            apic_id: 0,
            version: 0,
            max_lvt: 0,
            tsc_deadline_supported: false,
            timer_frequency: AtomicU64::new(0),
        }
    }
    
    /// Initialize the Local APIC
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Check if APIC is supported
        if !has_cpu_feature(CpuFeature::Apic) {
            return Err("APIC not supported");
        }
        
        // Check for x2APIC support
        self.x2apic_enabled = has_cpu_feature(CpuFeature::X2apic);
        self.tsc_deadline_supported = has_cpu_feature(CpuFeature::TscDeadline);
        
        if self.x2apic_enabled {
            self.init_x2apic()?;
        } else {
            self.init_xapic()?;
        }
        
        // Read APIC version and capabilities
        self.version = self.read_register(APIC_VERSION);
        self.max_lvt = ((self.version >> 16) & 0xFF) + 1;
        
        // Enable APIC
        self.enable();
        
        // Initialize timer
        self.init_timer()?;
        
        Ok(())
    }
    
    /// Initialize x2APIC mode
    fn init_x2apic(&mut self) -> Result<(), &'static str> {
        // Enable x2APIC mode
        unsafe {
            use x86_64::registers::model_specific::Msr;
            let mut apic_base = Msr::new(0x1B).read();
            apic_base |= (1 << 10) | (1 << 11); // Enable x2APIC and APIC
            Msr::new(0x1B).write(apic_base);
        }
        
        // Read APIC ID from x2APIC MSR
        self.apic_id = self.read_x2apic_msr(X2APIC_APICID);
        
        Ok(())
    }
    
    /// Initialize xAPIC mode (memory-mapped)
    fn init_xapic(&mut self) -> Result<(), &'static str> {
        // Get APIC base address from MSR
        let apic_base_msr = unsafe {
            use x86_64::registers::model_specific::Msr;
            Msr::new(0x1B).read()
        };
        
        let apic_base_phys = PhysAddr::new(apic_base_msr & 0xFFFFF000);
        
        // Map APIC registers to virtual memory
        self.base_addr = crate::memory::with_mapper(|mapper| {
            use x86_64::structures::paging::{Page, PageTableFlags};
            use x86_64::VirtAddr;
            
            let virt_addr = VirtAddr::new(0xFFFF_8000_0000_0000 + apic_base_phys.as_u64());
            let page: Page<Size4KiB> = Page::containing_address(virt_addr);
            let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(apic_base_phys);
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
            
            crate::memory::with_frame_allocator(|frame_allocator| {
                unsafe {
                    mapper.map_to(page, frame, flags, frame_allocator)
                        .map_err(|_| "Failed to map APIC registers")?
                        .flush();
                }
                Ok::<(), &str>(())
            })?;
            Ok::<VirtAddr, &str>(virt_addr)
        }).map_err(|_| "Failed to map APIC registers")?;
        
        // Enable APIC in MSR
        unsafe {
            use x86_64::registers::model_specific::Msr;
            let mut apic_base = Msr::new(0x1B).read();
            apic_base |= 1 << 11; // Enable APIC
            Msr::new(0x1B).write(apic_base);
        }
        
        // Read APIC ID
        self.apic_id = (self.read_register(APIC_ID) >> 24) & 0xFF;
        
        Ok(())
    }
    
    /// Enable the Local APIC
    fn enable(&mut self) {
        // Set spurious interrupt vector and enable APIC
        let spurious = 0x100 | 0xFF; // Enable APIC + spurious vector 0xFF
        self.write_register(APIC_SPURIOUS, spurious);
        
        // Clear error status
        self.write_register(APIC_ESR, 0);
        self.read_register(APIC_ESR);
        
        // Mask all LVT entries initially
        self.write_register(APIC_LVT_TIMER, 0x10000); // Masked
        self.write_register(APIC_LVT_LINT0, 0x10000); // Masked
        self.write_register(APIC_LVT_LINT1, 0x10000); // Masked
        self.write_register(APIC_LVT_ERROR, 0x10000); // Masked
        
        if self.max_lvt >= 4 {
            self.write_register(APIC_LVT_PERF, 0x10000); // Masked
        }
        if self.max_lvt >= 5 {
            self.write_register(APIC_LVT_THERMAL, 0x10000); // Masked
        }
    }
    
    /// Initialize APIC timer
    fn init_timer(&mut self) -> Result<(), &'static str> {
        if self.tsc_deadline_supported && tsc::is_invariant_available() {
            // Use TSC-deadline mode for high precision
            self.write_register(APIC_LVT_TIMER, 0x20 | (2 << 17)); // Vector 0x20, TSC-deadline mode
            
            // Use TSC frequency from arch module
            let tsc_freq = tsc::get_frequency();
            if tsc_freq > 0 {
                self.timer_frequency.store(tsc_freq, Ordering::SeqCst);
                crate::serial::_print(format_args!("[APIC] TSC-deadline timer initialized with frequency: {} Hz\n", tsc_freq));
            } else {
                return Err("TSC frequency not available");
            }
        } else {
            // Use periodic mode
            self.write_register(APIC_TIMER_DCR, 0x3); // Divide by 16
            self.write_register(APIC_LVT_TIMER, 0x20 | (1 << 17)); // Vector 0x20, periodic mode
            self.calibrate_timer_frequency();
        }
        
        Ok(())
    }
    

    
    /// Calibrate APIC timer frequency
    fn calibrate_timer_frequency(&mut self) {
        // Set initial count to maximum
        self.write_register(APIC_TIMER_ICR, 0xFFFFFFFF);
        
        // Wait 10ms using PIT
        self.pit_delay_ms(10);
        
        // Read current count
        let current_count = self.read_register(APIC_TIMER_CCR);
        let ticks_per_10ms = 0xFFFFFFFF - current_count;
        let frequency = ticks_per_10ms * 100; // 10ms -> 1s
        
        self.timer_frequency.store(frequency as u64, Ordering::SeqCst);
    }
    
    /// Delay using PIT for calibration
    fn pit_delay_ms(&self, ms: u32) {
        let target_ticks = ms * 1000; // PIT runs at ~1MHz after divider
        
        unsafe {
            // Configure PIT channel 2 for one-shot
            let mut cmd_port = Port::new(0x43);
            let mut data_port = Port::new(0x42);
            let mut gate_port = Port::new(0x61);
            
            cmd_port.write(0xB0u8); // Channel 2, lobyte/hibyte, one-shot
            
            // Set count
            data_port.write((target_ticks & 0xFF) as u8);
            data_port.write((target_ticks >> 8) as u8);
            
            // Enable gate
            let gate_val: u8 = gate_port.read() | 0x01;
            gate_port.write(gate_val);
            
            // Wait for completion
            while (gate_port.read() & 0x20) == 0 {
                core::hint::spin_loop();
            }
        }
    }
    
    /// Read APIC register
    fn read_register(&self, offset: u32) -> u32 {
        if self.x2apic_enabled {
            self.read_x2apic_msr(0x800 + (offset >> 4))
        } else {
            unsafe {
                let addr = self.base_addr.as_u64() + offset as u64;
                core::ptr::read_volatile(addr as *const u32)
            }
        }
    }
    
    /// Write APIC register
    fn write_register(&self, offset: u32, value: u32) {
        if self.x2apic_enabled {
            self.write_x2apic_msr(0x800 + (offset >> 4), value as u64);
        } else {
            unsafe {
                let addr = self.base_addr.as_u64() + offset as u64;
                core::ptr::write_volatile(addr as *mut u32, value);
            }
        }
    }
    
    /// Read x2APIC MSR
    fn read_x2apic_msr(&self, msr: u32) -> u32 {
        unsafe {
            use x86_64::registers::model_specific::Msr;
            Msr::new(msr).read() as u32
        }
    }
    
    /// Write x2APIC MSR
    fn write_x2apic_msr(&self, msr: u32, value: u64) {
        unsafe {
            use x86_64::registers::model_specific::Msr;
            Msr::new(msr).write(value);
        }
    }
    
    /// Send End of Interrupt
    pub fn send_eoi(&self) {
        self.write_register(APIC_EOI, 0);
    }
    
    /// Get APIC ID
    pub fn get_apic_id(&self) -> u32 {
        self.apic_id
    }
    
    /// Set timer for one-shot mode
    pub fn set_timer_oneshot(&self, microseconds: u64) {
        if self.tsc_deadline_supported && tsc::is_invariant_available() {
            // Use arch module TSC deadline timer
            tsc::deadline::set_deadline_us(microseconds);
        } else {
            let freq = self.timer_frequency.load(Ordering::SeqCst);
            let count = (freq * microseconds) / 1_000_000;
            
            self.write_register(APIC_LVT_TIMER, 0x20); // Vector 0x20, one-shot
            self.write_register(APIC_TIMER_ICR, count as u32);
        }
    }
    
    /// Set timer for periodic mode
    pub fn set_timer_periodic(&self, frequency_hz: u32) {
        if self.tsc_deadline_supported && tsc::is_invariant_available() {
            // TSC-deadline doesn't support periodic mode directly
            // The time module will handle reprogramming in the interrupt handler
            let interval_us = 1_000_000 / frequency_hz as u64;
            tsc::deadline::set_deadline_us(interval_us);
        } else {
            let apic_freq = self.timer_frequency.load(Ordering::SeqCst);
            let count = apic_freq / frequency_hz as u64;
            
            self.write_register(APIC_LVT_TIMER, 0x20 | (1 << 17)); // Vector 0x20, periodic
            self.write_register(APIC_TIMER_ICR, count as u32);
        }
    }
    
    /// Send Inter-Processor Interrupt
    pub fn send_ipi(&self, target_apic_id: u32, vector: u8, delivery_mode: DeliveryMode) {
        if self.x2apic_enabled {
            let icr = (target_apic_id as u64) << 32 | 
                     (delivery_mode as u64) << 8 | 
                     vector as u64;
            self.write_x2apic_msr(X2APIC_ICR, icr);
        } else {
            // Write high part first (destination)
            self.write_register(APIC_ICRH, target_apic_id << 24);
            
            // Write low part (vector and delivery mode)
            let icr_low = (delivery_mode as u32) << 8 | vector as u32;
            self.write_register(APIC_ICRL, icr_low);
        }
    }
    
    /// Send INIT IPI to all processors except self
    pub fn send_init_ipi_all_excluding_self(&self) {
        if self.x2apic_enabled {
            let icr = (3u64 << 18) | (DeliveryMode::Init as u64) << 8; // All excluding self
            self.write_x2apic_msr(X2APIC_ICR, icr);
        } else {
            self.write_register(APIC_ICRH, 0);
            let icr_low = (3 << 18) | (DeliveryMode::Init as u32) << 8; // All excluding self
            self.write_register(APIC_ICRL, icr_low);
        }
    }
    
    /// Send STARTUP IPI
    pub fn send_startup_ipi(&self, target_apic_id: u32, start_page: u8) {
        if self.x2apic_enabled {
            let icr = (target_apic_id as u64) << 32 | 
                     (DeliveryMode::StartUp as u64) << 8 | 
                     start_page as u64;
            self.write_x2apic_msr(X2APIC_ICR, icr);
        } else {
            self.write_register(APIC_ICRH, target_apic_id << 24);
            let icr_low = (DeliveryMode::StartUp as u32) << 8 | start_page as u32;
            self.write_register(APIC_ICRL, icr_low);
        }
    }
}

/// I/O APIC controller for handling external interrupts
#[derive(Debug)]
pub struct IoApic {
    base_addr: VirtAddr,
    id: u8,
    version: u8,
    max_redirection_entries: u8,
}

impl IoApic {
    /// Create a new I/O APIC instance
    pub fn new(base_addr: PhysAddr) -> Result<Self, &'static str> {
        let virt_addr = crate::memory::with_mapper(|mapper| {
            use x86_64::structures::paging::{Page, PageTableFlags};
            use x86_64::VirtAddr;
            
            let virt_addr = VirtAddr::new(0xFFFF_8000_0000_0000 + base_addr.as_u64());
            let page: Page<Size4KiB> = Page::containing_address(virt_addr);
            let frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(base_addr);
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
            
            crate::memory::with_frame_allocator(|frame_allocator| {
                unsafe {
                    mapper.map_to(page, frame, flags, frame_allocator)
                        .map_err(|_| "Failed to map I/O APIC registers")?
                        .flush();
                }
                Ok::<(), &str>(())
            })?;
            Ok::<VirtAddr, &str>(virt_addr)
        }).map_err(|_| "Failed to map I/O APIC registers")?;
        
        let mut ioapic = Self {
            base_addr: virt_addr,
            id: 0,
            version: 0,
            max_redirection_entries: 0,
        };
        
        // Read I/O APIC information
        let id_reg = ioapic.read_register(0x00);
        ioapic.id = ((id_reg >> 24) & 0x0F) as u8;
        
        let version_reg = ioapic.read_register(0x01);
        ioapic.version = (version_reg & 0xFF) as u8;
        ioapic.max_redirection_entries = ((version_reg >> 16) & 0xFF) as u8;
        
        Ok(ioapic)
    }
    
    /// Read I/O APIC register
    fn read_register(&self, register: u8) -> u32 {
        unsafe {
            // Write register index
            let index_addr = self.base_addr.as_u64();
            core::ptr::write_volatile(index_addr as *mut u32, register as u32);
            
            // Read data
            let data_addr = self.base_addr.as_u64() + 0x10;
            core::ptr::read_volatile(data_addr as *const u32)
        }
    }
    
    /// Write I/O APIC register
    fn write_register(&self, register: u8, value: u32) {
        unsafe {
            // Write register index
            let index_addr = self.base_addr.as_u64();
            core::ptr::write_volatile(index_addr as *mut u32, register as u32);
            
            // Write data
            let data_addr = self.base_addr.as_u64() + 0x10;
            core::ptr::write_volatile(data_addr as *mut u32, value);
        }
    }
    
    /// Configure redirection entry
    pub fn set_redirection_entry(&self, irq: u8, vector: u8, target_apic_id: u32, 
                                 delivery_mode: DeliveryMode, dest_mode: DestinationMode) {
        if irq > self.max_redirection_entries {
            return;
        }
        
        let entry_low = vector as u32 | 
                       (delivery_mode as u32) << 8 | 
                       (dest_mode as u32) << 11;
        
        let entry_high = target_apic_id << 24;
        
        // Write redirection entry (each entry is 64-bit, split into two 32-bit registers)
        self.write_register(0x10 + irq * 2, entry_low);
        self.write_register(0x10 + irq * 2 + 1, entry_high);
    }
    
    /// Mask IRQ
    pub fn mask_irq(&self, irq: u8) {
        if irq > self.max_redirection_entries {
            return;
        }
        
        let reg = 0x10 + irq * 2;
        let current = self.read_register(reg);
        self.write_register(reg, current | (1 << 16)); // Set mask bit
    }
    
    /// Unmask IRQ
    pub fn unmask_irq(&self, irq: u8) {
        if irq > self.max_redirection_entries {
            return;
        }
        
        let reg = 0x10 + irq * 2;
        let current = self.read_register(reg);
        self.write_register(reg, current & !(1 << 16)); // Clear mask bit
    }
}

/// SMP (Symmetric Multi-Processing) controller
#[derive(Debug)]
pub struct SmpController {
    local_apic: LocalApic,
    io_apics: Vec<IoApic>,
    _cpu_count: AtomicU32,
    online_cpus: AtomicU32,
}

impl SmpController {
    /// Create a new SMP controller
    pub fn new() -> Self {
        Self {
            local_apic: LocalApic::new(),
            io_apics: Vec::new(),
            _cpu_count: AtomicU32::new(1), // BSP
            online_cpus: AtomicU32::new(1),
        }
    }
    
    /// Initialize SMP system
    pub fn init(&mut self) -> Result<(), &'static str> {
        // Initialize Local APIC
        self.local_apic.init()?;
        
        // Discover and initialize I/O APICs
        self.discover_io_apics()?;
        
        // Start Application Processors (APs)
        self.start_application_processors()?;
        
        Ok(())
    }
    
    /// Discover I/O APICs from ACPI tables
    fn discover_io_apics(&mut self) -> Result<(), &'static str> {
        // TODO: Parse ACPI MADT table to find I/O APICs
        // For now, assume standard I/O APIC at 0xFEC00000
        let io_apic = IoApic::new(PhysAddr::new(0xFEC00000))?;
        self.io_apics.push(io_apic);
        
        Ok(())
    }
    
    /// Start Application Processors
    fn start_application_processors(&mut self) -> Result<(), &'static str> {
        let cpu_info = detect_cpu_info();
        let total_cpus = cpu_info.logical_cores;
        
        if total_cpus <= 1 {
            return Ok(()); // Single CPU system
        }
        
        // Send INIT IPI to all APs
        self.local_apic.send_init_ipi_all_excluding_self();
        
        // Wait 10ms
        self.delay_ms(10);
        
        // Send STARTUP IPI twice (as per Intel specification)
        for _ in 0..2 {
            // TODO: Set up AP startup code at physical address 0x8000
            self.local_apic.send_startup_ipi(0xFF, 0x08); // Broadcast to all APs, start at 0x8000
            self.delay_ms(1);
        }
        
        // Wait for APs to come online
        self.wait_for_aps(total_cpus);
        
        Ok(())
    }
    
    /// Wait for Application Processors to come online
    fn wait_for_aps(&self, expected_cpus: u32) {
        let timeout = 1000; // 1 second timeout
        for _ in 0..timeout {
            if self.online_cpus.load(Ordering::SeqCst) >= expected_cpus {
                break;
            }
            self.delay_ms(1);
        }
    }
    
    /// Simple delay function
    fn delay_ms(&self, ms: u32) {
        // Use APIC timer for delay
        self.local_apic.set_timer_oneshot(ms as u64 * 1000);
        
        // Wait for timer interrupt
        // TODO: Implement proper wait mechanism
        for _ in 0..(ms * 1000) {
            core::hint::spin_loop();
        }
    }
    
    /// Get Local APIC reference
    pub fn local_apic(&self) -> &LocalApic {
        &self.local_apic
    }
    
    /// Get Local APIC mutable reference
    pub fn local_apic_mut(&mut self) -> &mut LocalApic {
        &mut self.local_apic
    }
    
    /// Get I/O APIC reference
    pub fn io_apic(&self, index: usize) -> Option<&IoApic> {
        self.io_apics.get(index)
    }
    
    /// Register CPU as online
    pub fn register_cpu_online(&self) {
        self.online_cpus.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Get number of online CPUs
    pub fn get_online_cpu_count(&self) -> u32 {
        self.online_cpus.load(Ordering::SeqCst)
    }
}

// Global SMP controller instance
lazy_static! {
    static ref SMP_CONTROLLER: Mutex<SmpController> = Mutex::new(SmpController::new());
}

/// Initialize APIC and SMP system
pub fn init() -> Result<(), &'static str> {
    let mut controller = SMP_CONTROLLER.lock();
    controller.init()
}

/// Get SMP controller reference
pub fn get_smp_controller() -> &'static Mutex<SmpController> {
    &SMP_CONTROLLER
}

/// Check if APIC is enabled and initialized
pub fn is_apic_enabled() -> bool {
    let controller = SMP_CONTROLLER.lock();
    controller.local_apic().base_addr.as_u64() != 0
}

/// Send End of Interrupt to Local APIC
pub fn send_eoi() {
    let controller = SMP_CONTROLLER.lock();
    controller.local_apic().send_eoi();
}

/// Get current CPU's APIC ID
pub fn get_apic_id() -> u32 {
    let controller = SMP_CONTROLLER.lock();
    controller.local_apic().get_apic_id()
}

/// Set timer for one-shot mode
pub fn set_timer_oneshot(microseconds: u64) {
    let controller = SMP_CONTROLLER.lock();
    controller.local_apic().set_timer_oneshot(microseconds);
}

/// Set timer for periodic mode
pub fn set_timer_periodic(frequency_hz: u32) {
    let controller = SMP_CONTROLLER.lock();
    controller.local_apic().set_timer_periodic(frequency_hz);
}

/// APIC timer interrupt handler
pub extern "x86-interrupt" fn apic_timer_handler(_stack_frame: InterruptStackFrame) {
    // Handle timer interrupt
    crate::time::tick();
    crate::process::schedule_tick();
    
    // Send EOI
    send_eoi();
}

/// APIC spurious interrupt handler
pub extern "x86-interrupt" fn apic_spurious_handler(_stack_frame: InterruptStackFrame) {
    // Spurious interrupt - just send EOI
    send_eoi();
}

/// APIC error interrupt handler
pub extern "x86-interrupt" fn apic_error_handler(_stack_frame: InterruptStackFrame) {
    let controller = SMP_CONTROLLER.lock();
    let error_status = controller.local_apic().read_register(APIC_ESR);
    
    crate::serial_println!("APIC Error: 0x{:08X}", error_status);
    
    // Clear error status
    controller.local_apic().write_register(APIC_ESR, 0);
    
    send_eoi();
}