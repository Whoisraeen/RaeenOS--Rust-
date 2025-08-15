//! Resource Manager for service resource allocation and limits
//! Manages CPU, memory, disk, and network resources for user-space services

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use spin::{Mutex, RwLock};
use super::contracts::*;
use super::ServiceError;

/// Resource manager for service resource allocation
pub struct ResourceManager {
    allocations: RwLock<BTreeMap<u32, ResourceAllocation>>,
    global_limits: RwLock<GlobalResourceLimits>,
    usage_tracking: Mutex<Vec<ResourceUsageSnapshot>>,
    policies: RwLock<BTreeMap<String, ResourcePolicy>>,
    statistics: RwLock<ResourceStatistics>,
}

/// Resource allocation for a service
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub service_id: u32,
    pub cpu_quota_percent: f32,
    pub memory_limit_mb: u32,
    pub disk_quota_mb: u32,
    pub network_bandwidth_kbps: u32,
    pub max_file_descriptors: u32,
    pub max_threads: u32,
    pub priority: ResourcePriority,
    pub cgroup_path: Option<String>,
    pub enforcement_mode: EnforcementMode,
}

/// Global resource limits
#[derive(Debug, Clone)]
pub struct GlobalResourceLimits {
    pub total_cpu_cores: u32,
    pub total_memory_mb: u32,
    pub total_disk_mb: u32,
    pub total_network_bandwidth_kbps: u32,
    pub reserved_cpu_percent: f32,
    pub reserved_memory_mb: u32,
    pub max_services: u32,
}

/// Resource usage snapshot
#[derive(Debug, Clone)]
pub struct ResourceUsageSnapshot {
    pub service_id: u32,
    pub timestamp: u64,
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u32,
    pub disk_io_kb_per_sec: u32,
    pub network_io_kb_per_sec: u32,
    pub file_descriptors_used: u32,
    pub threads_active: u32,
    pub page_faults: u64,
    pub context_switches: u64,
}

/// Resource policy
#[derive(Debug, Clone)]
pub struct ResourcePolicy {
    pub name: String,
    pub description: String,
    pub cpu_quota_percent: f32,
    pub memory_limit_mb: u32,
    pub disk_quota_mb: u32,
    pub network_bandwidth_kbps: u32,
    pub max_file_descriptors: u32,
    pub max_threads: u32,
    pub priority: ResourcePriority,
    pub enforcement_mode: EnforcementMode,
    pub oom_score_adj: i32,
}

/// Resource priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourcePriority {
    System = 0,     // System services
    High = 1,       // Critical user services
    Normal = 2,     // Regular user services
    Low = 3,        // Background services
    Idle = 4,       // Idle/cleanup services
}

/// Resource enforcement modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforcementMode {
    Strict,         // Hard limits, kill on violation
    Throttle,       // Throttle on violation
    Monitor,        // Monitor only, no enforcement
    Adaptive,       // Adaptive limits based on system load
}

/// Resource statistics
#[derive(Debug, Clone, Default)]
pub struct ResourceStatistics {
    pub total_allocations: u32,
    pub active_allocations: u32,
    pub total_cpu_allocated_percent: f32,
    pub total_memory_allocated_mb: u32,
    pub total_disk_allocated_mb: u32,
    pub total_network_allocated_kbps: u32,
    pub enforcement_violations: u64,
    pub oom_kills: u32,
    pub throttling_events: u64,
    pub average_cpu_utilization: f32,
    pub average_memory_utilization: f32,
}

/// Resource violation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceViolation {
    CpuQuotaExceeded,
    MemoryLimitExceeded,
    DiskQuotaExceeded,
    NetworkBandwidthExceeded,
    FileDescriptorLimitExceeded,
    ThreadLimitExceeded,
}

/// Resource violation event
#[derive(Debug, Clone)]
pub struct ResourceViolationEvent {
    pub service_id: u32,
    pub violation_type: ResourceViolation,
    pub timestamp: u64,
    pub current_usage: f32,
    pub limit: f32,
    pub action_taken: ViolationAction,
}

