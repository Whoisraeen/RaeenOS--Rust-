//! Watchdog Manager - Per-subsystem monitoring and micro-restarts
//!
//! This module provides watchdog functionality for monitoring subsystem health
//! and performing automatic recovery actions including micro-restarts.

use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use spin::RwLock;
use super::{Subsystem, WatchdogAction};

/// Maximum number of watchdogs
const MAX_WATCHDOGS: usize = 256;

/// Default watchdog timeout in milliseconds
const DEFAULT_TIMEOUT_MS: u32 = 30000; // 30 seconds

/// Watchdog configuration
#[derive(Debug, Clone)]
pub struct WatchdogConfig {
    pub timeout_ms: u32,
    pub max_failures: u32,
    pub escalation_policy: EscalationPolicy,
    pub auto_restart: bool,
    pub restart_delay_ms: u32,
    pub max_restarts_per_hour: u32,
    pub health_check_interval_ms: u32,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_failures: 3,
            escalation_policy: EscalationPolicy::RestartThenPanic,
            auto_restart: true,
            restart_delay_ms: 1000, // 1 second
            max_restarts_per_hour: 10,
            health_check_interval_ms: 5000, // 5 seconds
        }
    }
}

/// Escalation policy for watchdog failures
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationPolicy {
    /// Just log warnings
    WarnOnly,
    /// Restart the subsystem
    Restart,
    /// Restart then panic if restart fails
    RestartThenPanic,
    /// Immediate panic
    Panic,
    /// Custom handler
    Custom,
}

/// Watchdog state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogState {
    Inactive,
    Active,
    Triggered,
    Recovering,
    Failed,
}

/// Watchdog instance
#[derive(Debug)]
pub struct Watchdog {
    pub id: u32,
    pub subsystem: Subsystem,
    pub name: String,
    pub config: WatchdogConfig,
    pub state: AtomicU8, // WatchdogState as u8
    pub last_heartbeat: AtomicU64,
    pub failure_count: AtomicU64,
    pub restart_count: AtomicU64,
    pub last_restart_time: AtomicU64,
    pub restart_handler: Option<RestartHandler>,
    pub health_check_handler: Option<HealthCheckHandler>,
    pub custom_escalation_handler: Option<EscalationHandler>,
}

/// Restart handler function type
pub type RestartHandler = fn(subsystem: Subsystem) -> Result<(), WatchdogError>;

/// Health check handler function type
pub type HealthCheckHandler = fn(subsystem: Subsystem) -> Result<bool, WatchdogError>;

/// Custom escalation handler function type
pub type EscalationHandler = fn(subsystem: Subsystem, failure_count: u64) -> WatchdogAction;

/// Watchdog error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogError {
    SubsystemNotFound,
    RestartFailed,
    HealthCheckFailed,
    TooManyRestarts,
    InvalidConfiguration,
    AlreadyExists,
}

/// Watchdog statistics
#[derive(Debug, Clone, Default)]
pub struct WatchdogStats {
    pub total_watchdogs: u32,
    pub active_watchdogs: u32,
    pub total_timeouts: u64,
    pub total_restarts: u64,
    pub total_panics: u64,
    pub average_heartbeat_interval_ms: u32,
    pub last_timeout_timestamp: u64,
}

/// Watchdog manager
pub struct WatchdogManager {
    watchdogs: RwLock<BTreeMap<u32, Watchdog>>,
    subsystem_to_id: RwLock<BTreeMap<Subsystem, u32>>,
    next_id: AtomicU64,
    global_enabled: AtomicU8,
    stats: RwLock<WatchdogStats>,
    monitor_thread_active: AtomicU8,
}

impl WatchdogManager {
    /// Create a new watchdog manager
    pub fn new() -> Self {
        Self {
            watchdogs: RwLock::new(BTreeMap::new()),
            subsystem_to_id: RwLock::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            global_enabled: AtomicU8::new(1),
            stats: RwLock::new(WatchdogStats::default()),
            monitor_thread_active: AtomicU8::new(0),
        }
    }

