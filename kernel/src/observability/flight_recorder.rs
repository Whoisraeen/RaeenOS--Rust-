//! Flight Recorder - Always-on event recording with bounded storage
//!
//! The flight recorder maintains a circular buffer of system events that can be
//! dumped on crash or analyzed for performance debugging. It's designed to be
//! always-on with minimal performance impact.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::{Mutex, RwLock};
use super::{ObservabilityEvent, ObservabilityError, Severity, Subsystem};

/// Maximum number of events in the flight recorder buffer
const MAX_FLIGHT_RECORDER_EVENTS: usize = 65536; // 64K events

/// Maximum size of event data in bytes
const MAX_EVENT_DATA_SIZE: usize = 512;

/// Flight recorder configuration
#[derive(Debug, Clone)]
pub struct FlightRecorderConfig {
    pub max_events: usize,
    pub max_event_size: usize,
    pub enable_compression: bool,
    pub redaction_enabled: bool,
    pub storage_path: Option<String>,
    pub auto_dump_on_crash: bool,
    pub retention_hours: u32,
}

impl Default for FlightRecorderConfig {
    fn default() -> Self {
        Self {
            max_events: MAX_FLIGHT_RECORDER_EVENTS,
            max_event_size: MAX_EVENT_DATA_SIZE,
            enable_compression: false, // Disabled for performance
            redaction_enabled: true,
            storage_path: None, // In-memory only by default
            auto_dump_on_crash: true,
            retention_hours: 72, // 72 hours as per requirements
        }
    }
}

/// Flight recorder event with metadata
#[derive(Debug, Clone)]
pub struct FlightRecorderEntry {
    pub timestamp_ns: u64,
    pub sequence_id: u64,
    pub thread_id: u32,
    pub cpu_id: u8,
    pub severity: Severity,
    pub subsystem: Subsystem,
    pub event: ObservabilityEvent,
    pub trace_id: Option<u128>,
    pub parent_span_id: Option<u64>,
    pub span_id: u64,
}

/// Flight recorder statistics
#[derive(Debug, Clone, Default)]
pub struct FlightRecorderStats {
    pub total_events_recorded: u64,
    pub events_dropped: u64,
    pub buffer_utilization_percent: u8,
    pub average_event_size_bytes: u32,
    pub last_dump_timestamp: u64,
    pub compression_ratio: f32,
    pub redacted_events: u64,
}

/// Flight recorder implementation
pub struct FlightRecorder {
    config: FlightRecorderConfig,
    buffer: Mutex<VecDeque<FlightRecorderEntry>>,
    sequence_counter: AtomicU64,
    span_counter: AtomicU64,
    stats: RwLock<FlightRecorderStats>,
    enabled: AtomicU64, // Using u64 for atomic bool
}

impl FlightRecorder {
    /// Create a new flight recorder with default configuration
    pub fn new() -> Result<Self, ObservabilityError> {
        Self::with_config(FlightRecorderConfig::default())
    }

    /// Create a new flight recorder with custom configuration
    pub fn with_config(config: FlightRecorderConfig) -> Result<Self, ObservabilityError> {
        if config.max_events == 0 || config.max_event_size == 0 {
            return Err(ObservabilityError::InvalidConfiguration);
        }

        Ok(Self {
            config,
            buffer: Mutex::new(VecDeque::with_capacity(MAX_FLIGHT_RECORDER_EVENTS)),
            sequence_counter: AtomicU64::new(1),
            span_counter: AtomicU64::new(1),
            stats: RwLock::new(FlightRecorderStats::default()),
            enabled: AtomicU64::new(1), // Enabled by default
        })
    }

    /// Record an event in the flight recorder
    pub fn record_event(&self, event: ObservabilityEvent) {
        if self.enabled.load(Ordering::Relaxed) == 0 {
            return;
        }

        let entry = FlightRecorderEntry {
            timestamp_ns: crate::time::get_timestamp_ns(),
            sequence_id: self.sequence_counter.fetch_add(1, Ordering::SeqCst),
            thread_id: self.get_current_thread_id(),
            cpu_id: self.get_current_cpu_id(),
            severity: self.determine_severity(&event),
            subsystem: self.determine_subsystem(&event),
            event: self.maybe_redact_event(event),
            trace_id: self.get_current_trace_id(),
            parent_span_id: self.get_current_parent_span_id(),
            span_id: self.span_counter.fetch_add(1, Ordering::SeqCst),
        };

        self.add_entry(entry);
    }

