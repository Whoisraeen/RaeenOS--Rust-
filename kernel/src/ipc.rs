use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::fmt;
use spin::{Mutex, RwLock};
use crate::process::ProcessId;
use crate::time::Timestamp;

static IPC_MANAGER: RwLock<IpcManager> = RwLock::new(IpcManager::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageId(u64);

impl MessageId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChannelId(u64);

impl ChannelId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemaphoreId(u64);

impl SemaphoreId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SharedMemoryId(u64);

impl SharedMemoryId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
    
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Data,
    Signal,
    Request,
    Response,
    Broadcast,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: MessageId,
    pub sender: ProcessId,
    pub recipient: Option<ProcessId>,
    pub message_type: MessageType,
    pub priority: MessagePriority,
    pub timestamp: Timestamp,
    pub data: Vec<u8>,
    pub reply_channel: Option<ChannelId>,
    pub timeout: Option<Timestamp>,
}

impl Message {
    pub fn new(
        id: MessageId,
        sender: ProcessId,
        recipient: Option<ProcessId>,
        message_type: MessageType,
        data: Vec<u8>
    ) -> Self {
        Self {
            id,
            sender,
            recipient,
            message_type,
            priority: MessagePriority::Normal,
            timestamp: crate::time::get_timestamp(),
            data,
            reply_channel: None,
            timeout: None,
        }
    }
    
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_reply_channel(mut self, channel: ChannelId) -> Self {
        self.reply_channel = Some(channel);
        self
    }
    
    pub fn with_timeout(mut self, timeout: Timestamp) -> Self {
        self.timeout = Some(timeout);
        self
    }
    
    pub fn is_expired(&self) -> bool {
        if let Some(timeout) = self.timeout {
            crate::time::get_timestamp() > timeout
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    Synchronous,
    Asynchronous,
    Broadcast,
    RequestResponse,
}

#[derive(Debug)]
pub struct Channel {
    pub id: ChannelId,
    pub name: Option<String>,
    pub channel_type: ChannelType,
    pub owner: ProcessId,
    pub subscribers: Vec<ProcessId>,
    pub message_queue: VecDeque<Message>,
    pub max_queue_size: usize,
    pub is_closed: bool,
    pub permissions: ChannelPermissions,
}

#[derive(Debug, Clone, Copy)]
pub struct ChannelPermissions {
    pub read: bool,
    pub write: bool,
    pub subscribe: bool,
    pub admin: bool,
}

impl ChannelPermissions {
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            subscribe: false,
            admin: false,
        }
    }
    
    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            subscribe: false,
            admin: false,
        }
    }
    
    pub fn full() -> Self {
        Self {
            read: true,
            write: true,
            subscribe: true,
            admin: true,
        }
    }
}

impl Channel {
    pub fn new(
        id: ChannelId,
        channel_type: ChannelType,
        owner: ProcessId,
        max_queue_size: usize
    ) -> Self {
        Self {
            id,
            name: None,
            channel_type,
            owner,
            subscribers: Vec::new(),
            message_queue: VecDeque::new(),
            max_queue_size,
            is_closed: false,
            permissions: ChannelPermissions::full(),
        }
    }
    
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    
    pub fn send_message(&mut self, message: Message) -> Result<(), IpcError> {
        if self.is_closed {
            return Err(IpcError::ChannelClosed);
        }
        
        if self.message_queue.len() >= self.max_queue_size {
            return Err(IpcError::QueueFull);
        }
        
        // Insert message based on priority
        let mut inserted = false;
        for (i, existing_msg) in self.message_queue.iter().enumerate() {
            if message.priority as u8 > existing_msg.priority as u8 {
                self.message_queue.insert(i, message);
                inserted = true;
                break;
            }
        }
        
        if !inserted {
            self.message_queue.push_back(message);
        }
        
        Ok(())
    }
    
