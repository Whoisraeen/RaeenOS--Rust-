//! Microkernel Architecture Transition
//!
//! This module implements the transition from monolithic kernel services to
//! user-space microservices with IPC contracts for rae-netd, rae-compositord,
//! and rae-assistantd.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use core::fmt;
use serde::{Deserialize, Serialize};

/// Service types in the microkernel architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ServiceType {
    /// Network service daemon (rae-netd)
    Network,
    /// Compositor service daemon (rae-compositord)
    Compositor,
    /// AI assistant service daemon (rae-assistantd)
    Assistant,
    /// Audio service daemon (rae-audiod)
    Audio,
    /// Storage service daemon (rae-storaged)
    Storage,
    /// Security service daemon (rae-secd)
    Security,
}

/// Service capability requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCapabilities {
    /// Hardware access permissions
    pub hardware_access: Vec<HardwareCapability>,
    /// Memory access permissions
    pub memory_access: MemoryCapability,
    /// IPC permissions
    pub ipc_permissions: IpcCapability,
    /// File system access
    pub filesystem_access: FilesystemCapability,
}

/// Hardware capability enumeration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HardwareCapability {
    /// Network interface access
    NetworkInterface,
    /// GPU/Graphics hardware access
    Graphics,
    /// Audio hardware access
    Audio,
    /// Storage device access
    Storage,
    /// Input device access
    Input,
    /// USB device access
    Usb,
    /// PCI device access with specific vendor/device ID
    PciDevice { vendor_id: u16, device_id: u16 },
}

/// Memory access capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCapability {
    /// Maximum heap size in bytes
    pub max_heap_size: u64,
    /// DMA buffer access
    pub dma_access: bool,
    /// Shared memory regions
    pub shared_memory: bool,
    /// Physical memory access (dangerous)
    pub physical_memory: bool,
}

/// IPC capability permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcCapability {
    /// Can create IPC endpoints
    pub create_endpoints: bool,
    /// Can connect to other services
    pub connect_services: Vec<ServiceType>,
    /// Can accept connections from other services
    pub accept_from_services: Vec<ServiceType>,
    /// Maximum number of concurrent connections
    pub max_connections: u32,
}

/// Filesystem access capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemCapability {
    /// Read access to paths
    pub read_paths: Vec<String>,
    /// Write access to paths
    pub write_paths: Vec<String>,
    /// Execute access to paths
    pub execute_paths: Vec<String>,
    /// Can create new files/directories
    pub create_files: bool,
}

/// IPC message types for service communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    /// Network service messages
    Network(NetworkMessage),
    /// Compositor service messages
    Compositor(CompositorMessage),
    /// Assistant service messages
    Assistant(AssistantMessage),
    /// Generic service control messages
    ServiceControl(ServiceControlMessage),
}

/// Network service IPC messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Initialize network interface
    InitInterface { interface_name: String },
    /// Configure IP address
    ConfigureIp { interface: String, ip: String, netmask: String },
    /// Send packet
    SendPacket { interface: String, data: Vec<u8> },
    /// Receive packet notification
    PacketReceived { interface: String, data: Vec<u8> },
    /// Get network statistics
    GetStats { interface: String },
    /// Network statistics response
    StatsResponse { rx_bytes: u64, tx_bytes: u64, rx_packets: u64, tx_packets: u64 },
}

/// Compositor service IPC messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompositorMessage {
    /// Create window surface
    CreateSurface { width: u32, height: u32, format: PixelFormat },
    /// Update surface content
    UpdateSurface { surface_id: u32, x: u32, y: u32, width: u32, height: u32, data: Vec<u8> },
    /// Destroy surface
    DestroySurface { surface_id: u32 },
    /// Set window properties
    SetWindowProperties { surface_id: u32, title: String, resizable: bool },
    /// Input event notification
    InputEvent { surface_id: u32, event: InputEvent },
    /// Frame rendered notification
    FrameRendered { surface_id: u32, timestamp: u64 },
}