    /// Record an event with explicit trace correlation
    pub fn record_event_with_trace(
        &self,
        event: ObservabilityEvent,
        trace_id: u128,
        parent_span_id: Option<u64>,
    ) {
        if self.enabled.load(Ordering::Relaxed) == 0 {
            return;
        }

        let entry = FlightRecorderEntry {
            timestamp_ns: crate::time::get_timestamp_ns(),
            sequence_id: self.sequence_counter.fetch_add(1, Ordering::SeqCst),
            thread_id: self.get_current_thread_id(),
            cpu_id: self.get_current_cpu_id(),
            severity: self.determine_severity(&event),
            subsystem: self.determine_subsystem(&event),
            event: self.maybe_redact_event(event),
            trace_id: Some(trace_id),
            parent_span_id,
            span_id: self.span_counter.fetch_add(1, Ordering::SeqCst),
        };

        self.add_entry(entry);
    }

    /// Add an entry to the circular buffer
    fn add_entry(&self, entry: FlightRecorderEntry) {
        let mut buffer = self.buffer.lock();
        
        // If buffer is full, remove oldest entry
        if buffer.len() >= self.config.max_events {
            buffer.pop_front();
            self.stats.write().events_dropped += 1;
        }

        buffer.push_back(entry);
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_events_recorded += 1;
            stats.buffer_utilization_percent = 
                ((buffer.len() * 100) / self.config.max_events) as u8;
        }
    }

    /// Dump the flight recorder buffer for crash analysis
    pub fn dump_on_crash(&self) -> Vec<FlightRecorderEntry> {
        let buffer = self.buffer.lock();
        let mut entries: Vec<_> = buffer.iter().cloned().collect();
        
        // Sort by sequence ID to ensure chronological order
        entries.sort_by_key(|e| e.sequence_id);
        
        // Update stats
        self.stats.write().last_dump_timestamp = crate::time::get_timestamp();
        
        entries
    }

    /// Get recent events for debugging
    pub fn get_recent_events(&self, count: usize) -> Vec<FlightRecorderEntry> {
        let buffer = self.buffer.lock();
        let start_idx = if buffer.len() > count {
            buffer.len() - count
        } else {
            0
        };
        
        buffer.range(start_idx..).cloned().collect()
    }

    /// Get events by subsystem
    pub fn get_events_by_subsystem(&self, subsystem: Subsystem) -> Vec<FlightRecorderEntry> {
        let buffer = self.buffer.lock();
        buffer.iter()
            .filter(|entry| entry.subsystem == subsystem)
            .cloned()
            .collect()
    }

    /// Get events by trace ID
    pub fn get_events_by_trace_id(&self, trace_id: u128) -> Vec<FlightRecorderEntry> {
        let buffer = self.buffer.lock();
        buffer.iter()
            .filter(|entry| entry.trace_id == Some(trace_id))
            .cloned()
            .collect()
    }

    /// Get flight recorder statistics
    pub fn get_stats(&self) -> FlightRecorderStats {
        self.stats.read().clone()
    }

    /// Enable or disable the flight recorder
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(if enabled { 1 } else { 0 }, Ordering::SeqCst);
    }

    /// Check if flight recorder is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed) != 0
    }

    /// Clear the flight recorder buffer
    pub fn clear(&self) {
        let mut buffer = self.buffer.lock();
        buffer.clear();
        
        // Reset stats
        let mut stats = self.stats.write();
        stats.buffer_utilization_percent = 0;
    }

    /// Update configuration
    pub fn update_config(&mut self, config: FlightRecorderConfig) -> Result<(), ObservabilityError> {
        if config.max_events == 0 || config.max_event_size == 0 {
            return Err(ObservabilityError::InvalidConfiguration);
        }
        
        self.config = config;
        Ok(())
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

    fn get_current_trace_id(&self) -> Option<u128> {
        // TODO: Get current trace ID from trace correlation
        None
    }

    fn get_current_parent_span_id(&self) -> Option<u64> {
        // TODO: Get current parent span ID from trace correlation
        None
    }

    fn determine_severity(&self, event: &ObservabilityEvent) -> Severity {
        match event {
            ObservabilityEvent::Service { result: super::ServiceResult::Failure, .. } => Severity::Error,
            ObservabilityEvent::Service { operation: super::ServiceOperation::Crash, .. } => Severity::Fatal,
            ObservabilityEvent::Watchdog { action: super::WatchdogAction::Panic, .. } => Severity::Fatal,
            ObservabilityEvent::Watchdog { action: super::WatchdogAction::Restart, .. } => Severity::Error,
            ObservabilityEvent::Watchdog { action: super::WatchdogAction::Warning, .. } => Severity::Warn,
            ObservabilityEvent::PageFault { resolved: false, .. } => Severity::Error,
            ObservabilityEvent::Interrupt { .. } => Severity::Trace,
            ObservabilityEvent::Syscall { .. } => Severity::Trace,
            ObservabilityEvent::Ipc { .. } => Severity::Debug,
            ObservabilityEvent::Memory { .. } => Severity::Debug,
            ObservabilityEvent::ContextSwitch { .. } => Severity::Trace,
            ObservabilityEvent::Tracepoint { .. } => Severity::Debug,
            _ => Severity::Info,
        }
    }

    fn determine_subsystem(&self, event: &ObservabilityEvent) -> Subsystem {
        match event {
            ObservabilityEvent::Syscall { .. } => Subsystem::Kernel,
            ObservabilityEvent::Ipc { .. } => Subsystem::Ipc,
            ObservabilityEvent::Memory { .. } => Subsystem::Memory,
            ObservabilityEvent::ContextSwitch { .. } => Subsystem::Scheduler,
            ObservabilityEvent::Interrupt { .. } => Subsystem::Interrupt,
            ObservabilityEvent::PageFault { .. } => Subsystem::Memory,
            ObservabilityEvent::Service { .. } => Subsystem::ServiceManager,
            ObservabilityEvent::Watchdog { subsystem, .. } => *subsystem,
            ObservabilityEvent::Tracepoint { subsystem, .. } => *subsystem,
            ObservabilityEvent::SystemBoot { .. } => Subsystem::Kernel,
            ObservabilityEvent::ProcessCreated { .. } => Subsystem::Scheduler,
            ObservabilityEvent::ProcessTerminated { .. } => Subsystem::Scheduler,
            ObservabilityEvent::Crash { subsystem, .. } => subsystem.unwrap_or(Subsystem::Unknown),
            ObservabilityEvent::TraceCompleted { .. } => Subsystem::Kernel,
        }
    }

    fn maybe_redact_event(&self, mut event: ObservabilityEvent) -> ObservabilityEvent {
        if !self.config.redaction_enabled {
            return event;
        }

        // Redact sensitive data based on event type
        match &mut event {
            ObservabilityEvent::Syscall { args, .. } => {
                // Redact potential sensitive syscall arguments
                // This is a simplified approach - real implementation would be more sophisticated
                for arg in args.iter_mut() {
                    if *arg > 0x1000 && *arg < 0x7fffffffffff {
                        // Looks like a pointer, redact it
                        *arg = 0xDEADBEEF;
                    }
                }
            },
            ObservabilityEvent::Tracepoint { data, .. } => {
                // Clear sensitive tracepoint data
                data.clear();
                data.extend_from_slice(b"[REDACTED]");
            },
            ObservabilityEvent::ProcessCreated { name, .. } => {
                // Redact process names that might contain sensitive info
                *name = String::from("[REDACTED]");
            },
            ObservabilityEvent::Crash { message, .. } => {
                // Redact crash messages that might contain sensitive data
                *message = String::from("[REDACTED]");
            },
            _ => {}
        }

        self.stats.write().redacted_events += 1;
        event
    }
}

