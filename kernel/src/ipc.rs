//! Inter-process communication for RaeenOS
//! Implements pipes, message queues, shared memory, capabilities, and MPSC rings
//! Features: per-process handle tables, capability revocation, flow control, audit logging
//! Enhanced with full capability-based security and performance monitoring

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};

use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use core::fmt::Debug;
use crate::capabilities::{CapabilityType, check_capability, Handle as CapabilityHandle};
use crate::process::ProcessId;

// IPC error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    InvalidHandle,
    PermissionDenied,
    BufferFull,
    BufferEmpty,
    InvalidSize,
    ObjectNotFound,
    HandleTableFull,
    HandleExpired,
    CapabilityRequired,
    TransferFailed,
    DelegationFailed,
    CreationFailed,
}

// IPC object types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcObjectType {
    Pipe,
    MessageQueue,
    SharedMemory,
    MpscRing,
    CapabilityEndpoint,
}

// Capability rights for IPC objects - enhanced with fine-grained permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcRights {
    pub read: bool,
    pub write: bool,
    pub signal: bool,
    pub map: bool,
    pub exec: bool,
    pub dup: bool,
    pub send: bool,
    pub recv: bool,
    // Enhanced rights for capability-based security
    pub transfer: bool,    // Can transfer this handle to other processes
    pub delegate: bool,    // Can create derived handles with reduced rights
    pub revoke: bool,      // Can revoke this handle
    pub inspect: bool,     // Can inspect handle metadata
}

impl IpcRights {
    pub const NONE: Self = Self {
        read: false, write: false, signal: false, map: false,
        exec: false, dup: false, send: false, recv: false,
        transfer: false, delegate: false, revoke: false, inspect: false,
    };
    
    pub const READ_WRITE: Self = Self {
        read: true, write: true, signal: false, map: false,
        exec: false, dup: false, send: false, recv: false,
        transfer: false, delegate: false, revoke: false, inspect: false,
    };
    
    pub const SEND_RECV: Self = Self {
        read: false, write: false, signal: false, map: false,
        exec: false, dup: false, send: true, recv: true,
        transfer: false, delegate: false, revoke: false, inspect: false,
    };
    
    pub const FULL_CONTROL: Self = Self {
        read: true, write: true, signal: true, map: true,
        exec: true, dup: true, send: true, recv: true,
        transfer: true, delegate: true, revoke: true, inspect: true,
    };
    
    pub fn can_shrink_to(&self, other: &Self) -> bool {
        (!other.read || self.read) &&
        (!other.write || self.write) &&
        (!other.signal || self.signal) &&
        (!other.map || self.map) &&
        (!other.exec || self.exec) &&
        (!other.dup || self.dup) &&
        (!other.send || self.send) &&
        (!other.recv || self.recv) &&
        (!other.transfer || self.transfer) &&
        (!other.delegate || self.delegate) &&
        (!other.revoke || self.revoke) &&
        (!other.inspect || self.inspect)
    }
    
    /// Convert IPC rights to capability permissions bitmask
    pub fn to_capability_permissions(&self) -> u64 {
        let mut perms = 0u64;
        if self.read { perms |= 1 << 0; }
        if self.write { perms |= 1 << 1; }
        if self.signal { perms |= 1 << 2; }
        if self.map { perms |= 1 << 3; }
        if self.exec { perms |= 1 << 4; }
        if self.dup { perms |= 1 << 5; }
        if self.send { perms |= 1 << 6; }
        if self.recv { perms |= 1 << 7; }
        if self.transfer { perms |= 1 << 8; }
        if self.delegate { perms |= 1 << 9; }
        if self.revoke { perms |= 1 << 10; }
        if self.inspect { perms |= 1 << 11; }
        perms
    }
    
    /// Convert capability permissions bitmask to IPC rights
    pub fn from_bits(bits: u32) -> Option<Self> {
        Some(Self {
            read: (bits & (1 << 0)) != 0,
            write: (bits & (1 << 1)) != 0,
            signal: (bits & (1 << 2)) != 0,
            map: (bits & (1 << 3)) != 0,
            exec: (bits & (1 << 4)) != 0,
            dup: (bits & (1 << 5)) != 0,
            send: (bits & (1 << 6)) != 0,
            recv: (bits & (1 << 7)) != 0,
            transfer: (bits & (1 << 8)) != 0,
            delegate: (bits & (1 << 9)) != 0,
            revoke: (bits & (1 << 10)) != 0,
            inspect: (bits & (1 << 11)) != 0,
        })
    }
}

// Pipe implementation
#[derive(Debug)]
struct Pipe {
    buffer: Vec<u8>,
    capacity: usize,
    read_pos: usize,
    write_pos: usize,
    readers: u32,
    writers: u32,
    closed: bool,
}

impl Pipe {
    fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            capacity,
            read_pos: 0,
            write_pos: 0,
            readers: 0,
            writers: 0,
            closed: false,
        }
    }
    
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        if self.closed && self.read_pos == self.write_pos {
            return Ok(0); // EOF
        }
        
        let available = if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            self.capacity - self.read_pos + self.write_pos
        };
        
        if available == 0 {
            return Err(()); // Would block
        }
        
        let to_read = core::cmp::min(buf.len(), available);
        let mut bytes_read = 0;
        
        for i in 0..to_read {
            buf[i] = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.capacity;
            bytes_read += 1;
        }
        
        Ok(bytes_read)
    }
    
    fn write(&mut self, buf: &[u8]) -> Result<usize, ()> {
        if self.closed {
            return Err(()); // Broken pipe
        }
        
        let available_space = if self.read_pos > self.write_pos {
            self.read_pos - self.write_pos - 1
        } else if self.read_pos == self.write_pos {
            self.capacity - 1
        } else {
            self.capacity - self.write_pos + self.read_pos - 1
        };
        
        if available_space == 0 {
            return Err(()); // Would block
        }
        
        let to_write = core::cmp::min(buf.len(), available_space);
        let mut bytes_written = 0;
        
        for i in 0..to_write {
            self.buffer[self.write_pos] = buf[i];
            self.write_pos = (self.write_pos + 1) % self.capacity;
            bytes_written += 1;
        }
        
        Ok(bytes_written)
    }
}

// Message queue implementation
#[derive(Debug)]
struct MessageQueue {
    messages: Vec<Vec<u8>>,
    max_messages: usize,
    max_message_size: usize,
}

impl MessageQueue {
    fn new(max_messages: usize, max_message_size: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_messages,
            max_message_size,
        }
    }
    
    fn send(&mut self, message: &[u8]) -> Result<(), ()> {
        if message.len() > self.max_message_size {
            return Err(()); // Message too large
        }
        
        if self.messages.len() >= self.max_messages {
            return Err(()); // Queue full
        }
        
        self.messages.push(message.to_vec());
        Ok(())
    }
    
    fn receive(&mut self) -> Result<Vec<u8>, ()> {
        if self.messages.is_empty() {
            return Err(()); // No messages
        }
        
        Ok(self.messages.remove(0))
    }
}

