//! RaeenOS User-Space Services
//! 
//! This module provides the microkernel service architecture for RaeenOS,
//! moving functionality from kernel syscalls to user-space IPC contracts.
//!
//! # Architecture Overview
//!
//! The RaeenOS microkernel architecture consists of:
//! - **Service Manager**: Coordinates all user-space services and handles IPC routing
//! - **Network Service** (`rae-networkd`): Handles all network operations
//! - **Graphics Service** (`rae-compositord`): Manages graphics, windows, and compositor
//! - **AI Service** (`rae-assistantd`): Provides AI assistant capabilities
//!
//! # Service Communication
//!
//! Services communicate via IPC using well-defined contracts:
//! - Each service exposes a specific set of capabilities
//! - Requests and responses are strongly typed
//! - The service manager routes messages between services and the kernel
//!
//! # Security Model
//!
//! - Services run in isolated user-space processes
//! - Capabilities are granted on a per-service basis
//! - IPC messages are validated and sandboxed
//! - Resource limits are enforced per service

use alloc::vec::Vec;
use alloc::string::String;
use spin::{Mutex, RwLock};

pub mod contracts;
pub mod manager;
pub mod network;
pub mod graphics;
pub mod ai;

use contracts::*;
use manager::ServiceManager;

/// Main service coordinator for RaeenOS
pub struct RaeenOSServices {
    service_manager: ServiceManager,
    network_service: Option<network::NetworkService>,
    graphics_service: Option<graphics::GraphicsService>,
    ai_service: Option<ai::AiService>,
    initialized: bool,
}

/// Service startup configuration
#[derive(Debug, Clone)]
pub struct ServiceStartupConfig {
    pub enable_network_service: bool,
    pub enable_graphics_service: bool,
    pub enable_ai_service: bool,
    pub service_manager_config: manager::ServiceManagerConfig,
}

impl Default for ServiceStartupConfig {
    fn default() -> Self {
        Self {
            enable_network_service: true,
            enable_graphics_service: true,
            enable_ai_service: true,
            service_manager_config: manager::ServiceManagerConfig::default(),
        }
    }
}

/// Global service registry errors
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceRegistryError {
    ServiceNotFound,
    ServiceAlreadyRegistered,
    ServiceNotInitialized,
    IpcRoutingFailed,
    CapabilityDenied,
    ResourceLimitExceeded,
    InvalidServiceConfig,
    ServiceUnhealthy,
}

impl RaeenOSServices {
    /// Create a new RaeenOS services coordinator
    pub fn new() -> Self {
        Self {
            service_manager: ServiceManager::new(),
            network_service: None,
            graphics_service: None,
            ai_service: None,
            initialized: false,
        }
    }
    
    /// Initialize all services with the given configuration
    pub fn initialize(&mut self, config: ServiceStartupConfig) -> Result<(), ServiceError> {
        if self.initialized {
            return Err(ServiceError::AlreadyInitialized);
        }
        
        // Initialize service manager first
        self.service_manager.initialize(config.service_manager_config)?;
        
        // Initialize network service if enabled
        if config.enable_network_service {
            let mut network_service = network::NetworkService::new();
            network_service.initialize()?;
            
            // Register with service manager
            self.service_manager.register_service(
                network_service.get_service_info().clone(),
                manager::ServiceConfig::default(),
            )?;
            
            self.network_service = Some(network_service);
        }
        
        // Initialize graphics service if enabled
        if config.enable_graphics_service {
            let mut graphics_service = graphics::GraphicsService::new();
            graphics_service.initialize()?;
            
            // Register with service manager
            self.service_manager.register_service(
                graphics_service.get_service_info().clone(),
                manager::ServiceConfig::default(),
            )?;
            
            self.graphics_service = Some(graphics_service);
        }
        
        // Initialize AI service if enabled
        if config.enable_ai_service {
            let mut ai_service = ai::AiService::new();
            ai_service.initialize()?;
            
            // Register with service manager
            self.service_manager.register_service(
                ai_service.get_service_info().clone(),
                manager::ServiceConfig::default(),
            )?;
            
            self.ai_service = Some(ai_service);
        }
        
        self.initialized = true;
        Ok(())
    }
    
    /// Route a message to the appropriate service
    pub fn route_message(&self, service_name: &str, message: &[u8]) -> Result<Vec<u8>, ServiceError> {
        if !self.initialized {
            return Err(ServiceError::NotInitialized);
        }
        
        // Use service manager to route the message
        self.service_manager.route_message(service_name, message)
    }
    
