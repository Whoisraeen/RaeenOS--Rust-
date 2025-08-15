//! Capability-based security system for RaeenOS
//! Implements per-process handle tables, capability revocation, and fine-grained permissions

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::{Mutex, RwLock};
use lazy_static::lazy_static;
use crate::process::ProcessId;

/// Unique capability identifier
pub type CapabilityId = u64;

/// Handle identifier within a process
pub type Handle = u32;

/// Capability types that can be granted to processes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityType {
    // File system capabilities
    FileRead,
    FileWrite,
    FileExecute,
    DirectoryCreate,
    DirectoryDelete,
    
    // Network capabilities
    NetworkBind,
    NetworkConnect,
    NetworkListen,
    NetworkRaw,
    
    // Graphics capabilities
    GraphicsFramebuffer,
    GraphicsWindow,
    GraphicsInput,
    GraphicsCompositor,
    
    // Audio capabilities
    AudioPlayback,
    AudioCapture,
    AudioMixer,
    
    // Process management capabilities
    ProcessCreate,
    ProcessKill,
    ProcessDebug,
    ProcessSetPriority,
    
    // Memory capabilities
    MemoryMap,
    MemoryUnmap,
    MemoryProtect,
    MemoryShared,
    
    // IPC capabilities
    IpcCreate,
    IpcConnect,
    IpcSend,
    IpcReceive,
    
    // Hardware capabilities
    HardwareDirect,
    HardwareInterrupt,
    HardwareDma,
    
    // System capabilities
    SystemShutdown,
    SystemReboot,
    SystemTime,
    SystemConfiguration,
}

/// Capability permissions and metadata
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: CapabilityId,
    pub capability_type: CapabilityType,
    pub owner_process: ProcessId,
    pub resource_path: Option<String>, // Optional resource identifier (e.g., file path)
    pub permissions: u64, // Bitmask of specific permissions
    pub expiry_time: Option<u64>, // Optional expiry timestamp
    pub revoked: bool,
    pub transferable: bool, // Can this capability be transferred to other processes?
    pub delegatable: bool, // Can this capability be used to create derived capabilities?
}

/// Handle table entry mapping handles to capabilities
#[derive(Debug, Clone)]
struct HandleEntry {
    capability_id: CapabilityId,
    access_count: u64,
    last_access: u64,
}

/// Per-process handle table
#[derive(Debug)]
struct HandleTable {
    process_id: ProcessId,
    handles: BTreeMap<Handle, HandleEntry>,
    next_handle: Handle,
}

impl HandleTable {
    fn new(process_id: ProcessId) -> Self {
        Self {
            process_id,
            handles: BTreeMap::new(),
            next_handle: 1, // Start from 1, 0 is invalid
        }
    }
    
    fn allocate_handle(&mut self, capability_id: CapabilityId) -> Handle {
        let handle = self.next_handle;
        self.next_handle += 1;
        
        let entry = HandleEntry {
            capability_id,
            access_count: 0,
            last_access: crate::time::get_timestamp(),
        };
        
        self.handles.insert(handle, entry);
        handle
    }
    
    fn get_capability(&mut self, handle: Handle) -> Option<CapabilityId> {
        if let Some(entry) = self.handles.get_mut(&handle) {
            entry.access_count += 1;
            entry.last_access = crate::time::get_timestamp();
            Some(entry.capability_id)
        } else {
            None
        }
    }
    
    fn revoke_handle(&mut self, handle: Handle) -> bool {
        self.handles.remove(&handle).is_some()
    }
    
    fn list_handles(&self) -> Vec<Handle> {
        self.handles.keys().copied().collect()
    }
}

/// Global capability system state
struct CapabilitySystem {
    capabilities: BTreeMap<CapabilityId, Capability>,
    handle_tables: BTreeMap<ProcessId, HandleTable>,
    next_capability_id: AtomicU64,
}

lazy_static! {
    static ref CAPABILITY_SYSTEM: RwLock<CapabilitySystem> = RwLock::new(CapabilitySystem {
        capabilities: BTreeMap::new(),
        handle_tables: BTreeMap::new(),
        next_capability_id: AtomicU64::new(1),
    });
}