// Shared memory implementation
#[derive(Debug)]
struct SharedMemory {
    data: Box<[u8]>,
    _size: usize,
    attached_processes: Vec<u32>,
}

impl SharedMemory {
    fn new(size: usize) -> Self {
        Self {
            data: vec![0; size].into_boxed_slice(),
            _size: size,
            attached_processes: Vec::new(),
        }
    }
    
    fn attach(&mut self, process_id: u32) -> Result<(), ()> {
        if !self.attached_processes.contains(&process_id) {
            self.attached_processes.push(process_id);
        }
        Ok(())
    }
    
    fn detach(&mut self, process_id: u32) {
        self.attached_processes.retain(|&pid| pid != process_id);
    }
}

// Backpressure policy for MPSC rings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressurePolicy {
    DropOldest,
    ParkWithTimeout(u64), // timeout in microseconds
    SpillBounded(usize),  // max spill buffer size
}

// MPSC ring buffer with flow control
#[derive(Debug)]
struct MpscRing {
    buffer: Vec<Vec<u8>>,
    capacity: usize,
    head: AtomicU32,
    tail: AtomicU32,
    credits: AtomicU32,
    _max_credits: u32,
    backpressure_policy: BackpressurePolicy,
    spill_buffer: Vec<Vec<u8>>,
    dropped_messages: AtomicU64,
    parked_senders: AtomicU32,
}

impl MpscRing {
    fn new(capacity: usize, max_credits: u32, policy: BackpressurePolicy) -> Self {
        Self {
            buffer: vec![Vec::new(); capacity],
            capacity,
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
            credits: AtomicU32::new(max_credits),
            _max_credits: max_credits,
            backpressure_policy: policy,
            spill_buffer: Vec::new(),
            dropped_messages: AtomicU64::new(0),
            parked_senders: AtomicU32::new(0),
        }
    }
    
    fn send(&mut self, message: Vec<u8>) -> Result<(), &'static str> {
        // Check if we have credits
        let current_credits = self.credits.load(Ordering::Acquire);
        if current_credits == 0 {
            return self.handle_backpressure(message);
        }
        
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        let next_head = (head + 1) % self.capacity as u32;
        
        if next_head == tail {
            return self.handle_backpressure(message).map_err(|_| "Ring buffer full");
        }
        
        // Consume a credit
        self.credits.fetch_sub(1, Ordering::Release);
        
        // Store the message
        self.buffer[head as usize] = message;
        self.head.store(next_head, Ordering::Release);
        
        Ok(())
    }
    
    fn receive(&mut self) -> Result<Vec<u8>, ()> {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        
        if head == tail {
            return Err(()); // Empty
        }
        
        let message = core::mem::take(&mut self.buffer[tail as usize]);
        let next_tail = (tail + 1) % self.capacity as u32;
        self.tail.store(next_tail, Ordering::Release);
        
        // Restore a credit
        self.credits.fetch_add(1, Ordering::Release);
        
        Ok(message)
    }
    
    fn handle_backpressure(&mut self, message: Vec<u8>) -> Result<(), &'static str> {
        match self.backpressure_policy {
            BackpressurePolicy::DropOldest => {
                // Drop the oldest message and insert the new one
                if let Ok(_) = self.receive() {
                    self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                    return self.send(message);
                }
                Err("Failed to drop oldest message")
            },
            BackpressurePolicy::ParkWithTimeout(_timeout) => {
                self.parked_senders.fetch_add(1, Ordering::Relaxed);
                // TODO: Implement actual parking with timeout
                // For now, just fail
                Err("Sender would block")
            },
            BackpressurePolicy::SpillBounded(max_spill) => {
                if self.spill_buffer.len() < max_spill {
                    self.spill_buffer.push(message);
                    Ok(())
                } else {
                    self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                    Err("Spill buffer full")
                }
            },
        }
    }
    
    fn get_stats(&self) -> MpscRingStats {
        MpscRingStats {
            capacity: self.capacity,
            current_size: self.get_current_size(),
            credits_available: self.credits.load(Ordering::Relaxed),
            dropped_messages: self.dropped_messages.load(Ordering::Relaxed),
            parked_senders: self.parked_senders.load(Ordering::Relaxed),
            spill_buffer_size: self.spill_buffer.len(),
        }
    }
    
    fn get_current_size(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);
        if head >= tail {
            (head - tail) as usize
        } else {
            (self.capacity as u32 - tail + head) as usize
        }
    }
}

// Statistics for MPSC rings
#[derive(Debug, Clone)]
pub struct MpscRingStats {
    pub capacity: usize,
    pub current_size: usize,
    pub credits_available: u32,
    pub dropped_messages: u64,
    pub parked_senders: u32,
    pub spill_buffer_size: usize,
}

// Audit log entry for IPC operations
#[derive(Debug, Clone)]
struct AuditLogEntry {
    _timestamp: u64,
    process_id: u32,
    _operation: String,
    _object_id: u32,
    _result: bool,
    _details: String,
}

// Audit log with bounded size and rate limiting
#[derive(Debug)]
struct AuditLog {
    entries: Vec<AuditLogEntry>,
    max_entries: usize,
    per_pid_counters: BTreeMap<u32, (u32, u64)>, // (count, last_reset_time)
    rate_limit_per_second: u32,
}

impl AuditLog {
    fn new(max_entries: usize, rate_limit_per_second: u32) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
            per_pid_counters: BTreeMap::new(),
            rate_limit_per_second,
        }
    }
    
    fn _get_recent_entries(&self, count: usize) -> Vec<AuditLogEntry> {
        self.entries.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }
    
    fn log(&mut self, process_id: u32, operation: String, object_id: u32, result: String, details: String) -> Result<(), ()> {
         let entry = AuditLogEntry {
             _timestamp: crate::time::get_uptime_ms() * 1000, // Convert to microseconds
            process_id,
            _operation: operation,
            _object_id: object_id,
            _result: result == "success",
            _details: details,
        };
        self.log_entry(entry)
    }
    
    fn log_entry(&mut self, entry: AuditLogEntry) -> Result<(), ()> {
        // Check rate limit for this PID
         let current_time = crate::time::get_uptime_ms() * 1000; // Convert to microseconds
        let current_second = current_time / 1_000_000;
        
        let (count, last_reset) = self.per_pid_counters
            .get(&entry.process_id)
            .copied()
            .unwrap_or((0, current_second));
        
        let (new_count, new_last_reset) = if current_second > last_reset {
            (1, current_second)
        } else {
            (count + 1, last_reset)
        };
        
        if new_count > self.rate_limit_per_second {
            return Err(()); // Rate limited
        }
        
        self.per_pid_counters.insert(entry.process_id, (new_count, new_last_reset));
        
        // Add entry to log
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0); // Remove oldest entry
        }
        
        self.entries.push(entry);
        Ok(())
    }
}

