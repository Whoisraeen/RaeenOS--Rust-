#![no_std]
#![feature(abi_x86_interrupt)]

pub mod serial;
pub mod gdt;
pub mod memory;
pub mod heap;
pub mod interrupts;
pub mod vmm;
pub mod arch;
// Temporarily keep other complex modules disabled until compiled individually
pub mod time;
pub mod process;
pub mod syscall;
pub mod filesystem;
// pub mod drivers;
// pub mod network;
// pub mod ipc;
pub mod graphics;
pub mod drivers;
pub mod network;
pub mod ipc;
pub mod ui;
pub mod sound;
pub mod security;
pub mod rae_assistant;
pub mod raeshell;
pub mod raepkg;
pub mod raede;
pub mod raekit;

extern crate alloc;

pub fn init(boot_info: &'static bootloader::BootInfo) {
    serial::init();
    gdt::init();
    let phys_offset = x86_64::VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_offset) };
    // Defer heap init until after we're sure bootloader finished its mappings
    let mut frame_alloc = unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };
    memory::set_physical_memory_offset(boot_info.physical_memory_offset);
    unsafe { memory::init_global_frame_allocator(&boot_info.memory_map) };
    let _ = heap::init_heap(&mut mapper, &mut frame_alloc);
    interrupts::init();
    vmm::init();
    process::init();
    
    // Initialize threading system
    let idle_pid = process::init_idle_thread();
    let demo_pid = process::spawn_demo_thread();
    crate::serial::_print(format_args!("Initialized idle thread (PID: {}) and demo thread (PID: {})\n", idle_pid, demo_pid));
    
    syscall::init();
    // other subsystems init later
}


