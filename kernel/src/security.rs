//! Security subsystem for RaeenOS
//! Implements capability-based security with sandbox levels

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;
use bitflags::bitflags;

// Security capabilities
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Capabilities: u64 {
        const READ_FILE = 1 << 0;
        const WRITE_FILE = 1 << 1;
        const EXECUTE_FILE = 1 << 2;
        const CREATE_FILE = 1 << 3;
        const DELETE_FILE = 1 << 4;
        const NETWORK_ACCESS = 1 << 5;
        const SYSTEM_CALL = 1 << 6;
        const MEMORY_ALLOC = 1 << 7;
        const DEVICE_ACCESS = 1 << 8;
        const PROCESS_SPAWN = 1 << 9;
        const PROCESS_KILL = 1 << 10;
        const GRAPHICS_ACCESS = 1 << 11;
        const AUDIO_ACCESS = 1 << 12;
        const KERNEL_MODULE = 1 << 13;
        const ADMIN_RIGHTS = 1 << 14;
        
        // Convenience combinations
        const FILE_ALL = Self::READ_FILE.bits() | Self::WRITE_FILE.bits() | 
                        Self::EXECUTE_FILE.bits() | Self::CREATE_FILE.bits() | 
                        Self::DELETE_FILE.bits();
        const PROCESS_ALL = Self::PROCESS_SPAWN.bits() | Self::PROCESS_KILL.bits();
        const SYSTEM_ALL = Self::SYSTEM_CALL.bits() | Self::MEMORY_ALLOC.bits() | 
                          Self::DEVICE_ACCESS.bits();
    }
}

// Sandbox levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SandboxLevel {
    None = 0,        // Full system access
    Low = 1,         // Limited file and network access
    Medium = 2,      // Restricted to user directories
    High = 3,        // Very limited access
    Strict = 4,      // Minimal access, no file system
}

impl From<u8> for SandboxLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => SandboxLevel::None,
            1 => SandboxLevel::Low,
            2 => SandboxLevel::Medium,
            3 => SandboxLevel::High,
            4 => SandboxLevel::Strict,
            _ => SandboxLevel::High, // Default to high security
        }
    }
}

// Process security context
#[derive(Debug, Clone)]
struct SecurityContext {
    capabilities: Capabilities,
    sandbox_level: SandboxLevel,
    allowed_paths: Vec<String>,
    denied_paths: Vec<String>,
    network_allowed: bool,
    max_memory: Option<usize>,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            capabilities: Capabilities::READ_FILE | Capabilities::MEMORY_ALLOC,
            sandbox_level: SandboxLevel::Medium,
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
            network_allowed: false,
            max_memory: Some(64 * 1024 * 1024), // 64MB default limit
        }
    }
}

// Security system state
struct SecuritySystem {
    process_contexts: BTreeMap<u32, SecurityContext>,
    global_policies: BTreeMap<String, bool>,
}

lazy_static! {
    static ref SECURITY_SYSTEM: Mutex<SecuritySystem> = Mutex::new(SecuritySystem {
        process_contexts: BTreeMap::new(),
        global_policies: BTreeMap::new(),
    });
}

// Initialize security for a new process
pub fn init_process_security(process_id: u32, parent_id: Option<u32>) -> Result<(), ()> {
    let mut security = SECURITY_SYSTEM.lock();
    
    let context = if let Some(parent) = parent_id {
        // Inherit from parent with reduced privileges
        if let Some(parent_context) = security.process_contexts.get(&parent) {
            let mut child_context = parent_context.clone();
            // Child processes get reduced capabilities
            child_context.capabilities.remove(Capabilities::PROCESS_SPAWN);
            child_context.capabilities.remove(Capabilities::ADMIN_RIGHTS);
            child_context
        } else {
            SecurityContext::default()
        }
    } else {
        // Root process gets full capabilities
        SecurityContext {
            capabilities: Capabilities::all(),
            sandbox_level: SandboxLevel::None,
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
            network_allowed: true,
            max_memory: None,
        }
    };
    
    security.process_contexts.insert(process_id, context);
    Ok(())
}

// Request permission for a specific operation
pub fn request_permission(process_id: u32, permission: &str) -> Result<bool, ()> {
    let security = SECURITY_SYSTEM.lock();
    
    let context = security.process_contexts.get(&process_id)
        .ok_or(())?;
    
    let required_capability = match permission {
        "file.read" => Capabilities::READ_FILE,
        "file.write" => Capabilities::WRITE_FILE,
        "file.execute" => Capabilities::EXECUTE_FILE,
        "file.create" => Capabilities::CREATE_FILE,
        "file.delete" => Capabilities::DELETE_FILE,
        "network.access" => Capabilities::NETWORK_ACCESS,
        "system.call" => Capabilities::SYSTEM_CALL,
        "memory.alloc" => Capabilities::MEMORY_ALLOC,
        "device.access" => Capabilities::DEVICE_ACCESS,
        "process.spawn" => Capabilities::PROCESS_SPAWN,
        "process.kill" => Capabilities::PROCESS_KILL,
        "graphics.access" => Capabilities::GRAPHICS_ACCESS,
        "audio.access" => Capabilities::AUDIO_ACCESS,
        "kernel.module" => Capabilities::KERNEL_MODULE,
        "admin.rights" => Capabilities::ADMIN_RIGHTS,
        _ => return Ok(false), // Unknown permission denied
    };
    
    // Check if process has the required capability
    let has_capability = context.capabilities.contains(required_capability);
    
    // Additional sandbox level checks
    let sandbox_allowed = match context.sandbox_level {
        SandboxLevel::None => true,
        SandboxLevel::Low => !matches!(permission, "kernel.module" | "admin.rights"),
        SandboxLevel::Medium => !matches!(permission, "kernel.module" | "admin.rights" | "device.access"),
        SandboxLevel::High => matches!(permission, "file.read" | "memory.alloc" | "graphics.access"),
        SandboxLevel::Strict => matches!(permission, "memory.alloc"),
    };
    
    Ok(has_capability && sandbox_allowed)
}

