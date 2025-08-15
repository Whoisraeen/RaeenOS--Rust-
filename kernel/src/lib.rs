#![no_std]
#![feature(abi_x86_interrupt)]

pub mod serial;
pub mod gdt;
pub mod memory;
pub mod heap;
pub mod interrupts;
pub mod vmm;
pub mod arch;
pub mod apic;
// pub mod uefi;
// Temporarily keep other complex modules disabled until compiled individually
pub mod time;
pub mod process;
pub mod syscall;
pub mod elf;
pub mod filesystem;
pub use filesystem as fs;
// pub mod drivers;
// pub mod network;
// pub mod ipc;
pub mod graphics;
pub mod vesa;
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
    
    // Initialize interrupts (legacy PIC mode for now)
    // TODO: Re-enable APIC initialization once compilation issues are resolved
    // if let Err(e) = apic::init() {
    //     crate::serial::_print(format_args!("[APIC] Failed to initialize: {}\n", e));
    //     // Fall back to legacy PIC mode
        interrupts::init();
    // } else {
    //     crate::serial::_print(format_args!("[APIC] Initialized successfully\n"));
    //     interrupts::init_with_apic();
    // }
    
    vmm::init();
    
    // Test VMM functionality (address space isolation and memory protection)
    if let Err(e) = vmm::run_vmm_tests() {
        crate::serial::_print(format_args!("[VMM] Tests failed: {}\n", e));
    }
    
    process::init();
    
    // Initialize threading system
    let idle_pid = process::init_idle_thread();
    let demo_pid = process::spawn_demo_thread();
    crate::serial::_print(format_args!("Initialized idle thread (PID: {}) and demo thread (PID: {})\n", idle_pid, demo_pid));
    
    syscall::init();
    
    // Initialize graphics - use VESA for now
    // TODO: Re-enable UEFI GOP initialization once compilation issues are resolved
    let graphics_initialized = false;
    // if let Some(framebuffer_info) = uefi::get_framebuffer_info() {
    //     crate::serial::_print(format_args!("[UEFI] Using GOP framebuffer {}x{} at {:?}\n", 
    //         framebuffer_info.width, framebuffer_info.height, framebuffer_info.base_addr));
    //     
    //     // Initialize framebuffer compositor with UEFI GOP framebuffer
    //     let pitch = framebuffer_info.pixels_per_scanline * 4; // Assume 32-bit pixels
    //     match graphics::init_framebuffer_compositor(
    //         framebuffer_info.base_addr.as_u64() as *mut u8,
    //         framebuffer_info.width,
    //         framebuffer_info.height,
    //         pitch,
    //         32 // Assume 32-bit color depth
    //     ) {
    //         Ok(_) => {
    //             crate::serial::_print(format_args!("[Graphics] UEFI GOP framebuffer compositor initialized\n"));
    //             true
    //         }
    //         Err(e) => {
    //             crate::serial::_print(format_args!("[Graphics] Failed to initialize UEFI GOP compositor: {}\n", e));
    //             false
    //         }
    //     }
    // } else {
    //     false
    // };
    
    // Fall back to VESA if UEFI GOP is not available
    if !graphics_initialized {
        if let Ok(framebuffer) = vesa::init() {
            crate::serial::_print(format_args!("[VESA] Initialized {}x{}x{} framebuffer at {:?}\n", 
                framebuffer.width, framebuffer.height, framebuffer.bpp, framebuffer.address));
            
            // Initialize framebuffer compositor with VESA framebuffer
            if let Err(e) = graphics::init_framebuffer_compositor(
                framebuffer.address,
                framebuffer.width,
                framebuffer.height,
                framebuffer.pitch,
                framebuffer.bpp
            ) {
                crate::serial::_print(format_args!("[Graphics] Failed to initialize framebuffer compositor: {}\n", e));
            } else {
                crate::serial::_print(format_args!("[Graphics] VESA framebuffer compositor initialized\n"));
                
                // Clear screen to black
                let _ = vesa::clear_screen(0x000000);
            }
        } else {
            crate::serial::_print(format_args!("[Graphics] Failed to initialize graphics mode, using text mode fallback\n"));
            // Initialize basic graphics without framebuffer
            let _ = graphics::init_graphics(80, 25);
        }
    }
    
    // Create a demo window to test the framebuffer compositor
    if let Ok(window_id) = graphics::create_demo_window() {
        crate::serial::_print(format_args!("[Graphics] Created demo window (ID: {})\n", window_id));
    }
    
    // Test the graphics system by rendering a frame
    graphics::render_frame();
    
    crate::serial::_print(format_args!("[Graphics] Initial frame rendered\n"));
    
    // other subsystems init later
}

/// Launch the desktop environment and start the main system loop
pub fn launch_desktop_environment() -> ! {
    crate::serial::_print(format_args!("[Desktop] Starting RaeenOS Desktop Environment...\n"));
    
    // Start boot animation
    show_boot_animation();
    
    // Initialize input system
    if let Err(e) = init_input_system() {
        crate::serial::_print(format_args!("[Desktop] Failed to initialize input system: {}\n", e));
    }
    
    // Launch shell in a new process
    if let Err(e) = launch_shell() {
        crate::serial::_print(format_args!("[Desktop] Failed to launch shell: {}\n", e));
    }
    
    // Start the main desktop event loop
    desktop_main_loop();
}

