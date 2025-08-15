//! Service Registry for managing service registration and discovery
//! Maintains a catalog of available services and their capabilities

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use spin::RwLock;
use super::contracts::*;
use super::{ServiceConfig, ServiceError};

/// Service registry for managing service discovery
pub struct ServiceRegistry {
    services: RwLock<BTreeMap<u32, RegisteredService>>,
    name_to_id: RwLock<BTreeMap<String, u32>>,
    capabilities: RwLock<BTreeMap<String, Vec<u32>>>, // capability -> service_ids
    dependencies: RwLock<BTreeMap<u32, Vec<String>>>, // service_id -> dependencies
}

/// Registered service information
#[derive(Debug, Clone)]
pub struct RegisteredService {
    pub id: u32,
    pub info: ServiceInfo,
    pub config: ServiceConfig,
    pub registration_time: u64,
    pub last_update: u64,
    pub version: String,
    pub metadata: ServiceMetadata,
}

/// Extended service metadata
#[derive(Debug, Clone)]
pub struct ServiceMetadata {
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<ServiceCategory>,
    pub minimum_os_version: String,
    pub supported_architectures: Vec<String>,
}

/// Service categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceCategory {
    System,
    Network,
    Graphics,
    Audio,
    Input,
    Storage,
    Security,
    Development,
    Gaming,
    Productivity,
    Multimedia,
    Communication,
    Utility,
}

/// Service discovery query
#[derive(Debug, Clone)]
pub struct ServiceQuery {
    pub name: Option<String>,
    pub capability: Option<String>,
    pub category: Option<ServiceCategory>,
    pub version_requirement: Option<VersionRequirement>,
    pub status: Option<ServiceStatus>,
    pub keywords: Vec<String>,
}

/// Version requirement specification
#[derive(Debug, Clone)]
pub struct VersionRequirement {
    pub operator: VersionOperator,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionOperator {
    Exact,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Compatible, // Semantic versioning compatible
}

/// Service status for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Available,
    Running,
    Stopped,
    Failed,
}