// Set sandbox level for a process
pub fn set_sandbox_level(process_id: u32, level: u8) -> Result<(), ()> {
    let mut security = SECURITY_SYSTEM.lock();
    
    let context = security.process_contexts.get_mut(&process_id)
        .ok_or(())?;
    
    let new_level = SandboxLevel::from(level);
    
    // Can only increase sandbox level (more restrictive), not decrease
    if new_level >= context.sandbox_level {
        context.sandbox_level = new_level;
        
        // Adjust capabilities based on sandbox level
        match new_level {
            SandboxLevel::Strict => {
                context.capabilities = Capabilities::MEMORY_ALLOC;
            }
            SandboxLevel::High => {
                context.capabilities &= Capabilities::READ_FILE | 
                                       Capabilities::MEMORY_ALLOC | 
                                       Capabilities::GRAPHICS_ACCESS;
            }
            SandboxLevel::Medium => {
                context.capabilities.remove(Capabilities::KERNEL_MODULE);
                context.capabilities.remove(Capabilities::ADMIN_RIGHTS);
                context.capabilities.remove(Capabilities::DEVICE_ACCESS);
            }
            SandboxLevel::Low => {
                context.capabilities.remove(Capabilities::KERNEL_MODULE);
                context.capabilities.remove(Capabilities::ADMIN_RIGHTS);
            }
            SandboxLevel::None => {}
        }
        
        Ok(())
    } else {
        Err(()) // Cannot reduce sandbox level
    }
}

// Get process permissions as a capability bitmask
pub fn get_process_permissions(process_id: u32) -> Result<Vec<u8>, ()> {
    let security = SECURITY_SYSTEM.lock();
    
    let context = security.process_contexts.get(&process_id)
        .ok_or(())?;
    
    // Return capabilities as bytes
    let caps_bytes = context.capabilities.bits().to_le_bytes();
    Ok(caps_bytes.to_vec())
}

// Grant specific capability to a process
pub fn grant_capability(process_id: u32, capability: Capabilities) -> Result<(), ()> {
    let mut security = SECURITY_SYSTEM.lock();
    
    let context = security.process_contexts.get_mut(&process_id)
        .ok_or(())?;
    
    // Only allow granting if sandbox level permits it
    let allowed = match context.sandbox_level {
        SandboxLevel::None => true,
        SandboxLevel::Low => !capability.intersects(Capabilities::KERNEL_MODULE | Capabilities::ADMIN_RIGHTS),
        SandboxLevel::Medium => !capability.intersects(Capabilities::KERNEL_MODULE | Capabilities::ADMIN_RIGHTS | Capabilities::DEVICE_ACCESS),
        SandboxLevel::High => capability.intersects(Capabilities::READ_FILE | Capabilities::MEMORY_ALLOC | Capabilities::GRAPHICS_ACCESS),
        SandboxLevel::Strict => capability == Capabilities::MEMORY_ALLOC,
    };
    
    if allowed {
        context.capabilities.insert(capability);
        Ok(())
    } else {
        Err(())
    }
}

// Revoke specific capability from a process
pub fn revoke_capability(process_id: u32, capability: Capabilities) -> Result<(), ()> {
    let mut security = SECURITY_SYSTEM.lock();
    
    let context = security.process_contexts.get_mut(&process_id)
        .ok_or(())?;
    
    context.capabilities.remove(capability);
    Ok(())
}

// Check if a file path is allowed for a process
pub fn check_path_access(process_id: u32, path: &str, operation: &str) -> Result<bool, ()> {
    let security = SECURITY_SYSTEM.lock();
    
    let context = security.process_contexts.get(&process_id)
        .ok_or(())?;
    
    // Check denied paths first
    for denied_path in &context.denied_paths {
        if path.starts_with(denied_path) {
            return Ok(false);
        }
    }
    
    // Check allowed paths
    if !context.allowed_paths.is_empty() {
        let mut allowed = false;
        for allowed_path in &context.allowed_paths {
            if path.starts_with(allowed_path) {
                allowed = true;
                break;
            }
        }
        if !allowed {
            return Ok(false);
        }
    }
    
    // Check operation permission
    let permission = match operation {
        "read" => "file.read",
        "write" => "file.write",
        "execute" => "file.execute",
        "create" => "file.create",
        "delete" => "file.delete",
        _ => return Ok(false),
    };
    
    request_permission(process_id, permission)
}

// Clean up security context when process exits
pub fn cleanup_process_security(process_id: u32) {
    let mut security = SECURITY_SYSTEM.lock();
    security.process_contexts.remove(&process_id);
}