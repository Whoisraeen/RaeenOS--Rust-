//! Inter-process communication for RaeenOS
//! Implements pipes, message queues, and shared memory

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::vec;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;

// IPC object types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcObjectType {
    Pipe,
    MessageQueue,
    SharedMemory,
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
    size: usize,
    attached_processes: Vec<u32>,
}

impl SharedMemory {
    fn new(size: usize) -> Self {
        Self {
            data: vec![0; size].into_boxed_slice(),
            size,
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

// IPC object wrapper
#[derive(Debug)]
enum IpcObject {
    Pipe(Pipe),
    MessageQueue(MessageQueue),
    SharedMemory(SharedMemory),
}

// File descriptor entry
#[derive(Debug)]
struct FileDescriptor {
    ipc_id: u32,
    object_type: IpcObjectType,
    readable: bool,
    writable: bool,
    process_id: u32,
}

// IPC system state
struct IpcSystem {
    objects: BTreeMap<u32, IpcObject>,
    file_descriptors: BTreeMap<u32, FileDescriptor>,
    next_ipc_id: u32,
    next_fd: u32,
}

lazy_static! {
    static ref IPC_SYSTEM: Mutex<IpcSystem> = Mutex::new(IpcSystem {
        objects: BTreeMap::new(),
        file_descriptors: BTreeMap::new(),
        next_ipc_id: 1,
        next_fd: 3, // Start after stdin(0), stdout(1), stderr(2)
    });
}

// Create a pipe and return read/write file descriptors
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
    
    // Create read file descriptor
    let read_fd = ipc.next_fd;
    ipc.next_fd += 1;
    
    ipc.file_descriptors.insert(read_fd, FileDescriptor {
        ipc_id: pipe_id,
        object_type: IpcObjectType::Pipe,
        readable: true,
        writable: false,
        process_id: current_pid as u32,
    });
    
    // Create write file descriptor
    let write_fd = ipc.next_fd;
    ipc.next_fd += 1;
    
    ipc.file_descriptors.insert(write_fd, FileDescriptor {
        ipc_id: pipe_id,
        object_type: IpcObjectType::Pipe,
        readable: false,
        writable: true,
        process_id: current_pid as u32,
    });
    
    Ok((read_fd, write_fd))
}

// Create a message queue
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
    
    // Create file descriptor
    let fd = ipc.next_fd;
    ipc.next_fd += 1;
    
    ipc.file_descriptors.insert(fd, FileDescriptor {
        ipc_id: queue_id,
        object_type: IpcObjectType::MessageQueue,
        readable: true,
        writable: true,
        process_id: current_pid as u32,
    });
    
    Ok(fd)
}

// Create shared memory
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
    shm.attach(current_pid as u32)?;
    
    ipc.objects.insert(shm_id, IpcObject::SharedMemory(shm));
    
    Ok(shm_id)
}

// Read from IPC object
pub fn ipc_read(fd: u32, buf: &mut [u8]) -> Result<usize, ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Extract the necessary information from file descriptor first
    let (ipc_id, process_id, readable) = {
        let file_desc = ipc.file_descriptors.get(&fd).ok_or(())?;
        (file_desc.ipc_id, file_desc.process_id, file_desc.readable)
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
    }
}

// Write to IPC object
pub fn ipc_write(fd: u32, buf: &[u8]) -> Result<usize, ()> {
    let mut ipc = IPC_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Extract needed values before mutable operations
    let ipc_id = {
        let file_desc = ipc.file_descriptors.get(&fd).ok_or(())?;
        
        // Check ownership and permissions
        if file_desc.process_id != current_pid as u32 || !file_desc.writable {
            return Err(());
        }
        
        file_desc.ipc_id
    };
    
    match ipc.objects.get_mut(&ipc_id).ok_or(())? {
        IpcObject::Pipe(pipe) => pipe.write(buf),
        IpcObject::MessageQueue(queue) => {
            queue.send(buf)?;
            Ok(buf.len())
        }
        IpcObject::SharedMemory(_) => Err(()), // Use attach/detach for shared memory
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
        
        (file_desc.ipc_id, file_desc.object_type, file_desc.readable, file_desc.writable)
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
}