    pub fn receive_message(&mut self) -> Option<Message> {
        // Remove expired messages
        self.message_queue.retain(|msg| !msg.is_expired());
        
        self.message_queue.pop_front()
    }
    
    pub fn peek_message(&self) -> Option<&Message> {
        self.message_queue.front()
    }
    
    pub fn subscribe(&mut self, process_id: ProcessId) -> Result<(), IpcError> {
        if self.is_closed {
            return Err(IpcError::ChannelClosed);
        }
        
        if !self.subscribers.contains(&process_id) {
            self.subscribers.push(process_id);
        }
        
        Ok(())
    }
    
    pub fn unsubscribe(&mut self, process_id: ProcessId) {
        self.subscribers.retain(|&id| id != process_id);
    }
    
    pub fn close(&mut self) {
        self.is_closed = true;
        self.message_queue.clear();
        self.subscribers.clear();
    }
}

#[derive(Debug)]
pub struct Semaphore {
    pub id: SemaphoreId,
    pub name: Option<String>,
    pub value: i32,
    pub max_value: i32,
    pub waiting_processes: VecDeque<ProcessId>,
    pub owner: ProcessId,
}

impl Semaphore {
    pub fn new(id: SemaphoreId, initial_value: i32, max_value: i32, owner: ProcessId) -> Self {
        Self {
            id,
            name: None,
            value: initial_value,
            max_value,
            waiting_processes: VecDeque::new(),
            owner,
        }
    }
    
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    
    pub fn wait(&mut self, process_id: ProcessId) -> Result<(), IpcError> {
        if self.value > 0 {
            self.value -= 1;
            Ok(())
        } else {
            self.waiting_processes.push_back(process_id);
            Err(IpcError::WouldBlock)
        }
    }
    
    pub fn try_wait(&mut self) -> Result<(), IpcError> {
        if self.value > 0 {
            self.value -= 1;
            Ok(())
        } else {
            Err(IpcError::WouldBlock)
        }
    }
    
    pub fn signal(&mut self) -> Option<ProcessId> {
        if let Some(waiting_process) = self.waiting_processes.pop_front() {
            Some(waiting_process)
        } else if self.value < self.max_value {
            self.value += 1;
            None
        } else {
            None
        }
    }
    
    pub fn get_value(&self) -> i32 {
        self.value
    }
}

#[derive(Debug)]
pub struct SharedMemory {
    pub id: SharedMemoryId,
    pub name: Option<String>,
    pub size: usize,
    pub owner: ProcessId,
    pub attached_processes: Vec<ProcessId>,
    pub permissions: SharedMemoryPermissions,
    pub physical_address: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct SharedMemoryPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl SharedMemoryPermissions {
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            execute: false,
        }
    }
    
    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            execute: false,
        }
    }
    
    pub fn read_execute() -> Self {
        Self {
            read: true,
            write: false,
            execute: true,
        }
    }
    
    pub fn full() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
        }
    }
}

impl SharedMemory {
    pub fn new(
        id: SharedMemoryId,
        size: usize,
        owner: ProcessId,
        permissions: SharedMemoryPermissions
    ) -> Self {
        Self {
            id,
            name: None,
            size,
            owner,
            attached_processes: Vec::new(),
            permissions,
            physical_address: None,
        }
    }
    
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }
    
    pub fn attach(&mut self, process_id: ProcessId) -> Result<(), IpcError> {
        if !self.attached_processes.contains(&process_id) {
            self.attached_processes.push(process_id);
        }
        Ok(())
    }
    
    pub fn detach(&mut self, process_id: ProcessId) {
        self.attached_processes.retain(|&id| id != process_id);
    }
    
    pub fn is_attached(&self, process_id: ProcessId) -> bool {
        self.attached_processes.contains(&process_id)
    }
}

