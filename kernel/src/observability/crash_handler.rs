//! Crash Handler - Crash detection, context capture, and recovery
//!
//! This module provides crash handling functionality including panic hooks,
//! crash context capture, and automatic recovery mechanisms.

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use alloc::format;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use spin::RwLock;
use super::{ObservabilityError, Subsystem};

/// Maximum crash context entries
const MAX_CRASH_CONTEXTS: usize = 16;

/// Maximum stack trace depth
const MAX_STACK_TRACE_DEPTH: usize = 32;

/// Crash type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CrashType {
    Panic,
    PageFault,
    GeneralProtectionFault,
    DoubleFault,
    StackOverflow,
    DivideByZero,
    InvalidOpcode,
    OutOfMemory,
    Timeout,
    AssertionFailure,
    Unknown,
}

/// Crash severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CrashSeverity {
    Info,
    Warning,
    Error,
    Critical,
    Fatal,
}

/// Recovery action to take after a crash
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryAction {
    None,
    RestartSubsystem,
    RestartProcess,
    RestartService,
    Reboot,
    Halt,
}

/// CPU register state at crash time
#[derive(Debug, Clone, Default)]
pub struct CpuState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cs: u16,
    pub ds: u16,
    pub es: u16,
    pub fs: u16,
    pub gs: u16,
    pub ss: u16,
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
}

/// Stack frame information
#[derive(Debug, Clone)]
pub struct StackFrame {
    pub instruction_pointer: u64,
    pub stack_pointer: u64,
    pub frame_pointer: u64,
    pub symbol_name: Option<String>,
    pub module_name: Option<String>,
    pub offset: u64,
}

/// Memory region information
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start_address: u64,
    pub end_address: u64,
    pub permissions: u8, // Read=1, Write=2, Execute=4
    pub region_type: MemoryRegionType,
    pub name: Option<String>,
}

/// Memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Code,
    Data,
    Stack,
    Heap,
    Kernel,
    Device,
    Unknown,
}

/// Crash context information
#[derive(Debug, Clone)]
pub struct CrashContext {
    pub id: u64,
    pub timestamp: u64,
    pub crash_type: CrashType,
    pub severity: CrashSeverity,
    pub subsystem: Option<Subsystem>,
    pub process_id: Option<u32>,
    pub thread_id: Option<u32>,
    pub error_code: Option<u64>,
    pub fault_address: Option<u64>,
    pub message: String,
    pub cpu_state: CpuState,
    pub stack_trace: Vec<StackFrame>,
    pub memory_regions: Vec<MemoryRegion>,
    pub recovery_action: RecoveryAction,
    pub recovery_attempted: bool,
    pub recovery_successful: bool,
}

/// Crash handler configuration
#[derive(Debug, Clone)]
pub struct CrashHandlerConfig {
    pub enable_stack_traces: bool,
    pub enable_memory_dump: bool,
    pub enable_auto_recovery: bool,
    pub max_recovery_attempts: u32,
    pub recovery_timeout_ms: u32,
    pub dump_to_flight_recorder: bool,
    pub halt_on_fatal: bool,
    pub reboot_on_critical: bool,
}

impl Default for CrashHandlerConfig {
    fn default() -> Self {
        Self {
            enable_stack_traces: true,
            enable_memory_dump: false, // Expensive, disabled by default
            enable_auto_recovery: true,
            max_recovery_attempts: 3,
            recovery_timeout_ms: 5000, // 5 seconds
            dump_to_flight_recorder: true,
            halt_on_fatal: true,
            reboot_on_critical: false, // Conservative default
        }
    }
}

/// Crash statistics
#[derive(Debug, Clone, Default)]
pub struct CrashStats {
    pub total_crashes: u64,
    pub crashes_by_type: BTreeMap<CrashType, u64>,
    pub crashes_by_severity: BTreeMap<CrashSeverity, u64>,
    pub crashes_by_subsystem: BTreeMap<Subsystem, u64>,
    pub recovery_attempts: u64,
    pub successful_recoveries: u64,
    pub failed_recoveries: u64,
    pub last_crash_timestamp: u64,
    pub uptime_since_last_crash: u64,
}

/// Crash handler
pub struct CrashHandler {
    config: RwLock<CrashHandlerConfig>,
    crash_contexts: RwLock<Vec<CrashContext>>,
    next_crash_id: AtomicU64,
    stats: RwLock<CrashStats>,
    recovery_handlers: RwLock<BTreeMap<Subsystem, RecoveryHandler>>,
    panic_hook_installed: AtomicU8,
}