// IPC object wrapper
// Capability endpoint for secure IPC communication
#[derive(Debug, Clone)]
pub struct CapabilityEndpoint {
    pub endpoint_id: u64,
    pub service_name: String,
    pub process_id: u32,
    pub capabilities: Vec<String>,
    pub message_queue: u32, // Handle to underlying message queue
    pub max_message_size: usize,
    pub created_at: u64,
}

impl CapabilityEndpoint {
    pub fn new(endpoint_id: u64, service_name: String, process_id: u32, message_queue: u32) -> Self {
        Self {
            endpoint_id,
            service_name,
            process_id,
            capabilities: Vec::new(),
            message_queue,
            max_message_size: 4096,
            created_at: crate::time::get_uptime_ms(),
        }
    }
    
    pub fn add_capability(&mut self, capability: String) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }
    
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
    
    pub fn send_message(&self, message: &[u8]) -> Result<(), IpcError> {
        if message.len() > self.max_message_size {
            return Err(IpcError::InvalidSize);
        }
        
        // Send through underlying message queue
        let mut ipc = IPC_SYSTEM.lock();
        ipc.write_to_object(self.process_id, self.message_queue, message)
            .map(|_| ())
    }
    
    pub fn receive_message(&self, buffer: &mut [u8]) -> Result<usize, IpcError> {
        // Receive from underlying message queue
        let mut ipc = IPC_SYSTEM.lock();
        ipc.read_from_object(self.process_id, self.message_queue, buffer)
    }
}

enum IpcObject {
    Pipe(Pipe),
    MessageQueue(MessageQueue),
    SharedMemory(SharedMemory),
    MpscRing(MpscRing),
    CapabilityEndpoint(CapabilityEndpoint),
}

// Enhanced file descriptor with capability-based rights
#[derive(Debug)]
struct FileDescriptor {
    ipc_id: u32,
    object_type: IpcObjectType,
    rights: IpcRights,
    process_id: u32,
    _generation: u32,
    _expiry_time: Option<u64>, // microseconds since boot
    _label: Option<String>,    // for bulk revocation
}

// Per-process handle table entry
#[derive(Debug, Clone)]
struct HandleEntry {
    _index: u32,
    _generation: u32,
    rights: IpcRights,
    object_id: u32,
    object_type: IpcObjectType,
    expiry_time: Option<u64>,
    label: Option<String>,
}

// Per-process handle table
#[derive(Debug)]
struct ProcessHandleTable {
    handles: BTreeMap<u32, HandleEntry>,
    next_handle: u32,
    _process_id: u32,
}

impl ProcessHandleTable {
    fn new(process_id: u32) -> Self {
        Self {
            handles: BTreeMap::new(),
            next_handle: 3, // Start after stdin(0), stdout(1), stderr(2)
            _process_id: process_id,
        }
    }
    
    fn allocate_handle(&mut self, object_id: u32, object_type: IpcObjectType, 
                      rights: IpcRights, expiry_time: Option<u64>, 
                      label: Option<String>) -> u32 {
        let handle_id = self.next_handle;
        self.next_handle += 1;
        
        let entry = HandleEntry {
            _index: handle_id,
            _generation: 1,
            rights,
            object_id,
            object_type,
            expiry_time,
            label,
        };
        
        self.handles.insert(handle_id, entry);
        handle_id
    }
    
    fn get_handle(&self, handle_id: u32) -> Option<&HandleEntry> {
        self.handles.get(&handle_id)
    }
    
    fn revoke_handle(&mut self, handle_id: u32) -> bool {
        self.handles.remove(&handle_id).is_some()
    }
    
    fn revoke_by_label(&mut self, label: &str) -> usize {
        let to_remove: Vec<u32> = self.handles
            .iter()
            .filter(|(_, entry)| {
                entry.label.as_ref().map_or(false, |l| l == label)
            })
            .map(|(id, _)| *id)
            .collect();
        
        let count = to_remove.len();
        for handle_id in to_remove {
            self.handles.remove(&handle_id);
        }
        count
    }
    
    fn cleanup_expired(&mut self, current_time: u64) -> usize {
        let to_remove: Vec<u32> = self.handles
            .iter()
            .filter(|(_, entry)| {
                entry.expiry_time.map_or(false, |expiry| current_time > expiry)
            })
            .map(|(id, _)| *id)
            .collect();
        
        let count = to_remove.len();
        for handle_id in to_remove {
            self.handles.remove(&handle_id);
        }
        count
    }
    
    fn clone_handle(&mut self, handle_id: u32, new_rights: IpcRights) -> Option<u32> {
        if let Some(entry) = self.handles.get(&handle_id) {
            // Rights can only shrink
            if !entry.rights.can_shrink_to(&new_rights) {
                return None;
            }
            
            let new_handle = self.allocate_handle(
                entry.object_id,
                entry.object_type,
                new_rights,
                entry.expiry_time,
                entry.label.clone(),
            );
            
            Some(new_handle)
        } else {
            None
        }
    }
}

// IPC system state
struct IpcSystem {
    objects: BTreeMap<u32, IpcObject>,
    file_descriptors: BTreeMap<u32, FileDescriptor>,
    handle_tables: BTreeMap<u32, ProcessHandleTable>, // process_id -> handle table
    next_ipc_id: u32,
    _next_fd_id: u32,
    audit_log: AuditLog,
}

impl IpcSystem {
    fn new() -> Self {
        Self {
            objects: BTreeMap::new(),
            file_descriptors: BTreeMap::new(),
            handle_tables: BTreeMap::new(),
            next_ipc_id: 1,
            _next_fd_id: 1,
            audit_log: AuditLog::new(1000, 100), // 1000 entry capacity, 100 ops/sec rate limit
        }
    }
    
    /// Validate that a process has the required capability for an IPC operation
    fn validate_capability(&self, process_id: u32, capability_handle: CapabilityHandle, capability_type: CapabilityType, permissions: u64) -> Result<(), IpcError> {
        match check_capability(process_id as ProcessId, capability_handle, capability_type, permissions) {
            Ok(true) => Ok(()),
            Ok(false) => Err(IpcError::PermissionDenied),
            Err(_) => Err(IpcError::CapabilityRequired),
        }
    }
    