/// Show boot animation during startup
fn show_boot_animation() {
    crate::serial::_print(format_args!("[Boot] Displaying boot animation...\n"));
    
    // Simple boot animation - fade in RaeenOS logo
    for frame in 0..30 {
        // Clear screen with gradient background
        let color = 0x001122 + (frame * 0x000811); // Gradual blue fade
        let _ = vesa::clear_screen(color);
        
        // Draw RaeenOS logo text in center
        if let Ok(window_id) = graphics::create_boot_window() {
            graphics::render_frame();
        }
        
        // Simple delay for animation timing
        for _ in 0..100000 {
            core::hint::spin_loop();
        }
    }
    
    crate::serial::_print(format_args!("[Boot] Boot animation complete\n"));
}

/// Initialize input system for keyboard and mouse
fn init_input_system() -> Result<(), &'static str> {
    crate::serial::_print(format_args!("[Input] Initializing input system...\n"));
    
    // Initialize keyboard driver
    drivers::keyboard::init();
    
    // Initialize PS/2 mouse driver
    let mouse = drivers::Ps2MouseDriver::new();
    match drivers::register_device(alloc::boxed::Box::new(mouse)) {
        Ok(mouse_id) => {
            crate::serial::_print(format_args!("[Input] Registered PS/2 mouse (ID: {})
", mouse_id));
        }
        Err(_) => {
            return Err("Failed to register PS/2 mouse");
        }
    }
    
    Ok(())
}

/// Launch the RaeShell in a new process
fn launch_shell() -> Result<(), &'static str> {
    crate::serial::_print(format_args!("[Desktop] Launching RaeShell...\n"));
    
    // Create a new shell session
    let session_id = raeshell::create_shell_session().map_err(|_| "Failed to create shell session")?;
    crate::serial::_print(format_args!("[Desktop] Created shell session (ID: {})\n", session_id));
    
    // Create shell window
    if let Ok(window_id) = graphics::create_shell_window() {
        crate::serial::_print(format_args!("[Desktop] Created shell window (ID: {})\n", window_id));
    }
    
    Ok(())
}

/// Main desktop event loop - handles input and window management
fn desktop_main_loop() -> ! {
    crate::serial::_print(format_args!("[Desktop] Starting main event loop...\n"));
    
    loop {
        // Process input events
        process_input_events();
        
        // Update window manager
        graphics::update_window_manager();
        
        // Render frame if needed
        graphics::render_frame();
        
        // Yield to other processes
        process::yield_current();
        
        // Small delay to prevent busy waiting
        for _ in 0..1000 {
            core::hint::spin_loop();
        }
    }
}

/// Process keyboard and mouse input events with enhanced routing
fn process_input_events() {
    // Process keyboard events with improved handling
    while let Some(key) = drivers::keyboard::get_key() {
        let pressed = (key & 0x80) == 0; // Check if key is pressed or released
        let key_code = (key & 0x7F) as u32; // Remove press/release bit
        
        // Enhanced keyboard event routing
        route_keyboard_event(key_code, pressed);
    }
    
    // Process mouse events with improved tracking
    if let Some((x, y, buttons)) = drivers::mouse::get_mouse_state() {
        // Enhanced mouse state tracking
        static mut LAST_MOUSE_STATE: (i32, i32, u8) = (0, 0, 0);
        unsafe {
            let (last_x, last_y, last_buttons) = LAST_MOUSE_STATE;
            
            // Handle mouse movement
            if x != last_x || y != last_y {
                route_mouse_move_event(x, y, last_x, last_y);
            }
            
            // Handle button state changes
            if buttons != last_buttons {
                for button in 0..3 {
                    let button_mask = 1 << button;
                    let was_pressed = (last_buttons & button_mask) != 0;
                    let is_pressed = (buttons & button_mask) != 0;
                    
                    if was_pressed != is_pressed {
                        route_mouse_button_event(x, y, button, is_pressed);
                    }
                }
            }
            
            LAST_MOUSE_STATE = (x, y, buttons);
        }
    }
}

/// Enhanced keyboard event routing with focus management
fn route_keyboard_event(key_code: u32, pressed: bool) {
    // Handle global hotkeys first
    if pressed {
        match key_code {
            0x3B => { // F1 - Show help
                graphics::show_help_overlay();
                return;
            }
            0x3C => { // F2 - Toggle performance overlay
                graphics::toggle_performance_overlay();
                return;
            }
            0x3D => { // F3 - Open task manager
                if let Ok(_) = graphics::create_task_manager_window() {
                    crate::serial::_print(format_args!("[Input] Opened task manager\n"));
                }
                return;
            }
            0x01 => { // ESC - Cancel current operation
                graphics::cancel_current_operation();
                return;
            }
            _ => {}
        }
    }
    
    // Route to focused window
    graphics::handle_keyboard_event(key_code, pressed);
}

/// Enhanced mouse movement event routing
fn route_mouse_move_event(x: i32, y: i32, last_x: i32, last_y: i32) {
    // Calculate movement delta
    let delta_x = x - last_x;
    let delta_y = y - last_y;
    
    // Update cursor position
    graphics::update_cursor_position(x, y);
    
    // Handle window dragging if in drag mode
    graphics::handle_window_drag(x, y, delta_x, delta_y);
    
    // Route hover events to windows
    graphics::handle_mouse_hover(x, y);
}

/// Enhanced mouse button event routing
fn route_mouse_button_event(x: i32, y: i32, button: u8, pressed: bool) {
    if pressed {
        // Handle window focus on click
        if let Some(window_id) = graphics::get_window_at_point(x, y) {
            let _ = graphics::set_input_focus(window_id);
        }
        
        // Check for window title bar clicks (for dragging)
        if button == 0 { // Left mouse button
            graphics::start_window_drag_if_title_bar(x, y);
        }
    } else {
        // Handle drag end
        if button == 0 {
            graphics::end_window_drag();
        }
    }
    
    // Route to appropriate window
    graphics::handle_mouse_event(x, y, button, pressed);
}


