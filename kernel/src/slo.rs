//! SLO (Service Level Objective) Harness for RaeenOS
//!
//! This module provides performance measurement, gate enforcement, and CI integration
//! for ensuring RaeenOS meets its performance targets across all subsystems.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use core::fmt;
use serde::{Deserialize, Serialize};

/// Performance measurement categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SloCategory {
    /// Input latency measurements (keyboard, mouse, touch)
    InputLatency,
    /// Compositor frame timing and jitter
    CompositorJitter,
    /// IPC round-trip time measurements
    IpcRtt,
    /// Anonymous page fault service time
    AnonPageFault,
    /// TLB shootdown latency
    TlbShootdown,
    /// NVMe storage I/O latency
    NvmeIo,
    /// System idle power consumption
    IdlePower,
    /// Filesystem chaos testing results
    ChaosFs,
    /// Memory allocation latency
    MemoryAlloc,
    /// Context switch timing
    ContextSwitch,
    /// Interrupt handling latency
    InterruptLatency,
    /// Network packet processing
    NetworkLatency,
    /// Audio buffer underruns
    AudioUnderruns,
    /// Graphics frame drops
    FrameDrops,
}

/// SLO measurement result with percentile statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloMeasurement {
    pub category: SloCategory,
    pub test_name: String,
    pub unit: String,
    pub samples: u64,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub p95: f64,
    pub p99: f64,
    pub p999: f64,
    pub timestamp_ns: u64,
    pub reference_sku: String,
    pub app_mix: String,
}

/// SLO gate definition with acceptance criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloGate {
    pub category: SloCategory,
    pub name: String,
    pub target_p99_us: f64,
    pub target_p95_us: f64,
    pub max_drift_percent: f64,
    pub enabled: bool,
}

/// SLO test results in schema-conformant JSON format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloResults {
    pub version: String,
    pub timestamp_ns: u64,
    pub reference_sku: String,
    pub app_mix: String,
    pub kernel_version: String,
    pub measurements: Vec<SloMeasurement>,
    pub gates: Vec<SloGateResult>,
    pub overall_pass: bool,
}

/// Individual gate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SloGateResult {
    pub gate: SloGate,
    pub measurement: Option<SloMeasurement>,
    pub pass: bool,
    pub reason: String,
}

/// SLO harness for performance measurement and gate enforcement
pub struct SloHarness {
    measurements: BTreeMap<SloCategory, Vec<SloMeasurement>>,
    gates: Vec<SloGate>,
    pub reference_sku: String,
    pub app_mix: String,
}

impl SloHarness {
    /// Create a new SLO harness with default gates
    pub fn new(reference_sku: String, app_mix: String) -> Self {
        let mut harness = Self {
            measurements: BTreeMap::new(),
            gates: Vec::new(),
            reference_sku,
            app_mix,
        };
        
        harness.setup_default_gates();
        harness
    }
    
    /// Setup default performance gates based on RaeenOS targets
    fn setup_default_gates(&mut self) {
        self.gates = vec![
            // Input latency: p99 < 2ms @ 90% CPU
            SloGate {
                category: SloCategory::InputLatency,
                name: "input_to_present_latency".to_string(),
                target_p99_us: 2000.0,
                target_p95_us: 1500.0,
                max_drift_percent: 5.0,
                enabled: true,
            },
            // Compositor jitter: p99 ≤ 0.3ms @ 120Hz
            SloGate {
                category: SloCategory::CompositorJitter,
                name: "compositor_frame_jitter".to_string(),
                target_p99_us: 300.0,
                target_p95_us: 200.0,
                max_drift_percent: 5.0,
                enabled: true,
            },
            // IPC RTT: same-core p99 ≤ 3µs
            SloGate {
                category: SloCategory::IpcRtt,
                name: "ipc_same_core_rtt".to_string(),
                target_p99_us: 3.0,
                target_p95_us: 2.0,
                max_drift_percent: 5.0,
                enabled: true,
            },
            // Anonymous page fault: p99 ≤ 15µs
            SloGate {
                category: SloCategory::AnonPageFault,
                name: "anon_page_fault_service".to_string(),
                target_p99_us: 15.0,
                target_p95_us: 10.0,
                max_drift_percent: 5.0,
                enabled: true,
            },
            // TLB shootdown: 64 pages/16 cores p99 ≤ 40µs
            SloGate {
                category: SloCategory::TlbShootdown,
                name: "tlb_shootdown_64p_16c".to_string(),
                target_p99_us: 40.0,
                target_p95_us: 30.0,
                max_drift_percent: 5.0,
                enabled: true,
            },
            // NVMe 4KiB QD=1: p99 ≤ 120µs (hot set)
            SloGate {
                category: SloCategory::NvmeIo,
                name: "nvme_4k_qd1_read".to_string(),
                target_p99_us: 120.0,
                target_p95_us: 80.0,
                max_drift_percent: 5.0,
                enabled: true,
            },
            // Idle power: laptop ≤ 0.8W (screen on)
            SloGate {
                category: SloCategory::IdlePower,
                name: "laptop_idle_power_screen_on".to_string(),
                target_p99_us: 800000.0, // 0.8W in µW
                target_p95_us: 700000.0,
                max_drift_percent: 10.0,
                enabled: true,
            },
            // Audio jitter: p99 < 200µs
            SloGate {
                category: SloCategory::AudioUnderruns,
                name: "audio_buffer_jitter".to_string(),
                target_p99_us: 200.0,
                target_p95_us: 150.0,
                max_drift_percent: 5.0,
                enabled: true,
            },
        ];
    }
    
