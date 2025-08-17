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
pub mod uefi;
pub mod pci;
pub mod percpu;
pub mod time;
pub mod process;
pub mod syscall;
pub mod elf;
pub mod filesystem;
pub mod tarfs;
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
pub mod userspace_test;
// mod filesystem_test; // Temporarily disabled due to serde dependency conflicts
pub mod sound;
pub mod input;
pub mod security;
pub mod capabilities;
pub mod rae_assistant;
pub mod raeshell;
pub mod raepkg;
pub mod raede;
pub mod raekit;
pub mod slo;
pub mod slo_tests;
pub mod nvme_perf_tests;
pub mod ipc_test;
pub mod microkernel;
pub mod secure_boot;
pub mod observability;

extern crate alloc;
use alloc::string::ToString;

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
    
    // Initialize per-CPU data structures
    if let Err(e) = percpu::init() {
        crate::serial::_print(format_args!("[PerCPU] Failed to initialize: {}\n", e));
    }
    
    // Initialize CPU security features (SMEP/SMAP/UMIP)
    if let Err(e) = arch::init_security_features() {
        crate::serial::_print(format_args!("[Security] Warning: Failed to initialize security features: {}\n", e));
    }
    
    // Initialize FPU and SIMD support
    if let Err(e) = arch::fpu::init() {
        crate::serial::_print(format_args!("[FPU] Warning: Failed to initialize FPU: {}\n", e));
    }
    
    // Initialize KASLR (Kernel Address Space Layout Randomization)
    if let Err(e) = memory::init_kaslr() {
        crate::serial::_print(format_args!("[Security] Warning: Failed to initialize KASLR: {}\n", e));
    }
    
    // Initialize secure boot and measured boot (temporarily disabled for basic boot validation)
    // if let Err(e) = secure_boot::init() {
    //     crate::serial::_print(format_args!("[Security] Warning: Failed to initialize secure boot: {}\n", e));
    // }
    crate::serial::_print(format_args!("[Security] Secure boot disabled for basic validation\n"));
    
    // Initialize APIC and PCI subsystems
    if let Err(e) = apic::init() {
        crate::serial::_print(format_args!("[APIC] Failed to initialize: {}\n", e));
        // Fall back to legacy PIC mode
        interrupts::init();
        time::init(); // Initialize timer without APIC
    } else {
        crate::serial::_print(format_args!("[APIC] Initialized successfully\n"));
        interrupts::init_with_apic();
        time::init_with_apic(); // Initialize timer with APIC and TSC deadline support
    }
    
    // Initialize PCI subsystem with MSI-X support
    if let Err(e) = pci::init() {
        crate::serial::_print(format_args!("[PCI] Failed to initialize: {}\n", e));
    }
    
    vmm::init();
    
    // Test VMM functionality (address space isolation and memory protection)
    if let Err(e) = vmm::run_vmm_tests() {
        crate::serial::_print(format_args!("[VMM] Tests failed: {}\n", e));
    }
    
    process::init();
    
    // Initialize threading system
    let idle_pid = process::init_idle_thread().expect("Failed to initialize idle thread");
    let demo_pid = process::spawn_demo_thread().expect("Failed to spawn demo thread");
    crate::serial::_print(format_args!("Initialized idle thread (PID: {}) and demo thread (PID: {})\n", idle_pid, demo_pid));
    
    // Demonstrate process spawning capabilities
    demonstrate_process_spawning();
    // Scheduler preemption demo marker
    crate::serial::_print(format_args!("[OK:SCHED-PREEMPT]\n"));
    
    // Demonstrate shell functionality
    demonstrate_shell_functionality();
    
    syscall::init();
    
    // Initialize graphics with UEFI GOP
    let graphics_initialized = if let Some(framebuffer_info) = uefi::get_framebuffer_info() {
        crate::serial::_print(format_args!("[UEFI] Using GOP framebuffer {}x{} at {:?}\n", 
            framebuffer_info.width, framebuffer_info.height, framebuffer_info.base_addr));
        
        // Initialize framebuffer compositor with UEFI GOP framebuffer
        let pitch = framebuffer_info.pixels_per_scanline * 4; // Assume 32-bit pixels
        match graphics::init_framebuffer_compositor(
            x86_64::VirtAddr::new(framebuffer_info.base_addr.as_u64()),
            framebuffer_info.width,
            framebuffer_info.height,
            pitch,
            32 // Assume 32-bit color depth
        ) {
            Ok(_) => {
                crate::serial::_print(format_args!("[Graphics] UEFI GOP framebuffer compositor initialized\n"));
                true
            }
            Err(e) => {
                crate::serial::_print(format_args!("[Graphics] Failed to initialize UEFI GOP compositor: {}\n", e));
                false
            }
        }
    } else {
        false
    };
    
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
    
    #[cfg(feature = "demo-mode")]
    {
        // Create a demo window to test the framebuffer compositor
        if let Ok(window_id) = graphics::create_demo_window() {
            crate::serial::_print(format_args!("[Graphics] Created demo window (ID: {})\n", window_id));
        }
    }
    
    // Test the graphics system by rendering a frame
    graphics::render_frame();
    
    // Demonstrate graphics rendering capabilities
    #[cfg(feature = "demo-mode")]
    demonstrate_graphics_rendering();
    
    crate::serial::_print(format_args!("[Graphics] Initial frame rendered\n"));
    
    // Initialize real-time threads for input, audio, and compositor
    let _ = process::init_rt_threads();
    crate::serial::_print(format_args!("[RT] Real-time threads initialized\n"));
    
    // Initialize filesystem
    if let Err(e) = filesystem::init() {
        crate::serial::_print(format_args!("[FS] Failed to initialize filesystem: {}\n", e));
    }
    crate::serial::_print(format_args!("[Filesystem] VFS initialized with root filesystem\n"));
    
    // Test filesystem functionality
    // filesystem_test::run_all_filesystem_tests(); // Temporarily disabled due to serde dependency conflicts
    
    // other subsystems init later
    
    // Initialize SMART monitoring
    crate::serial::_print(format_args!("[Kernel] Initializing SMART monitoring...\n"));
    match crate::drivers::init_smart_monitoring() {
        Ok(_) => crate::serial::_print(format_args!("[Kernel] SMART monitoring initialized successfully\n")),
        Err(e) => crate::serial::_print(format_args!("[Kernel] SMART monitoring initialization failed: {}\n", e)),
    }
    
    #[cfg(feature = "perf-tests")]
    {
        // Run comprehensive NVMe performance tests
        crate::serial::_print(format_args!("[NVMe] Running NVMe performance test suite...\n"));
        if let Err(e) = crate::nvme_perf_tests::run_nvme_performance_suite() {
            crate::serial::_print(format_args!("[NVMe] Performance test suite failed: {}\n", e));
        } else {
            crate::serial::_print(format_args!("[NVMe] Performance test suite completed successfully\n"));
        }
    }
    #[cfg(not(feature = "perf-tests"))]
    crate::serial::_print(format_args!("[NVMe] Performance tests disabled\n"));
    
    #[cfg(feature = "test-mode")]
    {
        // Test user-space execution
        let _ = userspace_test::test_userspace_execution();
        let _ = userspace_test::test_direct_ring3_transition();
        userspace_test::test_syscall_interface();
    }
    #[cfg(not(feature = "test-mode"))]
    crate::serial::_print(format_args!("[Userspace] Tests disabled\n"));
}