/// Initialize capability system for a new process
pub fn init_process_capabilities(process_id: ProcessId) {
    let mut system = CAPABILITY_SYSTEM.write();
    system.handle_tables.insert(process_id, HandleTable::new(process_id));
}

/// Clean up capabilities when a process exits
pub fn cleanup_process_capabilities(process_id: ProcessId) {
    let mut system = CAPABILITY_SYSTEM.write();
    
    // Remove the handle table
    system.handle_tables.remove(&process_id);
    
    // Revoke all capabilities owned by this process
    for capability in system.capabilities.values_mut() {
        if capability.owner_process == process_id {
            capability.revoked = true;
        }
    }
}

/// Create a new capability
pub fn create_capability(
    capability_type: CapabilityType,
    owner_process: ProcessId,
    resource_path: Option<String>,
    permissions: u64,
    transferable: bool,
    delegatable: bool,
) -> Result<CapabilityId, &'static str> {
    let mut system = CAPABILITY_SYSTEM.write();
    
    let capability_id = system.next_capability_id.fetch_add(1, Ordering::SeqCst);
    
    let capability = Capability {
        id: capability_id,
        capability_type,
        owner_process,
        resource_path,
        permissions,
        expiry_time: None,
        revoked: false,
        transferable,
        delegatable,
    };
    
    system.capabilities.insert(capability_id, capability);
    Ok(capability_id)
}

/// Grant a capability to a process by creating a handle
pub fn grant_capability(process_id: ProcessId, capability_id: CapabilityId) -> Result<Handle, &'static str> {
    let mut system = CAPABILITY_SYSTEM.write();
    
    // Check if capability exists and is not revoked
    let capability = system.capabilities.get(&capability_id)
        .ok_or("Capability not found")?;
    
    if capability.revoked {
        return Err("Capability has been revoked");
    }
    
    // Check if capability has expired
    if let Some(expiry) = capability.expiry_time {
        if crate::time::get_timestamp() > expiry {
            return Err("Capability has expired");
        }
    }
    
    // Get or create handle table for the process
    if !system.handle_tables.contains_key(&process_id) {
        system.handle_tables.insert(process_id, HandleTable::new(process_id));
    }
    
    let handle_table = system.handle_tables.get_mut(&process_id).unwrap();
    let handle = handle_table.allocate_handle(capability_id);
    
    Ok(handle)
}

/// Revoke a specific capability
pub fn revoke_capability(capability_id: CapabilityId) -> Result<(), &'static str> {
    let mut system = CAPABILITY_SYSTEM.write();
    
    let capability = system.capabilities.get_mut(&capability_id)
        .ok_or("Capability not found")?;
    
    capability.revoked = true;
    Ok(())
}

/// Revoke a handle from a specific process
pub fn revoke_handle(process_id: ProcessId, handle: Handle) -> Result<(), &'static str> {
    let mut system = CAPABILITY_SYSTEM.write();
    
    let handle_table = system.handle_tables.get_mut(&process_id)
        .ok_or("Process not found")?;
    
    if handle_table.revoke_handle(handle) {
        Ok(())
    } else {
        Err("Handle not found")
    }
}

/// Check if a process has a specific capability through a handle
pub fn check_capability(
    process_id: ProcessId,
    handle: Handle,
    required_type: CapabilityType,
    required_permissions: u64,
) -> Result<bool, &'static str> {
    let mut system = CAPABILITY_SYSTEM.write();
    
    let handle_table = system.handle_tables.get_mut(&process_id)
        .ok_or("Process not found")?;
    
    let capability_id = handle_table.get_capability(handle)
        .ok_or("Invalid handle")?;
    
    let capability = system.capabilities.get(&capability_id)
        .ok_or("Capability not found")?;
    
    if capability.revoked {
        return Ok(false);
    }
    
    // Check expiry
    if let Some(expiry) = capability.expiry_time {
        if crate::time::get_timestamp() > expiry {
            return Ok(false);
        }
    }
    
    // Check type and permissions
    Ok(capability.capability_type == required_type && 
       (capability.permissions & required_permissions) == required_permissions)
}