    /// Validate handle rights for an operation
    fn validate_handle_rights(&self, process_id: u32, handle_id: u32, required_rights: IpcRights) -> Result<(), IpcError> {
        let handle_table = self.handle_tables.get(&process_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        let handle = handle_table.get_handle(handle_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        if !handle.rights.can_shrink_to(&required_rights) {
            return Err(IpcError::PermissionDenied);
        }
        
        // Check if handle is expired
        if let Some(expiry) = handle.expiry_time {
            let current_time = crate::time::get_uptime_ms() * 1000; // Convert to microseconds
            if current_time > expiry {
                return Err(IpcError::HandleExpired);
            }
        }
        
        Ok(())
    }
    
    /// Read from IPC object with capability validation
    pub fn read_from_object(&mut self, process_id: u32, handle_id: u32, buffer: &mut [u8]) -> Result<usize, IpcError> {
        // Validate read rights
        let required_rights = IpcRights { read: true, ..IpcRights::NONE };
        self.validate_handle_rights(process_id, handle_id, required_rights)
            .map_err(|_| IpcError::PermissionDenied)?;
        
        let handle_table = self.handle_tables.get(&process_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        let handle = handle_table.get_handle(handle_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        let object = self.objects.get_mut(&handle.object_id)
            .ok_or(IpcError::ObjectNotFound)?;
        
        match object {
            IpcObject::Pipe(pipe) => {
                let result = pipe.read(buffer).map_err(|_| IpcError::BufferEmpty)?;
                
                let _ = self.audit_log.log(
                    process_id,
                    "read_from_pipe".to_string(),
                    handle.object_id,
                    "success".to_string(),
                    format!("handle={}, bytes={}", handle_id, result),
                );
                
                Ok(result)
            },
            IpcObject::SharedMemory(shm) => {
                let to_read = buffer.len().min(shm.data.len());
                buffer[..to_read].copy_from_slice(&shm.data[..to_read]);
                
                let _ = self.audit_log.log(
                    process_id,
                    "read_from_shared_memory".to_string(),
                    handle.object_id,
                    "success".to_string(),
                    format!("handle={}, bytes={}", handle_id, to_read),
                );
                
                Ok(to_read)
            },
            _ => Err(IpcError::InvalidHandle),
        }
    }
    
    /// Write to IPC object with capability validation
    pub fn write_to_object(&mut self, process_id: u32, handle_id: u32, data: &[u8]) -> Result<usize, IpcError> {
        // Validate write rights
        let required_rights = IpcRights { write: true, ..IpcRights::NONE };
        self.validate_handle_rights(process_id, handle_id, required_rights)
            .map_err(|_| IpcError::PermissionDenied)?;
        
        let handle_table = self.handle_tables.get(&process_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        let handle = handle_table.get_handle(handle_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        let object = self.objects.get_mut(&handle.object_id)
            .ok_or(IpcError::ObjectNotFound)?;
        
        match object {
            IpcObject::Pipe(pipe) => {
                let result = pipe.write(data).map_err(|_| IpcError::BufferFull)?;
                
                let _ = self.audit_log.log(
                    process_id,
                    "write_to_pipe".to_string(),
                    handle.object_id,
                    "success".to_string(),
                    format!("handle={}, bytes={}", handle_id, result),
                );
                
                Ok(result)
            },
            IpcObject::SharedMemory(shm) => {
                let to_write = data.len().min(shm.data.len());
                shm.data[..to_write].copy_from_slice(&data[..to_write]);
                
                let _ = self.audit_log.log(
                    process_id,
                    "write_to_shared_memory".to_string(),
                    handle.object_id,
                    "success".to_string(),
                    format!("handle={}, bytes={}", handle_id, to_write),
                );
                
                Ok(to_write)
            },
            _ => Err(IpcError::InvalidHandle),
        }
    }
    
    fn get_or_create_handle_table(&mut self, process_id: u32) -> &mut ProcessHandleTable {
        self.handle_tables.entry(process_id)
            .or_insert_with(|| ProcessHandleTable::new(process_id))
    }
    
    fn cleanup_process_handles(&mut self, process_id: u32) {
        self.handle_tables.remove(&process_id);
        
        // Also clean up old file descriptors for this process
        let to_remove: Vec<u32> = self.file_descriptors
            .iter()
            .filter(|(_, fd)| fd.process_id == process_id)
            .map(|(id, _)| *id)
            .collect();
        
        for fd_id in to_remove {
            self.file_descriptors.remove(&fd_id);
        }
    }
    
    fn create_mpsc_ring(&mut self, process_id: u32, capability_handle: CapabilityHandle, capacity: usize, 
                       policy: BackpressurePolicy, rights: IpcRights,
                       expiry_time: Option<u64>, label: Option<String>) -> Result<u32, IpcError> {
        // Validate capability to create IPC objects
        self.validate_capability(process_id, capability_handle, CapabilityType::IpcCreate, 0x01)?;
        
        if capacity == 0 || capacity > 65536 {
            return Err(IpcError::InvalidSize);
        }
        
        let ring = MpscRing::new(capacity, 1000, policy); // 1000 max credits
        let ipc_id = self.next_ipc_id;
        self.next_ipc_id += 1;
        
        self.objects.insert(ipc_id, IpcObject::MpscRing(ring));
        
        // Create handle in process handle table with delegation rights for creator
        let creator_rights = IpcRights { 
            delegate: true, transfer: true, revoke: true, 
            ..rights 
        };
        
        let handle_table = self.get_or_create_handle_table(process_id);
        let handle_id = handle_table.allocate_handle(
            ipc_id, 
            IpcObjectType::MpscRing, 
            creator_rights, 
            expiry_time, 
            label.clone()
        );
        
        // Log the operation
        let _ = self.audit_log.log(
            process_id,
            "create_mpsc_ring".to_string(),
            ipc_id,
            "success".to_string(),
            format!("capacity={}, policy={:?}, handle={}", capacity, policy, handle_id),
        );
        
        Ok(handle_id)
    }
    
    fn send_to_ring(&mut self, process_id: u32, handle_id: u32, 
                   data: &[u8]) -> Result<(), IpcError> {
        // Validate handle rights for send operation
        let required_rights = IpcRights { send: true, ..IpcRights::NONE };
        self.validate_handle_rights(process_id, handle_id, required_rights)?;
        
        // Get handle info
        let handle_table = self.handle_tables.get(&process_id)
            .ok_or(IpcError::ObjectNotFound)?;
        
        let handle = handle_table.get_handle(handle_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        // Get the ring object
        if let Some(IpcObject::MpscRing(ring)) = self.objects.get_mut(&handle.object_id) {
            let result = ring.send(data.to_vec()).map_err(|_| IpcError::BufferFull);
            
            let result_str = match result {
                Ok(_) => "success",
                Err(_) => "failed",
            };
            
            let _ = self.audit_log.log(
                process_id,
                "send_to_ring".to_string(),
                handle.object_id,
                result_str.to_string(),
                format!("handle={}, size={}", handle_id, data.len()),
            );
            
            result
        } else {
            Err(IpcError::ObjectNotFound)
        }
    }
    
    fn receive_from_ring(&mut self, process_id: u32, handle_id: u32) -> Result<Vec<u8>, IpcError> {
        // Validate handle rights for receive operation
        let required_rights = IpcRights { recv: true, ..IpcRights::NONE };
        self.validate_handle_rights(process_id, handle_id, required_rights)?;
        
        // Get handle info
        let handle_table = self.handle_tables.get(&process_id)
            .ok_or(IpcError::ObjectNotFound)?;
        
        let handle = handle_table.get_handle(handle_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        // Get the ring object
        if let Some(IpcObject::MpscRing(ring)) = self.objects.get_mut(&handle.object_id) {
            let result = ring.receive().map_err(|_| IpcError::BufferEmpty);
            
            let (result_str, size) = match &result {
                Ok(data) => ("success", data.len()),
                Err(_) => ("failed", 0),
            };
            
            let _ = self.audit_log.log(
                process_id,
                "receive_from_ring".to_string(),
                handle.object_id,
                result_str.to_string(),
                format!("handle={}, size={}", handle_id, size),
            );
            
            result
        } else {
            Err(IpcError::ObjectNotFound)
        }
    }
    
    fn revoke_handle(&mut self, process_id: u32, handle_id: u32) -> Result<(), &'static str> {
        let handle_table = self.handle_tables.get_mut(&process_id)
            .ok_or("Process has no handle table")?;
        
        if handle_table.revoke_handle(handle_id) {
            let _ = self.audit_log.log(
                process_id,
                "revoke_handle".to_string(),
                0, // No specific object
                "success".to_string(),
                format!("handle={}", handle_id),
            );
            Ok(())
        } else {
            Err("Handle not found")
        }
    }
    
    fn revoke_handles_by_label(&mut self, process_id: u32, label: &str) -> Result<usize, &'static str> {
        let handle_table = self.handle_tables.get_mut(&process_id)
            .ok_or("Process has no handle table")?;
        
        let count = handle_table.revoke_by_label(label);
        
        let _ = self.audit_log.log(
            process_id,
            "revoke_by_label".to_string(),
            0,
            "success".to_string(),
            format!("label={}, count={}", label, count),
        );
        
        Ok(count)
    }
    
    fn cleanup_expired_handles(&mut self, process_id: u32) -> Result<usize, &'static str> {
         let current_time = crate::time::get_uptime_ms() * 1000; // Convert to microseconds
        
        let handle_table = self.handle_tables.get_mut(&process_id)
            .ok_or("Process has no handle table")?;
        
        let count = handle_table.cleanup_expired(current_time);
        
        if count > 0 {
            let _ = self.audit_log.log(
                process_id,
                "cleanup_expired".to_string(),
                0,
                "success".to_string(),
                format!("count={}", count),
            );
        }
        
        Ok(count)
    }
    
    fn clone_handle(&mut self, process_id: u32, handle_id: u32, 
                   new_rights: IpcRights) -> Result<u32, IpcError> {
        // Validate delegate rights for cloning
        let required_rights = IpcRights { delegate: true, ..IpcRights::NONE };
        self.validate_handle_rights(process_id, handle_id, required_rights)?;
        
        let handle_table = self.handle_tables.get_mut(&process_id)
            .ok_or(IpcError::InvalidHandle)?;
        
        if let Some(new_handle) = handle_table.clone_handle(handle_id, new_rights) {
            let _ = self.audit_log.log(
                process_id,
                "clone_handle".to_string(),
                0,
                "success".to_string(),
                format!("original={}, new={}", handle_id, new_handle),
            );
            Ok(new_handle)
        } else {
            Err(IpcError::InvalidHandle)
        }
    }
    
    /// Transfer a handle to another process
    fn transfer_handle(&mut self, from_process: u32, to_process: u32, 
                      handle_id: u32, new_rights: IpcRights) -> Result<u32, IpcError> {
        // Validate transfer rights
        let required_rights = IpcRights { transfer: true, ..IpcRights::NONE };
        self.validate_handle_rights(from_process, handle_id, required_rights)?;
        
        // Get the original handle
        let (object_id, object_type, _original_rights, expiry_time, label) = {
            let from_table = self.handle_tables.get(&from_process)
                .ok_or(IpcError::InvalidHandle)?;
            
            let handle = from_table.get_handle(handle_id)
                .ok_or(IpcError::InvalidHandle)?;
            
            // Rights can only shrink during transfer
            if !handle.rights.can_shrink_to(&new_rights) {
                return Err(IpcError::InvalidHandle);
            }
            
            (handle.object_id, handle.object_type, handle.rights, 
             handle.expiry_time, handle.label.clone())
        };
        
        // Create handle in destination process
        let to_table = self.get_or_create_handle_table(to_process);
        let new_handle = to_table.allocate_handle(
            object_id, object_type, new_rights, expiry_time, label
        );
        
        // Remove from source process
        let from_table = self.handle_tables.get_mut(&from_process).unwrap();
        from_table.revoke_handle(handle_id);
        
        let _ = self.audit_log.log(
            from_process,
            "transfer_handle".to_string(),
            object_id,
            "success".to_string(),
            format!("to_process={}, old_handle={}, new_handle={}", to_process, handle_id, new_handle),
        );
        
        Ok(new_handle)
    }
    
    /// Delegate a handle to another process (original handle remains)
    fn delegate_handle(&mut self, from_process: u32, to_process: u32, 
                      handle_id: u32, new_rights: IpcRights, 
                      expiry_time: Option<u64>, label: Option<String>) -> Result<u32, IpcError> {
        // Validate delegate rights
        let required_rights = IpcRights { delegate: true, ..IpcRights::NONE };
        self.validate_handle_rights(from_process, handle_id, required_rights)?;
        
        // Get the original handle info
        let (object_id, object_type) = {
            let from_table = self.handle_tables.get(&from_process)
                .ok_or(IpcError::InvalidHandle)?;
            
            let handle = from_table.get_handle(handle_id)
                .ok_or(IpcError::InvalidHandle)?;
            
            // Rights can only shrink during delegation
            if !handle.rights.can_shrink_to(&new_rights) {
                return Err(IpcError::InvalidHandle);
            }
            
            (handle.object_id, handle.object_type)
        };
        
        // Create handle in destination process
        let to_table = self.get_or_create_handle_table(to_process);
        let new_handle = to_table.allocate_handle(
            object_id, object_type, new_rights, expiry_time, label
        );
        
        let _ = self.audit_log.log(
            from_process,
            "delegate_handle".to_string(),
            object_id,
            "success".to_string(),
            format!("to_process={}, source_handle={}, new_handle={}", to_process, handle_id, new_handle),
        );
        
        Ok(new_handle)
    }
}

lazy_static! {
    static ref IPC_SYSTEM: Mutex<IpcSystem> = Mutex::new(IpcSystem::new());
}

// Create a pipe with capability-based access
pub fn create_pipe() -> Result<(u32, u32), ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid as u32, "system.call").unwrap_or(false) {
        return Err(());
    }
    
    // Create pipe object
    let pipe_id = ipc.next_ipc_id;
    ipc.next_ipc_id += 1;
    
    let mut pipe = Pipe::new(4096); // 4KB buffer
    pipe.readers = 1;
    pipe.writers = 1;
    
    ipc.objects.insert(pipe_id, IpcObject::Pipe(pipe));
    
    // Create handles in process handle table
    let handle_table = ipc.get_or_create_handle_table(current_pid as u32);
    
    let read_handle = handle_table.allocate_handle(
        pipe_id,
        IpcObjectType::Pipe,
        IpcRights { read: true, write: false, ..IpcRights::NONE },
        None, // No expiry
        Some("pipe_read".to_string()),
    );
    
    let write_handle = handle_table.allocate_handle(
        pipe_id,
        IpcObjectType::Pipe,
        IpcRights { read: false, write: true, ..IpcRights::NONE },
        None, // No expiry
        Some("pipe_write".to_string()),
    );

    // Log the operation
    let _ = ipc.audit_log.log(
        current_pid as u32,
        "create_pipe".to_string(),
        pipe_id,
        "success".to_string(),
        format!("read_handle={}, write_handle={}", read_handle, write_handle),
    );

    Ok((read_handle, write_handle))
}

// Create a message queue with capability-based access
pub fn create_message_queue(max_messages: usize, max_message_size: usize) -> Result<u32, ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid as u32, "system.call").unwrap_or(false) {
        return Err(());
    }
    
    let queue_id = ipc.next_ipc_id;
    ipc.next_ipc_id += 1;
    
    let queue = MessageQueue::new(max_messages, max_message_size);
    ipc.objects.insert(queue_id, IpcObject::MessageQueue(queue));
    
    // Create handle in process handle table
    let handle_table = ipc.get_or_create_handle_table(current_pid as u32);
    let handle_id = handle_table.allocate_handle(
        queue_id,
        IpcObjectType::MessageQueue,
        IpcRights::SEND_RECV, // Full send/receive rights
        None, // No expiry
        Some("message_queue".to_string()),
    );

    // Log the operation
    let _ = ipc.audit_log.log(
        current_pid as u32,
        "create_message_queue".to_string(),
        queue_id,
        "success".to_string(),
        format!("max_messages={}, max_message_size={}, handle={}", max_messages, max_message_size, handle_id),
    );
    
    Ok(handle_id)
}

// Create shared memory with capability-based access
pub fn create_shared_memory(size: usize) -> Result<u32, ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid as u32, "system.call").unwrap_or(false) {
        return Err(());
    }
    
    let shm_id = ipc.next_ipc_id;
    ipc.next_ipc_id += 1;
    
    let mut shm = SharedMemory::new(size);
    let _ = shm.attach(current_pid as u32);
    
    ipc.objects.insert(shm_id, IpcObject::SharedMemory(shm));
    
    // Create handle in process handle table
    let handle_table = ipc.get_or_create_handle_table(current_pid as u32);
    let handle_id = handle_table.allocate_handle(
        shm_id,
        IpcObjectType::SharedMemory,
        IpcRights { read: true, write: true, map: true, ..IpcRights::NONE },
        None, // No expiry
        Some("shared_memory".to_string()),
    );

    // Log the operation
    let _ = ipc.audit_log.log(
        current_pid as u32,
        "create_shared_memory".to_string(),
        shm_id,
        "success".to_string(),
        format!("size={}, handle={}", size, handle_id),
    );
    
    Ok(handle_id)
}