#[derive(Debug)]
pub struct IpcManager {
    channels: BTreeMap<ChannelId, Channel>,
    semaphores: BTreeMap<SemaphoreId, Semaphore>,
    shared_memory: BTreeMap<SharedMemoryId, SharedMemory>,
    named_channels: BTreeMap<String, ChannelId>,
    named_semaphores: BTreeMap<String, SemaphoreId>,
    named_shared_memory: BTreeMap<String, SharedMemoryId>,
    next_channel_id: u64,
    next_semaphore_id: u64,
    next_shared_memory_id: u64,
    next_message_id: u64,
    process_channels: BTreeMap<ProcessId, Vec<ChannelId>>,
    process_semaphores: BTreeMap<ProcessId, Vec<SemaphoreId>>,
    process_shared_memory: BTreeMap<ProcessId, Vec<SharedMemoryId>>,
}

impl IpcManager {
    pub const fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
            semaphores: BTreeMap::new(),
            shared_memory: BTreeMap::new(),
            named_channels: BTreeMap::new(),
            named_semaphores: BTreeMap::new(),
            named_shared_memory: BTreeMap::new(),
            next_channel_id: 1,
            next_semaphore_id: 1,
            next_shared_memory_id: 1,
            next_message_id: 1,
            process_channels: BTreeMap::new(),
            process_semaphores: BTreeMap::new(),
            process_shared_memory: BTreeMap::new(),
        }
    }
    
    // Channel management
    pub fn create_channel(
        &mut self,
        channel_type: ChannelType,
        owner: ProcessId,
        max_queue_size: usize,
        name: Option<String>
    ) -> ChannelId {
        let id = ChannelId::new(self.next_channel_id);
        self.next_channel_id += 1;
        
        let mut channel = Channel::new(id, channel_type, owner, max_queue_size);
        if let Some(name) = name.clone() {
            channel = channel.with_name(name.clone());
            self.named_channels.insert(name, id);
        }
        
        self.channels.insert(id, channel);
        
        // Track channel ownership
        self.process_channels.entry(owner)
            .or_insert_with(Vec::new)
            .push(id);
        
        id
    }
    
    pub fn destroy_channel(&mut self, id: ChannelId, requester: ProcessId) -> Result<(), IpcError> {
        let channel = self.channels.get(&id).ok_or(IpcError::ChannelNotFound)?;
        
        if channel.owner != requester {
            return Err(IpcError::PermissionDenied);
        }
        
        // Remove from named channels if it has a name
        if let Some(ref name) = channel.name {
            self.named_channels.remove(name);
        }
        
        // Remove from process tracking
        if let Some(channels) = self.process_channels.get_mut(&channel.owner) {
            channels.retain(|&channel_id| channel_id != id);
        }
        
        self.channels.remove(&id);
        Ok(())
    }
    
    pub fn get_channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.get(&id)
    }
    
    pub fn get_channel_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channels.get_mut(&id)
    }
    
    pub fn find_channel_by_name(&self, name: &str) -> Option<ChannelId> {
        self.named_channels.get(name).copied()
    }
    
    // Message management
    pub fn send_message(
        &mut self,
        channel_id: ChannelId,
        sender: ProcessId,
        recipient: Option<ProcessId>,
        message_type: MessageType,
        data: Vec<u8>
    ) -> Result<MessageId, IpcError> {
        let message_id = MessageId::new(self.next_message_id);
        self.next_message_id += 1;
        
        let message = Message::new(message_id, sender, recipient, message_type, data);
        
        let channel = self.get_channel_mut(channel_id)
            .ok_or(IpcError::ChannelNotFound)?;
        
        if channel.is_closed {
            return Err(IpcError::ChannelClosed);
        }
        
        // Check permissions
        if !channel.permissions.write {
            return Err(IpcError::PermissionDenied);
        }
        
        channel.send_message(message)?;
        
        // For broadcast channels, send to all subscribers
        if channel.channel_type == ChannelType::Broadcast {
            for &subscriber in &channel.subscribers.clone() {
                if subscriber != sender {
                    // Notify subscriber by unblocking them if they're waiting for messages
                    crate::process::unblock_process(subscriber);
                }
            }
        }
        
        Ok(message_id)
    }
    
    pub fn receive_message(&mut self, channel_id: ChannelId, receiver: ProcessId) -> Result<Option<Message>, IpcError> {
        let channel = self.get_channel_mut(channel_id)
            .ok_or(IpcError::ChannelNotFound)?;
        
        if !channel.permissions.read {
            return Err(IpcError::PermissionDenied);
        }
        
        // For synchronous channels, only the owner or subscribers can receive
        if channel.channel_type == ChannelType::Synchronous {
            if channel.owner != receiver && !channel.subscribers.contains(&receiver) {
                return Err(IpcError::PermissionDenied);
            }
        }
        
        Ok(channel.receive_message())
    }
    
    // Semaphore management
    pub fn create_semaphore(
        &mut self,
        initial_value: i32,
        max_value: i32,
        owner: ProcessId,
        name: Option<String>
    ) -> SemaphoreId {
        let id = SemaphoreId::new(self.next_semaphore_id);
        self.next_semaphore_id += 1;
        
        let mut semaphore = Semaphore::new(id, initial_value, max_value, owner);
        if let Some(name) = name.clone() {
            semaphore = semaphore.with_name(name.clone());
            self.named_semaphores.insert(name, id);
        }
        
        self.semaphores.insert(id, semaphore);
        
        // Track semaphore ownership
        self.process_semaphores.entry(owner)
            .or_insert_with(Vec::new)
            .push(id);
        
        id
    }
    
    pub fn destroy_semaphore(&mut self, id: SemaphoreId, requester: ProcessId) -> Result<(), IpcError> {
        let semaphore = self.semaphores.get(&id).ok_or(IpcError::SemaphoreNotFound)?;
        
        if semaphore.owner != requester {
            return Err(IpcError::PermissionDenied);
        }
        
        // Remove from named semaphores if it has a name
        if let Some(ref name) = semaphore.name {
            self.named_semaphores.remove(name);
        }
        
        // Remove from process tracking
        if let Some(semaphores) = self.process_semaphores.get_mut(&semaphore.owner) {
            semaphores.retain(|&semaphore_id| semaphore_id != id);
        }
        
        self.semaphores.remove(&id);
        Ok(())
    }
    
    pub fn semaphore_wait(&mut self, id: SemaphoreId, process_id: ProcessId) -> Result<(), IpcError> {
        let semaphore = self.semaphores.get_mut(&id)
            .ok_or(IpcError::SemaphoreNotFound)?;
        
        semaphore.wait(process_id)
    }
    
    pub fn semaphore_signal(&mut self, id: SemaphoreId) -> Result<Option<ProcessId>, IpcError> {
        let semaphore = self.semaphores.get_mut(&id)
            .ok_or(IpcError::SemaphoreNotFound)?;
        
        Ok(semaphore.signal())
    }
    
    pub fn find_semaphore_by_name(&self, name: &str) -> Option<SemaphoreId> {
        self.named_semaphores.get(name).copied()
    }
    
    // Shared memory management
    pub fn create_shared_memory(
        &mut self,
        size: usize,
        owner: ProcessId,
        permissions: SharedMemoryPermissions,
        name: Option<String>
    ) -> SharedMemoryId {
        let id = SharedMemoryId::new(self.next_shared_memory_id);
        self.next_shared_memory_id += 1;
        
        let mut shared_mem = SharedMemory::new(id, size, owner, permissions);
        if let Some(name) = name.clone() {
            shared_mem = shared_mem.with_name(name.clone());
            self.named_shared_memory.insert(name, id);
        }
        
        self.shared_memory.insert(id, shared_mem);
        
        // Track shared memory ownership
        self.process_shared_memory.entry(owner)
            .or_insert_with(Vec::new)
            .push(id);
        
        id
    }
    
    pub fn destroy_shared_memory(&mut self, id: SharedMemoryId, requester: ProcessId) -> Result<(), IpcError> {
        let shared_mem = self.shared_memory.get(&id).ok_or(IpcError::SharedMemoryNotFound)?;
        
        if shared_mem.owner != requester {
            return Err(IpcError::PermissionDenied);
        }
        
        // Remove from named shared memory if it has a name
        if let Some(ref name) = shared_mem.name {
            self.named_shared_memory.remove(name);
        }
        
        // Remove from process tracking
        if let Some(shared_mems) = self.process_shared_memory.get_mut(&shared_mem.owner) {
            shared_mems.retain(|&shared_mem_id| shared_mem_id != id);
        }
        
        self.shared_memory.remove(&id);
        Ok(())
    }
    
    pub fn attach_shared_memory(&mut self, id: SharedMemoryId, process_id: ProcessId) -> Result<(), IpcError> {
        let shared_mem = self.shared_memory.get_mut(&id)
            .ok_or(IpcError::SharedMemoryNotFound)?;
        
        shared_mem.attach(process_id)
    }
    
    pub fn detach_shared_memory(&mut self, id: SharedMemoryId, process_id: ProcessId) -> Result<(), IpcError> {
        let shared_mem = self.shared_memory.get_mut(&id)
            .ok_or(IpcError::SharedMemoryNotFound)?;
        
        shared_mem.detach(process_id);
        Ok(())
    }
    
    pub fn find_shared_memory_by_name(&self, name: &str) -> Option<SharedMemoryId> {
        self.named_shared_memory.get(name).copied()
    }
    
    // Process cleanup
    pub fn cleanup_process_ipc(&mut self, process_id: ProcessId) {
        // Clean up channels
        if let Some(channels) = self.process_channels.remove(&process_id) {
            for channel_id in channels {
                let _ = self.destroy_channel(channel_id, process_id);
            }
        }
        
        // Clean up semaphores
        if let Some(semaphores) = self.process_semaphores.remove(&process_id) {
            for semaphore_id in semaphores {
                let _ = self.destroy_semaphore(semaphore_id, process_id);
            }
        }
        
        // Clean up shared memory
        if let Some(shared_mems) = self.process_shared_memory.remove(&process_id) {
            for shared_mem_id in shared_mems {
                let _ = self.destroy_shared_memory(shared_mem_id, process_id);
            }
        }
        
        // Remove process from all channel subscriptions
        for channel in self.channels.values_mut() {
            channel.unsubscribe(process_id);
        }
        
        // Remove process from semaphore waiting queues
        for semaphore in self.semaphores.values_mut() {
            semaphore.waiting_processes.retain(|&id| id != process_id);
        }
        
        // Detach process from all shared memory
        for shared_mem in self.shared_memory.values_mut() {
            shared_mem.detach(process_id);
        }
    }
}

