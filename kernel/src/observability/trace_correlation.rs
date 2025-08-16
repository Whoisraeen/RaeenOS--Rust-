//! Trace Correlation - Distributed tracing and correlation ID management
//!
//! This module provides trace correlation functionality for tracking requests
//! and operations across subsystem boundaries with correlation IDs.

use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use alloc::vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;
use super::{ObservabilityError, Subsystem};

/// Maximum number of active traces
const MAX_ACTIVE_TRACES: usize = 1024;

/// Maximum trace depth (nested operations)
#[allow(dead_code)]
const MAX_TRACE_DEPTH: usize = 32;

/// Maximum spans per trace
const MAX_SPANS_PER_TRACE: usize = 256;

/// Trace ID type
pub type TraceId = u128;

/// Span ID type
pub type SpanId = u64;

/// Correlation ID type
pub type CorrelationId = u128;

/// Trace context propagation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracePropagation {
    /// No propagation
    None,
    /// Propagate within same process
    Process,
    /// Propagate across processes
    System,
    /// Propagate across network boundaries
    Network,
}

/// Span kind classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    /// Internal operation
    Internal,
    /// Server handling a request
    Server,
    /// Client making a request
    Client,
    /// Producer sending a message
    Producer,
    /// Consumer receiving a message
    Consumer,
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// Span is active
    Active,
    /// Span completed successfully
    Ok,
    /// Span completed with error
    Error,
    /// Span was cancelled
    Cancelled,
    /// Span timed out
    Timeout,
}

/// Trace sampling decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingDecision {
    /// Do not sample this trace
    NotSampled,
    /// Sample this trace
    Sampled,
    /// Sample this trace and record it
    SampledAndRecorded,
}

/// Span attribute value
#[derive(Debug, Clone)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Bytes(Vec<u8>),
}

/// Span event
#[derive(Debug, Clone)]
pub struct SpanEvent {
    pub timestamp: u64,
    pub name: String,
    pub attributes: BTreeMap<String, AttributeValue>,
}

/// Span link to another span
#[derive(Debug, Clone)]
pub struct SpanLink {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub attributes: BTreeMap<String, AttributeValue>,
}

/// Trace span
#[derive(Debug, Clone)]
pub struct Span {
    pub span_id: SpanId,
    pub trace_id: TraceId,
    pub parent_span_id: Option<SpanId>,
    pub operation_name: String,
    pub subsystem: Subsystem,
    pub kind: SpanKind,
    pub status: SpanStatus,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub duration_ns: Option<u64>,
    pub attributes: BTreeMap<String, AttributeValue>,
    pub events: Vec<SpanEvent>,
    pub links: Vec<SpanLink>,
    pub error_message: Option<String>,
    pub sampling_decision: SamplingDecision,
}

/// Trace context
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: TraceId,
    pub correlation_id: CorrelationId,
    pub root_span_id: SpanId,
    pub current_span_id: Option<SpanId>,
    pub propagation: TracePropagation,
    pub sampling_decision: SamplingDecision,
    pub baggage: BTreeMap<String, String>,
    pub creation_time: u64,
    pub last_activity: u64,
}

/// Active trace information
#[derive(Debug, Clone)]
pub struct ActiveTrace {
    pub context: TraceContext,
    pub spans: Vec<Span>,
    pub span_stack: Vec<SpanId>, // For nested spans
    pub total_spans: u32,
    pub completed_spans: u32,
    pub error_count: u32,
}

/// Trace correlation configuration
#[derive(Debug, Clone)]
pub struct TraceCorrelationConfig {
    pub enabled: bool,
    pub default_sampling_rate: f64, // 0.0 to 1.0
    pub max_trace_duration_ms: u64,
    pub auto_finish_orphaned_spans: bool,
    pub propagate_baggage: bool,
    pub record_events: bool,
    pub record_links: bool,
    pub max_attributes_per_span: usize,
    pub max_events_per_span: usize,
    pub max_links_per_span: usize,
}