// Read from IPC object
pub fn ipc_read(fd: u32, buf: &mut [u8]) -> Result<usize, ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Extract the necessary information from file descriptor first
    let (ipc_id, process_id, readable) = {
        let file_desc = ipc.file_descriptors.get(&fd).ok_or(())?;
        (file_desc.ipc_id, file_desc.process_id, file_desc.rights.read)
    };
    
    // Check ownership and permissions
    if process_id != current_pid as u32 || !readable {
        return Err(());
    }
    
    match ipc.objects.get_mut(&ipc_id).ok_or(())? {
        IpcObject::Pipe(pipe) => pipe.read(buf),
        IpcObject::MessageQueue(queue) => {
            let message = queue.receive()?;
            let to_copy = core::cmp::min(buf.len(), message.len());
            buf[..to_copy].copy_from_slice(&message[..to_copy]);
            Ok(to_copy)
        }
        IpcObject::SharedMemory(_) => Err(()), // Use attach/detach for shared memory
        IpcObject::MpscRing(_) => Err(()), // Use receive_from_ring for MPSC rings
        IpcObject::CapabilityEndpoint(endpoint) => {
            endpoint.receive_message(buf).map_err(|_| ())
        }
    }
}

// Write to IPC object
pub fn ipc_write(fd: u32, buf: &[u8]) -> Result<usize, ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Extract needed values before mutable operations
    let (ipc_id, _writable) = {
        let file_desc = ipc.file_descriptors.get(&fd).ok_or(())?;
        
        // Check ownership and permissions
        if file_desc.process_id != current_pid as u32 || !file_desc.rights.write {
            return Err(());
        }
        
        (file_desc.ipc_id, file_desc.rights.write)
    };
    
    match ipc.objects.get_mut(&ipc_id).ok_or(())? {
        IpcObject::Pipe(pipe) => pipe.write(buf),
        IpcObject::MessageQueue(queue) => {
            queue.send(buf)?;
            Ok(buf.len())
        }
        IpcObject::SharedMemory(_) => Err(()), // Use attach/detach for shared memory
        IpcObject::MpscRing(_) => Err(()), // Use send_to_ring for MPSC rings
        IpcObject::CapabilityEndpoint(endpoint) => {
            endpoint.send_message(buf).map_err(|_| ())?;
            Ok(buf.len())
        }
    }
}

