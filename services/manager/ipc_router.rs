//! IPC Router for service-to-service and kernel-to-service communication
//! Handles message routing, serialization, and capability enforcement

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use spin::{Mutex, RwLock};
use serde::{Serialize, Deserialize};
use crate::ipc::{CapabilityEndpoint, IpcRights, IpcObject};
use crate::process::ProcessId;
use super::contracts::*;

/// IPC message router
pub struct IpcRouter {
    routes: RwLock<BTreeMap<String, RouteInfo>>,
    message_queue: Mutex<Vec<PendingMessage>>,
    capabilities: RwLock<BTreeMap<ProcessId, Vec<String>>>,
}

/// Route information for services
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub service_name: String,
    pub endpoint: CapabilityEndpoint,
    pub process_id: ProcessId,
    pub message_types: Vec<String>,
    pub access_rights: IpcRights,
}

/// Pending message in the queue
#[derive(Debug, Clone)]
pub struct PendingMessage {
    pub id: u64,
    pub target_service: String,
    pub sender_process: ProcessId,
    pub message_type: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub priority: MessagePriority,
}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// IPC message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    pub message_id: u64,
    pub sender_process: ProcessId,
    pub target_service: String,
    pub message_type: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub reply_expected: bool,
}

/// IPC response envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub response_to: u64,
    pub sender_service: String,
    pub success: bool,
    pub payload: Vec<u8>,
    pub error_code: Option<u32>,
    pub timestamp: u64,
}

/// Router statistics
#[derive(Debug, Clone, Default)]
pub struct RouterStatistics {
    pub messages_routed: u64,
    pub messages_failed: u64,
    pub average_latency_us: u32,
    pub active_routes: u32,
    pub queue_depth: u32,
}

static NEXT_MESSAGE_ID: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(1);