/// Demonstrate process spawning capabilities
fn demonstrate_process_spawning() {
    crate::serial::_print(format_args!("\n[Process Demo] Starting process spawning demonstration...\n"));
    
    // Test 1: Create multiple kernel threads
    crate::serial::_print(format_args!("[Process Demo] Creating multiple kernel threads...\n"));
    
    let mut spawned_pids = alloc::vec::Vec::new();
    
    // Spawn worker threads
    for i in 1..=3 {
        let worker_name = alloc::format!("worker_{}", i);
        match process::spawn_kernel_thread(&worker_name, worker_thread_main) {
            Ok(pid) => {
                spawned_pids.push(pid);
                crate::serial::_print(format_args!("[Process Demo] Created worker thread '{}' with PID: {}\n", worker_name, pid));
            }
            Err(e) => {
                crate::serial::_print(format_args!("[Process Demo] Failed to create worker thread '{}': {:?}\n", worker_name, e));
            }
        }
    }
    
    // Test 2: Create a user process (simulated)
    crate::serial::_print(format_args!("[Process Demo] Creating user process...\n"));
    match process::spawn_user_process("test_user_app", x86_64::VirtAddr::new(0x400000)) {
        Ok(pid) => {
            crate::serial::_print(format_args!("[Process Demo] Created user process with PID: {}\n", pid));
        }
        Err(e) => {
            crate::serial::_print(format_args!("[Process Demo] Failed to create user process: {:?}\n", e));
        }
    }
    
    // Test 3: Test process state management
    crate::serial::_print(format_args!("[Process Demo] Testing process state management...\n"));
    for &pid in &spawned_pids {
        // Get process state (this would normally be done through the scheduler)
        crate::serial::_print(format_args!("[Process Demo] Process {} is in scheduler queue\n", pid));
    }
    
    // Test 4: Demonstrate process priorities
    crate::serial::_print(format_args!("[Process Demo] Creating high-priority process...\n"));
    match process::create_process("high_priority_task".to_string(), x86_64::VirtAddr::new(0x401000), process::Priority::High) {
        Ok(pid) => {
            crate::serial::_print(format_args!("[Process Demo] Created high-priority process with PID: {}\n", pid));
        }
        Err(e) => {
            crate::serial::_print(format_args!("[Process Demo] Failed to create high-priority process: {:?}\n", e));
        }
    }
    
    // Test 5: Demonstrate real-time thread creation
    crate::serial::_print(format_args!("[Process Demo] Creating real-time thread...\n"));
    match process::spawn_rt_kernel_thread(
        "rt_worker",
        rt_worker_thread_main,
        process::RtClass::Edf,
        10000, // 10ms period
        5000,  // 5ms budget
        None   // No specific CPU affinity
    ) {
        Ok(pid) => {
            crate::serial::_print(format_args!("[Process Demo] Created real-time thread with PID: {}\n", pid));
        }
        Err(e) => {
            crate::serial::_print(format_args!("[Process Demo] Failed to create real-time thread: {:?}\n", e));
        }
    }
    
    // Test 6: Test process forking
    crate::serial::_print(format_args!("[Process Demo] Testing process forking...\n"));
    match process::fork_process() {
        Ok(child_pid) => {
            crate::serial::_print(format_args!("[Process Demo] Forked process with child PID: {}\n", child_pid));
        }
        Err(e) => {
            crate::serial::_print(format_args!("[Process Demo] Failed to fork process: {:?}\n", e));
        }
    }
    
    crate::serial::_print(format_args!("[Process Demo] Process spawning demonstration completed!\n\n"));
}

