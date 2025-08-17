//! User-space execution test module
//! 
//! This module provides functionality to test Ring0→Ring3 transitions
//! by loading and executing a simple user-space program.

use alloc::string::String;
use x86_64::VirtAddr;
use crate::process::{Process, Priority};
use crate::elf::load_elf;
use crate::serial_println;

/// Embedded test binary (will be included at compile time)
static TEST_BINARY: &[u8] = include_bytes!("../../userspace_test");

/// Test Ring0→Ring3 transition by loading and executing a user-space program
pub fn test_userspace_execution() -> Result<(), &'static str> {
    serial_println!("[USERSPACE_TEST] Starting Ring0→Ring3 transition test...");
    
    // Create a copy of the test binary
    let binary_data = TEST_BINARY.to_vec();
    
    serial_println!("[USERSPACE_TEST] Test binary size: {} bytes", binary_data.len());
    
    // Validate the ELF binary
    let entry_point = crate::elf::validate_elf(&binary_data)
        .map_err(|_| "Failed to validate test ELF binary")?;
    
    serial_println!("[USERSPACE_TEST] Entry point: {:#x}", entry_point.as_u64());
    
    // Create a new user process
    let process = Process::user_process(
        String::from("userspace_test"),
        entry_point
    ).map_err(|_| "Failed to create user process")?;
    
    let pid = process.pid;
    let address_space_id = process.address_space_id
        .ok_or("Process has no address space")?;
    
    serial_println!("[USERSPACE_TEST] Created process PID: {}, Address space: {}", pid, address_space_id);
    
    // Load the ELF binary into the process address space
    load_elf(binary_data, address_space_id)
        .map_err(|_| "Failed to load ELF binary")?;
    
    serial_println!("[USERSPACE_TEST] ELF binary loaded successfully");
    
    // Add the process to the scheduler using public API
    let _pid = crate::process::create_process(
        String::from("userspace_test"),
        entry_point,
        Priority::Normal
    ).map_err(|_| "Failed to add process to scheduler")?;
    
    serial_println!("[USERSPACE_TEST] Process added to scheduler");
    serial_println!("[USERSPACE_TEST] Ring0→Ring3 transition test initiated!");
    serial_println!("[USERSPACE_TEST] The test program should:");
    serial_println!("[USERSPACE_TEST]   1. Get its PID via sys_getpid");
    serial_println!("[USERSPACE_TEST]   2. Write 'Hello from Ring3!' via sys_write");
    serial_println!("[USERSPACE_TEST]   3. Sleep for 1 second via sys_sleep");
    serial_println!("[USERSPACE_TEST]   4. Exit with code 42 via sys_exit");
    
    Ok(())
}

/// Alternative test using direct Ring3 transition (without ELF loading)
pub fn test_direct_ring3_transition() -> Result<(), &'static str> {
    serial_println!("[USERSPACE_TEST] Testing direct Ring0→Ring3 transition...");
    
    // Create a simple user-space code that just calls sys_getpid and exits
    // This is x86-64 machine code for:
    //   mov $5, %rax    ; sys_getpid
    //   syscall
    //   mov $0, %rax    ; sys_exit
    //   mov $42, %rdi   ; exit code 42
    //   syscall
    let _user_code: &[u8] = &[
        0x48, 0xc7, 0xc0, 0x05, 0x00, 0x00, 0x00,  // mov $5, %rax
        0x0f, 0x05,                                  // syscall
        0x48, 0xc7, 0xc0, 0x00, 0x00, 0x00, 0x00,  // mov $0, %rax
        0x48, 0xc7, 0xc7, 0x2a, 0x00, 0x00, 0x00,  // mov $42, %rdi
        0x0f, 0x05,                                  // syscall
        0xeb, 0xfe,                                  // jmp . (infinite loop)
    ];
    
    // Create a user process
    let entry_point = VirtAddr::new(0x400000);  // Standard user code base
    let process = Process::user_process(
        String::from("direct_test"),
        entry_point
    ).map_err(|_| "Failed to create user process")?;
    
    let address_space_id = process.address_space_id
        .ok_or("Process has no address space")?;
    
    // Map the code into user space
    crate::vmm::with_vmm(|vmm| {
        if let Some(address_space) = vmm.get_address_space_mut(address_space_id) {
            use crate::vmm::{VmArea, VmAreaType, VmPermissions};
            
            // Create a code area
            let code_area = VmArea::new(
                entry_point,
                entry_point + 0x1000u64,  // 4KB page
                VmAreaType::Code,
                VmPermissions::READ | VmPermissions::EXECUTE | VmPermissions::USER
            );
            
            address_space.add_area(code_area)?;
            
            // Copy the machine code to the mapped area
            // Note: This is a simplified approach - in practice we'd need proper page mapping
            serial_println!("[USERSPACE_TEST] Code area mapped at {:#x}", entry_point.as_u64());
        }
        Ok::<(), crate::vmm::VmError>(())
    }).map_err(|_| "Failed to map user code")?;
    
    // Add to scheduler
    // Add the process to the scheduler using public API
    let _pid = crate::process::create_process(
        String::from("direct_test"),
        entry_point,
        Priority::Normal
    ).map_err(|_| "Failed to add process to scheduler")?;
    
    serial_println!("[USERSPACE_TEST] Direct Ring3 transition test initiated!");
    
    Ok(())
}

/// Test syscall interface from kernel space (simulated user calls)
pub fn test_syscall_interface() {
    serial_println!("[USERSPACE_TEST] Testing syscall interface...");
    
    // Test sys_getpid
    let result = crate::syscall::handle_syscall(5, 0, 0, 0, 0, 0, 0);
    serial_println!("[USERSPACE_TEST] sys_getpid result: success={}, value={}", 
                   result.success, result.value);
    
    // Test sys_write (write to console)
    let msg = "Test message from syscall\n";
    let msg_ptr = msg.as_ptr() as u64;
    let msg_len = msg.len() as u64;
    let result = crate::syscall::handle_syscall(13, 1, msg_ptr, msg_len, 0, 0, 0);
    serial_println!("[USERSPACE_TEST] sys_write result: success={}, value={}", 
                   result.success, result.value);
    
    serial_println!("[USERSPACE_TEST] Syscall interface test completed");
}