impl IpcRouter {
    /// Create a new IPC router
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(BTreeMap::new()),
            message_queue: Mutex::new(Vec::new()),
            capabilities: RwLock::new(BTreeMap::new()),
        }
    }
    
    /// Register a route for a service
    pub fn register_route(
        &self,
        service_name: String,
        endpoint: CapabilityEndpoint,
        process_id: ProcessId,
        message_types: Vec<String>,
        access_rights: IpcRights,
    ) -> Result<(), RouterError> {
        let route_info = RouteInfo {
            service_name: service_name.clone(),
            endpoint,
            process_id,
            message_types,
            access_rights,
        };
        
        self.routes.write().insert(service_name, route_info);
        Ok(())
    }
    
    /// Unregister a route
    pub fn unregister_route(&self, service_name: &str) -> Result<(), RouterError> {
        self.routes.write().remove(service_name)
            .ok_or(RouterError::RouteNotFound)?;
        Ok(())
    }
    
    /// Grant capability to a process for accessing a service
    pub fn grant_capability(
        &self,
        process_id: ProcessId,
        service_name: String,
    ) -> Result<(), RouterError> {
        let mut capabilities = self.capabilities.write();
        capabilities.entry(process_id)
            .or_insert_with(Vec::new)
            .push(service_name);
        Ok(())
    }
    
    /// Revoke capability from a process
    pub fn revoke_capability(
        &self,
        process_id: ProcessId,
        service_name: &str,
    ) -> Result<(), RouterError> {
        let mut capabilities = self.capabilities.write();
        if let Some(caps) = capabilities.get_mut(&process_id) {
            caps.retain(|s| s != service_name);
        }
        Ok(())
    }
    
    /// Check if a process has capability to access a service
    pub fn has_capability(&self, process_id: ProcessId, service_name: &str) -> bool {
        let capabilities = self.capabilities.read();
        capabilities.get(&process_id)
            .map(|caps| caps.iter().any(|s| s == service_name))
            .unwrap_or(false)
    }
    
    /// Route a message to a service
    pub fn route_message(
        &self,
        endpoint: CapabilityEndpoint,
        message: &[u8],
        sender_process: ProcessId,
    ) -> Result<Vec<u8>, RouterError> {
        // Deserialize the IPC message
        let ipc_message: IpcMessage = bincode::deserialize(message)
            .map_err(|_| RouterError::InvalidMessage)?;
        
        // Check capabilities
        if !self.has_capability(sender_process, &ipc_message.target_service) {
            return Err(RouterError::PermissionDenied);
        }
        
        // Find the route
        let route = {
            let routes = self.routes.read();
            routes.get(&ipc_message.target_service)
                .cloned()
                .ok_or(RouterError::RouteNotFound)?
        };
        
        // Check message type is supported
        if !route.message_types.contains(&ipc_message.message_type) {
            return Err(RouterError::UnsupportedMessageType);
        }
        
        // Route the message based on priority
        match self.determine_priority(&ipc_message) {
            MessagePriority::Critical => {
                // Send immediately
                self.send_message_direct(&route, &ipc_message)
            }
            _ => {
                // Queue for processing
                self.queue_message(&route, ipc_message)?;
                Ok(Vec::new()) // Async processing
            }
        }
    }
    
    /// Send message directly to service
    fn send_message_direct(
        &self,
        route: &RouteInfo,
        message: &IpcMessage,
    ) -> Result<Vec<u8>, RouterError> {
        // Serialize message for transmission
        let serialized = bincode::serialize(message)
            .map_err(|_| RouterError::SerializationFailed)?;
        
        // Send through IPC endpoint
        route.endpoint.send(&serialized)
            .map_err(|_| RouterError::TransmissionFailed)?;
        
        // Wait for response if expected
        if message.reply_expected {
            let response_data = route.endpoint.receive()
                .map_err(|_| RouterError::ReceiveFailed)?;
            
            // Deserialize response
            let response: IpcResponse = bincode::deserialize(&response_data)
                .map_err(|_| RouterError::InvalidResponse)?;
            
            if response.success {
                Ok(response.payload)
            } else {
                Err(RouterError::ServiceError(response.error_code.unwrap_or(0)))
            }
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Queue message for asynchronous processing
    fn queue_message(
        &self,
        route: &RouteInfo,
        message: IpcMessage,
    ) -> Result<(), RouterError> {
        let pending = PendingMessage {
            id: message.message_id,
            target_service: message.target_service.clone(),
            sender_process: message.sender_process,
            message_type: message.message_type.clone(),
            payload: message.payload,
            timestamp: message.timestamp,
            priority: self.determine_priority(&message),
        };
        
        let mut queue = self.message_queue.lock();
        queue.push(pending);
        
        // Sort by priority (highest first)
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Ok(())
    }
    
    /// Determine message priority based on content
    fn determine_priority(&self, message: &IpcMessage) -> MessagePriority {
        match message.message_type.as_str() {
            // Critical system messages
            "shutdown" | "emergency" | "security_alert" => MessagePriority::Critical,
            
            // High priority messages
            "health_check" | "resource_limit" | "error_report" => MessagePriority::High,
            
            // Normal priority messages
            "status_update" | "config_change" | "log_message" => MessagePriority::Normal,
            
            // Low priority messages
            "metrics" | "debug_info" | "trace" => MessagePriority::Low,
            
            // Default to normal
            _ => MessagePriority::Normal,
        }
    }
    
    /// Process queued messages
    pub fn process_queue(&self) -> Result<u32, RouterError> {
        let mut queue = self.message_queue.lock();
        let mut processed = 0;
        
        // Process up to 10 messages per call to avoid blocking
        let batch_size = core::cmp::min(queue.len(), 10);
        
        for _ in 0..batch_size {
            if let Some(pending) = queue.pop() {
                // Find route for the message
                let route = {
                    let routes = self.routes.read();
                    routes.get(&pending.target_service).cloned()
                };
                
                if let Some(route) = route {
                    // Reconstruct IPC message
                    let ipc_message = IpcMessage {
                        message_id: pending.id,
                        sender_process: pending.sender_process,
                        target_service: pending.target_service,
                        message_type: pending.message_type,
                        payload: pending.payload,
                        timestamp: pending.timestamp,
                        reply_expected: false, // Queued messages don't expect replies
                    };
                    
                    // Send the message
                    if self.send_message_direct(&route, &ipc_message).is_ok() {
                        processed += 1;
                    }
                }
            }
        }
        
        Ok(processed)
    }
    
    /// Create an IPC message for routing
    pub fn create_message(
        sender_process: ProcessId,
        target_service: String,
        message_type: String,
        payload: Vec<u8>,
        reply_expected: bool,
    ) -> IpcMessage {
        IpcMessage {
            message_id: NEXT_MESSAGE_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst),
            sender_process,
            target_service,
            message_type,
            payload,
            timestamp: crate::time::get_timestamp(),
            reply_expected,
        }
    }
    
    /// Create an IPC response
    pub fn create_response(
        response_to: u64,
        sender_service: String,
        success: bool,
        payload: Vec<u8>,
        error_code: Option<u32>,
    ) -> IpcResponse {
        IpcResponse {
            response_to,
            sender_service,
            success,
            payload,
            error_code,
            timestamp: crate::time::get_timestamp(),
        }
    }
    
    /// Get router statistics
    pub fn get_statistics(&self) -> RouterStatistics {
        let routes = self.routes.read();
        let queue = self.message_queue.lock();
        
        RouterStatistics {
            messages_routed: 0, // TODO: Track this
            messages_failed: 0, // TODO: Track this
            average_latency_us: 0, // TODO: Track this
            active_routes: routes.len() as u32,
            queue_depth: queue.len() as u32,
        }
    }
    
    /// List all registered routes
    pub fn list_routes(&self) -> Vec<String> {
        self.routes.read().keys().cloned().collect()
    }
    
    /// Get route information
    pub fn get_route_info(&self, service_name: &str) -> Option<RouteInfo> {
        self.routes.read().get(service_name).cloned()
    }
    
    /// Clear message queue
    pub fn clear_queue(&self) {
        self.message_queue.lock().clear();
    }
}

