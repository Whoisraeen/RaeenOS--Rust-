//! Observability infrastructure for RaeenOS
//!
//! This module provides comprehensive observability features including:
//! - Always-on flight recorder with bounded storage
//! - USDT-style tracepoints for dynamic instrumentation
//! - Crash-only services with micro-reboots
//! - Per-subsystem watchdogs
//! - Unified trace correlation across IPC boundaries

pub mod flight_recorder;
pub mod tracepoints;
pub mod watchdog;
pub mod crash_handler;
pub mod trace_correlation;

use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use spin::Mutex;

/// Global observability state
static OBSERVABILITY: Mutex<Option<ObservabilitySystem>> = Mutex::new(None);
static TRACE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Main observability system
pub struct ObservabilitySystem {
    pub flight_recorder: flight_recorder::FlightRecorder,
    pub tracepoints: tracepoints::TracepointRegistry,
    pub watchdog: watchdog::WatchdogManager,
    pub crash_handler: crash_handler::CrashHandler,
    pub trace_correlation: trace_correlation::TraceCorrelationManager,
    enabled: AtomicU8,
    next_trace_id: AtomicU64,
}

impl ObservabilitySystem {
    /// Initialize the observability system
    pub fn initialize(&mut self) -> Result<(), ObservabilityError> {
        // Initialize crash handler and install panic hook
        self.crash_handler.initialize();
        
        // Initialize flight recorder
        // Initialize tracepoint registry
        // Set up watchdog monitoring
        // Start monitoring threads
        
        Ok(())
    }

    /// Record an observability event
    pub fn record_event(&self, event: ObservabilityEvent) {
        if self.enabled.load(Ordering::Relaxed) == 0 {
            return;
        }
        
        self.flight_recorder.record_event(event);
    }

    /// Generate a new trace ID
    pub fn generate_trace_id(&self) -> u64 {
        self.next_trace_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Enable/disable observability
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(if enabled { 1 } else { 0 }, Ordering::SeqCst);
    }

    /// Check if observability is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed) != 0
    }

    /// Periodic maintenance - should be called regularly
    pub fn periodic_maintenance(&self) {
        // Check watchdogs
        self.watchdog.check_watchdogs();
        
        // Clean up expired traces
        self.trace_correlation.cleanup_expired_traces();
        
        // Perform flight recorder maintenance
        // (e.g., compress old entries, clean up buffers)
    }
}

/// Initialize the observability system
pub fn init_observability() -> Result<(), ObservabilityError> {
    let mut obs = OBSERVABILITY.lock();
    if obs.is_some() {
        return Err(ObservabilityError::AlreadyInitialized);
    }

    let system = ObservabilitySystem {
        flight_recorder: flight_recorder::FlightRecorder::new()?,
        tracepoints: tracepoints::TracepointRegistry::new(),
        watchdog: watchdog::WatchdogManager::new(),
        crash_handler: crash_handler::CrashHandler::new(),
        trace_correlation: trace_correlation::TraceCorrelationManager::new(),
        enabled: AtomicU8::new(1),
        next_trace_id: AtomicU64::new(1),
    };

    *obs = Some(system);
    Ok(())
}

/// Generate a new 128-bit trace ID
pub fn generate_trace_id() -> u128 {
    let high = TRACE_ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u128;
    let low = crate::time::get_timestamp() as u128;
    (high << 64) | low
}

/// Get reference to the observability system
pub fn with_observability<F, R>(f: F) -> Result<R, ObservabilityError>
where
    F: FnOnce(&ObservabilitySystem) -> R,
{
    let obs = OBSERVABILITY.lock();
    match obs.as_ref() {
        Some(system) => Ok(f(system)),
        None => Err(ObservabilityError::NotInitialized),
    }
}

/// Get mutable reference to the observability system
pub fn with_observability_mut<F, R>(f: F) -> Result<R, ObservabilityError>
where
    F: FnOnce(&mut ObservabilitySystem) -> R,
{
    let mut obs = OBSERVABILITY.lock();
    match obs.as_mut() {
        Some(system) => Ok(f(system)),
        None => Err(ObservabilityError::NotInitialized),
    }
}

/// Observability error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservabilityError {
    NotInitialized,
    AlreadyInitialized,
    BufferFull,
    InvalidTracepoint,
    WatchdogTimeout,
    CrashHandlerFailed,
    CorrelationFailed,
    StorageFull,
    InvalidConfiguration,
    ResourceExhausted,
    NotEnabled,
    TraceNotFound,
    SpanNotFound,
}

/// Severity levels for observability events
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