/// Assistant service IPC messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssistantMessage {
    /// Process text query
    TextQuery { query: String, context: String },
    /// Text response
    TextResponse { response: String, confidence: f32 },
    /// Process voice input
    VoiceInput { audio_data: Vec<u8>, sample_rate: u32 },
    /// Voice transcription result
    VoiceTranscription { text: String, confidence: f32 },
    /// Generate voice output
    GenerateVoice { text: String, voice_id: String },
    /// Voice output ready
    VoiceOutput { audio_data: Vec<u8>, sample_rate: u32 },
    /// Get assistant capabilities
    GetCapabilities,
    /// Assistant capabilities response
    CapabilitiesResponse { features: Vec<String> },
}

/// Service control messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceControlMessage {
    /// Start service
    Start { service_type: ServiceType },
    /// Stop service
    Stop { service_type: ServiceType },
    /// Restart service
    Restart { service_type: ServiceType },
    /// Get service status
    GetStatus { service_type: ServiceType },
    /// Service status response
    StatusResponse { service_type: ServiceType, status: ServiceStatus },
    /// Service health check
    HealthCheck { service_type: ServiceType },
    /// Health check response
    HealthResponse { service_type: ServiceType, healthy: bool, details: String },
}

/// Pixel format for compositor surfaces
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PixelFormat {
    Rgba8888,
    Rgb888,
    Bgra8888,
    Bgr888,
}

/// Input events for compositor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEvent {
    KeyPress { key_code: u32, modifiers: u32 },
    KeyRelease { key_code: u32, modifiers: u32 },
    MouseMove { x: i32, y: i32 },
    MouseButton { button: u32, pressed: bool, x: i32, y: i32 },
    MouseWheel { delta_x: i32, delta_y: i32, x: i32, y: i32 },
    Touch { id: u32, x: i32, y: i32, pressure: f32 },
}

/// Service status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
    Crashed,
}

/// Service registry for managing microkernel services
pub struct ServiceRegistry {
    services: BTreeMap<ServiceType, ServiceInfo>,
    ipc_endpoints: BTreeMap<ServiceType, u64>, // Service -> IPC endpoint ID
    capabilities: BTreeMap<ServiceType, ServiceCapabilities>,
}

/// Service information
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ServiceInfo {
    process_id: u64,
    status: ServiceStatus,
    start_time: u64,
    restart_count: u32,
    last_health_check: u64,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: BTreeMap::new(),
            ipc_endpoints: BTreeMap::new(),
            capabilities: BTreeMap::new(),
        }
    }
    
    /// Register a new service
    pub fn register_service(
        &mut self,
        service_type: ServiceType,
        process_id: u64,
        capabilities: ServiceCapabilities,
    ) -> Result<(), &'static str> {
        if self.services.contains_key(&service_type) {
            return Err("Service already registered");
        }
        
        let service_info = ServiceInfo {
            process_id,
            status: ServiceStatus::Starting,
            start_time: crate::time::get_timestamp_ns(),
            restart_count: 0,
            last_health_check: 0,
        };
        
        self.services.insert(service_type, service_info);
        self.capabilities.insert(service_type, capabilities);
        
        Ok(())
    }
    
    /// Unregister a service
    pub fn unregister_service(&mut self, service_type: ServiceType) -> Result<(), &'static str> {
        if !self.services.contains_key(&service_type) {
            return Err("Service not registered");
        }
        
        self.services.remove(&service_type);
        self.ipc_endpoints.remove(&service_type);
        self.capabilities.remove(&service_type);
        
        Ok(())
    }
    
    /// Update service status
    pub fn update_service_status(
        &mut self,
        service_type: ServiceType,
        status: ServiceStatus,
    ) -> Result<(), &'static str> {
        if let Some(service_info) = self.services.get_mut(&service_type) {
            service_info.status = status;
            Ok(())
        } else {
            Err("Service not found")
        }
    }
    
    /// Get service status
    pub fn get_service_status(&self, service_type: ServiceType) -> Option<ServiceStatus> {
        self.services.get(&service_type).map(|info| info.status)
    }
    
    /// Set IPC endpoint for service
    pub fn set_ipc_endpoint(
        &mut self,
        service_type: ServiceType,
        endpoint_id: u64,
    ) -> Result<(), &'static str> {
        if !self.services.contains_key(&service_type) {
            return Err("Service not registered");
        }
        
        self.ipc_endpoints.insert(service_type, endpoint_id);
        Ok(())
    }
    
    /// Get IPC endpoint for service
    pub fn get_ipc_endpoint(&self, service_type: ServiceType) -> Option<u64> {
        self.ipc_endpoints.get(&service_type).copied()
    }
    
    /// Check if service has capability
    pub fn has_capability(
        &self,
        service_type: ServiceType,
        capability: &HardwareCapability,
    ) -> bool {
        if let Some(caps) = self.capabilities.get(&service_type) {
            caps.hardware_access.contains(capability)
        } else {
            false
        }
    }
    
    /// Get all running services
    pub fn get_running_services(&self) -> Vec<ServiceType> {
        self.services
            .iter()
            .filter(|(_, info)| info.status == ServiceStatus::Running)
            .map(|(service_type, _)| *service_type)
            .collect()
    }
    
    /// Perform health check on all services
    pub fn health_check_all(&mut self) -> Vec<(ServiceType, bool)> {
        let current_time = crate::time::get_timestamp_ns();
        let mut results = Vec::new();
        
        for (service_type, service_info) in self.services.iter_mut() {
            // Simple health check: service should respond within 1 second
            let healthy = match service_info.status {
                ServiceStatus::Running => {
                    // In a real implementation, we would send a health check IPC message
                    // For now, just check if the process is still alive
                    crate::process::is_process_alive(service_info.process_id)
                },
                _ => false,
            };
            
            service_info.last_health_check = current_time;
            results.push((*service_type, healthy));
            
            // Update status if unhealthy
            if !healthy && service_info.status == ServiceStatus::Running {
                service_info.status = ServiceStatus::Failed;
            }
        }
        
        results
    }
}

