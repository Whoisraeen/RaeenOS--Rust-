//! Filesystem testing module
//! 
//! This module contains comprehensive tests for the RaeenOS filesystem,
//! including basic operations, crash-safe features, and power-fail testing.

use crate::filesystem::{CrashSafeFileSystem, PowerFailOperation, PowerFailTestResult, FileType};
use crate::serial_println;
use alloc::vec::Vec;
use alloc::string::String;

/// Test basic file operations (create, write, read, delete)
pub fn test_basic_file_operations() -> Result<(), ()> {
    serial_println!("[FS_TEST] Testing basic file operations...");
    
    let test_path = "/tmp/test_file.txt";
    let test_data = b"Hello, RaeenOS filesystem!";
    
    // Test file creation
    filesystem::create_file(test_path).map_err(|_| ())?;
    serial_println!("[FS_TEST] ✓ File created: {}", test_path);
    
    // Test file writing
    let fd = filesystem::open_file(test_path)?;
    filesystem::write_file(fd, test_data)?;
    filesystem::close_file(fd)?;
    serial_println!("[FS_TEST] ✓ Data written to file");
    
    // Test file reading
    let read_data = filesystem::read_file(test_path)?;
    assert_eq!(read_data, test_data);
    serial_println!("[FS_TEST] ✓ Data read and verified");
    
    // Test file deletion
    filesystem::remove(test_path).map_err(|_| ())?;
    serial_println!("[FS_TEST] ✓ File deleted");
    
    Ok(())
}

/// Test directory operations (create, list, remove)
pub fn test_directory_operations() -> Result<(), ()> {
    serial_println!("[FS_TEST] Testing directory operations...");
    
    let test_dir = "/tmp/test_directory";
    let test_file = "/tmp/test_directory/nested_file.txt";
    
    // Test directory creation
    filesystem::create_directory(test_dir).map_err(|_| ())?;
    serial_println!("[FS_TEST] ✓ Directory created: {}", test_dir);
    
    // Test file creation in directory
    filesystem::create_file(test_file).map_err(|_| ())?;
    let fd = filesystem::open_file(test_file)?;
    filesystem::write_file(fd, b"nested content")?;
    filesystem::close_file(fd)?;
    serial_println!("[FS_TEST] ✓ File created in directory");
    
    // Test directory listing
    let entries = filesystem::list_directory(test_dir).map_err(|_| ())?;
    assert!(entries.len() > 0);
    serial_println!("[FS_TEST] ✓ Directory listing: {} entries", entries.len());
    
    // Cleanup
    filesystem::remove(test_file).map_err(|_| ())?;
    filesystem::remove(test_dir).map_err(|_| ())?;
    serial_println!("[FS_TEST] ✓ Directory cleanup completed");
    
    Ok(())
}

/// Test file seeking operations
pub fn test_file_seeking() -> Result<(), ()> {
    serial_println!("[FS_TEST] Testing file seeking operations...");
    
    let test_path = "/tmp/seek_test.txt";
    let test_data = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    
    // Create and write initial data
    filesystem::create_file(test_path).map_err(|_| ())?;
    let fd = filesystem::open_file(test_path)?;
    filesystem::write_file(fd, test_data)?;
    filesystem::close_file(fd)?;
    
    // Test seeking and partial reads
    let fd = filesystem::open_file(test_path)?;
    
    // Seek to middle and read
    filesystem::seek_file(fd, 10)?;
    let mut buffer = [0u8; 5];
    let bytes_read = filesystem::read_file_to_buffer(fd, &mut buffer)?;
    assert_eq!(bytes_read, 5);
    assert_eq!(&buffer, b"ABCDE");
    
    // Seek to end and read (should read 0 bytes)
    filesystem::seek_file(fd, test_data.len() as u64)?;
    let bytes_read = filesystem::read_file_to_buffer(fd, &mut buffer)?;
    assert_eq!(bytes_read, 0);
    
    filesystem::close_file(fd)?;
    filesystem::remove(test_path).map_err(|_| ())?;
    
    serial_println!("[FS_TEST] ✓ File seeking operations completed");
    Ok(())
}

