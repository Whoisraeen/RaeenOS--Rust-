//! USDT-style Tracepoints for Dynamic Instrumentation
//!
//! This module provides a tracepoint system similar to USDT (Userland Statically Defined Tracing)
//! that allows dynamic enabling/disabling of instrumentation points throughout the kernel.

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use spin::RwLock;
use super::{ObservabilityError, Subsystem};

/// Maximum number of tracepoints
const MAX_TRACEPOINTS: usize = 4096;

/// Maximum size of tracepoint data
const MAX_TRACEPOINT_DATA_SIZE: usize = 1024;

/// Tracepoint definition
#[derive(Debug)]
pub struct TracepointDefinition {
    pub id: u32,
    pub name: String,
    pub subsystem: Subsystem,
    pub description: String,
    pub enabled: AtomicU8, // 0 = disabled, 1 = enabled
    pub hit_count: AtomicU64,
    pub last_hit_timestamp: AtomicU64,
    pub probe_functions: Vec<TracepointProbe>,
}

/// Tracepoint probe function
#[derive(Debug, Clone)]
pub struct TracepointProbe {
    pub id: u32,
    pub name: String,
    pub handler: TracepointHandler,
    pub enabled: bool,
}

/// Tracepoint handler function type
pub type TracepointHandler = fn(&TracepointEvent);

/// Tracepoint event data
#[derive(Debug, Clone)]
pub struct TracepointEvent {
    pub tracepoint_id: u32,
    pub timestamp_ns: u64,
    pub thread_id: u32,
    pub cpu_id: u8,
    pub data: Vec<u8>,
    pub args: [u64; 8], // Up to 8 arguments
    pub arg_count: u8,
}

/// Tracepoint registry
pub struct TracepointRegistry {
    tracepoints: RwLock<BTreeMap<u32, TracepointDefinition>>,
    name_to_id: RwLock<BTreeMap<String, u32>>,
    next_id: AtomicU64,
    next_probe_id: AtomicU64,
    global_enabled: AtomicU8,
    stats: RwLock<TracepointStats>,
}

/// Tracepoint statistics
#[derive(Debug, Clone, Default)]
pub struct TracepointStats {
    pub total_tracepoints: u32,
    pub enabled_tracepoints: u32,
    pub total_hits: u64,
    pub total_probes: u32,
    pub enabled_probes: u32,
    pub events_dropped: u64,
}

/// Tracepoint filter criteria
#[derive(Debug, Clone)]
pub struct TracepointFilter {
    pub subsystem: Option<Subsystem>,
    pub name_pattern: Option<String>,
    pub enabled_only: bool,
    pub min_hit_count: Option<u64>,
}

impl TracepointRegistry {
    /// Create a new tracepoint registry
    pub fn new() -> Self {
        Self {
            tracepoints: RwLock::new(BTreeMap::new()),
            name_to_id: RwLock::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            next_probe_id: AtomicU64::new(1),
            global_enabled: AtomicU8::new(1), // Enabled by default
            stats: RwLock::new(TracepointStats::default()),
        }
    }

    /// Register a new tracepoint
    pub fn register_tracepoint(
        &self,
        name: &str,
        subsystem: Subsystem,
        description: &str,
    ) -> Result<u32, ObservabilityError> {
        let mut tracepoints = self.tracepoints.write();
        let mut name_to_id = self.name_to_id.write();
        
        // Check if tracepoint already exists
        if name_to_id.contains_key(name) {
            return Err(ObservabilityError::InvalidTracepoint);
        }
        
        // Check capacity
        if tracepoints.len() >= MAX_TRACEPOINTS {
            return Err(ObservabilityError::StorageFull);
        }
        
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as u32;
        
        let tracepoint = TracepointDefinition {
            id,
            name: name.to_string(),
            subsystem,
            description: description.to_string(),
            enabled: AtomicU8::new(0), // Disabled by default
            hit_count: AtomicU64::new(0),
            last_hit_timestamp: AtomicU64::new(0),
            probe_functions: Vec::new(),
        };
        
        tracepoints.insert(id, tracepoint);
        name_to_id.insert(name.to_string(), id);
        
        // Update stats
        self.stats.write().total_tracepoints += 1;
        
        Ok(id)
    }