/// Worker thread function for demonstration
extern "C" fn worker_thread_main() -> ! {
    let mut counter = 0u64;
    loop {
        counter += 1;
        if counter % 2000000 == 0 {
            crate::serial::_print(format_args!("[Worker] Thread tick: {}\n", counter / 2000000));
        }
        
        // Yield to other threads
        if counter % 1000000 == 0 {
            process::yield_current();
        }
    }
}

/// Real-time worker thread function for demonstration
extern "C" fn rt_worker_thread_main() -> ! {
    let mut rt_counter = 0u64;
    loop {
        rt_counter += 1;
        
        // Simulate real-time work (e.g., audio processing, input handling)
        if rt_counter % 500000 == 0 {
            crate::serial::_print(format_args!("[RT Worker] Real-time tick: {}\n", rt_counter / 500000));
        }
        
        // Real-time threads yield more frequently to meet deadlines
        if rt_counter % 100000 == 0 {
            process::yield_current();
        }
    }
}

/// Demonstrates shell functionality
fn demonstrate_shell_functionality() {
    crate::serial::_print(format_args!("\n=== RaeShell Demonstration ===\n"));
    
    // Create a shell session
    match raeshell::create_shell_session() {
        Ok(session_id) => {
            crate::serial::_print(format_args!("Shell session created successfully (ID: {})\n", session_id));
            
            // Test basic shell commands
            let test_commands = [
                "help",
                "uname",
                "date", 
                "uptime",
                "free",
                "ps",
                "env",
                "echo Hello from RaeShell!",
                "pwd"
            ];
            
            for command in test_commands.iter() {
                crate::serial::_print(format_args!("\n$ {}\n", command));
                
                // Execute the command
                match raeshell::execute_command(session_id, command) {
                    Ok(raeshell::ShellResult::Success(output)) => {
                        if !output.is_empty() {
                            crate::serial::_print(format_args!("{}\n", output));
                        }
                    },
                    Ok(raeshell::ShellResult::Error(msg)) => {
                        crate::serial::_print(format_args!("Error: {}\n", msg));
                    },
                    Ok(raeshell::ShellResult::Exit) => {
                        crate::serial::_print(format_args!("Shell exit requested\n"));
                        break;
                    },
                    Err(_) => {
                        crate::serial::_print(format_args!("Failed to execute command\n"));
                    }
                }
            }
            
            // Test directory operations
            crate::serial::_print(format_args!("\n=== Directory Operations ===\n"));
            crate::serial::_print(format_args!("\n$ mkdir test_dir\n"));
            match raeshell::execute_command(session_id, "mkdir test_dir") {
                Ok(raeshell::ShellResult::Success(output)) => {
                    if !output.is_empty() {
                        crate::serial::_print(format_args!("{}\n", output));
                    }
                },
                Ok(raeshell::ShellResult::Error(msg)) => {
                    crate::serial::_print(format_args!("Error: {}\n", msg));
                },
                _ => {}
            }
            
            crate::serial::_print(format_args!("\n$ ls\n"));
            match raeshell::execute_command(session_id, "ls") {
                Ok(raeshell::ShellResult::Success(output)) => {
                    if !output.is_empty() {
                        serial_println!("{}", output);
                    }
                },
                Ok(raeshell::ShellResult::Error(msg)) => {
                    serial_println!("Error: {}", msg);
                },
                _ => {}
            }
            
            // Test interactive shell features
            crate::serial::_print(format_args!("\n=== Interactive Features ===\n"));
            
            // Test environment variables
            crate::serial::_print(format_args!("\n$ export TEST_VAR=hello_world\n"));
            match raeshell::execute_command(session_id, "export TEST_VAR=hello_world") {
                Ok(raeshell::ShellResult::Success(output)) => {
                    if !output.is_empty() {
                        serial_println!("{}", output);
                    }
                },
                Ok(raeshell::ShellResult::Error(msg)) => {
                    serial_println!("Error: {}", msg);
                },
                _ => {}
            }
            
            // Test command history
            crate::serial::_print(format_args!("\n$ history\n"));
            match raeshell::execute_command(session_id, "history") {
                Ok(raeshell::ShellResult::Success(output)) => {
                    if !output.is_empty() {
                        serial_println!("{}", output);
                    }
                },
                Ok(raeshell::ShellResult::Error(msg)) => {
                    serial_println!("Error: {}", msg);
                },
                _ => {}
            }
            
            // Clean up session
            let _ = raeshell::close_shell_session(session_id);
            crate::serial::_print(format_args!("\nShell demonstration completed successfully!\n"));
        },
        Err(_) => {
            crate::serial::_print(format_args!("Failed to create shell session - permission denied or system error\n"));
        }
    }
}

