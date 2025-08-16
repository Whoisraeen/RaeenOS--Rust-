use spin::Mutex;
use uart_16550::SerialPort;
use core::fmt::Write;

// SAFETY: This is unsafe because:
// - 0x3F8 is the standard COM1 serial port I/O address on x86 systems
// - SerialPort::new requires unsafe because it accesses hardware I/O ports
// - The port address must be valid and not conflict with other hardware
// - Serial port access requires proper initialization before use
pub static SERIAL1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });

pub fn init() {
    let mut serial = SERIAL1.lock();
    serial.init();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = write!($crate::serial::SERIAL1.lock(), $($arg)*);
        }
    };
}

#[macro_export]
macro_rules! serial_println {
    () => { $crate::serial_print!("\n") };
    ($($arg:tt)*) => {
        $crate::serial_print!("{}\n", format_args!($($arg)*));
    };
}

pub fn _print(args: core::fmt::Arguments) {
    let _ = SERIAL1.lock().write_fmt(args);
}