/// Subsystem identifiers for observability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Subsystem {
    Kernel,
    Memory,
    Scheduler,
    Filesystem,
    Network,
    Graphics,
    Audio,
    Input,
    Ipc,
    Power,
    Security,
    Storage,
    Usb,
    Pci,
    Acpi,
    Timer,
    Interrupt,
    Smp,
    Virtualization,
    Unknown,
    Ai,
    Compositor,
    PackageManager,
    ServiceManager,
}

/// Event types for the flight recorder
#[derive(Debug, Clone)]
pub enum ObservabilityEvent {
    /// System call entry/exit
    Syscall {
        syscall_id: u32,
        pid: u32,
        entry: bool,
        args: [u64; 6],
        result: Option<i64>,
    },
    /// IPC message send/receive
    Ipc {
        from_pid: u32,
        to_pid: u32,
        message_type: u32,
        size: u32,
        trace_id: u128,
    },
    /// Memory allocation/deallocation
    Memory {
        operation: MemoryOperation,
        address: u64,
        size: u64,
        pid: u32,
    },
    /// Context switch
    ContextSwitch {
        from_pid: u32,
        to_pid: u32,
        reason: ContextSwitchReason,
    },
    /// Interrupt handling
    Interrupt {
        vector: u8,
        duration_ns: u64,
        nested: bool,
    },
    /// Page fault
    PageFault {
        address: u64,
        error_code: u32,
        pid: u32,
        resolved: bool,
        duration_ns: u64,
    },
    /// Service lifecycle
    Service {
        service_id: u32,
        operation: ServiceOperation,
        result: ServiceResult,
    },
    /// Watchdog event
    Watchdog {
        subsystem: Subsystem,
        timeout_ms: u32,
        action: WatchdogAction,
    },
    /// Custom tracepoint
    Tracepoint {
        name: String,
        subsystem: Subsystem,
        data: Vec<u8>,
    },
    /// System boot event
    SystemBoot {
        timestamp: u64,
        boot_stage: String,
    },
    /// Process created
    ProcessCreated {
        pid: u32,
        name: String,
        parent_pid: Option<u32>,
    },
    /// Process terminated
    ProcessTerminated {
        pid: u32,
        exit_code: i32,
        signal: Option<u32>,
    },
    /// Crash event
    Crash {
        crash_type: String,
        severity: String,
        subsystem: Option<Subsystem>,
        message: String,
        recovery_action: String,
    },
    /// Trace completed
    TraceCompleted {
        trace_id: u128,
        correlation_id: u64,
        duration_ms: u64,
        span_count: u32,
        error_count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryOperation {
    Allocate,
    Deallocate,
    Map,
    Unmap,
    Protect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextSwitchReason {
    Preemption,
    Yield,
    Block,
    Exit,
    Signal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceOperation {
    Start,
    Stop,
    Restart,
    HealthCheck,
    Crash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceResult {
    Success,
    Failure,
    Timeout,
    Killed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogAction {
    Warning,
    Restart,
    Panic,
    Ignore,
}

/// Record an observability event
pub fn record_event(event: ObservabilityEvent) {
    let _ = with_observability_mut(|obs| {
        obs.flight_recorder.record_event(event);
    });
}

/// Access observability system (read-only)
pub fn with_observability_readonly<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&ObservabilitySystem) -> R,
{
    OBSERVABILITY.lock().as_ref().map(f)
}

/// Perform periodic observability maintenance
pub fn periodic_maintenance() {
    let _ = with_observability(|obs| {
        obs.periodic_maintenance();
    });
}

/// Macro for easy tracepoint creation
#[macro_export]
macro_rules! trace_event {
    ($subsystem:expr, $name:expr, $($arg:expr),*) => {
        {
            use alloc::vec;
            let mut data = vec![];
            $(
                // Serialize arguments - simplified for now
                let arg_bytes = format!("{:?}", $arg).into_bytes();
                data.extend_from_slice(&arg_bytes);
                data.push(b'|'); // separator
            )*
            
            $crate::observability::record_event(
                $crate::observability::ObservabilityEvent::Tracepoint {
                    name: $name.into(),
                    subsystem: $subsystem,
                    data,
                }
            );
        }
    };
}

/// Macro for syscall tracing
#[macro_export]
macro_rules! trace_syscall {
    (entry, $id:expr, $pid:expr, $args:expr) => {
        $crate::observability::record_event(
            $crate::observability::ObservabilityEvent::Syscall {
                syscall_id: $id,
                pid: $pid,
                entry: true,
                args: $args,
                result: None,
            }
        );
    };
    (exit, $id:expr, $pid:expr, $result:expr) => {
        $crate::observability::record_event(
            $crate::observability::ObservabilityEvent::Syscall {
                syscall_id: $id,
                pid: $pid,
                entry: false,
                args: [0; 6],
                result: Some($result),
            }
        );
    };
}