// Close IPC file descriptor
pub fn close_ipc_fd(fd: u32) -> Result<(), ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Extract needed values before mutable operations
    let (ipc_id, object_type, readable, writable) = {
        let file_desc = ipc.file_descriptors.get(&fd).ok_or(())?;
        
        // Check ownership
        if file_desc.process_id != current_pid as u32 {
            return Err(());
        }
        
        (file_desc.ipc_id, file_desc.object_type, file_desc.rights.read, file_desc.rights.write)
    };
    
    ipc.file_descriptors.remove(&fd);
    
    // Update reference counts for pipes
    if object_type == IpcObjectType::Pipe {
        if let Some(IpcObject::Pipe(pipe)) = ipc.objects.get_mut(&ipc_id) {
            if readable {
                pipe.readers = pipe.readers.saturating_sub(1);
            }
            if writable {
                pipe.writers = pipe.writers.saturating_sub(1);
            }
            
            // Close pipe if no more readers or writers
            if pipe.readers == 0 || pipe.writers == 0 {
                pipe.closed = true;
            }
        }
    }
    
    Ok(())
}

/// Create a capability endpoint for secure service communication
pub fn create_capability_endpoint(
    service_name: String,
    process_id: u32,
    capabilities: Vec<String>,
) -> Result<u32, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    
    // Create underlying message queue for the endpoint
    let queue_id = ipc.next_ipc_id;
    ipc.next_ipc_id += 1;
    
    let queue = MessageQueue::new(100, 4096); // 100 messages, 4KB each
    ipc.objects.insert(queue_id, IpcObject::MessageQueue(queue));
    
    // Create the capability endpoint
    let endpoint_id = ipc.next_ipc_id;
    ipc.next_ipc_id += 1;
    
    let mut endpoint = CapabilityEndpoint::new(
        endpoint_id as u64,
        service_name.clone(),
        process_id,
        queue_id,
    );
    
    // Add capabilities
    for capability in capabilities {
        endpoint.add_capability(capability);
    }
    
    ipc.objects.insert(endpoint_id, IpcObject::CapabilityEndpoint(endpoint));
    
    // Create handle in process handle table
    let handle_table = ipc.get_or_create_handle_table(process_id);
    let handle_id = handle_table.allocate_handle(
        endpoint_id,
        IpcObjectType::CapabilityEndpoint,
        IpcRights::SEND_RECV,
        None,
        Some(service_name.clone()),
    );
    
    // Log the operation
    let _ = ipc.audit_log.log(
        process_id,
        "create_capability_endpoint".to_string(),
        endpoint_id,
        "success".to_string(),
        format!("service={}, handle={}", service_name, handle_id),
    );
    
    Ok(handle_id)
}