    /// Register a new watchdog for a subsystem
    pub fn register_watchdog(
        &self,
        subsystem: Subsystem,
        name: &str,
        config: WatchdogConfig,
    ) -> Result<u32, WatchdogError> {
        let mut watchdogs = self.watchdogs.write();
        let mut subsystem_to_id = self.subsystem_to_id.write();
        
        // Check if watchdog already exists for this subsystem
        if subsystem_to_id.contains_key(&subsystem) {
            return Err(WatchdogError::AlreadyExists);
        }
        
        // Check capacity
        if watchdogs.len() >= MAX_WATCHDOGS {
            return Err(WatchdogError::InvalidConfiguration);
        }
        
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as u32;
        
        let watchdog = Watchdog {
            id,
            subsystem,
            name: name.to_string(),
            config,
            state: AtomicU8::new(WatchdogState::Inactive as u8),
            last_heartbeat: AtomicU64::new(crate::time::get_timestamp()),
            failure_count: AtomicU64::new(0),
            restart_count: AtomicU64::new(0),
            last_restart_time: AtomicU64::new(0),
            restart_handler: None,
            health_check_handler: None,
            custom_escalation_handler: None,
        };
        
        watchdogs.insert(id, watchdog);
        subsystem_to_id.insert(subsystem, id);
        
        // Update stats
        self.stats.write().total_watchdogs += 1;
        
        Ok(id)
    }

    /// Set restart handler for a watchdog
    pub fn set_restart_handler(
        &self,
        subsystem: Subsystem,
        handler: RestartHandler,
    ) -> Result<(), WatchdogError> {
        let subsystem_to_id = self.subsystem_to_id.read();
        if let Some(&id) = subsystem_to_id.get(&subsystem) {
            drop(subsystem_to_id);
            let mut watchdogs = self.watchdogs.write();
            if let Some(watchdog) = watchdogs.get_mut(&id) {
                watchdog.restart_handler = Some(handler);
                Ok(())
            } else {
                Err(WatchdogError::SubsystemNotFound)
            }
        } else {
            Err(WatchdogError::SubsystemNotFound)
        }
    }

    /// Set health check handler for a watchdog
    pub fn set_health_check_handler(
        &self,
        subsystem: Subsystem,
        handler: HealthCheckHandler,
    ) -> Result<(), WatchdogError> {
        let subsystem_to_id = self.subsystem_to_id.read();
        if let Some(&id) = subsystem_to_id.get(&subsystem) {
            drop(subsystem_to_id);
            let mut watchdogs = self.watchdogs.write();
            if let Some(watchdog) = watchdogs.get_mut(&id) {
                watchdog.health_check_handler = Some(handler);
                Ok(())
            } else {
                Err(WatchdogError::SubsystemNotFound)
            }
        } else {
            Err(WatchdogError::SubsystemNotFound)
        }
    }

    /// Start monitoring a subsystem
    pub fn start_watchdog(&self, subsystem: Subsystem) -> Result<(), WatchdogError> {
        let subsystem_to_id = self.subsystem_to_id.read();
        if let Some(&id) = subsystem_to_id.get(&subsystem) {
            drop(subsystem_to_id);
            let watchdogs = self.watchdogs.read();
            if let Some(watchdog) = watchdogs.get(&id) {
                watchdog.state.store(WatchdogState::Active as u8, Ordering::SeqCst);
                watchdog.last_heartbeat.store(crate::time::get_timestamp(), Ordering::SeqCst);
                
                // Update stats
                self.stats.write().active_watchdogs += 1;
                
                // Start monitor thread if not already active
                self.ensure_monitor_thread_active();
                
                Ok(())
            } else {
                Err(WatchdogError::SubsystemNotFound)
            }
        } else {
            Err(WatchdogError::SubsystemNotFound)
        }
    }