/// Default service capabilities for each service type
impl ServiceType {
    pub fn default_capabilities(&self) -> ServiceCapabilities {
        match self {
            ServiceType::Network => ServiceCapabilities {
                hardware_access: vec![HardwareCapability::NetworkInterface],
                memory_access: MemoryCapability {
                    max_heap_size: 64 * 1024 * 1024, // 64MB
                    dma_access: true,
                    shared_memory: true,
                    physical_memory: false,
                },
                ipc_permissions: IpcCapability {
                    create_endpoints: true,
                    connect_services: vec![ServiceType::Security],
                    accept_from_services: vec![
                        ServiceType::Compositor,
                        ServiceType::Assistant,
                        ServiceType::Audio,
                    ],
                    max_connections: 100,
                },
                filesystem_access: FilesystemCapability {
                    read_paths: vec!["/etc/network".to_string(), "/proc/net".to_string()],
                    write_paths: vec!["/var/log/network".to_string()],
                    execute_paths: vec![],
                    create_files: false,
                },
            },
            ServiceType::Compositor => ServiceCapabilities {
                hardware_access: vec![HardwareCapability::Graphics, HardwareCapability::Input],
                memory_access: MemoryCapability {
                    max_heap_size: 256 * 1024 * 1024, // 256MB
                    dma_access: true,
                    shared_memory: true,
                    physical_memory: false,
                },
                ipc_permissions: IpcCapability {
                    create_endpoints: true,
                    connect_services: vec![ServiceType::Audio, ServiceType::Network],
                    accept_from_services: vec![
                        ServiceType::Assistant,
                        ServiceType::Audio,
                    ],
                    max_connections: 50,
                },
                filesystem_access: FilesystemCapability {
                    read_paths: vec!["/usr/share/fonts".to_string(), "/etc/compositor".to_string()],
                    write_paths: vec!["/var/log/compositor".to_string()],
                    execute_paths: vec![],
                    create_files: false,
                },
            },
            ServiceType::Assistant => ServiceCapabilities {
                hardware_access: vec![HardwareCapability::Audio],
                memory_access: MemoryCapability {
                    max_heap_size: 512 * 1024 * 1024, // 512MB
                    dma_access: false,
                    shared_memory: true,
                    physical_memory: false,
                },
                ipc_permissions: IpcCapability {
                    create_endpoints: true,
                    connect_services: vec![
                        ServiceType::Network,
                        ServiceType::Compositor,
                        ServiceType::Audio,
                        ServiceType::Storage,
                    ],
                    accept_from_services: vec![
                        ServiceType::Compositor,
                        ServiceType::Audio,
                    ],
                    max_connections: 20,
                },
                filesystem_access: FilesystemCapability {
                    read_paths: vec!["/usr/share/ai-models".to_string(), "/etc/assistant".to_string()],
                    write_paths: vec!["/var/log/assistant".to_string(), "/var/cache/assistant".to_string()],
                    execute_paths: vec![],
                    create_files: true,
                },
            },
            ServiceType::Audio => ServiceCapabilities {
                hardware_access: vec![HardwareCapability::Audio],
                memory_access: MemoryCapability {
                    max_heap_size: 32 * 1024 * 1024, // 32MB
                    dma_access: true,
                    shared_memory: true,
                    physical_memory: false,
                },
                ipc_permissions: IpcCapability {
                    create_endpoints: true,
                    connect_services: vec![],
                    accept_from_services: vec![
                        ServiceType::Compositor,
                        ServiceType::Assistant,
                    ],
                    max_connections: 10,
                },
                filesystem_access: FilesystemCapability {
                    read_paths: vec!["/etc/audio".to_string()],
                    write_paths: vec!["/var/log/audio".to_string()],
                    execute_paths: vec![],
                    create_files: false,
                },
            },
            ServiceType::Storage => ServiceCapabilities {
                hardware_access: vec![HardwareCapability::Storage],
                memory_access: MemoryCapability {
                    max_heap_size: 128 * 1024 * 1024, // 128MB
                    dma_access: true,
                    shared_memory: true,
                    physical_memory: false,
                },
                ipc_permissions: IpcCapability {
                    create_endpoints: true,
                    connect_services: vec![ServiceType::Security],
                    accept_from_services: vec![
                        ServiceType::Assistant,
                        ServiceType::Compositor,
                    ],
                    max_connections: 30,
                },
                filesystem_access: FilesystemCapability {
                    read_paths: vec!["/".to_string()],
                    write_paths: vec!["/".to_string()],
                    execute_paths: vec![],
                    create_files: true,
                },
            },
            ServiceType::Security => ServiceCapabilities {
                hardware_access: vec![],
                memory_access: MemoryCapability {
                    max_heap_size: 16 * 1024 * 1024, // 16MB
                    dma_access: false,
                    shared_memory: false,
                    physical_memory: false,
                },
                ipc_permissions: IpcCapability {
                    create_endpoints: true,
                    connect_services: vec![],
                    accept_from_services: vec![
                        ServiceType::Network,
                        ServiceType::Storage,
                    ],
                    max_connections: 5,
                },
                filesystem_access: FilesystemCapability {
                    read_paths: vec!["/etc/security".to_string()],
                    write_paths: vec!["/var/log/security".to_string()],
                    execute_paths: vec![],
                    create_files: false,
                },
            },
        }
    }
}