    /// Enable a tracepoint by ID
    pub fn enable_tracepoint(&self, id: u32) -> Result<(), ObservabilityError> {
        let tracepoints = self.tracepoints.read();
        if let Some(tracepoint) = tracepoints.get(&id) {
            let was_enabled = tracepoint.enabled.swap(1, Ordering::SeqCst) != 0;
            if !was_enabled {
                self.stats.write().enabled_tracepoints += 1;
            }
            Ok(())
        } else {
            Err(ObservabilityError::InvalidTracepoint)
        }
    }

    /// Disable a tracepoint by ID
    pub fn disable_tracepoint(&self, id: u32) -> Result<(), ObservabilityError> {
        let tracepoints = self.tracepoints.read();
        if let Some(tracepoint) = tracepoints.get(&id) {
            let was_enabled = tracepoint.enabled.swap(0, Ordering::SeqCst) != 0;
            if was_enabled {
                self.stats.write().enabled_tracepoints -= 1;
            }
            Ok(())
        } else {
            Err(ObservabilityError::InvalidTracepoint)
        }
    }

    /// Enable a tracepoint by name
    pub fn enable_tracepoint_by_name(&self, name: &str) -> Result<(), ObservabilityError> {
        let name_to_id = self.name_to_id.read();
        if let Some(&id) = name_to_id.get(name) {
            drop(name_to_id);
            self.enable_tracepoint(id)
        } else {
            Err(ObservabilityError::InvalidTracepoint)
        }
    }

    /// Disable a tracepoint by name
    pub fn disable_tracepoint_by_name(&self, name: &str) -> Result<(), ObservabilityError> {
        let name_to_id = self.name_to_id.read();
        if let Some(&id) = name_to_id.get(name) {
            drop(name_to_id);
            self.disable_tracepoint(id)
        } else {
            Err(ObservabilityError::InvalidTracepoint)
        }
    }

    /// Enable all tracepoints for a subsystem
    pub fn enable_subsystem(&self, subsystem: Subsystem) -> Result<u32, ObservabilityError> {
        let tracepoints = self.tracepoints.read();
        let mut enabled_count = 0;
        
        for tracepoint in tracepoints.values() {
            if tracepoint.subsystem == subsystem {
                let was_enabled = tracepoint.enabled.swap(1, Ordering::SeqCst) != 0;
                if !was_enabled {
                    enabled_count += 1;
                }
            }
        }
        
        self.stats.write().enabled_tracepoints += enabled_count;
        Ok(enabled_count)
    }

    /// Disable all tracepoints for a subsystem
    pub fn disable_subsystem(&self, subsystem: Subsystem) -> Result<u32, ObservabilityError> {
        let tracepoints = self.tracepoints.read();
        let mut disabled_count = 0;
        
        for tracepoint in tracepoints.values() {
            if tracepoint.subsystem == subsystem {
                let was_enabled = tracepoint.enabled.swap(0, Ordering::SeqCst) != 0;
                if was_enabled {
                    disabled_count += 1;
                }
            }
        }
        
        self.stats.write().enabled_tracepoints -= disabled_count;
        Ok(disabled_count)
    }