    /// Stop monitoring a subsystem
    pub fn stop_watchdog(&self, subsystem: Subsystem) -> Result<(), WatchdogError> {
        let subsystem_to_id = self.subsystem_to_id.read();
        if let Some(&id) = subsystem_to_id.get(&subsystem) {
            drop(subsystem_to_id);
            let watchdogs = self.watchdogs.read();
            if let Some(watchdog) = watchdogs.get(&id) {
                let was_active = watchdog.state.swap(WatchdogState::Inactive as u8, Ordering::SeqCst) == WatchdogState::Active as u8;
                
                if was_active {
                    self.stats.write().active_watchdogs -= 1;
                }
                
                Ok(())
            } else {
                Err(WatchdogError::SubsystemNotFound)
            }
        } else {
            Err(WatchdogError::SubsystemNotFound)
        }
    }

    /// Send heartbeat for a subsystem
    pub fn heartbeat(&self, subsystem: Subsystem) -> Result<(), WatchdogError> {
        let subsystem_to_id = self.subsystem_to_id.read();
        if let Some(&id) = subsystem_to_id.get(&subsystem) {
            drop(subsystem_to_id);
            let watchdogs = self.watchdogs.read();
            if let Some(watchdog) = watchdogs.get(&id) {
                watchdog.last_heartbeat.store(crate::time::get_timestamp(), Ordering::SeqCst);
                
                // Reset state to active if it was triggered
                let current_state = watchdog.state.load(Ordering::Relaxed);
                if current_state == WatchdogState::Triggered as u8 {
                    watchdog.state.store(WatchdogState::Active as u8, Ordering::SeqCst);
                }
                
                Ok(())
            } else {
                Err(WatchdogError::SubsystemNotFound)
            }
        } else {
            Err(WatchdogError::SubsystemNotFound)
        }
    }

    /// Check all watchdogs for timeouts
    pub fn check_watchdogs(&self) {
        if self.global_enabled.load(Ordering::Relaxed) == 0 {
            return;
        }
        
        let current_time = crate::time::get_timestamp();
        let watchdogs = self.watchdogs.read();
        
        for watchdog in watchdogs.values() {
            let state = watchdog.state.load(Ordering::Relaxed);
            if state != WatchdogState::Active as u8 {
                continue;
            }
            
            let last_heartbeat = watchdog.last_heartbeat.load(Ordering::Relaxed);
            let timeout_ms = watchdog.config.timeout_ms as u64;
            
            if current_time - last_heartbeat > timeout_ms {
                self.handle_watchdog_timeout(watchdog, current_time);
            } else if watchdog.config.health_check_interval_ms > 0 {
                // Perform periodic health check
                let last_check_time = last_heartbeat;
                let check_interval = watchdog.config.health_check_interval_ms as u64;
                
                if current_time - last_check_time > check_interval {
                    self.perform_health_check(watchdog);
                }
            }
        }
    }

    /// Handle watchdog timeout
    fn handle_watchdog_timeout(&self, watchdog: &Watchdog, current_time: u64) {
        // Mark as triggered
        watchdog.state.store(WatchdogState::Triggered as u8, Ordering::SeqCst);
        
        let failure_count = watchdog.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        
        // Update stats
        {
            let mut stats = self.stats.write();
            stats.total_timeouts += 1;
            stats.last_timeout_timestamp = current_time;
        }
        
        // Determine action based on escalation policy
        let action = match watchdog.config.escalation_policy {
            EscalationPolicy::WarnOnly => WatchdogAction::Warning,
            EscalationPolicy::Restart => {
                if failure_count <= watchdog.config.max_failures as u64 {
                    WatchdogAction::Restart
                } else {
                    WatchdogAction::Warning
                }
            },
            EscalationPolicy::RestartThenPanic => {
                if failure_count <= watchdog.config.max_failures as u64 {
                    WatchdogAction::Restart
                } else {
                    WatchdogAction::Panic
                }
            },
            EscalationPolicy::Panic => WatchdogAction::Panic,
            EscalationPolicy::Custom => {
                if let Some(handler) = watchdog.custom_escalation_handler {
                    handler(watchdog.subsystem, failure_count)
                } else {
                    WatchdogAction::Warning
                }
            },
        };
        
        // Record watchdog event
        super::record_event(super::ObservabilityEvent::Watchdog {
            subsystem: watchdog.subsystem,
            timeout_ms: watchdog.config.timeout_ms,
            action,
        });
        
        // Execute action
        match action {
            WatchdogAction::Warning => {
                // Just log the warning - already recorded above
            },
            WatchdogAction::Restart => {
                self.attempt_restart(watchdog);
            },
            WatchdogAction::Panic => {
                self.stats.write().total_panics += 1;
                crate::serial::_print(format_args!("[WATCHDOG] CRITICAL: Timeout for subsystem {:?} - system would panic in production\n", watchdog.subsystem));
                // In production, this would be a panic. For now, mark as failed.
                watchdog.state.store(WatchdogState::Failed as u8, Ordering::SeqCst);
            },
            WatchdogAction::Ignore => {
                // Do nothing
            },
        }
    }