    /// Record a performance measurement
    pub fn record_measurement(&mut self, measurement: SloMeasurement) {
        self.measurements
            .entry(measurement.category)
            .or_insert_with(Vec::new)
            .push(measurement);
    }
    
    /// Add or update a performance gate
    pub fn add_gate(&mut self, gate: SloGate) {
        if let Some(existing) = self.gates.iter_mut().find(|g| g.category == gate.category && g.name == gate.name) {
            *existing = gate;
        } else {
            self.gates.push(gate);
        }
    }
    
    /// Run all enabled gates and generate results
    pub fn run_gates(&self) -> SloResults {
        let mut gate_results = Vec::new();
        let mut overall_pass = true;
        
        for gate in &self.gates {
            if !gate.enabled {
                continue;
            }
            
            let measurement = self.get_latest_measurement(gate.category);
            let (pass, reason) = if let Some(ref m) = measurement {
                self.evaluate_gate(gate, m)
            } else {
                (false, "No measurement available".to_string())
            };
            
            if !pass {
                overall_pass = false;
            }
            
            gate_results.push(SloGateResult {
                gate: gate.clone(),
                measurement: measurement.clone(),
                pass,
                reason,
            });
        }
        
        let all_measurements: Vec<SloMeasurement> = self.measurements
            .values()
            .flatten()
            .cloned()
            .collect();
        
        SloResults {
            version: "1.0".to_string(),
            timestamp_ns: crate::time::get_timestamp_ns(),
            reference_sku: self.reference_sku.clone(),
            app_mix: self.app_mix.clone(),
            kernel_version: env!("CARGO_PKG_VERSION").to_string(),
            measurements: all_measurements,
            gates: gate_results,
            overall_pass,
        }
    }
    
    /// Get the latest measurement for a category
    fn get_latest_measurement(&self, category: SloCategory) -> Option<SloMeasurement> {
        self.measurements
            .get(&category)?
            .last()
            .cloned()
    }
    
    /// Evaluate a gate against a measurement
    fn evaluate_gate(&self, gate: &SloGate, measurement: &SloMeasurement) -> (bool, String) {
        // Check p99 target
        if measurement.p99 > gate.target_p99_us {
            return (false, format!(
                "p99 {:.2}µs exceeds target {:.2}µs",
                measurement.p99, gate.target_p99_us
            ));
        }
        
        // Check p95 target
        if measurement.p95 > gate.target_p95_us {
            return (false, format!(
                "p95 {:.2}µs exceeds target {:.2}µs",
                measurement.p95, gate.target_p95_us
            ));
        }
        
        (true, "All targets met".to_string())
    }
    