/// Format flight recorder entries for human consumption
pub fn format_flight_recorder_dump(entries: &[FlightRecorderEntry]) -> String {
    use alloc::format;
    
    let mut output = String::new();
    output.push_str("=== FLIGHT RECORDER DUMP ===\n");
    output.push_str(&format!("Total entries: {}\n", entries.len()));
    output.push_str(&format!("Time range: {} - {}\n", 
        entries.first().map(|e| e.timestamp_ns).unwrap_or(0),
        entries.last().map(|e| e.timestamp_ns).unwrap_or(0)
    ));
    output.push_str("\n");

    for entry in entries {
        output.push_str(&format!(
            "[{}] {:?}:{:?} CPU{} T{} S{} ",
            entry.timestamp_ns,
            entry.severity,
            entry.subsystem,
            entry.cpu_id,
            entry.thread_id,
            entry.sequence_id
        ));
        
        if let Some(trace_id) = entry.trace_id {
            output.push_str(&format!("TR:{:x} ", trace_id));
        }
        
        output.push_str(&format!("SP:{} ", entry.span_id));
        
        if let Some(parent) = entry.parent_span_id {
            output.push_str(&format!("PS:{} ", parent));
        }
        
        output.push_str(&format!("{:?}\n", entry.event));
    }

    output
}