/// Test crash-safe filesystem with power-fail injection
pub fn test_crash_safe_filesystem() -> Result<(), ()> {
    serial_println!("[FS_TEST] Testing crash-safe filesystem...");
    
    let mut fs = CrashSafeFileSystem::new("test_crash_safe".to_string());
    
    // Test basic scrub on clean filesystem
    let scrub_result = fs.scrub().map_err(|_| ())?;
    assert!(scrub_result.is_clean);
    assert_eq!(scrub_result.checksum_errors.len(), 0);
    serial_println!("[FS_TEST] ✓ Clean filesystem scrub passed");
    
    // Test power-fail scenarios
    let operations = vec![
        PowerFailOperation::Write {
            data: b"test data 1".to_vec(),
            path: "/test1.txt".to_string(),
        },
        PowerFailOperation::Write {
            data: b"test data 2".to_vec(),
            path: "/test2.txt".to_string(),
        },
        PowerFailOperation::Delete {
            path: "/test1.txt".to_string(),
        },
    ];
    
    let test_result = fs.power_fail_test(operations).map_err(|_| ())?;
    
    serial_println!("[FS_TEST] Power-fail test results:");
    serial_println!("[FS_TEST]   Total cycles: {}", test_result.total_cycles);
    serial_println!("[FS_TEST]   Successful: {}", test_result.successful_cycles);
    serial_println!("[FS_TEST]   Power failures: {}", test_result.power_fail_cycles);
    serial_println!("[FS_TEST]   Corruption detected: {}", test_result.corruption_detected);
    
    // Verify no corruption was detected
    assert!(!test_result.corruption_detected);
    serial_println!("[FS_TEST] ✓ Power-fail testing completed without corruption");
    
    Ok(())
}

/// Test filesystem scrub functionality
pub fn test_filesystem_scrub() -> Result<(), ()> {
    serial_println!("[FS_TEST] Testing filesystem scrub functionality...");
    
    let mut fs = CrashSafeFileSystem::new("test_scrub".to_string());
    
    // Create some test files
    fs.create("/test_file.txt", FileType::Regular).map_err(|_| ())?;
    fs.create("/test_dir", FileType::Directory).map_err(|_| ())?;
    
    // Run scrub
    let scrub_result = fs.scrub().map_err(|_| ())?;
    
    serial_println!("[FS_TEST] Scrub results:");
    serial_println!("[FS_TEST]   Blocks checked: {}", scrub_result.blocks_checked);
    serial_println!("[FS_TEST]   Checksum errors: {}", scrub_result.checksum_errors.len());
    serial_println!("[FS_TEST]   Orphaned blocks: {}", scrub_result.orphaned_blocks.len());
    serial_println!("[FS_TEST]   Superblock errors: {}", scrub_result.superblock_errors);
    serial_println!("[FS_TEST]   Journal errors: {}", scrub_result.journal_errors);
    serial_println!("[FS_TEST]   Filesystem clean: {}", scrub_result.is_clean);
    
    assert!(scrub_result.is_clean);
    assert_eq!(scrub_result.checksum_errors.len(), 0);
    assert_eq!(scrub_result.orphaned_blocks.len(), 0);
    
    serial_println!("[FS_TEST] ✓ Filesystem scrub completed successfully");
    Ok(())
}

/// Run comprehensive power-fail testing (10k cycles)
pub fn test_power_fail_10k_cycles() -> Result<(), ()> {
    serial_println!("[FS_TEST] Running 10k power-fail cycles test...");
    
    let mut fs = CrashSafeFileSystem::new("test_10k_cycles".to_string());
    let mut total_operations = Vec::new();
    
    // Generate 10,000 test operations
    for i in 0..10000 {
        let operation = if i % 3 == 0 {
            PowerFailOperation::Write {
                data: format!("test data {}", i).into_bytes(),
                path: format!("/test_{}.txt", i),
            }
        } else if i % 3 == 1 {
            PowerFailOperation::Write {
                data: format!("updated data {}", i).into_bytes(),
                path: format!("/test_{}.txt", i / 3),
            }
        } else {
            PowerFailOperation::Delete {
                path: format!("/test_{}.txt", i / 3),
            }
        };
        total_operations.push(operation);
    }
    
    let test_result = fs.power_fail_test(total_operations).map_err(|_| ())?;
    
    serial_println!("[FS_TEST] 10k cycles test results:");
    serial_println!("[FS_TEST]   Total cycles: {}", test_result.total_cycles);
    serial_println!("[FS_TEST]   Successful: {}", test_result.successful_cycles);
    serial_println!("[FS_TEST]   Power failures: {}", test_result.power_fail_cycles);
    serial_println!("[FS_TEST]   Corruption detected: {}", test_result.corruption_detected);
    
    // Critical requirement: 0 metadata corruption
    assert!(!test_result.corruption_detected);
    assert_eq!(test_result.total_cycles, 10000);
    
    // Run final scrub to verify filesystem integrity
    let final_scrub = fs.scrub().map_err(|_| ())?;
    assert!(final_scrub.is_clean);
    
    serial_println!("[FS_TEST] ✓ 10k power-fail cycles completed with 0 corruption");
    Ok(())
}

/// Test all filesystem functionality
pub fn run_all_filesystem_tests() -> Result<(), ()> {
    serial_println!("[FS_TEST] Starting comprehensive filesystem tests...");
    
    test_basic_file_operations()?;
    test_directory_operations()?;
    test_file_seeking()?;
    test_crash_safe_filesystem()?;
    test_filesystem_scrub()?;
    
    // Note: 10k cycles test is commented out for normal testing
    // Uncomment for full production validation
    // test_power_fail_10k_cycles()?;
    
    serial_println!("[FS_TEST] ✓ All filesystem tests passed!");
    Ok(())
}