    /// Fire a tracepoint
    pub fn fire_tracepoint(&self, id: u32, args: &[u64], data: &[u8]) {
        // Quick check if globally disabled
        if self.global_enabled.load(Ordering::Relaxed) == 0 {
            return;
        }
        
        let tracepoints = self.tracepoints.read();
        if let Some(tracepoint) = tracepoints.get(&id) {
            // Quick check if tracepoint is enabled
            if tracepoint.enabled.load(Ordering::Relaxed) == 0 {
                return;
            }
            
            // Update hit statistics
            tracepoint.hit_count.fetch_add(1, Ordering::Relaxed);
            tracepoint.last_hit_timestamp.store(
                crate::time::get_timestamp_ns(),
                Ordering::Relaxed
            );
            
            // Create event
            let mut event_args = [0u64; 8];
            let arg_count = core::cmp::min(args.len(), 8);
            event_args[..arg_count].copy_from_slice(&args[..arg_count]);
            
            let mut event_data = Vec::new();
            if data.len() <= MAX_TRACEPOINT_DATA_SIZE {
                event_data.extend_from_slice(data);
            } else {
                // Truncate data if too large
                event_data.extend_from_slice(&data[..MAX_TRACEPOINT_DATA_SIZE]);
                self.stats.write().events_dropped += 1;
            }
            
            let event = TracepointEvent {
                tracepoint_id: id,
                timestamp_ns: crate::time::get_timestamp_ns(),
                thread_id: self.get_current_thread_id(),
                cpu_id: self.get_current_cpu_id(),
                data: event_data,
                args: event_args,
                arg_count: arg_count as u8,
            };
            
            // Execute probe functions
            for probe in &tracepoint.probe_functions {
                if probe.enabled {
                    (probe.handler)(&event);
                }
            }
            
            // Record in flight recorder
            super::record_event(super::ObservabilityEvent::Tracepoint {
                name: tracepoint.name.clone(),
                subsystem: tracepoint.subsystem,
                data: event.data.clone(),
            });
            
            self.stats.write().total_hits += 1;
        }
    }

    /// Fire a tracepoint by name
    pub fn fire_tracepoint_by_name(&self, name: &str, args: &[u64], data: &[u8]) {
        let name_to_id = self.name_to_id.read();
        if let Some(&id) = name_to_id.get(name) {
            drop(name_to_id);
            self.fire_tracepoint(id, args, data);
        }
    }

    /// Add a probe function to a tracepoint
    pub fn add_probe(
        &self,
        tracepoint_id: u32,
        name: &str,
        handler: TracepointHandler,
    ) -> Result<u32, ObservabilityError> {
        let mut tracepoints = self.tracepoints.write();
        if let Some(tracepoint) = tracepoints.get_mut(&tracepoint_id) {
            let probe_id = self.next_probe_id.fetch_add(1, Ordering::SeqCst) as u32;
            
            let probe = TracepointProbe {
                id: probe_id,
                name: name.to_string(),
                handler,
                enabled: true,
            };
            
            tracepoint.probe_functions.push(probe);
            self.stats.write().total_probes += 1;
            self.stats.write().enabled_probes += 1;
            
            Ok(probe_id)
        } else {
            Err(ObservabilityError::InvalidTracepoint)
        }
    }

    /// Remove a probe function
    pub fn remove_probe(
        &self,
        tracepoint_id: u32,
        probe_id: u32,
    ) -> Result<(), ObservabilityError> {
        let mut tracepoints = self.tracepoints.write();
        if let Some(tracepoint) = tracepoints.get_mut(&tracepoint_id) {
            if let Some(pos) = tracepoint.probe_functions.iter().position(|p| p.id == probe_id) {
                let probe = tracepoint.probe_functions.remove(pos);
                self.stats.write().total_probes -= 1;
                if probe.enabled {
                    self.stats.write().enabled_probes -= 1;
                }
                Ok(())
            } else {
                Err(ObservabilityError::InvalidTracepoint)
            }
        } else {
            Err(ObservabilityError::InvalidTracepoint)
        }
    }