#[derive(Debug)]
pub enum IpcError {
    ChannelNotFound,
    ChannelClosed,
    SemaphoreNotFound,
    SharedMemoryNotFound,
    PermissionDenied,
    QueueFull,
    WouldBlock,
    InvalidOperation,
    Timeout,
    MessageTooLarge,
    InvalidMessageType,
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IpcError::ChannelNotFound => write!(f, "Channel not found"),
            IpcError::ChannelClosed => write!(f, "Channel is closed"),
            IpcError::SemaphoreNotFound => write!(f, "Semaphore not found"),
            IpcError::SharedMemoryNotFound => write!(f, "Shared memory not found"),
            IpcError::PermissionDenied => write!(f, "Permission denied"),
            IpcError::QueueFull => write!(f, "Message queue is full"),
            IpcError::WouldBlock => write!(f, "Operation would block"),
            IpcError::InvalidOperation => write!(f, "Invalid operation"),
            IpcError::Timeout => write!(f, "Operation timed out"),
            IpcError::MessageTooLarge => write!(f, "Message too large"),
            IpcError::InvalidMessageType => write!(f, "Invalid message type"),
        }
    }
}

pub type IpcResult<T> = Result<T, IpcError>;

// Public API functions
pub fn init() {
    // IPC manager is already initialized as a static
}