    /// Attempt to restart a subsystem
    fn attempt_restart(&self, watchdog: &Watchdog) {
        let current_time = crate::time::get_timestamp();
        let last_restart = watchdog.last_restart_time.load(Ordering::Relaxed);
        
        // Check restart rate limiting
        let hour_ms = 3600000; // 1 hour in milliseconds
        if current_time - last_restart < hour_ms {
            let restart_count = watchdog.restart_count.load(Ordering::Relaxed);
            if restart_count >= watchdog.config.max_restarts_per_hour as u64 {
                // Too many restarts, mark as failed instead of panic
                self.stats.write().total_panics += 1;
                crate::serial::_print(format_args!("[WATCHDOG] CRITICAL: Too many restarts for subsystem {:?} - marking as failed\n", watchdog.subsystem));
                watchdog.state.store(WatchdogState::Failed as u8, Ordering::SeqCst);
                return;
            }
        }
        
        // Mark as recovering
        watchdog.state.store(WatchdogState::Recovering as u8, Ordering::SeqCst);
        
        // Attempt restart
        let restart_result = if let Some(handler) = watchdog.restart_handler {
            handler(watchdog.subsystem)
        } else {
            // Default restart behavior
            self.default_restart_handler(watchdog.subsystem)
        };
        
        match restart_result {
            Ok(()) => {
                // Restart successful
                watchdog.restart_count.fetch_add(1, Ordering::SeqCst);
                watchdog.last_restart_time.store(current_time, Ordering::SeqCst);
                watchdog.state.store(WatchdogState::Active as u8, Ordering::SeqCst);
                watchdog.last_heartbeat.store(current_time, Ordering::SeqCst);
                
                self.stats.write().total_restarts += 1;
            },
            Err(_) => {
                // Restart failed
                watchdog.state.store(WatchdogState::Failed as u8, Ordering::SeqCst);
                
                // Escalate based on policy
                if watchdog.config.escalation_policy == EscalationPolicy::RestartThenPanic {
                    self.stats.write().total_panics += 1;
                    crate::serial::_print(format_args!("[WATCHDOG] CRITICAL: Failed to restart subsystem {:?} - would panic in production\n", watchdog.subsystem));
                    // Keep as failed instead of panicking
                }
            },
        }
    }

    /// Perform health check for a watchdog
    fn perform_health_check(&self, watchdog: &Watchdog) {
        if let Some(handler) = watchdog.health_check_handler {
            match handler(watchdog.subsystem) {
                Ok(healthy) => {
                    if healthy {
                        // Update heartbeat on successful health check
                        watchdog.last_heartbeat.store(crate::time::get_timestamp(), Ordering::SeqCst);
                    } else {
                        // Health check failed, treat as timeout
                        self.handle_watchdog_timeout(watchdog, crate::time::get_timestamp());
                    }
                },
                Err(_) => {
                    // Health check error, treat as timeout
                    self.handle_watchdog_timeout(watchdog, crate::time::get_timestamp());
                },
            }
        }
    }

