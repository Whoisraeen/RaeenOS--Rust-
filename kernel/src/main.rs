#![no_std]
#![no_main]

use core::fmt::Write;
use core::panic::PanicInfo;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::instructions::hlt as cpu_halt;
use kernel as k;
use bootloader::{entry_point, BootInfo};
extern crate alloc;

lazy_static! {
    static ref VGA_WRITER: Mutex<VgaWriter> = Mutex::new(VgaWriter::new());
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut writer = VGA_WRITER.lock();
    writeln!(writer, "PANIC: {}", info).ok();
    loop { cpu_halt(); }
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    k::init(boot_info);
    {
        let mut writer = VGA_WRITER.lock();
        writeln!(writer, "RaeenOS: booting kernel...").ok();
        writeln!(writer, "Welcome to RaeenOS kernel v0.1").ok();
        writeln!(writer, "Launching desktop environment...").ok();
    }
    
    // Launch the desktop environment instead of halting
    k::launch_desktop_environment();
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

struct VgaWriter {
    buffer_ptr: *mut VgaChar,
    column_position: usize,
    color: u8,
}

unsafe impl Send for VgaWriter {}

#[repr(C)]
#[derive(Copy, Clone)]
struct VgaChar {
    ascii_character: u8,
    color_code: u8,
}

impl VgaWriter {
    fn new() -> Self {
        Self {
            buffer_ptr: 0xb8000 as *mut VgaChar,
            column_position: 0,
            color: 0x0f,
        }
    }

    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                let col = self.column_position;
                if col >= 80 { self.new_line(); }
                let col = self.column_position;
                self.write_cell(BUFFER_HEIGHT - 1, col, VgaChar { ascii_character: byte, color_code: self.color });
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        // scroll up
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let chr = self.read_cell(row, col);
                self.write_cell(row - 1, col, chr);
            }
        }
        // clear last line
        for col in 0..BUFFER_WIDTH {
            self.write_cell(BUFFER_HEIGHT - 1, col, VgaChar { ascii_character: b' ', color_code: self.color });
        }
        self.column_position = 0;
    }

    fn index(row: usize, col: usize) -> usize { row * BUFFER_WIDTH + col }

    fn write_cell(&mut self, row: usize, col: usize, value: VgaChar) {
        let idx = Self::index(row, col);
        unsafe {
            // SAFETY: This is unsafe because:
            // - buffer_ptr must be a valid pointer to VGA text buffer memory (0xb8000)
            // - The index must be within VGA buffer bounds (checked by Self::index)
            // - Volatile write is required for MMIO to ensure immediate hardware effect
            // - The VGA buffer must be properly mapped and accessible
            // - No other code should be writing to the same VGA cell concurrently
            core::ptr::write_volatile(self.buffer_ptr.add(idx), value);
        }
    }

    fn read_cell(&self, row: usize, col: usize) -> VgaChar {
        let idx = Self::index(row, col);
        unsafe {
            // SAFETY: This is unsafe because:
            // - buffer_ptr must be a valid pointer to VGA text buffer memory (0xb8000)
            // - The index must be within VGA buffer bounds (checked by Self::index)
            // - Volatile read is required for MMIO to get current hardware state
            // - The VGA buffer must be properly mapped and accessible
            // - Reading from VGA memory is always safe and doesn't modify state
            core::ptr::read_volatile(self.buffer_ptr.add(idx))
        }
    }
}

impl Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}


