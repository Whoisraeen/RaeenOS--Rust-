//! SLO Harness Tests and Examples
//!
//! This module demonstrates how to use the SLO harness for performance measurement
//! and gate enforcement in RaeenOS.

use crate::slo::*;
use crate::slo_measure;
use alloc::vec;
use alloc::string::ToString;

/// Example SLO measurement for input latency
pub fn test_input_latency_measurement() {
    // Initialize SLO harness
    init_slo_harness("test-sku".to_string(), "desktop-mix".to_string());
    
    // Simulate input latency measurements (in microseconds)
    let latency_samples = vec![
        1200.0, 1150.0, 1300.0, 1100.0, 1250.0,
        1180.0, 1220.0, 1160.0, 1280.0, 1140.0,
        1320.0, 1190.0, 1240.0, 1170.0, 1290.0,
        1130.0, 1260.0, 1200.0, 1210.0, 1180.0,
    ];
    
    // Record measurement using the macro
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::InputLatency,
            "keyboard_to_display",
            "microseconds",
            latency_samples.len() as u64,
            latency_samples
        );
    });
    
    crate::serial_println!("[SLO Test] Input latency measurement recorded");
}

/// Example SLO measurement for compositor jitter
pub fn test_compositor_jitter_measurement() {
    // Simulate compositor frame jitter measurements (in microseconds)
    let jitter_samples = vec![
        150.0, 120.0, 180.0, 110.0, 160.0,
        140.0, 170.0, 130.0, 190.0, 125.0,
        200.0, 135.0, 175.0, 145.0, 185.0,
        115.0, 165.0, 155.0, 195.0, 142.0,
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::CompositorJitter,
            "frame_timing_jitter",
            "microseconds",
            jitter_samples.len() as u64,
            jitter_samples
        );
    });
    
    crate::serial_println!("[SLO Test] Compositor jitter measurement recorded");
}

/// Example SLO measurement for IPC round-trip time
pub fn test_ipc_rtt_measurement() {
    // Simulate IPC RTT measurements (in microseconds)
    let rtt_samples = vec![
        2.1, 1.8, 2.3, 1.9, 2.0,
        2.2, 1.7, 2.4, 1.6, 2.1,
        2.5, 1.9, 2.0, 2.3, 1.8,
        2.2, 2.1, 1.7, 2.4, 2.0,
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::IpcRtt,
            "same_core_message_passing",
            "microseconds",
            rtt_samples.len() as u64,
            rtt_samples
        );
    });
    
    crate::serial_println!("[SLO Test] IPC RTT measurement recorded");
}

/// Example SLO measurement for page fault service time
pub fn test_page_fault_measurement() {
    // Simulate anonymous page fault service times (in microseconds)
    let fault_samples = vec![
        12.0, 10.5, 13.2, 9.8, 11.5,
        12.8, 10.2, 14.1, 9.5, 11.8,
        13.5, 10.8, 12.2, 11.0, 13.0,
        9.2, 12.5, 11.3, 13.8, 10.7,
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::AnonPageFault,
            "anonymous_page_allocation",
            "microseconds",
            fault_samples.len() as u64,
            fault_samples
        );
    });
    
    crate::serial_println!("[SLO Test] Page fault measurement recorded");
}

/// Run all SLO gate evaluations and display results
pub fn test_slo_gate_evaluation() {
    // Run all measurements first
    test_input_latency_measurement();
    test_compositor_jitter_measurement();
    test_ipc_rtt_measurement();
    test_page_fault_measurement();
    
    // Evaluate gates and display results
    if let Some(results_json) = export_slo_results() {
        crate::serial_println!("[SLO Test] Gate evaluation results:");
        crate::serial_println!("{}", results_json);
        
        // Check if gates pass
        with_slo_harness(|harness| {
            let results = harness.run_gates();
            if results.overall_pass {
                crate::serial_println!("[SLO Test] ✅ All performance gates PASSED");
            } else {
                crate::serial_println!("[SLO Test] ❌ Some performance gates FAILED");
                for gate_result in &results.gates {
                    if !gate_result.pass {
                        crate::serial_println!(
                            "[SLO Test]   - {}: {}",
                            gate_result.gate.name,
                            gate_result.reason
                        );
                    }
                }
            }
        });
    } else {
        crate::serial_println!("[SLO Test] Failed to export SLO results");
    }
}

/// Example of adding a custom SLO gate
pub fn test_custom_slo_gate() {
    let custom_gate = SloGate {
        category: SloCategory::MemoryAlloc,
        name: "heap_allocation_latency".to_string(),
        target_p99_us: 50.0,  // 50µs target
        target_p95_us: 30.0,  // 30µs target
        max_drift_percent: 10.0,
        enabled: true,
    };
    
    with_slo_harness(|harness| {
        harness.add_gate(custom_gate);
        crate::serial_println!("[SLO Test] Custom memory allocation gate added");
    });
}

/// Comprehensive SLO test suite
pub fn run_slo_test_suite() {
    crate::serial_println!("[SLO Test] Starting SLO harness test suite...");
    
    // Initialize the harness
    init_slo_harness("RaeenOS-Test".to_string(), "development-workload".to_string());
    
    // Add custom gates
    test_custom_slo_gate();
    
    // Run measurements and gate evaluation
    test_slo_gate_evaluation();
    
    crate::serial_println!("[SLO Test] SLO harness test suite completed");
}

/// Performance measurement example using the PerformanceCounter
pub fn test_performance_counter_integration() {
    use crate::time::PerformanceCounter;
    
    crate::serial_println!("[SLO Test] Testing performance counter integration...");
    
    // Simulate some work and measure it
    let counter = PerformanceCounter::new();
    
    // Simulate work (busy loop)
    for _ in 0..1000 {
        core::hint::spin_loop();
    }
    
    let elapsed_ns = counter.elapsed_ns();
    let elapsed_us = counter.elapsed_us();
    
    crate::serial_println!(
        "[SLO Test] Simulated work took {} ns ({} µs)",
        elapsed_ns,
        elapsed_us
    );
    
    // Record this as an SLO measurement
    with_slo_harness(|harness| {
        let measurement = SloMeasurement {
            category: SloCategory::MemoryAlloc,
            test_name: "simulated_work".to_string(),
            unit: "microseconds".to_string(),
            samples: 1,
            min: elapsed_us as f64,
            max: elapsed_us as f64,
            mean: elapsed_us as f64,
            median: elapsed_us as f64,
            p95: elapsed_us as f64,
            p99: elapsed_us as f64,
            p999: elapsed_us as f64,
            timestamp_ns: crate::time::get_timestamp_ns(),
            reference_sku: "RaeenOS-Test".to_string(),
            app_mix: "development-workload".to_string(),
        };
        
        harness.record_measurement(measurement);
    });
    
    crate::serial_println!("[SLO Test] Performance counter integration test completed");
}