    /// Handle network request
    pub fn handle_network_request(&self, request: contracts::network::NetworkRequest) -> Result<contracts::network::NetworkResponse, ServiceError> {
        if let Some(ref network_service) = self.network_service {
            network_service.handle_request(request)
        } else {
            Err(ServiceError::ServiceNotAvailable)
        }
    }
    
    /// Handle graphics request
    pub fn handle_graphics_request(&self, request: contracts::graphics::GraphicsRequest) -> Result<contracts::graphics::GraphicsResponse, ServiceError> {
        if let Some(ref graphics_service) = self.graphics_service {
            graphics_service.handle_request(request)
        } else {
            Err(ServiceError::ServiceNotAvailable)
        }
    }
    
    /// Handle AI request
    pub fn handle_ai_request(&self, request: contracts::ai::AiRequest) -> Result<contracts::ai::AiResponse, ServiceError> {
        if let Some(ref ai_service) = self.ai_service {
            ai_service.handle_request(request)
        } else {
            Err(ServiceError::ServiceNotAvailable)
        }
    }
    
    /// Get service information for a specific service
    pub fn get_service_info(&self, service_name: &str) -> Result<ServiceInfo, ServiceRegistryError> {
        match service_name {
            "rae-networkd" => {
                if let Some(ref service) = self.network_service {
                    Ok(service.get_service_info().clone())
                } else {
                    Err(ServiceRegistryError::ServiceNotFound)
                }
            }
            "rae-compositord" => {
                if let Some(ref service) = self.graphics_service {
                    Ok(service.get_service_info().clone())
                } else {
                    Err(ServiceRegistryError::ServiceNotFound)
                }
            }
            "rae-assistantd" => {
                if let Some(ref service) = self.ai_service {
                    Ok(service.get_service_info().clone())
                } else {
                    Err(ServiceRegistryError::ServiceNotFound)
                }
            }
            _ => Err(ServiceRegistryError::ServiceNotFound),
        }
    }
    
    /// List all available services
    pub fn list_services(&self) -> Vec<ServiceInfo> {
        let mut services = Vec::new();
        
        if let Some(ref service) = self.network_service {
            services.push(service.get_service_info().clone());
        }
        
        if let Some(ref service) = self.graphics_service {
            services.push(service.get_service_info().clone());
        }
        
        if let Some(ref service) = self.ai_service {
            services.push(service.get_service_info().clone());
        }
        
        services
    }
    
    /// Perform health check on all services
    pub fn health_check(&self) -> Result<Vec<(String, HealthStatus)>, ServiceError> {
        let mut health_statuses = Vec::new();
        
        if let Some(ref service) = self.network_service {
            let info = service.get_service_info();
            health_statuses.push((info.name.clone(), info.health_status.clone()));
        }
        
        if let Some(ref service) = self.graphics_service {
            let info = service.get_service_info();
            health_statuses.push((info.name.clone(), info.health_status.clone()));
        }
        
        if let Some(ref service) = self.ai_service {
            let info = service.get_service_info();
            health_statuses.push((info.name.clone(), info.health_status.clone()));
        }
        
        Ok(health_statuses)
    }
    
    /// Get service manager statistics
    pub fn get_service_manager_statistics(&self) -> manager::ServiceManagerStatistics {
        self.service_manager.get_statistics()
    }
    
    /// Render frame (for graphics service)
    pub fn render_frame(&self) -> Result<(), ServiceError> {
        if let Some(ref graphics_service) = self.graphics_service {
            graphics_service.render_frame()
        } else {
            Err(ServiceError::ServiceNotAvailable)
        }
    }
    
    /// Perform periodic maintenance on all services
    pub fn perform_maintenance(&self) -> Result<(), ServiceError> {
        // Perform service manager maintenance
        self.service_manager.perform_maintenance()?;
        
        // Perform network service maintenance
        if let Some(ref network_service) = self.network_service {
            network_service.perform_maintenance()?;
        }
        
        // Graphics service maintenance is handled in render loop
        
        // Perform AI service maintenance
        if let Some(ref ai_service) = self.ai_service {
            ai_service.perform_maintenance()?;
        }
        
        Ok(())
    }
    
