//! Service contracts for RaeenOS microkernel architecture
//! Defines IPC schemas and interfaces for user-space services

pub mod network;
pub mod graphics;
pub mod ai;

use alloc::vec::Vec;
use alloc::string::String;
use serde::{Serialize, Deserialize};

/// Service discovery and lifecycle management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub version: u32,
    pub capabilities: Vec<String>,
    pub process_id: u32,
    pub ipc_handle: u32,
}

/// Standard service response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceResponse<T> {
    Success(T),
    Error { code: u32, message: String },
}

/// Service lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceEvent {
    Started { service: ServiceInfo },
    Stopped { service_name: String, process_id: u32 },
    Failed { service_name: String, error: String },
    HealthCheck { service_name: String, status: HealthStatus },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

/// Service manager interface
pub trait ServiceManager {
    fn register_service(&mut self, info: ServiceInfo) -> Result<(), String>;
    fn unregister_service(&mut self, service_name: &str) -> Result<(), String>;
    fn discover_service(&self, service_name: &str) -> Option<ServiceInfo>;
    fn list_services(&self) -> Vec<ServiceInfo>;
    fn health_check(&self, service_name: &str) -> Result<HealthStatus, String>;
}

/// Common error codes for all services
pub mod error_codes {
    pub const SUCCESS: u32 = 0;
    pub const INVALID_REQUEST: u32 = 1;
    pub const PERMISSION_DENIED: u32 = 2;
    pub const RESOURCE_EXHAUSTED: u32 = 3;
    pub const SERVICE_UNAVAILABLE: u32 = 4;
    pub const TIMEOUT: u32 = 5;
    pub const INTERNAL_ERROR: u32 = 6;
}