    /// List tracepoints with optional filtering
    pub fn list_tracepoints(&self, filter: Option<TracepointFilter>) -> Vec<Tracepoint> {
        let tracepoints = self.tracepoints.read();
        let mut result = Vec::new();
        
        for tracepoint in tracepoints.values() {
            let enabled = tracepoint.enabled.load(Ordering::Relaxed) != 0;
            let hit_count = tracepoint.hit_count.load(Ordering::Relaxed);
            
            // Apply filter
            if let Some(ref f) = filter {
                if let Some(subsystem) = f.subsystem {
                    if tracepoint.subsystem != subsystem {
                        continue;
                    }
                }
                
                if f.enabled_only && !enabled {
                    continue;
                }
                
                if let Some(min_hits) = f.min_hit_count {
                    if hit_count < min_hits {
                        continue;
                    }
                }
                
                if let Some(ref pattern) = f.name_pattern {
                    if !tracepoint.name.contains(pattern) {
                        continue;
                    }
                }
            }
            
            result.push(Tracepoint {
                id: tracepoint.id,
                name: tracepoint.name.clone(),
                subsystem: tracepoint.subsystem,
                description: tracepoint.description.clone(),
                enabled,
                hit_count,
                last_hit_timestamp: tracepoint.last_hit_timestamp.load(Ordering::Relaxed),
                probe_count: tracepoint.probe_functions.len() as u32,
            });
        }
        
        result
    }

    /// Get tracepoint statistics
    pub fn get_stats(&self) -> TracepointStats {
        self.stats.read().clone()
    }

    /// Enable/disable all tracepoints globally
    pub fn set_global_enabled(&self, enabled: bool) {
        self.global_enabled.store(if enabled { 1 } else { 0 }, Ordering::SeqCst);
    }

    /// Check if tracepoints are globally enabled
    pub fn is_global_enabled(&self) -> bool {
        self.global_enabled.load(Ordering::Relaxed) != 0
    }

    // Helper methods
    
    fn get_current_thread_id(&self) -> u32 {
        // TODO: Get actual thread ID from scheduler
        0
    }

    fn get_current_cpu_id(&self) -> u8 {
        // TODO: Get actual CPU ID
        0
    }
}

/// Tracepoint information for listing
#[derive(Debug)]
pub struct Tracepoint {
    pub id: u32,
    pub name: String,
    pub subsystem: Subsystem,
    pub description: String,
    pub enabled: bool,
    pub hit_count: u64,
    pub last_hit_timestamp: u64,
    pub probe_count: u32,
}

/// Macro for defining tracepoints
#[macro_export]
macro_rules! define_tracepoint {
    ($name:expr, $subsystem:expr, $description:expr) => {
        {
            static TRACEPOINT_ID: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);
            
            let id = TRACEPOINT_ID.load(core::sync::atomic::Ordering::Relaxed);
            if id == 0 {
                if let Ok(new_id) = $crate::observability::with_observability(|obs| {
                    obs.tracepoints.register_tracepoint($name, $subsystem, $description)
                }) {
                    if let Ok(registered_id) = new_id {
                        TRACEPOINT_ID.store(registered_id, core::sync::atomic::Ordering::SeqCst);
                        registered_id
                    } else {
                        0
                    }
                } else {
                    0
                }
            } else {
                id
            }
        }
    };
}

/// Macro for firing tracepoints
#[macro_export]
macro_rules! fire_tracepoint {
    ($id:expr, $($arg:expr),*) => {
        {
            let args = [$($arg as u64),*];
            let data: &[u8] = &[];
            let _ = $crate::observability::with_observability(|obs| {
                obs.tracepoints.fire_tracepoint($id, &args, data);
            });
        }
    };
    ($id:expr, $data:expr, $($arg:expr),*) => {
        {
            let args = [$($arg as u64),*];
            let _ = $crate::observability::with_observability(|obs| {
                obs.tracepoints.fire_tracepoint($id, &args, $data);
            });
        }
    };
}

/// Macro for conditional tracepoint firing (only if enabled)
#[macro_export]
macro_rules! trace_if_enabled {
    ($name:expr, $subsystem:expr, $($arg:expr),*) => {
        {
            let _ = $crate::observability::with_observability(|obs| {
                let args = [$($arg as u64),*];
                let data: &[u8] = &[];
                obs.tracepoints.fire_tracepoint_by_name($name, &args, data);
            });
        }
    };
}