// Channel API
pub fn create_channel(
    channel_type: ChannelType,
    owner: ProcessId,
    max_queue_size: usize,
    name: Option<String>
) -> ChannelId {
    IPC_MANAGER.write().create_channel(channel_type, owner, max_queue_size, name)
}

pub fn destroy_channel(id: ChannelId, requester: ProcessId) -> IpcResult<()> {
    IPC_MANAGER.write().destroy_channel(id, requester)
}

pub fn send_message(
    channel_id: ChannelId,
    sender: ProcessId,
    recipient: Option<ProcessId>,
    message_type: MessageType,
    data: Vec<u8>
) -> IpcResult<MessageId> {
    IPC_MANAGER.write().send_message(channel_id, sender, recipient, message_type, data)
}

pub fn receive_message(channel_id: ChannelId, receiver: ProcessId) -> IpcResult<Option<Message>> {
    IPC_MANAGER.write().receive_message(channel_id, receiver)
}

pub fn subscribe_channel(channel_id: ChannelId, process_id: ProcessId) -> IpcResult<()> {
    let mut manager = IPC_MANAGER.write();
    let channel = manager.get_channel_mut(channel_id)
        .ok_or(IpcError::ChannelNotFound)?;
    channel.subscribe(process_id)
}

pub fn unsubscribe_channel(channel_id: ChannelId, process_id: ProcessId) -> IpcResult<()> {
    let mut manager = IPC_MANAGER.write();
    let channel = manager.get_channel_mut(channel_id)
        .ok_or(IpcError::ChannelNotFound)?;
    channel.unsubscribe(process_id);
    Ok(())
}