    /// Default restart handler
    fn default_restart_handler(&self, subsystem: Subsystem) -> Result<(), WatchdogError> {
        // Default implementation - would need to be customized per subsystem
        match subsystem {
            Subsystem::ServiceManager => {
                // Restart service manager
                // TODO: Implement actual restart logic
                Ok(())
            },
            Subsystem::Network => {
                // Restart network stack
                // TODO: Implement actual restart logic
                Ok(())
            },
            Subsystem::Graphics => {
                // Restart graphics subsystem
                // TODO: Implement actual restart logic
                Ok(())
            },
            _ => {
                // Generic restart - just reset state
                Ok(())
            },
        }
    }

    /// Ensure monitor thread is active
    fn ensure_monitor_thread_active(&self) {
        if self.monitor_thread_active.swap(1, Ordering::SeqCst) == 0 {
            // Start monitor thread
            // TODO: Implement actual thread spawning
            // For now, this would be called periodically by the scheduler
        }
    }

    /// Get watchdog statistics
    pub fn get_stats(&self) -> WatchdogStats {
        self.stats.read().clone()
    }

    /// Get watchdog status for a subsystem
    pub fn get_watchdog_status(&self, subsystem: Subsystem) -> Option<WatchdogStatus> {
        let subsystem_to_id = self.subsystem_to_id.read();
        if let Some(&id) = subsystem_to_id.get(&subsystem) {
            drop(subsystem_to_id);
            let watchdogs = self.watchdogs.read();
            if let Some(watchdog) = watchdogs.get(&id) {
                Some(WatchdogStatus {
                    id: watchdog.id,
                    subsystem: watchdog.subsystem,
                    name: watchdog.name.clone(),
                    state: unsafe {
                        // SAFETY: This is unsafe because:
                        // - transmute converts between types with same memory representation
                        // - The atomic u8 value must represent a valid WatchdogState enum variant
                        // - WatchdogState enum must be repr(u8) with known discriminant values
                        // - The loaded value must be one of the defined enum variants (0-4)
                        // - This assumes the atomic was only written with valid enum values
                        core::mem::transmute(watchdog.state.load(Ordering::Relaxed))
                    },
                    last_heartbeat: watchdog.last_heartbeat.load(Ordering::Relaxed),
                    failure_count: watchdog.failure_count.load(Ordering::Relaxed),
                    restart_count: watchdog.restart_count.load(Ordering::Relaxed),
                    timeout_ms: watchdog.config.timeout_ms,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Enable/disable watchdog system globally
    pub fn set_global_enabled(&self, enabled: bool) {
        self.global_enabled.store(if enabled { 1 } else { 0 }, Ordering::SeqCst);
    }

    /// Check if watchdog system is globally enabled
    pub fn is_global_enabled(&self) -> bool {
        self.global_enabled.load(Ordering::Relaxed) != 0
    }
}

/// Watchdog status information
#[derive(Debug, Clone)]
pub struct WatchdogStatus {
    pub id: u32,
    pub subsystem: Subsystem,
    pub name: String,
    pub state: WatchdogState,
    pub last_heartbeat: u64,
    pub failure_count: u64,
    pub restart_count: u64,
    pub timeout_ms: u32,
}

/// Macro for easy watchdog heartbeat
#[macro_export]
macro_rules! watchdog_heartbeat {
    ($subsystem:expr) => {
        let _ = $crate::observability::with_observability(|obs| {
            let _ = obs.watchdog.heartbeat($subsystem);
        });
    };
}

/// Macro for registering a watchdog
#[macro_export]
macro_rules! register_watchdog {
    ($subsystem:expr, $name:expr) => {
        $crate::observability::with_observability_mut(|obs| {
            obs.watchdog.register_watchdog(
                $subsystem,
                $name,
                $crate::observability::watchdog::WatchdogConfig::default()
            )
        })
    };
    ($subsystem:expr, $name:expr, $config:expr) => {
        $crate::observability::with_observability_mut(|obs| {
            obs.watchdog.register_watchdog($subsystem, $name, $config)
        })
    };
}