/// Service discovery result
#[derive(Debug, Clone)]
pub struct ServiceDiscoveryResult {
    pub services: Vec<RegisteredService>,
    pub total_count: u32,
    pub query_time_ms: u32,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: RwLock::new(BTreeMap::new()),
            name_to_id: RwLock::new(BTreeMap::new()),
            capabilities: RwLock::new(BTreeMap::new()),
            dependencies: RwLock::new(BTreeMap::new()),
        }
    }
    
    /// Register a new service
    pub fn register(
        &self,
        service_id: u32,
        info: ServiceInfo,
        config: ServiceConfig,
    ) -> Result<(), ServiceError> {
        // Check if service name is already registered
        {
            let name_to_id = self.name_to_id.read();
            if name_to_id.contains_key(&info.name) {
                return Err(ServiceError::ServiceAlreadyExists);
            }
        }
        
        // Create registered service entry
        let registered_service = RegisteredService {
            id: service_id,
            info: info.clone(),
            config: config.clone(),
            registration_time: crate::time::get_timestamp(),
            last_update: crate::time::get_timestamp(),
            version: info.version.clone(),
            metadata: ServiceMetadata {
                author: "Unknown".to_string(),
                license: "Unknown".to_string(),
                homepage: None,
                documentation: None,
                repository: None,
                keywords: Vec::new(),
                categories: vec![ServiceCategory::System], // Default category
                minimum_os_version: "0.1.0".to_string(),
                supported_architectures: vec!["x86_64".to_string()],
            },
        };
        
        // Register service
        {
            let mut services = self.services.write();
            services.insert(service_id, registered_service);
        }
        
        // Register name mapping
        {
            let mut name_to_id = self.name_to_id.write();
            name_to_id.insert(info.name.clone(), service_id);
        }
        
        // Register capabilities
        {
            let mut capabilities = self.capabilities.write();
            for capability in &info.capabilities {
                capabilities.entry(capability.clone())
                    .or_insert_with(Vec::new)
                    .push(service_id);
            }
        }
        
        // Register dependencies
        {
            let mut dependencies = self.dependencies.write();
            dependencies.insert(service_id, config.dependencies.clone());
        }
        
        Ok(())
    }
    
    /// Unregister a service
    pub fn unregister(&self, service_id: u32) -> Result<(), ServiceError> {
        // Get service info before removal
        let service_info = {
            let services = self.services.read();
            services.get(&service_id)
                .map(|s| s.info.clone())
                .ok_or(ServiceError::ServiceNotFound)?
        };
        
        // Remove from services
        {
            let mut services = self.services.write();
            services.remove(&service_id);
        }
        
        // Remove name mapping
        {
            let mut name_to_id = self.name_to_id.write();
            name_to_id.remove(&service_info.name);
        }
        
        // Remove from capabilities
        {
            let mut capabilities = self.capabilities.write();
            for capability in &service_info.capabilities {
                if let Some(service_list) = capabilities.get_mut(capability) {
                    service_list.retain(|&id| id != service_id);
                    if service_list.is_empty() {
                        capabilities.remove(capability);
                    }
                }
            }
        }
        
        // Remove dependencies
        {
            let mut dependencies = self.dependencies.write();
            dependencies.remove(&service_id);
        }
        
        Ok(())
    }
    
    /// Find service by name
    pub fn find_by_name(&self, name: &str) -> Option<RegisteredService> {
        let name_to_id = self.name_to_id.read();
        let service_id = name_to_id.get(name)?;
        
        let services = self.services.read();
        services.get(service_id).cloned()
    }
    
    /// Find service by ID
    pub fn find_by_id(&self, service_id: u32) -> Option<RegisteredService> {
        let services = self.services.read();
        services.get(&service_id).cloned()
    }
    
    /// Find services by capability
    pub fn find_by_capability(&self, capability: &str) -> Vec<RegisteredService> {
        let capabilities = self.capabilities.read();
        let service_ids = capabilities.get(capability)
            .map(|ids| ids.clone())
            .unwrap_or_default();
        
        let services = self.services.read();
        service_ids.iter()
            .filter_map(|&id| services.get(&id).cloned())
            .collect()
    }
    
    /// Discover services based on query
    pub fn discover(&self, query: ServiceQuery) -> ServiceDiscoveryResult {
        let start_time = crate::time::get_timestamp();
        let services = self.services.read();
        
        let mut results: Vec<RegisteredService> = services.values()
            .filter(|service| self.matches_query(service, &query))
            .cloned()
            .collect();
        
        // Sort results by relevance (name match first, then capabilities)
        results.sort_by(|a, b| {
            if let Some(ref name) = query.name {
                let a_exact = a.info.name == *name;
                let b_exact = b.info.name == *name;
                match (a_exact, b_exact) {
                    (true, false) => core::cmp::Ordering::Less,
                    (false, true) => core::cmp::Ordering::Greater,
                    _ => a.info.name.cmp(&b.info.name),
                }
            } else {
                a.info.name.cmp(&b.info.name)
            }
        });
        
        let end_time = crate::time::get_timestamp();
        let query_time_ms = ((end_time - start_time) / 1000) as u32; // Convert to ms
        
        ServiceDiscoveryResult {
            total_count: results.len() as u32,
            services: results,
            query_time_ms,
        }
    }
    
    /// Check if a service matches the query criteria
    fn matches_query(&self, service: &RegisteredService, query: &ServiceQuery) -> bool {
        // Check name
        if let Some(ref name) = query.name {
            if !service.info.name.contains(name) {
                return false;
            }
        }
        
        // Check capability
        if let Some(ref capability) = query.capability {
            if !service.info.capabilities.contains(capability) {
                return false;
            }
        }
        
        // Check category
        if let Some(category) = query.category {
            if !service.metadata.categories.contains(&category) {
                return false;
            }
        }
        
        // Check version requirement
        if let Some(ref version_req) = query.version_requirement {
            if !self.version_matches(&service.version, version_req) {
                return false;
            }
        }
        
        // Check keywords
        if !query.keywords.is_empty() {
            let service_keywords = &service.metadata.keywords;
            let has_keyword = query.keywords.iter()
                .any(|keyword| {
                    service_keywords.iter().any(|sk| sk.contains(keyword)) ||
                    service.info.name.contains(keyword) ||
                    service.info.description.contains(keyword)
                });
            if !has_keyword {
                return false;
            }
        }
        
        true
    }
    
    /// Check if version matches requirement
    fn version_matches(&self, version: &str, requirement: &VersionRequirement) -> bool {
        // Simplified version comparison - in a real implementation,
        // this would use proper semantic versioning
        match requirement.operator {
            VersionOperator::Exact => version == requirement.version,
            VersionOperator::GreaterThan => version > &requirement.version,
            VersionOperator::GreaterThanOrEqual => version >= &requirement.version,
            VersionOperator::LessThan => version < &requirement.version,
            VersionOperator::LessThanOrEqual => version <= &requirement.version,
            VersionOperator::Compatible => {
                // Simplified compatible check - same major version
                let version_parts: Vec<&str> = version.split('.').collect();
                let req_parts: Vec<&str> = requirement.version.split('.').collect();
                version_parts.get(0) == req_parts.get(0)
            }
        }
    }
    
    /// List all registered services
    pub fn list_all(&self) -> Vec<RegisteredService> {
        let services = self.services.read();
        services.values().cloned().collect()
    }
    
    /// Get service dependencies
    pub fn get_dependencies(&self, service_id: u32) -> Vec<String> {
        let dependencies = self.dependencies.read();
        dependencies.get(&service_id)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Check if all dependencies are satisfied
    pub fn check_dependencies(&self, service_id: u32) -> Result<(), Vec<String>> {
        let deps = self.get_dependencies(service_id);
        let mut missing = Vec::new();
        
        let name_to_id = self.name_to_id.read();
        for dep in deps {
            if !name_to_id.contains_key(&dep) {
                missing.push(dep);
            }
        }
        
        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
    
    /// Get services that depend on a given service
    pub fn get_dependents(&self, service_name: &str) -> Vec<u32> {
        let dependencies = self.dependencies.read();
        dependencies.iter()
            .filter_map(|(&service_id, deps)| {
                if deps.contains(&service_name.to_string()) {
                    Some(service_id)
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Update service metadata
    pub fn update_metadata(
        &self,
        service_id: u32,
        metadata: ServiceMetadata,
    ) -> Result<(), ServiceError> {
        let mut services = self.services.write();
        let service = services.get_mut(&service_id)
            .ok_or(ServiceError::ServiceNotFound)?;
        
        service.metadata = metadata;
        service.last_update = crate::time::get_timestamp();
        
        Ok(())
    }
    
    /// Get registry statistics
    pub fn get_statistics(&self) -> RegistryStatistics {
        let services = self.services.read();
        let capabilities = self.capabilities.read();
        
        let total_services = services.len() as u32;
        let total_capabilities = capabilities.len() as u32;
        
        let categories_count = services.values()
            .flat_map(|s| &s.metadata.categories)
            .fold(BTreeMap::new(), |mut acc, &cat| {
                *acc.entry(cat).or_insert(0) += 1;
                acc
            });
        
        RegistryStatistics {
            total_services,
            total_capabilities,
            categories_count,
            average_dependencies: if total_services > 0 {
                self.dependencies.read().values()
                    .map(|deps| deps.len())
                    .sum::<usize>() as f32 / total_services as f32
            } else {
                0.0
            },
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStatistics {
    pub total_services: u32,
    pub total_capabilities: u32,
    pub categories_count: BTreeMap<ServiceCategory, u32>,
    pub average_dependencies: f32,
}

/// Extended service error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    ServiceNotFound,
    ServiceAlreadyExists,
    InvalidVersion,
    DependencyNotMet,
    CircularDependency,
    InternalError,
}

impl From<RegistryError> for ServiceError {
    fn from(err: RegistryError) -> Self {
        match err {
            RegistryError::ServiceNotFound => ServiceError::ServiceNotFound,
            RegistryError::ServiceAlreadyExists => ServiceError::ServiceAlreadyExists,
            RegistryError::DependencyNotMet => ServiceError::DependencyNotMet,
            _ => ServiceError::InternalError,
        }
    }
}

/// Helper functions for common registry operations
pub fn create_service_query() -> ServiceQuery {
    ServiceQuery {
        name: None,
        capability: None,
        category: None,
        version_requirement: None,
        status: None,
        keywords: Vec::new(),
    }
}

pub fn create_version_requirement(
    operator: VersionOperator,
    version: String,
) -> VersionRequirement {
    VersionRequirement { operator, version }
}