/// Transfer a capability from one process to another
pub fn transfer_capability(
    from_process: ProcessId,
    to_process: ProcessId,
    handle: Handle,
) -> Result<Handle, &'static str> {
    let mut system = CAPABILITY_SYSTEM.write();
    
    // Get the capability ID from the source process
    let from_table = system.handle_tables.get_mut(&from_process)
        .ok_or("Source process not found")?;
    
    let capability_id = from_table.get_capability(handle)
        .ok_or("Invalid handle")?;
    
    // Check if the capability is transferable
    let capability = system.capabilities.get(&capability_id)
        .ok_or("Capability not found")?;
    
    if !capability.transferable {
        return Err("Capability is not transferable");
    }
    
    if capability.revoked {
        return Err("Capability has been revoked");
    }
    
    // Create handle in destination process
    if !system.handle_tables.contains_key(&to_process) {
        system.handle_tables.insert(to_process, HandleTable::new(to_process));
    }
    
    let to_table = system.handle_tables.get_mut(&to_process).unwrap();
    let new_handle = to_table.allocate_handle(capability_id);
    
    // Optionally revoke the original handle
    let from_table = system.handle_tables.get_mut(&from_process).unwrap();
    from_table.revoke_handle(handle);
    
    Ok(new_handle)
}

/// List all capabilities for a process
pub fn list_process_capabilities(process_id: ProcessId) -> Vec<(Handle, CapabilityType, u64)> {
    let mut system = CAPABILITY_SYSTEM.write();
    let mut result = Vec::new();
    
    if let Some(handle_table) = system.handle_tables.get_mut(&process_id) {
        for handle in handle_table.list_handles() {
            if let Some(capability_id) = handle_table.get_capability(handle) {
                if let Some(capability) = system.capabilities.get(&capability_id) {
                    if !capability.revoked {
                        result.push((handle, capability.capability_type, capability.permissions));
                    }
                }
            }
        }
    }
    
    result
}

/// Create a derived capability with reduced permissions
pub fn derive_capability(
    process_id: ProcessId,
    parent_handle: Handle,
    new_permissions: u64,
    resource_path: Option<String>,
) -> Result<CapabilityId, &'static str> {
    let mut system = CAPABILITY_SYSTEM.write();
    
    // Get parent capability
    let handle_table = system.handle_tables.get_mut(&process_id)
        .ok_or("Process not found")?;
    
    let parent_capability_id = handle_table.get_capability(parent_handle)
        .ok_or("Invalid handle")?;
    
    let parent_capability = system.capabilities.get(&parent_capability_id)
        .ok_or("Parent capability not found")?;
    
    if !parent_capability.delegatable {
        return Err("Parent capability is not delegatable");
    }
    
    if parent_capability.revoked {
        return Err("Parent capability has been revoked");
    }
    
    // Ensure new permissions are a subset of parent permissions
    if (new_permissions & parent_capability.permissions) != new_permissions {
        return Err("New permissions exceed parent capability permissions");
    }
    
    // Create derived capability
    let capability_id = system.next_capability_id.fetch_add(1, Ordering::SeqCst);
    
    let derived_capability = Capability {
        id: capability_id,
        capability_type: parent_capability.capability_type,
        owner_process: process_id,
        resource_path,
        permissions: new_permissions,
        expiry_time: parent_capability.expiry_time, // Inherit expiry
        revoked: false,
        transferable: parent_capability.transferable,
        delegatable: false, // Derived capabilities cannot be further delegated
    };
    
    system.capabilities.insert(capability_id, derived_capability);
    Ok(capability_id)
}

/// Set expiry time for a capability
pub fn set_capability_expiry(capability_id: CapabilityId, expiry_time: u64) -> Result<(), &'static str> {
    let mut system = CAPABILITY_SYSTEM.write();
    
    let capability = system.capabilities.get_mut(&capability_id)
        .ok_or("Capability not found")?;
    
    capability.expiry_time = Some(expiry_time);
    Ok(())
}

/// Garbage collect expired capabilities
pub fn gc_expired_capabilities() {
    let mut system = CAPABILITY_SYSTEM.write();
    let current_time = crate::time::get_timestamp();
    
    for capability in system.capabilities.values_mut() {
        if let Some(expiry) = capability.expiry_time {
            if current_time > expiry {
                capability.revoked = true;
            }
        }
    }
}