/// Actions taken on resource violations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationAction {
    None,
    Warning,
    Throttle,
    Kill,
    Restart,
}

/// Resource allocation request
#[derive(Debug, Clone)]
pub struct AllocationRequest {
    pub service_id: u32,
    pub policy_name: Option<String>,
    pub cpu_quota_percent: Option<f32>,
    pub memory_limit_mb: Option<u32>,
    pub disk_quota_mb: Option<u32>,
    pub network_bandwidth_kbps: Option<u32>,
    pub max_file_descriptors: Option<u32>,
    pub max_threads: Option<u32>,
    pub priority: Option<ResourcePriority>,
    pub enforcement_mode: Option<EnforcementMode>,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        let global_limits = GlobalResourceLimits {
            total_cpu_cores: 4, // TODO: Detect actual CPU cores
            total_memory_mb: 8192, // TODO: Detect actual memory
            total_disk_mb: 1024 * 1024, // 1TB
            total_network_bandwidth_kbps: 1000 * 1024, // 1Gbps
            reserved_cpu_percent: 20.0,
            reserved_memory_mb: 1024,
            max_services: 256,
        };
        
        let mut manager = Self {
            allocations: RwLock::new(BTreeMap::new()),
            global_limits: RwLock::new(global_limits),
            usage_tracking: Mutex::new(Vec::new()),
            policies: RwLock::new(BTreeMap::new()),
            statistics: RwLock::new(ResourceStatistics::default()),
        };
        
        // Initialize default policies
        manager.initialize_default_policies();
        