/// Get capability endpoint by handle
pub fn get_capability_endpoint(process_id: u32, handle_id: u32) -> Result<CapabilityEndpoint, IpcError> {
    let ipc = IPC_SYSTEM.lock();
    
    let handle_table = ipc.handle_tables.get(&process_id)
        .ok_or(IpcError::InvalidHandle)?;
    
    let handle = handle_table.get_handle(handle_id)
        .ok_or(IpcError::InvalidHandle)?;
    
    if handle.object_type != IpcObjectType::CapabilityEndpoint {
        return Err(IpcError::InvalidHandle);
    }
    
    let object = ipc.objects.get(&handle.object_id)
        .ok_or(IpcError::ObjectNotFound)?;
    
    if let IpcObject::CapabilityEndpoint(endpoint) = object {
        Ok(endpoint.clone())
    } else {
        Err(IpcError::InvalidHandle)
    }
}

// Attach to shared memory
pub fn attach_shared_memory(shm_id: u32) -> Result<*mut u8, ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid as u32, "system.call").unwrap_or(false) {
        return Err(());
    }
    
    if let Some(IpcObject::SharedMemory(shm)) = ipc.objects.get_mut(&shm_id) {
        shm.attach(current_pid as u32)?;
        Ok(shm.data.as_mut_ptr())
    } else {
        Err(())
    }
}

// Detach from shared memory
pub fn detach_shared_memory(shm_id: u32) -> Result<(), ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    if let Some(IpcObject::SharedMemory(shm)) = ipc.objects.get_mut(&shm_id) {
        shm.detach(current_pid as u32);
        Ok(())
    } else {
        Err(())
    }
}

// Clean up IPC objects for a process
pub fn cleanup_process_ipc(process_id: u32) {
    let mut ipc = IPC_SYSTEM.lock();
    
    // Close all file descriptors owned by the process
    let fds_to_close: Vec<u32> = ipc.file_descriptors
        .iter()
        .filter(|(_, fd)| fd.process_id == process_id)
        .map(|(&fd, _)| fd)
        .collect();
    
    for fd in fds_to_close {
        let _ = close_ipc_fd(fd);
    }
    
    // Detach from all shared memory objects
    let shm_ids: Vec<u32> = ipc.objects
        .iter()
        .filter_map(|(&id, obj)| {
            if let IpcObject::SharedMemory(shm) = obj {
                if shm.attached_processes.contains(&process_id) {
                    Some(id)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    
    for shm_id in shm_ids {
        let _ = detach_shared_memory(shm_id);
    }
    
    // Clean up process handle table
    ipc.cleanup_process_handles(process_id);
}

// Create an MPSC ring with flow control
pub fn create_mpsc_ring(process_id: u32, capability_handle: CapabilityHandle, capacity: usize, policy: BackpressurePolicy) -> Result<u32, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.create_mpsc_ring(
        process_id,
        capability_handle,
        capacity, 
        policy, 
        IpcRights::SEND_RECV, 
        None, 
        Some("mpsc_ring".to_string())
    )
}

// Send data to an MPSC ring
pub fn send_to_ring(process_id: u32, handle_id: u32, data: &[u8]) -> Result<(), IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.send_to_ring(process_id, handle_id, data)
}

// Receive data from an MPSC ring
pub fn receive_from_ring(process_id: u32, handle_id: u32) -> Result<Vec<u8>, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.receive_from_ring(process_id, handle_id)
}

// Revoke a specific handle
pub fn revoke_handle(process_id: u32, handle_id: u32) -> Result<(), IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.revoke_handle(process_id, handle_id).map_err(|_| IpcError::InvalidHandle)
}

// Revoke all handles with a specific label (bulk revocation)
pub fn revoke_handles_by_label(process_id: u32, label: &str) -> Result<usize, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.revoke_handles_by_label(process_id, label).map_err(|_| IpcError::InvalidHandle)
}

// Clean up expired handles for a process
pub fn cleanup_expired_handles(process_id: u32) -> Result<usize, &'static str> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.cleanup_expired_handles(process_id)
}

// Clone a handle with reduced rights
pub fn clone_handle(process_id: u32, handle_id: u32, new_rights: IpcRights) -> Result<u32, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.clone_handle(process_id, handle_id, new_rights)
}

// Get MPSC ring statistics
pub fn get_ring_stats(process_id: u32, handle_id: u32) -> Result<MpscRingStats, IpcError> {
    let ipc = IPC_SYSTEM.lock();
    
    // Check handle permissions
    let handle_table = ipc.handle_tables.get(&process_id)
        .ok_or(IpcError::InvalidHandle)?;
    
    let handle = handle_table.get_handle(handle_id)
        .ok_or(IpcError::InvalidHandle)?;
    
    // Get the ring object
    if let Some(IpcObject::MpscRing(ring)) = ipc.objects.get(&handle.object_id) {
        Ok(ring.get_stats())
    } else {
        Err(IpcError::ObjectNotFound)
    }
}