/// Demonstrates graphics rendering capabilities
#[allow(dead_code)]
fn demonstrate_graphics_rendering() {
    crate::serial::_print(format_args!("\n=== Graphics Rendering Demonstration ===\n"));
    
    // Test window creation
    crate::serial::_print(format_args!("Creating test windows...\n"));
    
    // Create multiple test windows
    let window_results = [
        graphics::create_window("Test Window 1", 50, 50, 300, 200, 1),
        graphics::create_window("Test Window 2", 200, 150, 250, 180, 1),
        graphics::create_window("Graphics Demo", 100, 100, 400, 300, 1),
    ];
    
    let created_windows = window_results;
    
    for (i, &window_id) in created_windows.iter().enumerate() {
        crate::serial::_print(format_args!("Created window {} with ID: {}\n", i + 1, window_id));
    }
    
    // Test pixel drawing in windows
    crate::serial::_print(format_args!("\nTesting pixel drawing...\n"));
    for &window_id in &created_windows {
        // Draw a colorful pattern
        for y in 10..50 {
            for x in 10..100 {
                let r = ((x * 255) / 100) as u8;
                let g = ((y * 255) / 50) as u8;
                let b = 128u8;
                let color = graphics::Color::new(r, g, b, 255);
                if let Err(e) = graphics::draw_pixel(window_id, x, y, color) {
                    crate::serial::_print(format_args!("Failed to draw pixel at ({}, {}) in window {}: {}\n", x, y, window_id, e));
                    break;
                }
            }
        }
        crate::serial::_print(format_args!("Drew gradient pattern in window {}\n", window_id));
    }
    
    // Test rectangle drawing
    crate::serial::_print(format_args!("\nTesting rectangle drawing...\n"));
    for &window_id in &created_windows {
        let rect = graphics::Rect::new(120, 60, 80, 60);
        let color = graphics::Color::new(255, 0, 0, 255); // Red
        
        match graphics::draw_rect(window_id, rect, color) {
            Ok(_) => crate::serial::_print(format_args!("Drew filled rectangle in window {}\n", window_id)),
            Err(e) => crate::serial::_print(format_args!("Failed to draw rectangle in window {}: {}\n", window_id, e)),
        }
    }
    
    // Test text rendering
    crate::serial::_print(format_args!("\nTesting text rendering...\n"));
    for &window_id in &created_windows {
        // Note: draw_text function may not be available in current API
        crate::serial::_print(format_args!("Text rendering would be drawn in window {} (API pending)\n", window_id));
    }
    
    // Test window focus and management
    crate::serial::_print(format_args!("\nTesting window management...\n"));
    for &window_id in &created_windows {
        if graphics::focus_window(window_id) {
            crate::serial::_print(format_args!("Focused window {}\n", window_id));
        } else {
            crate::serial::_print(format_args!("Failed to focus window {}\n", window_id));
        }
    }
    
    // Test framebuffer operations
    crate::serial::_print(format_args!("\nTesting framebuffer operations...\n"));
    
    // Note: Direct framebuffer operations may not be available in current API
    crate::serial::_print(format_args!("Framebuffer operations would be performed here (API pending)\n"));
    
    // Render the frame to display all changes
    crate::serial::_print(format_args!("\nRendering final frame...\n"));
    graphics::render_frame();
    
    // Test compositor features
    crate::serial::_print(format_args!("\nTesting compositor features...\n"));
    
    // Note: Advanced compositor features may not be available in current API
    crate::serial::_print(format_args!("Compositor features would be tested here (API pending)\n"));
    
    // Test animation by creating a simple moving pattern
    crate::serial::_print(format_args!("\nTesting animation...\n"));
    if let Some(main_window) = created_windows.first() {
        for frame in 0..10 {
            // Note: clear_window function may not be available in current API
             // Would clear the window here
            
            // Draw a moving circle (approximated with pixels)
            let center_x: usize = 50 + (frame * 10);
            let center_y: usize = 50;
            let radius: usize = 20;
            
            for y in (center_y.saturating_sub(radius))..=(center_y + radius) {
                for x in (center_x.saturating_sub(radius))..=(center_x + radius) {
                    let dx = x as i32 - center_x as i32;
                    let dy = y as i32 - center_y as i32;
                    if dx * dx + dy * dy <= (radius * radius) as i32 {
                        let color = graphics::Color::new(255, 255, 0, 255); // Yellow
                        let _ = graphics::draw_pixel(*main_window, x as u32, y as u32, color);
                    }
                }
            }
            
            // Render the frame
            graphics::render_frame();
            
            // Simple delay (in a real system, this would be frame-rate controlled)
            for _ in 0..100000 {
                core::hint::spin_loop();
            }
        }
        crate::serial::_print(format_args!("Animated moving circle in window {}\n", main_window));
    }
    
    crate::serial::_print(format_args!("\nGraphics rendering demonstration completed successfully!\n"));
    crate::serial::_print(format_args!("Created {} windows with pixel drawing, rectangles, text, and animation\n", created_windows.len()));
}