    /// Export results as JSON for CI integration
    pub fn export_json(&self) -> Result<String, &'static str> {
        let results = self.run_gates();
        serde_json::to_string_pretty(&results)
            .map_err(|_| "Failed to serialize SLO results")
    }
    
    /// Check if CI should pass based on gate results
    pub fn ci_should_pass(&self, historical_results: &[SloResults]) -> bool {
        let current = self.run_gates();
        
        // Require two consecutive passes or ≤5% drift vs 7-day median
        if current.overall_pass {
            // Check for consecutive passes
            if let Some(last) = historical_results.last() {
                if last.overall_pass {
                    return true; // Two consecutive passes
                }
            }
            
            // Check drift against 7-day median
            if historical_results.len() >= 7 {
                return self.check_drift_threshold(&current, historical_results);
            }
        }
        
        false
    }
    
    /// Check if current results are within drift threshold of historical median
    fn check_drift_threshold(&self, current: &SloResults, historical: &[SloResults]) -> bool {
        // Take last 7 days of results
        let recent = &historical[historical.len().saturating_sub(7)..];
        
        for gate_result in &current.gates {
            if let Some(measurement) = &gate_result.measurement {
                let historical_p99s: Vec<f64> = recent
                    .iter()
                    .filter_map(|r| {
                        r.gates
                            .iter()
                            .find(|g| g.gate.category == gate_result.gate.category)
                            .and_then(|g| g.measurement.as_ref())
                            .map(|m| m.p99)
                    })
                    .collect();
                
                if !historical_p99s.is_empty() {
                    let median = self.calculate_median(&historical_p99s);
                    let drift_percent = ((measurement.p99 - median) / median * 100.0).abs();
                    
                    if drift_percent > gate_result.gate.max_drift_percent {
                        return false;
                    }
                }
            }
        }
        
        true
    }
    
    /// Calculate median of a sorted vector
    fn calculate_median(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        
        let len = sorted.len();
        if len % 2 == 0 {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        } else {
            sorted[len / 2]
        }
    }
}

/// Macro for easy SLO measurement recording
#[macro_export]
macro_rules! slo_measure {
    ($harness:expr, $category:expr, $test_name:expr, $unit:expr, $samples:expr, $values:expr) => {
        {
            let mut sorted_values = $values.clone();
            sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
            
            let len = sorted_values.len();
            let min = sorted_values[0];
            let max = sorted_values[len - 1];
            let mean = sorted_values.iter().sum::<f64>() / len as f64;
            let median = if len % 2 == 0 {
                (sorted_values[len / 2 - 1] + sorted_values[len / 2]) / 2.0
            } else {
                sorted_values[len / 2]
            };
            
            let p95_idx = ((len as f64 * 0.95) as usize).min(len - 1);
            let p99_idx = ((len as f64 * 0.99) as usize).min(len - 1);
            let p999_idx = ((len as f64 * 0.999) as usize).min(len - 1);
            
            let measurement = $crate::slo::SloMeasurement {
                category: $category,
                test_name: $test_name.to_string(),
                unit: $unit.to_string(),
                samples: $samples,
                min,
                max,
                mean,
                median,
                p95: sorted_values[p95_idx],
                p99: sorted_values[p99_idx],
                p999: sorted_values[p999_idx],
                timestamp_ns: $crate::time::get_timestamp_ns(),
                reference_sku: $harness.reference_sku.clone(),
                app_mix: $harness.app_mix.clone(),
            };
            
            $harness.record_measurement(measurement);
        }
    };
}

/// Global SLO harness instance
static SLO_HARNESS: spin::Mutex<Option<SloHarness>> = spin::Mutex::new(None);

/// Initialize the global SLO harness
pub fn init_slo_harness(reference_sku: String, app_mix: String) {
    let mut harness = SLO_HARNESS.lock();
    *harness = Some(SloHarness::new(reference_sku, app_mix));
}

/// Get a reference to the global SLO harness
pub fn with_slo_harness<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut SloHarness) -> R,
{
    let mut harness = SLO_HARNESS.lock();
    harness.as_mut().map(f)
}

/// Export SLO results to JSON for CI integration
pub fn export_slo_results() -> Option<String> {
    with_slo_harness(|harness| harness.export_json().ok()).flatten()
}

impl fmt::Display for SloCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SloCategory::InputLatency => write!(f, "input_latency"),
            SloCategory::CompositorJitter => write!(f, "compositor_jitter"),
            SloCategory::IpcRtt => write!(f, "ipc_rtt"),
            SloCategory::AnonPageFault => write!(f, "anon_page_fault"),
            SloCategory::TlbShootdown => write!(f, "tlb_shootdown"),
            SloCategory::NvmeIo => write!(f, "nvme_io"),
            SloCategory::IdlePower => write!(f, "idle_power"),
            SloCategory::ChaosFs => write!(f, "chaos_fs"),
            SloCategory::MemoryAlloc => write!(f, "memory_alloc"),
            SloCategory::ContextSwitch => write!(f, "context_switch"),
            SloCategory::InterruptLatency => write!(f, "interrupt_latency"),
            SloCategory::NetworkLatency => write!(f, "network_latency"),
            SloCategory::AudioUnderruns => write!(f, "audio_underruns"),
            SloCategory::FrameDrops => write!(f, "frame_drops"),
        }
    }
}