/// Recovery handler function type
pub type RecoveryHandler = fn(context: &CrashContext) -> Result<(), CrashRecoveryError>;

/// Crash recovery error types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CrashRecoveryError {
    SubsystemNotFound,
    RecoveryFailed,
    RecoveryTimeout,
    TooManyAttempts,
    InvalidContext,
    NoRecoveryHandler,
}

impl CrashHandler {
    /// Create a new crash handler
    pub fn new() -> Self {
        Self {
            config: RwLock::new(CrashHandlerConfig::default()),
            crash_contexts: RwLock::new(Vec::new()),
            next_crash_id: AtomicU64::new(1),
            stats: RwLock::new(CrashStats::default()),
            recovery_handlers: RwLock::new(BTreeMap::new()),
            panic_hook_installed: AtomicU8::new(0),
        }
    }

    /// Initialize crash handler and install panic hook
    pub fn initialize(&self) {
        if self.panic_hook_installed.swap(1, Ordering::SeqCst) == 0 {
            // Install panic hook
                // Note: set_hook is not available in no_std
            // In a real implementation, this would integrate with the bootloader's panic handler
            // Panic handling would be done through the bootloader or a custom panic handler
        }
    }

    /// Handle a crash event
    pub fn handle_crash(
        &self,
        crash_type: CrashType,
        severity: CrashSeverity,
        subsystem: Option<Subsystem>,
        process_id: Option<u32>,
        thread_id: Option<u32>,
        error_code: Option<u64>,
        fault_address: Option<u64>,
        message: &str,
    ) -> Result<u64, ObservabilityError> {
        let crash_id = self.next_crash_id.fetch_add(1, Ordering::SeqCst);
        let timestamp = crate::time::get_timestamp();
        
        // Capture CPU state (would need actual implementation)
        let cpu_state = self.capture_cpu_state();
        
        // Capture stack trace
        let stack_trace = if self.config.read().enable_stack_traces {
            self.capture_stack_trace()
        } else {
            Vec::new()
        };
        
        // Capture memory regions
        let memory_regions = if self.config.read().enable_memory_dump {
            self.capture_memory_regions()
        } else {
            Vec::new()
        };
        
        // Determine recovery action
        let recovery_action = self.determine_recovery_action(crash_type, severity, subsystem);
        
        let context = CrashContext {
            id: crash_id,
            timestamp,
            crash_type,
            severity,
            subsystem,
            process_id,
            thread_id,
            error_code,
            fault_address,
            message: message.to_string(),
            cpu_state,
            stack_trace,
            memory_regions,
            recovery_action,
            recovery_attempted: false,
            recovery_successful: false,
        };
        
        // Store crash context
        {
            let mut contexts = self.crash_contexts.write();
            if contexts.len() >= MAX_CRASH_CONTEXTS {
                contexts.remove(0); // Remove oldest
            }
            contexts.push(context.clone());
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_crashes += 1;
            *stats.crashes_by_type.entry(crash_type).or_insert(0) += 1;
            *stats.crashes_by_severity.entry(severity).or_insert(0) += 1;
            if let Some(subsys) = subsystem {
                *stats.crashes_by_subsystem.entry(subsys).or_insert(0) += 1;
            }
            stats.last_crash_timestamp = timestamp;
        }
        
        // Record in flight recorder if enabled
        if self.config.read().dump_to_flight_recorder {
            super::record_event(super::ObservabilityEvent::Crash {
                crash_type: format!("{:?}", crash_type),
                severity: format!("{:?}", severity),
                subsystem,
                message: message.to_string(),
                recovery_action: format!("{:?}", recovery_action),
            });
        }
        
        // Attempt recovery if enabled and appropriate
        if self.config.read().enable_auto_recovery && recovery_action != RecoveryAction::None {
            let _ = self.attempt_recovery(&context);
        }
        
        // Handle fatal crashes
        match severity {
            CrashSeverity::Fatal => {
                if self.config.read().halt_on_fatal {
                    // Halt the system
                    self.halt_system(&context);
                }
            },
            CrashSeverity::Critical => {
                if self.config.read().reboot_on_critical {
                    // Reboot the system
                    self.reboot_system(&context);
                }
            },
            _ => {}
        }
        
        Ok(crash_id)
    }

    /// Register a recovery handler for a subsystem
    pub fn register_recovery_handler(
        &self,
        subsystem: Subsystem,
        handler: RecoveryHandler,
    ) {
        self.recovery_handlers.write().insert(subsystem, handler);
    }