impl Default for TraceCorrelationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_sampling_rate: 0.1, // 10% sampling by default
            max_trace_duration_ms: 300000, // 5 minutes
            auto_finish_orphaned_spans: true,
            propagate_baggage: true,
            record_events: true,
            record_links: true,
            max_attributes_per_span: 64,
            max_events_per_span: 32,
            max_links_per_span: 16,
        }
    }
}

/// Trace correlation statistics
#[derive(Debug, Clone, Default)]
pub struct TraceCorrelationStats {
    pub total_traces_created: u64,
    pub total_spans_created: u64,
    pub total_spans_completed: u64,
    pub active_traces: u32,
    pub active_spans: u32,
    pub sampled_traces: u64,
    pub dropped_traces: u64,
    pub orphaned_spans: u64,
    pub average_trace_duration_ms: u64,
    pub average_spans_per_trace: f64,
}

/// Sampling strategy
pub trait SamplingStrategy {
    fn should_sample(
        &self,
        trace_id: TraceId,
        operation_name: &str,
        subsystem: Subsystem,
        parent_sampled: Option<bool>,
    ) -> SamplingDecision;
}

/// Simple probabilistic sampling
pub struct ProbabilisticSampler {
    sampling_rate: f64,
}

impl ProbabilisticSampler {
    pub fn new(sampling_rate: f64) -> Self {
        Self {
            sampling_rate: sampling_rate.clamp(0.0, 1.0),
        }
    }
}

impl SamplingStrategy for ProbabilisticSampler {
    fn should_sample(
        &self,
        trace_id: TraceId,
        _operation_name: &str,
        _subsystem: Subsystem,
        parent_sampled: Option<bool>,
    ) -> SamplingDecision {
        // If parent is sampled, always sample
        if parent_sampled == Some(true) {
            return SamplingDecision::SampledAndRecorded;
        }
        
        // Use trace ID for deterministic sampling
        let hash = (trace_id as u64) ^ ((trace_id >> 64) as u64);
        let threshold = (self.sampling_rate * u64::MAX as f64) as u64;
        
        if hash < threshold {
            SamplingDecision::SampledAndRecorded
        } else {
            SamplingDecision::NotSampled
        }
    }
}

/// Trace correlation manager
pub struct TraceCorrelationManager {
    config: RwLock<TraceCorrelationConfig>,
    active_traces: RwLock<BTreeMap<TraceId, ActiveTrace>>,
    correlation_to_trace: RwLock<BTreeMap<CorrelationId, TraceId>>,
    next_trace_id: AtomicU64,
    next_span_id: AtomicU64,
    next_correlation_id: AtomicU64,
    stats: RwLock<TraceCorrelationStats>,
    sampler: RwLock<Box<dyn SamplingStrategy + Send + Sync>>,
    current_context: RwLock<Option<TraceContext>>,
}

impl TraceCorrelationManager {
    /// Create a new trace correlation manager
    pub fn new() -> Self {
        let config = TraceCorrelationConfig::default();
        let sampler = Box::new(ProbabilisticSampler::new(config.default_sampling_rate));
        
        Self {
            config: RwLock::new(config),
            active_traces: RwLock::new(BTreeMap::new()),
            correlation_to_trace: RwLock::new(BTreeMap::new()),
            next_trace_id: AtomicU64::new(1),
            next_span_id: AtomicU64::new(1),
            next_correlation_id: AtomicU64::new(1),
            stats: RwLock::new(TraceCorrelationStats::default()),
            sampler: RwLock::new(sampler),
            current_context: RwLock::new(None),
        }
    }

    /// Generate a new trace ID
    pub fn generate_trace_id(&self) -> TraceId {
        let high = self.next_trace_id.fetch_add(1, Ordering::SeqCst) as u128;
        let low = crate::time::get_timestamp() as u128;
        (high << 64) | low
    }

