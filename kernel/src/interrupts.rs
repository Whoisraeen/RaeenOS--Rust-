use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use pic8259::ChainedPics;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// SAFETY: This is unsafe because:
// - ChainedPics::new accesses hardware I/O ports for PIC configuration
// - PIC_1_OFFSET and PIC_2_OFFSET must be valid interrupt vector offsets
// - This must only be called once during system initialization
// - The offsets must not conflict with CPU exception vectors (0-31)
pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Mouse.as_usize()].set_handler_fn(mouse_interrupt_handler);
        idt
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    Mouse,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 { self as u8 }
    fn as_usize(self) -> usize { usize::from(self.as_u8()) }
}

pub fn init() {
    IDT.load();
    // SAFETY: This is unsafe because:
    // - Initializes hardware PICs via I/O port access
    // - Must only be called once during system initialization
    // - Requires that interrupts are disabled during initialization
    // - PIC configuration affects global interrupt routing
    unsafe { PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

/// Initialize interrupts with APIC support
pub fn init_with_apic() {
    IDT.load();
    // Don't initialize legacy PICs when using APIC
    // APIC initialization handles interrupt routing
    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("INT3 breakpoint: {:?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    crate::serial::_print(format_args!(
        "[PANIC] DOUBLE FAULT (error: 0x{:x}):\n",
        error_code
    ));
    crate::serial::_print(format_args!(
        "  RIP: 0x{:x}\n  RSP: 0x{:x}\n",
        stack_frame.instruction_pointer.as_u64(),
        stack_frame.stack_pointer.as_u64()
    ));
    crate::serial::_print(format_args!(
        "  CS: 0x{:x}  SS: 0x{:x}  FLAGS: 0x{:x}\n",
        stack_frame.code_segment,
        stack_frame.stack_segment,
        stack_frame.cpu_flags
    ));
    
    // Print current process info if available
    if let Some(cpu_data) = crate::percpu::current_cpu_data() {
        let current_pid = cpu_data.get_current_process();
        crate::serial::_print(format_args!("  Current PID: {}\n", current_pid));
    }
    
    // Halt the system - double fault is generally unrecoverable
    crate::serial::_print(format_args!("System halted due to double fault\n"));
    loop { 
        x86_64::instructions::hlt(); 
    }
}

extern "x86-interrupt" fn device_not_available_handler(
    _stack_frame: InterruptStackFrame,
) {
    // Device Not Available (#NM) - FPU access when CR0.TS is set
    // This enables lazy FPU context switching
    
    // Clear the TS bit to allow FPU access
    crate::arch::fpu::disable_lazy_switching();
    
    // Mark the current process as having used FPU
    let current_pid = crate::percpu::get_current_process();
    if current_pid != 0 {
        // Check if there are any processes running
        if crate::process::get_process_count() > 0 {
            // We need mutable access, but we can't get it while holding the lock
            // For now, just disable lazy switching and let the process continue
            // TODO: Implement proper lazy FPU handling with per-process tracking
        }
    }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    use crate::vmm::VmError;
use alloc::format;
    use crate::process::get_current_process_id;
    
    
    let fault_addr = Cr2::read();
    
    // Check if this is a stack expansion request
      if !error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
          // Page not present - might be stack expansion
          let current_pid = get_current_process_id();
          let expansion_result = crate::vmm::handle_page_fault(fault_addr, error_code.bits());
          
          match expansion_result {
              Ok(()) => {
                  // Stack expansion successful, return to continue execution
                  return;
              }
              Err(VmError::StackOverflow) => {
                  // Stack overflow detected - trigger crash handler
                  crate::serial_println!(
                      "STACK OVERFLOW detected at {:?} for process {}",
                      fault_addr, current_pid
                  );
                  
                  // Report stack overflow to crash handler
                  let _ = crate::observability::with_observability_mut(|obs| {
                       obs.crash_handler.handle_crash(
                           crate::observability::crash_handler::CrashType::StackOverflow,
                           crate::observability::crash_handler::CrashSeverity::Critical,
                          Some(crate::observability::Subsystem::Memory),
                          Some(current_pid as u32),
                          None, // thread_id
                          Some(error_code.bits()),
                          Some(fault_addr.as_u64()),
                          &format!("Stack overflow at {:?}", fault_addr)
                      )
                  });
                  
                  // Terminate the current process
                  crate::process::terminate_current_process();
              }
              Err(_) => {
                  // Other VMM error - fall through to general page fault handling
              }
          }
      }
    
    // General page fault handling
    crate::serial_println!(
        "PAGE FAULT @ {:?} | error={:?} | frame={:?}",
        fault_addr, error_code, stack_frame
    );
    
    // Report general page fault to crash handler
    let _ = crate::observability::with_observability_mut(|obs| {
         obs.crash_handler.handle_crash(
             crate::observability::crash_handler::CrashType::PageFault,
             crate::observability::crash_handler::CrashSeverity::Error,
            Some(crate::observability::Subsystem::Memory),
            Some(get_current_process_id() as u32),
            None, // thread_id
            Some(error_code.bits()),
            Some(fault_addr.as_u64()),
            &format!("Page fault at {:?} with error {:?}", fault_addr, error_code)
        )
    });
    
    // For now, halt on unhandled page faults
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Tick time and schedule
    crate::time::tick();
    crate::process::schedule_tick();
    
    if crate::apic::is_apic_enabled() {
        crate::apic::send_eoi();
    } else {
        // SAFETY: This is unsafe because:
        // - Sends EOI (End of Interrupt) signal to PIC hardware via I/O ports
        // - Must only be called from within the corresponding interrupt handler
        // - Timer interrupt vector must match the configured PIC offset
        // - Required to re-enable further timer interrupts
        unsafe {
            PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
        }
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle keyboard interrupt using the new driver
    crate::drivers::keyboard::handle_interrupt();
    
    if crate::apic::is_apic_enabled() {
        crate::apic::send_eoi();
    } else {
        // SAFETY: This is unsafe because:
        // - Sends EOI (End of Interrupt) signal to PIC hardware via I/O ports
        // - Must only be called from within the corresponding interrupt handler
        // - Keyboard interrupt vector must match the configured PIC offset
        // - Required to re-enable further keyboard interrupts
        unsafe {
            PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
        }
    }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Handle mouse interrupt using the new driver
    crate::drivers::mouse::handle_interrupt();
    
    if crate::apic::is_apic_enabled() {
        crate::apic::send_eoi();
    } else {
        // SAFETY: This is unsafe because:
        // - Sends EOI (End of Interrupt) signal to PIC hardware via I/O ports
        // - Must only be called from within the corresponding interrupt handler
        // - Mouse interrupt vector must match the configured PIC offset
        // - Required to re-enable further mouse interrupts
        unsafe {
            PICS.lock().notify_end_of_interrupt(InterruptIndex::Mouse.as_u8());
        }
    }
}