    /// Attempt to recover from a crash
    pub fn attempt_recovery(&self, context: &CrashContext) -> Result<(), CrashRecoveryError> {
        let config = self.config.read();
        
        // Check if recovery is enabled
        if !config.enable_auto_recovery {
            return Err(CrashRecoveryError::NoRecoveryHandler);
        }
        
        // Check recovery attempts
        let mut stats = self.stats.write();
        if stats.recovery_attempts >= config.max_recovery_attempts as u64 {
            return Err(CrashRecoveryError::TooManyAttempts);
        }
        
        stats.recovery_attempts += 1;
        drop(stats);
        drop(config);
        
        // Mark recovery as attempted
        {
            let mut contexts = self.crash_contexts.write();
            if let Some(ctx) = contexts.iter_mut().find(|c| c.id == context.id) {
                ctx.recovery_attempted = true;
            }
        }
        
        // Attempt recovery based on action
        let recovery_result = match context.recovery_action {
            RecoveryAction::None => Ok(()),
            RecoveryAction::RestartSubsystem => {
                if let Some(subsystem) = context.subsystem {
                    self.restart_subsystem(subsystem, context)
                } else {
                    Err(CrashRecoveryError::InvalidContext)
                }
            },
            RecoveryAction::RestartProcess => {
                if let Some(process_id) = context.process_id {
                    self.restart_process(process_id, context)
                } else {
                    Err(CrashRecoveryError::InvalidContext)
                }
            },
            RecoveryAction::RestartService => {
                self.restart_service(context)
            },
            RecoveryAction::Reboot => {
                self.reboot_system(context);
                Ok(()) // Won't return
            },
            RecoveryAction::Halt => {
                self.halt_system(context);
                Ok(()) // Won't return
            },
        };
        
        // Update recovery status
        let recovery_successful = recovery_result.is_ok();
        {
            let mut contexts = self.crash_contexts.write();
            if let Some(ctx) = contexts.iter_mut().find(|c| c.id == context.id) {
                ctx.recovery_successful = recovery_successful;
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            if recovery_successful {
                stats.successful_recoveries += 1;
            } else {
                stats.failed_recoveries += 1;
            }
        }
        
        recovery_result
    }

    /// Restart a subsystem
    fn restart_subsystem(
        &self,
        subsystem: Subsystem,
        context: &CrashContext,
    ) -> Result<(), CrashRecoveryError> {
        let handlers = self.recovery_handlers.read();
        if let Some(handler) = handlers.get(&subsystem) {
            handler(context).map_err(|_| CrashRecoveryError::RecoveryFailed)
        } else {
            // Default subsystem restart logic
            self.default_subsystem_restart(subsystem)
        }
    }

    /// Restart a process
    fn restart_process(
        &self,
        _process_id: u32,
        _context: &CrashContext,
    ) -> Result<(), CrashRecoveryError> {
        // TODO: Implement process restart logic
        // This would involve:
        // 1. Terminating the crashed process
        // 2. Cleaning up its resources
        // 3. Restarting it with the same parameters
        Ok(())
    }

    /// Restart a service
    fn restart_service(&self, _context: &CrashContext) -> Result<(), CrashRecoveryError> {
        // TODO: Implement service restart logic
        // This would involve restarting the service manager or specific service
        Ok(())
    }

    /// Default subsystem restart implementation
    fn default_subsystem_restart(&self, subsystem: Subsystem) -> Result<(), CrashRecoveryError> {
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
                // Generic restart
                Ok(())
            },
        }
    }

    /// Determine appropriate recovery action
    fn determine_recovery_action(
        &self,
        crash_type: CrashType,
        severity: CrashSeverity,
        subsystem: Option<Subsystem>,
    ) -> RecoveryAction {
        match severity {
            CrashSeverity::Fatal => RecoveryAction::Halt,
            CrashSeverity::Critical => {
                match crash_type {
                    CrashType::DoubleFault | CrashType::StackOverflow => RecoveryAction::Reboot,
                    _ => {
                        if subsystem.is_some() {
                            RecoveryAction::RestartSubsystem
                        } else {
                            RecoveryAction::Reboot
                        }
                    }
                }
            },
            CrashSeverity::Error => {
                if subsystem.is_some() {
                    RecoveryAction::RestartSubsystem
                } else {
                    RecoveryAction::RestartService
                }
            },
            CrashSeverity::Warning | CrashSeverity::Info => RecoveryAction::None,
        }
    }

    /// Capture CPU state (placeholder implementation)
    fn capture_cpu_state(&self) -> CpuState {
        // TODO: Implement actual CPU state capture
        // This would involve reading CPU registers
        CpuState::default()
    }

    /// Capture stack trace
    fn capture_stack_trace(&self) -> Vec<StackFrame> {
        let mut stack_trace = Vec::new();
        
        // TODO: Implement actual stack walking
        // This would involve:
        // 1. Walking the stack frames
        // 2. Resolving symbols
        // 3. Getting module information
        
        // Placeholder implementation
        for i in 0..MAX_STACK_TRACE_DEPTH.min(8) {
            stack_trace.push(StackFrame {
                instruction_pointer: 0x1000 + (i as u64 * 0x100),
                stack_pointer: 0x7fff0000 + (i as u64 * 0x1000),
                frame_pointer: 0x7fff0000 + (i as u64 * 0x1000) + 8,
                symbol_name: Some(format!("function_{}", i)),
                module_name: Some("kernel".to_string()),
                offset: i as u64 * 0x10,
            });
        }
        
        stack_trace
    }

    /// Capture memory regions
    fn capture_memory_regions(&self) -> Vec<MemoryRegion> {
        let mut regions = Vec::new();
        
        // TODO: Implement actual memory region enumeration
        // This would involve walking the page tables and memory maps
        
        // Placeholder implementation
        regions.push(MemoryRegion {
            start_address: 0x100000,
            end_address: 0x200000,
            permissions: 5, // Read + Execute
            region_type: MemoryRegionType::Code,
            name: Some("kernel_text".to_string()),
        });
        
        regions.push(MemoryRegion {
            start_address: 0x200000,
            end_address: 0x300000,
            permissions: 3, // Read + Write
            region_type: MemoryRegionType::Data,
            name: Some("kernel_data".to_string()),
        });
        
        regions
    }

    /// Halt the system
    fn halt_system(&self, _context: &CrashContext) {
        // TODO: Implement system halt
        // This would involve:
        // 1. Stopping all CPUs
        // 2. Disabling interrupts
        // 3. Halting the processor
        loop {
            unsafe {
                // SAFETY: This is unsafe because:
                // - The `hlt` instruction is a privileged operation requiring kernel mode
                // - This halts the CPU until the next interrupt occurs
                // - Used in crash handling context where system stability is compromised
                // - The inline assembly syntax must be correct for x86-64
                // - This is part of emergency system shutdown procedure
                core::arch::asm!("hlt");
            }
        }
    }

    /// Reboot the system
    fn reboot_system(&self, _context: &CrashContext) {
        // TODO: Implement system reboot
        // This would involve:
        // 1. Flushing caches
        // 2. Syncing filesystems
        // 3. Triggering a reboot via ACPI or keyboard controller
        unsafe {
            // Triple fault to force reboot
            core::arch::asm!("int3");
        }
    }

    /// Get crash statistics
    pub fn get_stats(&self) -> CrashStats {
        self.stats.read().clone()
    }

    /// Get recent crash contexts
    pub fn get_recent_crashes(&self, count: usize) -> Vec<CrashContext> {
        let contexts = self.crash_contexts.read();
        let start = if contexts.len() > count {
            contexts.len() - count
        } else {
            0
        };
        contexts[start..].to_vec()
    }

    /// Get crash context by ID
    pub fn get_crash_context(&self, crash_id: u64) -> Option<CrashContext> {
        self.crash_contexts
            .read()
            .iter()
            .find(|c| c.id == crash_id)
            .cloned()
    }

    /// Update configuration
    pub fn update_config(&self, config: CrashHandlerConfig) {
        *self.config.write() = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> CrashHandlerConfig {
        self.config.read().clone()
    }
}

/// Macro for reporting crashes
#[macro_export]
macro_rules! report_crash {
    ($crash_type:expr, $severity:expr, $message:expr) => {
        $crate::observability::with_observability_mut(|obs| {
            let _ = obs.crash_handler.handle_crash(
                $crash_type,
                $severity,
                None,
                None,
                None,
                None,
                None,
                $message,
            );
        })
    };
    ($crash_type:expr, $severity:expr, $subsystem:expr, $message:expr) => {
        $crate::observability::with_observability_mut(|obs| {
            let _ = obs.crash_handler.handle_crash(
                $crash_type,
                $severity,
                Some($subsystem),
                None,
                None,
                None,
                None,
                $message,
            );
        })
    };
}