/// Router error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterError {
    RouteNotFound,
    PermissionDenied,
    InvalidMessage,
    UnsupportedMessageType,
    SerializationFailed,
    TransmissionFailed,
    ReceiveFailed,
    InvalidResponse,
    ServiceError(u32),
    QueueFull,
    InternalError,
}

/// Helper functions for common routing operations
pub fn route_network_request(
    router: &IpcRouter,
    request: network::NetworkRequest,
    sender_process: ProcessId,
) -> Result<network::NetworkResponse, RouterError> {
    let payload = bincode::serialize(&request)
        .map_err(|_| RouterError::SerializationFailed)?;
    
    let message = IpcRouter::create_message(
        sender_process,
        "rae-networkd".to_string(),
        "network_request".to_string(),
        payload,
        true,
    );
    
    let serialized_message = bincode::serialize(&message)
        .map_err(|_| RouterError::SerializationFailed)?;
    
    // This would need the actual endpoint, simplified for now
    let response_data = Vec::new(); // TODO: Get from actual routing
    
    let response: network::NetworkResponse = bincode::deserialize(&response_data)
        .map_err(|_| RouterError::InvalidResponse)?;
    
    Ok(response)
}

pub fn route_graphics_request(
    router: &IpcRouter,
    request: graphics::GraphicsRequest,
    sender_process: ProcessId,
) -> Result<graphics::GraphicsResponse, RouterError> {
    let payload = bincode::serialize(&request)
        .map_err(|_| RouterError::SerializationFailed)?;
    
    let message = IpcRouter::create_message(
        sender_process,
        "rae-compositord".to_string(),
        "graphics_request".to_string(),
        payload,
        true,
    );
    
    let serialized_message = bincode::serialize(&message)
        .map_err(|_| RouterError::SerializationFailed)?;
    
    // This would need the actual endpoint, simplified for now
    let response_data = Vec::new(); // TODO: Get from actual routing
    
    let response: graphics::GraphicsResponse = bincode::deserialize(&response_data)
        .map_err(|_| RouterError::InvalidResponse)?;
    
    Ok(response)
}

pub fn route_ai_request(
    router: &IpcRouter,
    request: ai::AiRequest,
    sender_process: ProcessId,
) -> Result<ai::AiResponse, RouterError> {
    let payload = bincode::serialize(&request)
        .map_err(|_| RouterError::SerializationFailed)?;
    
    let message = IpcRouter::create_message(
        sender_process,
        "rae-assistantd".to_string(),
        "ai_request".to_string(),
        payload,
        true,
    );
    
    let serialized_message = bincode::serialize(&message)
        .map_err(|_| RouterError::SerializationFailed)?;
    
    // This would need the actual endpoint, simplified for now
    let response_data = Vec::new(); // TODO: Get from actual routing
    
    let response: ai::AiResponse = bincode::deserialize(&response_data)
        .map_err(|_| RouterError::InvalidResponse)?;
    
    Ok(response)
}