        manager
    }
    
    /// Initialize default resource policies
    fn initialize_default_policies(&self) {
        let policies = vec![
            ResourcePolicy {
                name: "system".into(),
                description: "System services policy".into(),
                cpu_quota_percent: 50.0,
                memory_limit_mb: 2048,
                disk_quota_mb: 10240,
                network_bandwidth_kbps: 100 * 1024,
                max_file_descriptors: 1024,
                max_threads: 64,
                priority: ResourcePriority::System,
                enforcement_mode: EnforcementMode::Strict,
                oom_score_adj: -1000,
            },
            ResourcePolicy {
                name: "high_priority".into(),
                description: "High priority services policy".into(),
                cpu_quota_percent: 30.0,
                memory_limit_mb: 1024,
                disk_quota_mb: 5120,
                network_bandwidth_kbps: 50 * 1024,
                max_file_descriptors: 512,
                max_threads: 32,
                priority: ResourcePriority::High,
                enforcement_mode: EnforcementMode::Throttle,
                oom_score_adj: -500,
            },
            ResourcePolicy {
                name: "normal".into(),
                description: "Normal services policy".into(),
                cpu_quota_percent: 20.0,
                memory_limit_mb: 512,
                disk_quota_mb: 2048,
                network_bandwidth_kbps: 25 * 1024,
                max_file_descriptors: 256,
                max_threads: 16,
                priority: ResourcePriority::Normal,
                enforcement_mode: EnforcementMode::Adaptive,
                oom_score_adj: 0,
            },
            ResourcePolicy {
                name: "background".into(),
                description: "Background services policy".into(),
                cpu_quota_percent: 10.0,
                memory_limit_mb: 256,
                disk_quota_mb: 1024,
                network_bandwidth_kbps: 10 * 1024,
                max_file_descriptors: 128,
                max_threads: 8,
                priority: ResourcePriority::Low,
                enforcement_mode: EnforcementMode::Monitor,
                oom_score_adj: 500,
            },
        ];
        
        let mut policy_map = self.policies.write();
        for policy in policies {
            policy_map.insert(policy.name.clone(), policy);
        }
    }
    
    /// Allocate resources for a service
    pub fn allocate_resources(
        &self,
        request: AllocationRequest,
    ) -> Result<ResourceAllocation, ServiceError> {
        // Check if service already has allocation
        {
            let allocations = self.allocations.read();
            if allocations.contains_key(&request.service_id) {
                return Err(ServiceError::ResourceAlreadyAllocated);
            }
        }
        
        // Get policy if specified
        let policy = if let Some(policy_name) = &request.policy_name {
            let policies = self.policies.read();
            policies.get(policy_name).cloned()
        } else {
            None
        };
        
        // Create allocation based on request and policy
        let allocation = self.create_allocation(request, policy)?;
        
        // Validate allocation against global limits
        self.validate_allocation(&allocation)?;
        
        // Apply the allocation
        self.apply_allocation(&allocation)?;
        
        // Store allocation
        {
            let mut allocations = self.allocations.write();
            allocations.insert(allocation.service_id, allocation.clone());
        }
        
        // Update statistics
        {
            let mut stats = self.statistics.write();
            stats.total_allocations += 1;
            stats.active_allocations += 1;
            stats.total_cpu_allocated_percent += allocation.cpu_quota_percent;
            stats.total_memory_allocated_mb += allocation.memory_limit_mb;
            stats.total_disk_allocated_mb += allocation.disk_quota_mb;
            stats.total_network_allocated_kbps += allocation.network_bandwidth_kbps;
        }
        
        Ok(allocation)
    }
    
    /// Create allocation from request and policy
    fn create_allocation(
        &self,
        request: AllocationRequest,
        policy: Option<ResourcePolicy>,
    ) -> Result<ResourceAllocation, ServiceError> {
        let default_policy = {
            let policies = self.policies.read();
            policies.get("normal").cloned()
                .ok_or(ServiceError::PolicyNotFound)?
        };
        
        let base_policy = policy.unwrap_or(default_policy);
        
        Ok(ResourceAllocation {
            service_id: request.service_id,
            cpu_quota_percent: request.cpu_quota_percent
                .unwrap_or(base_policy.cpu_quota_percent),
            memory_limit_mb: request.memory_limit_mb
                .unwrap_or(base_policy.memory_limit_mb),
            disk_quota_mb: request.disk_quota_mb
                .unwrap_or(base_policy.disk_quota_mb),
            network_bandwidth_kbps: request.network_bandwidth_kbps
                .unwrap_or(base_policy.network_bandwidth_kbps),
            max_file_descriptors: request.max_file_descriptors
                .unwrap_or(base_policy.max_file_descriptors),
            max_threads: request.max_threads
                .unwrap_or(base_policy.max_threads),
            priority: request.priority
                .unwrap_or(base_policy.priority),
            cgroup_path: None, // TODO: Generate cgroup path
            enforcement_mode: request.enforcement_mode
                .unwrap_or(base_policy.enforcement_mode),
        })
    }
    
    /// Validate allocation against global limits
    fn validate_allocation(&self, allocation: &ResourceAllocation) -> Result<(), ServiceError> {
        let global_limits = self.global_limits.read();
        let current_stats = self.statistics.read();
        
        // Check CPU allocation
        let available_cpu = (global_limits.total_cpu_cores as f32 * 100.0) - global_limits.reserved_cpu_percent;
        if current_stats.total_cpu_allocated_percent + allocation.cpu_quota_percent > available_cpu {
            return Err(ServiceError::InsufficientResources);
        }
        
        // Check memory allocation
        let available_memory = global_limits.total_memory_mb - global_limits.reserved_memory_mb;
        if current_stats.total_memory_allocated_mb + allocation.memory_limit_mb > available_memory {
            return Err(ServiceError::InsufficientResources);
        }
        
        // Check service count
        if current_stats.active_allocations >= global_limits.max_services {
            return Err(ServiceError::TooManyServices);
        }
        
        Ok(())
    }
    
    /// Apply allocation (create cgroups, set limits, etc.)
    fn apply_allocation(&self, allocation: &ResourceAllocation) -> Result<(), ServiceError> {
        // TODO: Implement actual resource enforcement
        // This would involve:
        // 1. Creating cgroups for the service
        // 2. Setting CPU, memory, and I/O limits
        // 3. Configuring network bandwidth limits
        // 4. Setting up monitoring
        
        Ok(())
    }
    
    /// Deallocate resources for a service
    pub fn deallocate_resources(&self, service_id: u32) -> Result<(), ServiceError> {
        // Get allocation
        let allocation = {
            let mut allocations = self.allocations.write();
            allocations.remove(&service_id)
                .ok_or(ServiceError::ResourceNotAllocated)?
        };
        
        // Remove enforcement (cleanup cgroups, etc.)
        self.remove_allocation(&allocation)?;
        
        // Update statistics
        {
            let mut stats = self.statistics.write();
            if stats.active_allocations > 0 {
                stats.active_allocations -= 1;
            }
            stats.total_cpu_allocated_percent -= allocation.cpu_quota_percent;
            stats.total_memory_allocated_mb -= allocation.memory_limit_mb;
            stats.total_disk_allocated_mb -= allocation.disk_quota_mb;
            stats.total_network_allocated_kbps -= allocation.network_bandwidth_kbps;
        }
        
        Ok(())
    }
    
    /// Remove allocation enforcement
    fn remove_allocation(&self, allocation: &ResourceAllocation) -> Result<(), ServiceError> {
        // TODO: Implement cleanup
        // This would involve:
        // 1. Removing cgroups
        // 2. Cleaning up monitoring
        // 3. Releasing network bandwidth reservations
        
        Ok(())
    }
    
    /// Update resource allocation for a service
    pub fn update_allocation(
        &self,
        service_id: u32,
        request: AllocationRequest,
    ) -> Result<ResourceAllocation, ServiceError> {
        // Get current allocation
        let current_allocation = {
            let allocations = self.allocations.read();
            allocations.get(&service_id)
                .ok_or(ServiceError::ResourceNotAllocated)?
                .clone()
        };
        
        // Create new allocation
        let mut new_allocation = current_allocation.clone();
        
        if let Some(cpu) = request.cpu_quota_percent {
            new_allocation.cpu_quota_percent = cpu;
        }
        if let Some(memory) = request.memory_limit_mb {
            new_allocation.memory_limit_mb = memory;
        }
        if let Some(disk) = request.disk_quota_mb {
            new_allocation.disk_quota_mb = disk;
        }
        if let Some(network) = request.network_bandwidth_kbps {
            new_allocation.network_bandwidth_kbps = network;
        }
        if let Some(fds) = request.max_file_descriptors {
            new_allocation.max_file_descriptors = fds;
        }
        if let Some(threads) = request.max_threads {
            new_allocation.max_threads = threads;
        }
        if let Some(priority) = request.priority {
            new_allocation.priority = priority;
        }
        if let Some(mode) = request.enforcement_mode {
            new_allocation.enforcement_mode = mode;
        }
        
        // Validate new allocation
        self.validate_allocation_update(&current_allocation, &new_allocation)?;
        
        // Apply changes
        self.apply_allocation(&new_allocation)?;
        
        // Update stored allocation
        {
            let mut allocations = self.allocations.write();
            allocations.insert(service_id, new_allocation.clone());
        }
        
        // Update statistics
        {
            let mut stats = self.statistics.write();
            stats.total_cpu_allocated_percent += 
                new_allocation.cpu_quota_percent - current_allocation.cpu_quota_percent;
            stats.total_memory_allocated_mb += 
                new_allocation.memory_limit_mb - current_allocation.memory_limit_mb;
            stats.total_disk_allocated_mb += 
                new_allocation.disk_quota_mb - current_allocation.disk_quota_mb;
            stats.total_network_allocated_kbps += 
                new_allocation.network_bandwidth_kbps - current_allocation.network_bandwidth_kbps;
        }
        
        Ok(new_allocation)
    }
    
    /// Validate allocation update
    fn validate_allocation_update(
        &self,
        current: &ResourceAllocation,
        new: &ResourceAllocation,
    ) -> Result<(), ServiceError> {
        let global_limits = self.global_limits.read();
        let current_stats = self.statistics.read();
        
        // Calculate deltas
        let cpu_delta = new.cpu_quota_percent - current.cpu_quota_percent;
        let memory_delta = new.memory_limit_mb as i64 - current.memory_limit_mb as i64;
        
        // Check if increases are within limits
        if cpu_delta > 0.0 {
            let available_cpu = (global_limits.total_cpu_cores as f32 * 100.0) - 
                global_limits.reserved_cpu_percent - current_stats.total_cpu_allocated_percent;
            if cpu_delta > available_cpu {
                return Err(ServiceError::InsufficientResources);
            }
        }
        
        if memory_delta > 0 {
            let available_memory = (global_limits.total_memory_mb - global_limits.reserved_memory_mb - 
                current_stats.total_memory_allocated_mb) as i64;
            if memory_delta > available_memory {
                return Err(ServiceError::InsufficientResources);
            }
        }
        
        Ok(())
    }
    
    /// Record resource usage
    pub fn record_usage(&self, usage: ResourceUsageSnapshot) {
        let mut tracking = self.usage_tracking.lock();
        tracking.push(usage);
        
        // Keep only recent snapshots (last 1000)
        if tracking.len() > 1000 {
            tracking.drain(0..tracking.len() - 1000);
        }
    }
    
    /// Check for resource violations
    pub fn check_violations(&self) -> Vec<ResourceViolationEvent> {
        let mut violations = Vec::new();
        let allocations = self.allocations.read();
        let tracking = self.usage_tracking.lock();
        
        // Get latest usage for each service
        let mut latest_usage = BTreeMap::new();
        for usage in tracking.iter().rev() {
            latest_usage.entry(usage.service_id).or_insert(usage);
        }
        
        // Check each allocation against its usage
        for (service_id, allocation) in allocations.iter() {
            if let Some(usage) = latest_usage.get(service_id) {
                violations.extend(self.check_service_violations(*service_id, allocation, usage));
            }
        }
        
        violations
    }
    
    /// Check violations for a specific service
    fn check_service_violations(
        &self,
        service_id: u32,
        allocation: &ResourceAllocation,
        usage: &ResourceUsageSnapshot,
    ) -> Vec<ResourceViolationEvent> {
        let mut violations = Vec::new();
        
        // Check CPU quota
        if usage.cpu_usage_percent > allocation.cpu_quota_percent {
            violations.push(ResourceViolationEvent {
                service_id,
                violation_type: ResourceViolation::CpuQuotaExceeded,
                timestamp: usage.timestamp,
                current_usage: usage.cpu_usage_percent,
                limit: allocation.cpu_quota_percent,
                action_taken: self.get_violation_action(allocation.enforcement_mode, ResourceViolation::CpuQuotaExceeded),
            });
        }
        
        // Check memory limit
        if usage.memory_usage_mb > allocation.memory_limit_mb {
            violations.push(ResourceViolationEvent {
                service_id,
                violation_type: ResourceViolation::MemoryLimitExceeded,
                timestamp: usage.timestamp,
                current_usage: usage.memory_usage_mb as f32,
                limit: allocation.memory_limit_mb as f32,
                action_taken: self.get_violation_action(allocation.enforcement_mode, ResourceViolation::MemoryLimitExceeded),
            });
        }
        
        // Check file descriptor limit
        if usage.file_descriptors_used > allocation.max_file_descriptors {
            violations.push(ResourceViolationEvent {
                service_id,
                violation_type: ResourceViolation::FileDescriptorLimitExceeded,
                timestamp: usage.timestamp,
                current_usage: usage.file_descriptors_used as f32,
                limit: allocation.max_file_descriptors as f32,
                action_taken: self.get_violation_action(allocation.enforcement_mode, ResourceViolation::FileDescriptorLimitExceeded),
            });
        }
        
        // Check thread limit
        if usage.threads_active > allocation.max_threads {
            violations.push(ResourceViolationEvent {
                service_id,
                violation_type: ResourceViolation::ThreadLimitExceeded,
                timestamp: usage.timestamp,
                current_usage: usage.threads_active as f32,
                limit: allocation.max_threads as f32,
                action_taken: self.get_violation_action(allocation.enforcement_mode, ResourceViolation::ThreadLimitExceeded),
            });
        }
        
        violations
    }
    
    /// Get violation action based on enforcement mode
    fn get_violation_action(
        &self,
        enforcement_mode: EnforcementMode,
        violation_type: ResourceViolation,
    ) -> ViolationAction {
        match enforcement_mode {
            EnforcementMode::Strict => {
                match violation_type {
                    ResourceViolation::MemoryLimitExceeded => ViolationAction::Kill,
                    _ => ViolationAction::Throttle,
                }
            }
            EnforcementMode::Throttle => ViolationAction::Throttle,
            EnforcementMode::Monitor => ViolationAction::Warning,
            EnforcementMode::Adaptive => ViolationAction::Throttle,
        }
    }
    
    /// Get resource allocation for a service
    pub fn get_allocation(&self, service_id: u32) -> Option<ResourceAllocation> {
        let allocations = self.allocations.read();
        allocations.get(&service_id).cloned()
    }
    
    /// Get resource statistics
    pub fn get_statistics(&self) -> ResourceStatistics {
        let stats = self.statistics.read();
        stats.clone()
    }
    
    /// Get global resource limits
    pub fn get_global_limits(&self) -> GlobalResourceLimits {
        let limits = self.global_limits.read();
        limits.clone()
    }
    
    /// Update global resource limits
    pub fn update_global_limits(&self, limits: GlobalResourceLimits) {
        let mut global_limits = self.global_limits.write();
        *global_limits = limits;
    }
    
    /// List all resource policies
    pub fn list_policies(&self) -> Vec<ResourcePolicy> {
        let policies = self.policies.read();
        policies.values().cloned().collect()
    }
    
    /// Add or update a resource policy
    pub fn update_policy(&self, policy: ResourcePolicy) {
        let mut policies = self.policies.write();
        policies.insert(policy.name.clone(), policy);
    }
    
    /// Remove a resource policy
    pub fn remove_policy(&self, name: &str) -> Result<(), ServiceError> {
        let mut policies = self.policies.write();
        policies.remove(name)
            .ok_or(ServiceError::PolicyNotFound)?;
        Ok(())
    }
    
    /// Get recent usage history for a service
    pub fn get_usage_history(&self, service_id: u32, limit: usize) -> Vec<ResourceUsageSnapshot> {
        let tracking = self.usage_tracking.lock();
        tracking.iter()
            .filter(|usage| usage.service_id == service_id)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
}

/// Helper functions for resource management
pub fn create_allocation_request(service_id: u32, policy_name: &str) -> AllocationRequest {
    AllocationRequest {
        service_id,
        policy_name: Some(policy_name.into()),
        cpu_quota_percent: None,
        memory_limit_mb: None,
        disk_quota_mb: None,
        network_bandwidth_kbps: None,
        max_file_descriptors: None,
        max_threads: None,
        priority: None,
        enforcement_mode: None,
    }
}

pub fn create_usage_snapshot(service_id: u32) -> ResourceUsageSnapshot {
    ResourceUsageSnapshot {
        service_id,
        timestamp: crate::time::get_timestamp(),
        cpu_usage_percent: 0.0,
        memory_usage_mb: 0,
        disk_io_kb_per_sec: 0,
        network_io_kb_per_sec: 0,
        file_descriptors_used: 0,
        threads_active: 0,
        page_faults: 0,
        context_switches: 0,
    }
}