    /// Generate a new span ID
    pub fn generate_span_id(&self) -> SpanId {
        self.next_span_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Generate a new correlation ID
    pub fn generate_correlation_id(&self) -> CorrelationId {
        let high = self.next_correlation_id.fetch_add(1, Ordering::SeqCst) as u128;
        let low = crate::time::get_timestamp() as u128;
        (high << 64) | low
    }

    /// Start a new trace
    pub fn start_trace(
        &self,
        operation_name: &str,
        subsystem: Subsystem,
        propagation: TracePropagation,
    ) -> Result<TraceContext, ObservabilityError> {
        if !self.config.read().enabled {
            return Err(ObservabilityError::NotEnabled);
        }
        
        let trace_id = self.generate_trace_id();
        let correlation_id = self.generate_correlation_id();
        let root_span_id = self.generate_span_id();
        
        // Determine sampling decision
        let sampling_decision = self.sampler.read().should_sample(
            trace_id,
            operation_name,
            subsystem,
            None,
        );
        
        let context = TraceContext {
            trace_id,
            correlation_id,
            root_span_id,
            current_span_id: Some(root_span_id),
            propagation,
            sampling_decision,
            baggage: BTreeMap::new(),
            creation_time: crate::time::get_timestamp(),
            last_activity: crate::time::get_timestamp(),
        };
        
        // Create root span
        let root_span = Span {
            span_id: root_span_id,
            trace_id,
            parent_span_id: None,
            operation_name: operation_name.to_string(),
            subsystem,
            kind: SpanKind::Internal,
            status: SpanStatus::Active,
            start_time: crate::time::get_timestamp(),
            end_time: None,
            duration_ns: None,
            attributes: BTreeMap::new(),
            events: Vec::new(),
            links: Vec::new(),
            error_message: None,
            sampling_decision,
        };
        
        // Create active trace
        let active_trace = ActiveTrace {
            context: context.clone(),
            spans: vec![root_span],
            span_stack: vec![root_span_id],
            total_spans: 1,
            completed_spans: 0,
            error_count: 0,
        };
        
        // Store active trace
        {
            let mut active_traces = self.active_traces.write();
            if active_traces.len() >= MAX_ACTIVE_TRACES {
                // Remove oldest trace
                if let Some((&oldest_trace_id, _)) = active_traces.iter().next() {
                    let oldest_trace_id = oldest_trace_id;
                    active_traces.remove(&oldest_trace_id);
                    self.stats.write().dropped_traces += 1;
                }
            }
            active_traces.insert(trace_id, active_trace);
        }
        
        // Update correlation mapping
        self.correlation_to_trace.write().insert(correlation_id, trace_id);
        
        // Update statistics
        {
            let mut stats = self.stats.write();
            stats.total_traces_created += 1;
            stats.total_spans_created += 1;
            stats.active_traces += 1;
            stats.active_spans += 1;
            if sampling_decision != SamplingDecision::NotSampled {
                stats.sampled_traces += 1;
            }
        }
        
        // Set as current context
        *self.current_context.write() = Some(context.clone());
        
        Ok(context)
    }

    /// Start a new span within an existing trace
    pub fn start_span(
        &self,
        trace_id: TraceId,
        operation_name: &str,
        subsystem: Subsystem,
        kind: SpanKind,
    ) -> Result<SpanId, ObservabilityError> {
        let mut active_traces = self.active_traces.write();
        
        if let Some(active_trace) = active_traces.get_mut(&trace_id) {
            // Check span limit
            if active_trace.spans.len() >= MAX_SPANS_PER_TRACE {
                return Err(ObservabilityError::ResourceExhausted);
            }
            
            let span_id = self.generate_span_id();
            let parent_span_id = active_trace.span_stack.last().copied();
            
            let span = Span {
                span_id,
                trace_id,
                parent_span_id,
                operation_name: operation_name.to_string(),
                subsystem,
                kind,
                status: SpanStatus::Active,
                start_time: crate::time::get_timestamp(),
                end_time: None,
                duration_ns: None,
                attributes: BTreeMap::new(),
                events: Vec::new(),
                links: Vec::new(),
                error_message: None,
                sampling_decision: active_trace.context.sampling_decision,
            };
            
            active_trace.spans.push(span);
            active_trace.span_stack.push(span_id);
            active_trace.total_spans += 1;
            active_trace.context.current_span_id = Some(span_id);
            active_trace.context.last_activity = crate::time::get_timestamp();
            
            // Update statistics
            {
                let mut stats = self.stats.write();
                stats.total_spans_created += 1;
                stats.active_spans += 1;
            }
            
            Ok(span_id)
        } else {
            Err(ObservabilityError::TraceNotFound)
        }
    }

    /// Finish a span
    pub fn finish_span(
        &self,
        trace_id: TraceId,
        span_id: SpanId,
        status: SpanStatus,
        error_message: Option<String>,
    ) -> Result<(), ObservabilityError> {
        let mut active_traces = self.active_traces.write();
        
        if let Some(active_trace) = active_traces.get_mut(&trace_id) {
            // Find and update the span
            if let Some(span) = active_trace.spans.iter_mut().find(|s| s.span_id == span_id) {
                let end_time = crate::time::get_timestamp();
                span.end_time = Some(end_time);
                span.duration_ns = Some(end_time - span.start_time);
                span.status = status;
                span.error_message = error_message;
                
                active_trace.completed_spans += 1;
                if status == SpanStatus::Error {
                    active_trace.error_count += 1;
                }
                
                // Remove from span stack if it's the current span
                if let Some(pos) = active_trace.span_stack.iter().position(|&id| id == span_id) {
                    active_trace.span_stack.remove(pos);
                    active_trace.context.current_span_id = active_trace.span_stack.last().copied();
                }
                
                active_trace.context.last_activity = end_time;
                
                // Update statistics
                {
                    let mut stats = self.stats.write();
                    stats.total_spans_completed += 1;
                    stats.active_spans -= 1;
                }
                
                // Check if trace is complete
                if active_trace.completed_spans == active_trace.total_spans {
                    let _ = self.finish_trace_internal(trace_id, &mut active_traces);
                }
                
                Ok(())
            } else {
                Err(ObservabilityError::SpanNotFound)
            }
        } else {
            Err(ObservabilityError::TraceNotFound)
        }
    }

    /// Add attribute to a span
    pub fn add_span_attribute(
        &self,
        trace_id: TraceId,
        span_id: SpanId,
        key: &str,
        value: AttributeValue,
    ) -> Result<(), ObservabilityError> {
        let mut active_traces = self.active_traces.write();
        
        if let Some(active_trace) = active_traces.get_mut(&trace_id) {
            if let Some(span) = active_trace.spans.iter_mut().find(|s| s.span_id == span_id) {
                let max_attributes = self.config.read().max_attributes_per_span;
                if span.attributes.len() < max_attributes {
                    span.attributes.insert(key.to_string(), value);
                    Ok(())
                } else {
                    Err(ObservabilityError::ResourceExhausted)
                }
            } else {
                Err(ObservabilityError::SpanNotFound)
            }
        } else {
            Err(ObservabilityError::TraceNotFound)
        }
    }

    /// Add event to a span
    pub fn add_span_event(
        &self,
        trace_id: TraceId,
        span_id: SpanId,
        name: &str,
        attributes: BTreeMap<String, AttributeValue>,
    ) -> Result<(), ObservabilityError> {
        if !self.config.read().record_events {
            return Ok(());
        }
        
        let mut active_traces = self.active_traces.write();
        
        if let Some(active_trace) = active_traces.get_mut(&trace_id) {
            if let Some(span) = active_trace.spans.iter_mut().find(|s| s.span_id == span_id) {
                let max_events = self.config.read().max_events_per_span;
                if span.events.len() < max_events {
                    span.events.push(SpanEvent {
                        timestamp: crate::time::get_timestamp(),
                        name: name.to_string(),
                        attributes,
                    });
                    Ok(())
                } else {
                    Err(ObservabilityError::ResourceExhausted)
                }
            } else {
                Err(ObservabilityError::SpanNotFound)
            }
        } else {
            Err(ObservabilityError::TraceNotFound)
        }
    }

    /// Get current trace context
    pub fn get_current_context(&self) -> Option<TraceContext> {
        self.current_context.read().clone()
    }

    /// Set current trace context
    pub fn set_current_context(&self, context: Option<TraceContext>) {
        *self.current_context.write() = context;
    }

    /// Get trace by correlation ID
    pub fn get_trace_by_correlation(&self, correlation_id: CorrelationId) -> Option<ActiveTrace> {
        let correlation_to_trace = self.correlation_to_trace.read();
        if let Some(&trace_id) = correlation_to_trace.get(&correlation_id) {
            self.active_traces.read().get(&trace_id).cloned()
        } else {
            None
        }
    }

    /// Finish a trace
    pub fn finish_trace(&self, trace_id: TraceId) -> Result<(), ObservabilityError> {
        let mut active_traces = self.active_traces.write();
        self.finish_trace_internal(trace_id, &mut active_traces)
    }

    /// Internal trace finishing logic
    fn finish_trace_internal(
        &self,
        trace_id: TraceId,
        active_traces: &mut BTreeMap<TraceId, ActiveTrace>,
    ) -> Result<(), ObservabilityError> {
        if let Some(active_trace) = active_traces.remove(&trace_id) {
            // Remove correlation mapping
            self.correlation_to_trace.write().remove(&active_trace.context.correlation_id);
            
            // Update statistics
            {
                let mut stats = self.stats.write();
                stats.active_traces -= 1;
                
                let duration = crate::time::get_timestamp() - active_trace.context.creation_time;
                let current_avg = stats.average_trace_duration_ms;
                let total_traces = stats.total_traces_created;
                stats.average_trace_duration_ms = 
                    (current_avg * (total_traces - 1) + duration) / total_traces;
                
                let current_spans_avg = stats.average_spans_per_trace;
                stats.average_spans_per_trace = 
                    (current_spans_avg * (total_traces - 1) as f64 + active_trace.total_spans as f64) / total_traces as f64;
            }
            
            // Record trace completion event
            super::record_event(super::ObservabilityEvent::TraceCompleted {
                trace_id,
                correlation_id: active_trace.context.correlation_id as u64,
                duration_ms: (crate::time::get_timestamp() - active_trace.context.creation_time) / 1000,
                span_count: active_trace.total_spans,
                error_count: active_trace.error_count,
            });
            
            Ok(())
        } else {
            Err(ObservabilityError::TraceNotFound)
        }
    }

    /// Clean up expired traces
    pub fn cleanup_expired_traces(&self) {
        let current_time = crate::time::get_timestamp();
        let max_duration = self.config.read().max_trace_duration_ms;
        
        let mut active_traces = self.active_traces.write();
        let mut expired_traces = Vec::new();
        
        for (&trace_id, active_trace) in active_traces.iter() {
            if current_time - active_trace.context.creation_time > max_duration {
                expired_traces.push(trace_id);
            }
        }
        
        for trace_id in expired_traces {
            let _ = self.finish_trace_internal(trace_id, &mut active_traces);
        }
    }

    /// Get trace correlation statistics
    pub fn get_stats(&self) -> TraceCorrelationStats {
        self.stats.read().clone()
    }

    /// Update configuration
    pub fn update_config(&self, config: TraceCorrelationConfig) {
        *self.config.write() = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> TraceCorrelationConfig {
        self.config.read().clone()
    }
}

/// Macro for starting a traced operation
#[macro_export]
macro_rules! trace_operation {
    ($operation_name:expr, $subsystem:expr) => {
        $crate::observability::with_observability_mut(|obs| {
            obs.trace_correlation.start_trace(
                $operation_name,
                $subsystem,
                $crate::observability::trace_correlation::TracePropagation::Process,
            )
        })
    };
    ($operation_name:expr, $subsystem:expr, $propagation:expr) => {
        $crate::observability::with_observability_mut(|obs| {
            obs.trace_correlation.start_trace($operation_name, $subsystem, $propagation)
        })
    };
}

/// Macro for starting a span
#[macro_export]
macro_rules! trace_span {
    ($trace_id:expr, $operation_name:expr, $subsystem:expr) => {
        $crate::observability::with_observability_mut(|obs| {
            obs.trace_correlation.start_span(
                $trace_id,
                $operation_name,
                $subsystem,
                $crate::observability::trace_correlation::SpanKind::Internal,
            )
        })
    };
    ($trace_id:expr, $operation_name:expr, $subsystem:expr, $kind:expr) => {
        $crate::observability::with_observability_mut(|obs| {
            obs.trace_correlation.start_span($trace_id, $operation_name, $subsystem, $kind)
        })
    };
}