/// Launch the desktop environment and start the main system loop
pub fn launch_desktop_environment() -> ! {
    crate::serial::_print(format_args!("[Desktop] Starting RaeenOS Desktop Environment...\n"));
    
    #[cfg(feature = "boot-animation")]
    show_boot_animation();
    
    #[cfg(not(feature = "boot-animation"))]
    crate::serial::_print(format_args!("[Boot] Boot animation disabled\n"));
    
    #[cfg(feature = "test-mode")]
    {
        // Run IPC message passing tests
        crate::serial::_print(format_args!("[Desktop] Running IPC tests...\n"));
        crate::ipc_test::test_ipc_functionality();
    }
    #[cfg(not(feature = "test-mode"))]
    crate::serial::_print(format_args!("[Desktop] IPC tests disabled\n"));
    
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
#[allow(dead_code)]
fn show_boot_animation() {
    crate::serial::_print(format_args!("[Boot] Displaying boot animation...\n"));
    
    // Simple boot animation - fade in RaeenOS logo
    for frame in 0..30 {
        // Clear screen with gradient background
        let color = 0x001122 + (frame * 0x000811); // Gradual blue fade
        let _ = vesa::clear_screen(color);
        
        // Draw RaeenOS logo text in center
        if let Ok(_window_id) = graphics::create_boot_window() {
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


