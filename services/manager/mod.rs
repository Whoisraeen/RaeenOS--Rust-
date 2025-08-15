//! Service Manager for RaeenOS microkernel architecture
//! Manages user-space services and IPC routing

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::{Mutex, RwLock};
use crate::ipc::{IpcObject, CapabilityEndpoint, IpcRights};
use crate::process::ProcessId;
use super::contracts::*;

pub mod service_registry;
pub mod ipc_router;
pub mod health_monitor;
pub mod resource_manager;

use service_registry::ServiceRegistry;
use ipc_router::IpcRouter;
use health_monitor::HealthMonitor;
use resource_manager::ResourceManager;

/// Global service manager instance
static SERVICE_MANAGER: Mutex<Option<ServiceManager>> = Mutex::new(None);
static NEXT_SERVICE_ID: AtomicU32 = AtomicU32::new(1);

/// Service manager for coordinating user-space services
pub struct ServiceManager {
    registry: ServiceRegistry,
    router: IpcRouter,
    health_monitor: HealthMonitor,
    resource_manager: ResourceManager,
    services: RwLock<BTreeMap<u32, ServiceInstance>>,
    endpoints: RwLock<BTreeMap<String, CapabilityEndpoint>>,
}

/// Service instance information
#[derive(Debug, Clone)]
pub struct ServiceInstance {
    pub id: u32,
    pub info: ServiceInfo,
    pub process_id: ProcessId,
    pub endpoint: CapabilityEndpoint,
    pub status: ServiceStatus,
    pub start_time: u64,
    pub restart_count: u32,
    pub last_health_check: u64,
    pub resource_usage: ResourceUsage,
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
    Restarting,
}

/// Resource usage tracking
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    pub memory_mb: u32,
    pub cpu_percent: f32,
    pub disk_io_mb: u32,
    pub network_io_mb: u32,
    pub ipc_messages: u64,
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub auto_restart: bool,
    pub max_restarts: u32,
    pub restart_delay_ms: u32,
    pub health_check_interval_ms: u32,
    pub resource_limits: ResourceLimits,
    pub dependencies: Vec<String>,
    pub environment: BTreeMap<String, String>,
}

/// Resource limits for services
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory_mb: Option<u32>,
    pub max_cpu_percent: Option<f32>,
    pub max_disk_io_mb_per_sec: Option<u32>,
    pub max_network_io_mb_per_sec: Option<u32>,
    pub max_ipc_messages_per_sec: Option<u32>,
}

impl ServiceManager {
    /// Initialize the global service manager
    pub fn init() -> Result<(), &'static str> {
        let mut manager = SERVICE_MANAGER.lock();
        if manager.is_some() {
            return Err("Service manager already initialized");
        }
        
        *manager = Some(ServiceManager {
            registry: ServiceRegistry::new(),
            router: IpcRouter::new(),
            health_monitor: HealthMonitor::new(),
            resource_manager: ResourceManager::new(),
            services: RwLock::new(BTreeMap::new()),
            endpoints: RwLock::new(BTreeMap::new()),
        });
        