pub fn find_channel_by_name(name: &str) -> Option<ChannelId> {
    IPC_MANAGER.read().find_channel_by_name(name)
}

// Semaphore API
pub fn create_semaphore(
    initial_value: i32,
    max_value: i32,
    owner: ProcessId,
    name: Option<String>
) -> SemaphoreId {
    IPC_MANAGER.write().create_semaphore(initial_value, max_value, owner, name)
}

pub fn destroy_semaphore(id: SemaphoreId, requester: ProcessId) -> IpcResult<()> {
    IPC_MANAGER.write().destroy_semaphore(id, requester)
}

pub fn semaphore_wait(id: SemaphoreId, process_id: ProcessId) -> IpcResult<()> {
    IPC_MANAGER.write().semaphore_wait(id, process_id)
}

pub fn semaphore_signal(id: SemaphoreId) -> IpcResult<Option<ProcessId>> {
    IPC_MANAGER.write().semaphore_signal(id)
}

pub fn find_semaphore_by_name(name: &str) -> Option<SemaphoreId> {
    IPC_MANAGER.read().find_semaphore_by_name(name)
}

// Shared memory API
pub fn create_shared_memory(
    size: usize,
    owner: ProcessId,
    permissions: SharedMemoryPermissions,
    name: Option<String>
) -> SharedMemoryId {
    IPC_MANAGER.write().create_shared_memory(size, owner, permissions, name)
}

pub fn destroy_shared_memory(id: SharedMemoryId, requester: ProcessId) -> IpcResult<()> {
    IPC_MANAGER.write().destroy_shared_memory(id, requester)
}

pub fn attach_shared_memory(id: SharedMemoryId, process_id: ProcessId) -> IpcResult<()> {
    IPC_MANAGER.write().attach_shared_memory(id, process_id)
}