/// Global service registry
static SERVICE_REGISTRY: spin::Mutex<Option<ServiceRegistry>> = spin::Mutex::new(None);

/// Initialize the microkernel service registry
pub fn init_service_registry() {
    let mut registry = SERVICE_REGISTRY.lock();
    *registry = Some(ServiceRegistry::new());
}

/// Access the global service registry
pub fn with_service_registry<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut ServiceRegistry) -> R,
{
    let mut registry = SERVICE_REGISTRY.lock();
    registry.as_mut().map(f)
}

/// Register a microkernel service
pub fn register_microkernel_service(
    service_type: ServiceType,
    process_id: u64,
) -> Result<(), &'static str> {
    let capabilities = service_type.default_capabilities();
    
    with_service_registry(|registry| {
        registry.register_service(service_type, process_id, capabilities)
    })
    .unwrap_or(Err("Service registry not initialized"))
}

/// Internal function to send IPC message to service endpoint
fn send_ipc_message_to_service(endpoint_id: u64, data: &[u8]) -> Result<(), crate::ipc::IpcError> {
    // Get current process ID
    let current_pid = crate::process::get_current_process_id() as u32;
    
    // For microkernel services, we use the endpoint_id as the handle_id
    // In a full implementation, there would be a mapping from endpoint_id to handle_id
    let handle_id = endpoint_id as u32;
    
    // Send via IPC ring buffer (preferred for microkernel architecture)
    crate::ipc::send_to_ring(current_pid, handle_id, data)
}