    /// Shutdown all services gracefully
    pub fn shutdown(&mut self) -> Result<(), ServiceError> {
        if !self.initialized {
            return Ok(());
        }
        
        // Shutdown services in reverse order of initialization
        
        // Shutdown AI service
        if let Some(mut ai_service) = self.ai_service.take() {
            ai_service.shutdown()?;
        }
        
        // Shutdown graphics service
        if let Some(mut graphics_service) = self.graphics_service.take() {
            graphics_service.shutdown()?;
        }
        
        // Shutdown network service
        if let Some(mut network_service) = self.network_service.take() {
            network_service.shutdown()?;
        }
        
        // Shutdown service manager last
        self.service_manager.shutdown()?;
        
        self.initialized = false;
        Ok(())
    }
    
    /// Check if services are initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get service manager reference
    pub fn get_service_manager(&self) -> &ServiceManager {
        &self.service_manager
    }
}

/// Global services instance (for kernel integration)
static SERVICES: Mutex<Option<RaeenOSServices>> = Mutex::new(None);

/// Initialize global services
pub fn initialize_services(config: ServiceStartupConfig) -> Result<(), ServiceError> {
    let mut services_guard = SERVICES.lock();
    
    if services_guard.is_some() {
        return Err(ServiceError::AlreadyInitialized);
    }
    
    let mut services = RaeenOSServices::new();
    services.initialize(config)?;
    
    *services_guard = Some(services);
    Ok(())
}

/// Get global services instance
pub fn get_services() -> Result<&'static Mutex<Option<RaeenOSServices>>, ServiceError> {
    Ok(&SERVICES)
}

/// Route message to service (global function for kernel integration)
pub fn route_to_service(service_name: &str, message: &[u8]) -> Result<Vec<u8>, ServiceError> {
    let services_guard = SERVICES.lock();
    
    if let Some(ref services) = *services_guard {
        services.route_message(service_name, message)
    } else {
        Err(ServiceError::NotInitialized)
    }
}

/// Handle network syscall via service (for kernel integration)
pub fn handle_network_syscall(request: contracts::network::NetworkRequest) -> Result<contracts::network::NetworkResponse, ServiceError> {
    let services_guard = SERVICES.lock();
    
    if let Some(ref services) = *services_guard {
        services.handle_network_request(request)
    } else {
        Err(ServiceError::NotInitialized)
    }
}

/// Handle graphics syscall via service (for kernel integration)
pub fn handle_graphics_syscall(request: contracts::graphics::GraphicsRequest) -> Result<contracts::graphics::GraphicsResponse, ServiceError> {
    let services_guard = SERVICES.lock();
    
    if let Some(ref services) = *services_guard {
        services.handle_graphics_request(request)
    } else {
        Err(ServiceError::NotInitialized)
    }
}

/// Handle AI syscall via service (for kernel integration)
pub fn handle_ai_syscall(request: contracts::ai::AiRequest) -> Result<contracts::ai::AiResponse, ServiceError> {
    let services_guard = SERVICES.lock();
    
    if let Some(ref services) = *services_guard {
        services.handle_ai_request(request)
    } else {
        Err(ServiceError::NotInitialized)
    }
}

/// Render frame (for kernel integration)
pub fn render_frame() -> Result<(), ServiceError> {
    let services_guard = SERVICES.lock();
    
    if let Some(ref services) = *services_guard {
        services.render_frame()
    } else {
        Err(ServiceError::NotInitialized)
    }
}

/// Perform maintenance on all services (for kernel integration)
pub fn perform_services_maintenance() -> Result<(), ServiceError> {
    let services_guard = SERVICES.lock();
    
    if let Some(ref services) = *services_guard {
        services.perform_maintenance()
    } else {
        Err(ServiceError::NotInitialized)
    }
}

/// Shutdown all services (for kernel integration)
pub fn shutdown_services() -> Result<(), ServiceError> {
    let mut services_guard = SERVICES.lock();
    
    if let Some(mut services) = services_guard.take() {
        services.shutdown()
    } else {
        Ok(())
    }
}

/// Get service health status (for kernel integration)
pub fn get_services_health() -> Result<Vec<(String, HealthStatus)>, ServiceError> {
    let services_guard = SERVICES.lock();
    
    if let Some(ref services) = *services_guard {
        services.health_check()
    } else {
        Err(ServiceError::NotInitialized)
    }
}

/// List all available services (for kernel integration)
pub fn list_available_services() -> Result<Vec<ServiceInfo>, ServiceError> {
    let services_guard = SERVICES.lock();
    
    if let Some(ref services) = *services_guard {
        Ok(services.list_services())
    } else {
        Err(ServiceError::NotInitialized)
    }
}