pub fn detach_shared_memory(id: SharedMemoryId, process_id: ProcessId) -> IpcResult<()> {
    IPC_MANAGER.write().detach_shared_memory(id, process_id)
}

pub fn find_shared_memory_by_name(name: &str) -> Option<SharedMemoryId> {
    IPC_MANAGER.read().find_shared_memory_by_name(name)
}

// Process cleanup
pub fn cleanup_process_ipc(process_id: ProcessId) {
    IPC_MANAGER.write().cleanup_process_ipc(process_id);
}

// Utility functions
pub fn get_channel_info(id: ChannelId) -> Option<(ChannelType, ProcessId, usize, bool)> {
    let manager = IPC_MANAGER.read();
    let channel = manager.get_channel(id)?;
    Some((channel.channel_type, channel.owner, channel.message_queue.len(), channel.is_closed))
}

pub fn get_semaphore_info(id: SemaphoreId) -> Option<(i32, i32, usize)> {
    let manager = IPC_MANAGER.read();
    let semaphore = manager.semaphores.get(&id)?;
    Some((semaphore.value, semaphore.max_value, semaphore.waiting_processes.len()))
}

pub fn get_shared_memory_info(id: SharedMemoryId) -> Option<(usize, ProcessId, usize)> {
    let manager = IPC_MANAGER.read();
    let shared_mem = manager.shared_memory.get(&id)?;
    Some((shared_mem.size, shared_mem.owner, shared_mem.attached_processes.len()))
}

// High-level messaging patterns
pub fn send_request_response(
    channel_id: ChannelId,
    sender: ProcessId,
    recipient: ProcessId,
    request_data: Vec<u8>,
    timeout_ms: u64
) -> IpcResult<Vec<u8>> {
    // Create a temporary response channel
    let response_channel = create_channel(
        ChannelType::Synchronous,
        sender,
        1,
        None
    );
    
    // Send request with reply channel
    let mut manager = IPC_MANAGER.write();
    let message_id = MessageId::new(manager.next_message_id);
    manager.next_message_id += 1;
    
    let mut message = Message::new(
        message_id,
        sender,
        Some(recipient),
        MessageType::Request,
        request_data
    );
    message = message.with_reply_channel(response_channel);
    message = message.with_timeout(crate::time::get_timestamp() + timeout_ms);
    
    let channel = manager.get_channel_mut(channel_id)
        .ok_or(IpcError::ChannelNotFound)?;
    channel.send_message(message)?;
    
    drop(manager);
    
    // Wait for response
    let start_time = crate::time::get_timestamp();
    loop {
        if let Some(response) = receive_message(response_channel, sender)? {
            if response.message_type == MessageType::Response {
                let _ = destroy_channel(response_channel, sender);
                return Ok(response.data);
            }
        }
        
        if crate::time::get_timestamp() - start_time > timeout_ms {
            let _ = destroy_channel(response_channel, sender);
            return Err(IpcError::Timeout);
        }
        
        // Yield to scheduler to allow other processes to run
        crate::process::yield_current();
        crate::time::sleep_ms(1);
    }
}

pub fn broadcast_message(
    channel_id: ChannelId,
    sender: ProcessId,
    data: Vec<u8>
) -> IpcResult<()> {
    send_message(channel_id, sender, None, MessageType::Broadcast, data)?;
    Ok(())
}

// Event system
pub fn create_event_channel(owner: ProcessId, name: Option<String>) -> ChannelId {
    create_channel(ChannelType::Broadcast, owner, 1000, name)
}

pub fn emit_event(
    event_channel: ChannelId,
    sender: ProcessId,
    event_data: Vec<u8>
) -> IpcResult<()> {
    send_message(event_channel, sender, None, MessageType::Event, event_data)?;
    Ok(())
}

pub fn listen_for_events(event_channel: ChannelId, listener: ProcessId) -> IpcResult<()> {
    subscribe_channel(event_channel, listener)
}