/// Transfer a handle to another process (capability-based)
pub fn transfer_handle(from_process: u32, to_process: u32, handle_id: u32, new_rights: IpcRights) -> Result<u32, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.transfer_handle(from_process, to_process, handle_id, new_rights).map_err(|_| IpcError::TransferFailed)
}

/// Delegate a handle to another process (capability-based)
pub fn delegate_handle(from_process: u32, to_process: u32, handle_id: u32, 
                      new_rights: IpcRights, expiry_time: Option<u64>, 
                      label: Option<String>) -> Result<u32, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    ipc.delegate_handle(from_process, to_process, handle_id, new_rights, expiry_time, label).map_err(|_| IpcError::DelegationFailed)
}

/// Create a pipe with enhanced capability validation
pub fn create_pipe_with_capabilities(process_id: u32, capability_handle: CapabilityHandle, buffer_size: usize) -> Result<(u32, u32), IpcError> {
    let mut system = IPC_SYSTEM.lock();
    
    // Validate IPC creation capability
    system.validate_capability(process_id, capability_handle, CapabilityType::IpcCreate, 0x01)?;
    
    let pipe_id = system.next_ipc_id;
    system.next_ipc_id += 1;
    
    let pipe = Pipe::new(buffer_size);
    system.objects.insert(pipe_id, IpcObject::Pipe(pipe));
    
    let handle_table = system.get_or_create_handle_table(process_id);
    
    let read_handle = handle_table.allocate_handle(
        pipe_id,
        IpcObjectType::Pipe,
        IpcRights::READ_WRITE,
        None,
        None,
    );
    
    let write_handle = handle_table.allocate_handle(
        pipe_id,
        IpcObjectType::Pipe,
        IpcRights::READ_WRITE,
        None,
        None,
    );
    
    Ok((read_handle, write_handle))
}

/// Create a message queue with enhanced capability validation
pub fn create_message_queue_with_capabilities(process_id: u32, capability_handle: CapabilityHandle, max_messages: usize, max_message_size: usize) -> Result<u32, IpcError> {
    let mut system = IPC_SYSTEM.lock();
    
    // Validate IPC creation capability
    system.validate_capability(process_id, capability_handle, CapabilityType::IpcCreate, 0x01)?;
    
    let queue_id = system.next_ipc_id;
    system.next_ipc_id += 1;
    
    let queue = MessageQueue::new(max_messages, max_message_size);
    system.objects.insert(queue_id, IpcObject::MessageQueue(queue));
    
    let handle_table = system.get_or_create_handle_table(process_id);
    
    let handle = handle_table.allocate_handle(
        queue_id,
        IpcObjectType::MessageQueue,
        IpcRights::SEND_RECV,
        None,
        None,
    );
    
    Ok(handle)
}

/// Create shared memory with enhanced capability validation
pub fn create_shared_memory_with_capabilities(process_id: u32, capability_handle: CapabilityHandle, size: usize) -> Result<u32, IpcError> {
    let mut system = IPC_SYSTEM.lock();
    
    // Validate memory mapping capability
    system.validate_capability(process_id, capability_handle, CapabilityType::MemoryShared, 0x01)?;
    
    let shm_id = system.next_ipc_id;
    system.next_ipc_id += 1;
    
    let shm = SharedMemory::new(size);
    system.objects.insert(shm_id, IpcObject::SharedMemory(shm));
    
    let handle_table = system.get_or_create_handle_table(process_id);
    
    let handle = handle_table.allocate_handle(
        shm_id,
        IpcObjectType::SharedMemory,
        IpcRights { read: true, write: true, map: true, ..IpcRights::NONE },
        None,
        None,
    );
    
    Ok(handle)
}

/// Read from IPC object with capability validation
pub fn read_from_ipc_object(process_id: u32, handle_id: u32, buffer: &mut [u8]) -> Result<usize, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    
    // Validate handle rights for reading
    ipc.validate_handle_rights(process_id, handle_id, IpcRights { read: true, ..IpcRights::NONE })?;
    
    ipc.read_from_object(process_id, handle_id, buffer)
}

/// Write to IPC object with capability validation
pub fn write_to_ipc_object(process_id: u32, handle_id: u32, data: &[u8]) -> Result<usize, IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    
    // Validate handle rights for writing
    ipc.validate_handle_rights(process_id, handle_id, IpcRights { write: true, ..IpcRights::NONE })?;
    
    ipc.write_to_object(process_id, handle_id, data)
}

/// Metadata about a handle for inspection
#[derive(Debug, Clone)]
pub struct HandleMetadata {
    pub object_id: u32,
    pub object_type: String,
    pub rights: IpcRights,
    pub expiry: Option<u64>,
}

/// Inspect handle metadata (requires inspect rights)
pub fn inspect_handle_metadata(process_id: u32, handle_id: u32) -> Result<HandleMetadata, IpcError> {
    let ipc = IPC_SYSTEM.lock();
    
    // Validate inspect rights
    let required_rights = IpcRights { inspect: true, ..IpcRights::NONE };
    ipc.validate_handle_rights(process_id, handle_id, required_rights)?;
    
    let handle_table = ipc.handle_tables.get(&process_id)
        .ok_or(IpcError::InvalidHandle)?;
    
    let handle = handle_table.get_handle(handle_id)
        .ok_or(IpcError::InvalidHandle)?;
    
    let object_type = ipc.objects.get(&handle.object_id)
        .map(|obj| match obj {
            IpcObject::Pipe(_) => "pipe",
            IpcObject::MessageQueue(_) => "message_queue",
            IpcObject::SharedMemory(_) => "shared_memory",
            IpcObject::MpscRing(_) => "mpsc_ring",
            IpcObject::CapabilityEndpoint(_) => "capability_endpoint",
        })
        .unwrap_or("unknown");
    
    Ok(HandleMetadata {
        object_id: handle.object_id,
        object_type: object_type.to_string(),
        rights: handle.rights,
        expiry: handle.expiry_time,
    })
}

/// Revoke a handle (requires revoke rights)
pub fn revoke_handle_with_validation(process_id: u32, handle_id: u32) -> Result<(), IpcError> {
    let mut ipc = IPC_SYSTEM.lock();
    
    // Validate revoke rights
    let required_rights = IpcRights { revoke: true, ..IpcRights::NONE };
    ipc.validate_handle_rights(process_id, handle_id, required_rights)?;
    
    let handle_table = ipc.handle_tables.get_mut(&process_id)
        .ok_or(IpcError::InvalidHandle)?;
    
    let handle = handle_table.handles.remove(&handle_id)
        .ok_or(IpcError::InvalidHandle)?;
    
    let _ = ipc.audit_log.log(
        process_id,
        "revoke_handle_validated".to_string(),
        handle.object_id,
        "success".to_string(),
        format!("handle={}", handle_id),
    );
    
    Ok(())
}