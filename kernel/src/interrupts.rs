use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use pic8259::ChainedPics;
use x86_64::instructions::port::Port;

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
        // idt.double_fault.set_handler_fn(double_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
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

extern "x86-interrupt" fn _double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
    crate::serial_println!("DOUBLE FAULT: {:?}", stack_frame);
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    let addr = Cr2::read();
    crate::serial_println!(
        "PAGE FAULT @ {:?} | error={:?} | frame={:?}",
        addr, error_code, stack_frame
    );
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
    let mut port = Port::new(0x60);
    // SAFETY: This is unsafe because:
    // - Reads from keyboard controller I/O port 0x60
    // - Port 0x60 is the standard keyboard data port on x86 systems
    // - Must be called from keyboard interrupt handler to clear the interrupt
    // - Reading clears the keyboard controller's output buffer
    let _scancode: u8 = unsafe { port.read() };
    
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