/// Send IPC message to a service
pub fn send_service_message(
    target_service: ServiceType,
    message: IpcMessage,
) -> Result<(), &'static str> {
    let endpoint_id = with_service_registry(|registry| {
        registry.get_ipc_endpoint(target_service)
    })
    .flatten()
    .ok_or("Service endpoint not found")?;
    
    // Serialize message
    let serialized = serde_json::to_vec(&message)
        .map_err(|_| "Failed to serialize message")?;
    
    // Send via IPC using message queue or ring buffer
    // For now, we'll use a simple approach - in a full implementation,
    // this would route through the appropriate IPC mechanism
    send_ipc_message_to_service(endpoint_id, &serialized)
        .map_err(|_| "Failed to send IPC message")?;
    
    Ok(())
}

/// Handle incoming service message
pub fn handle_service_message(
    from_service: ServiceType,
    message_data: &[u8],
) -> Result<Option<IpcMessage>, &'static str> {
    // Deserialize message
    let message: IpcMessage = serde_json::from_slice(message_data)
        .map_err(|_| "Failed to deserialize message")?;
    
    // Process message based on type
    match &message {
        IpcMessage::ServiceControl(control_msg) => {
            handle_service_control_message(from_service, control_msg)?;
        },
        IpcMessage::Network(_) => {
            // Forward to network service or handle in kernel
        },
        IpcMessage::Compositor(_) => {
            // Forward to compositor service or handle in kernel
        },
        IpcMessage::Assistant(_) => {
            // Forward to assistant service or handle in kernel
        },
    }
    
    Ok(Some(message))
}

/// Handle service control messages
fn handle_service_control_message(
    from_service: ServiceType,
    message: &ServiceControlMessage,
) -> Result<(), &'static str> {
    match message {
        ServiceControlMessage::GetStatus { service_type } => {
            let status = with_service_registry(|registry| {
                registry.get_service_status(*service_type)
            })
            .flatten()
            .unwrap_or(ServiceStatus::Stopped);
            
            let response = IpcMessage::ServiceControl(ServiceControlMessage::StatusResponse {
                service_type: *service_type,
                status,
            });
            
            send_service_message(from_service, response)?;
        },
        ServiceControlMessage::HealthCheck { service_type } => {
            let healthy = with_service_registry(|registry| {
                registry.get_service_status(*service_type) == Some(ServiceStatus::Running)
            })
            .unwrap_or(false);
            
            let response = IpcMessage::ServiceControl(ServiceControlMessage::HealthResponse {
                service_type: *service_type,
                healthy,
                details: if healthy { "Service running".to_string() } else { "Service not running".to_string() },
            });
            
            send_service_message(from_service, response)?;
        },
        _ => {
            // Other control messages would be handled here
        }
    }
    
    Ok(())
}

/// Perform health checks on all services
pub fn perform_service_health_checks() -> Vec<(ServiceType, bool)> {
    with_service_registry(|registry| {
        registry.health_check_all()
    })
    .unwrap_or_default()
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceType::Network => write!(f, "rae-netd"),
            ServiceType::Compositor => write!(f, "rae-compositord"),
            ServiceType::Assistant => write!(f, "rae-assistantd"),
            ServiceType::Audio => write!(f, "rae-audiod"),
            ServiceType::Storage => write!(f, "rae-storaged"),
            ServiceType::Security => write!(f, "rae-secd"),
        }
    }
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceStatus::Stopped => write!(f, "stopped"),
            ServiceStatus::Starting => write!(f, "starting"),
            ServiceStatus::Running => write!(f, "running"),
            ServiceStatus::Stopping => write!(f, "stopping"),
            ServiceStatus::Failed => write!(f, "failed"),
            ServiceStatus::Crashed => write!(f, "crashed"),
        }
    }
}