        Ok(())
    }
    
    /// Get the global service manager instance
    pub fn instance() -> Result<&'static Mutex<Option<ServiceManager>>, &'static str> {
        Ok(&SERVICE_MANAGER)
    }
    
    /// Register a new service
    pub fn register_service(
        &mut self,
        info: ServiceInfo,
        process_id: ProcessId,
        config: ServiceConfig,
    ) -> Result<u32, ServiceError> {
        let service_id = NEXT_SERVICE_ID.fetch_add(1, Ordering::SeqCst);
        
        // Create IPC endpoint for the service
        let endpoint = CapabilityEndpoint::new(
            format!("service_{}", service_id),
            IpcRights::READ | IpcRights::WRITE,
        ).map_err(|_| ServiceError::EndpointCreationFailed)?;
        
        // Register with service registry
        self.registry.register(service_id, info.clone(), config.clone())?;
        
        // Create service instance
        let instance = ServiceInstance {
            id: service_id,
            info: info.clone(),
            process_id,
            endpoint: endpoint.clone(),
            status: ServiceStatus::Starting,
            start_time: crate::time::get_timestamp(),
            restart_count: 0,
            last_health_check: 0,
            resource_usage: ResourceUsage::default(),
        };
        
        // Store service instance
        self.services.write().insert(service_id, instance);
        
        // Store endpoint by service name
        self.endpoints.write().insert(info.name.clone(), endpoint);
        
        // Start health monitoring
        self.health_monitor.start_monitoring(service_id, config.health_check_interval_ms)?;
        
        // Set up resource monitoring
        self.resource_manager.set_limits(service_id, config.resource_limits)?;
        
        Ok(service_id)
    }
    
    /// Unregister a service
    pub fn unregister_service(&mut self, service_id: u32) -> Result<(), ServiceError> {
        let mut services = self.services.write();
        let service = services.remove(&service_id)
            .ok_or(ServiceError::ServiceNotFound)?;
        
        // Remove from registry
        self.registry.unregister(service_id)?;
        
        // Remove endpoint
        self.endpoints.write().remove(&service.info.name);
        
        // Stop monitoring
        self.health_monitor.stop_monitoring(service_id)?;
        self.resource_manager.remove_limits(service_id)?;
        
        Ok(())
    }
    
    /// Start a service
    pub fn start_service(&mut self, service_id: u32) -> Result<(), ServiceError> {
        let mut services = self.services.write();
        let service = services.get_mut(&service_id)
            .ok_or(ServiceError::ServiceNotFound)?;
        
        if service.status != ServiceStatus::Stopped {
            return Err(ServiceError::InvalidState);
        }
        
        service.status = ServiceStatus::Starting;
        service.start_time = crate::time::get_timestamp();
        
        // TODO: Actually start the service process
        // This would involve spawning the service binary in user-space
        
        service.status = ServiceStatus::Running;
        Ok(())
    }
    
    /// Stop a service
    pub fn stop_service(&mut self, service_id: u32) -> Result<(), ServiceError> {
        let mut services = self.services.write();
        let service = services.get_mut(&service_id)
            .ok_or(ServiceError::ServiceNotFound)?;
        
        if service.status != ServiceStatus::Running {
            return Err(ServiceError::InvalidState);
        }
        
        service.status = ServiceStatus::Stopping;
        
        // TODO: Send shutdown signal to service process
        
        service.status = ServiceStatus::Stopped;
        Ok(())
    }
    
    /// Restart a service
    pub fn restart_service(&mut self, service_id: u32) -> Result<(), ServiceError> {
        let mut services = self.services.write();
        let service = services.get_mut(&service_id)
            .ok_or(ServiceError::ServiceNotFound)?;
        
        service.status = ServiceStatus::Restarting;
        service.restart_count += 1;
        
        // TODO: Implement graceful restart
        
        service.status = ServiceStatus::Running;
        service.start_time = crate::time::get_timestamp();
        
        Ok(())
    }
    
    /// Get service information
    pub fn get_service_info(&self, service_id: u32) -> Result<ServiceInstance, ServiceError> {
        let services = self.services.read();
        services.get(&service_id)
            .cloned()
            .ok_or(ServiceError::ServiceNotFound)
    }
    
    /// List all services
    pub fn list_services(&self) -> Vec<ServiceInstance> {
        self.services.read().values().cloned().collect()
    }
    
    /// Find service by name
    pub fn find_service_by_name(&self, name: &str) -> Option<ServiceInstance> {
        let services = self.services.read();
        services.values().find(|s| s.info.name == name).cloned()
    }
    
    /// Route IPC message to service
    pub fn route_message(
        &mut self,
        target_service: &str,
        message: &[u8],
        sender_process: ProcessId,
    ) -> Result<Vec<u8>, ServiceError> {
        // Check if target service exists
        let endpoint = {
            let endpoints = self.endpoints.read();
            endpoints.get(target_service)
                .cloned()
                .ok_or(ServiceError::ServiceNotFound)?
        };
        
        // Route through IPC router
        self.router.route_message(endpoint, message, sender_process)
            .map_err(|_| ServiceError::IpcRoutingFailed)
    }
    
    /// Perform health check on all services
    pub fn health_check(&mut self) -> Result<Vec<HealthStatus>, ServiceError> {
        let service_ids: Vec<u32> = self.services.read().keys().cloned().collect();
        let mut results = Vec::new();
        
        for service_id in service_ids {
            match self.health_monitor.check_health(service_id) {
                Ok(status) => {
                    results.push(status);
                    
                    // Update service status based on health check
                    if let Ok(mut services) = self.services.try_write() {
                        if let Some(service) = services.get_mut(&service_id) {
                            service.last_health_check = crate::time::get_timestamp();
                            
                            match status {
                                HealthStatus::Healthy => {
                                    if service.status == ServiceStatus::Failed {
                                        service.status = ServiceStatus::Running;
                                    }
                                }
                                HealthStatus::Unhealthy => {
                                    service.status = ServiceStatus::Failed;
                                }
                                HealthStatus::Unknown => {
                                    // Keep current status
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    results.push(HealthStatus::Unknown);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Update resource usage for a service
    pub fn update_resource_usage(
        &mut self,
        service_id: u32,
        usage: ResourceUsage,
    ) -> Result<(), ServiceError> {
        let mut services = self.services.write();
        let service = services.get_mut(&service_id)
            .ok_or(ServiceError::ServiceNotFound)?;
        
        service.resource_usage = usage;
        
        // Check resource limits
        self.resource_manager.check_limits(service_id, &service.resource_usage)?;
        
        Ok(())
    }
    
    /// Get system-wide service statistics
    pub fn get_statistics(&self) -> ServiceStatistics {
        let services = self.services.read();
        let total_services = services.len() as u32;
        let running_services = services.values()
            .filter(|s| s.status == ServiceStatus::Running)
            .count() as u32;
        let failed_services = services.values()
            .filter(|s| s.status == ServiceStatus::Failed)
            .count() as u32;
        
        let total_memory = services.values()
            .map(|s| s.resource_usage.memory_mb)
            .sum();
        
        let total_cpu = services.values()
            .map(|s| s.resource_usage.cpu_percent)
            .sum();
        
        let total_ipc_messages = services.values()
            .map(|s| s.resource_usage.ipc_messages)
            .sum();
        
        ServiceStatistics {
            total_services,
            running_services,
            failed_services,
            total_memory_mb: total_memory,
            total_cpu_percent: total_cpu,
            total_ipc_messages,
            uptime_seconds: crate::time::get_uptime_seconds(),
        }
    }
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct ServiceStatistics {
    pub total_services: u32,
    pub running_services: u32,
    pub failed_services: u32,
    pub total_memory_mb: u32,
    pub total_cpu_percent: f32,
    pub total_ipc_messages: u64,
    pub uptime_seconds: u64,
}

/// Service manager errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceError {
    ServiceNotFound,
    InvalidState,
    EndpointCreationFailed,
    IpcRoutingFailed,
    ResourceLimitExceeded,
    HealthCheckFailed,
    DependencyNotMet,
    PermissionDenied,
    InternalError,
}

/// Convenience functions for global service manager access
pub fn init_service_manager() -> Result<(), &'static str> {
    ServiceManager::init()
}

pub fn register_service(
    info: ServiceInfo,
    process_id: ProcessId,
    config: ServiceConfig,
) -> Result<u32, ServiceError> {
    let manager_lock = ServiceManager::instance()
        .map_err(|_| ServiceError::InternalError)?;
    let mut manager_opt = manager_lock.lock();
    let manager = manager_opt.as_mut()
        .ok_or(ServiceError::InternalError)?;
    manager.register_service(info, process_id, config)
}

pub fn route_ipc_message(
    target_service: &str,
    message: &[u8],
    sender_process: ProcessId,
) -> Result<Vec<u8>, ServiceError> {
    let manager_lock = ServiceManager::instance()
        .map_err(|_| ServiceError::InternalError)?;
    let mut manager_opt = manager_lock.lock();
    let manager = manager_opt.as_mut()
        .ok_or(ServiceError::InternalError)?;
    manager.route_message(target_service, message, sender_process)
}

pub fn get_service_statistics() -> Result<ServiceStatistics, ServiceError> {
    let manager_lock = ServiceManager::instance()
        .map_err(|_| ServiceError::InternalError)?;
    let manager_opt = manager_lock.lock();
    let manager = manager_opt.as_ref()
        .ok_or(ServiceError::InternalError)?;
    Ok(manager.get_statistics())
}