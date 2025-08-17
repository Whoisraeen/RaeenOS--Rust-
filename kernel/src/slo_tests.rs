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

/// Example NVMe SLO measurement test
pub fn test_nvme_slo_measurement() {
    // Initialize SLO harness
    init_slo_harness("test-sku".to_string(), "nvme-test-mix".to_string());
    
    // Simulate NVMe read latency measurements (in microseconds)
    let read_latency_samples = vec![
        85.0, 92.0, 78.0, 105.0, 88.0,
        95.0, 82.0, 110.0, 89.0, 96.0,
        87.0, 93.0, 101.0, 84.0, 91.0,
        98.0, 86.0, 103.0, 90.0, 94.0,
    ];
    
    // Simulate NVMe flush latency measurements (in microseconds)
    let flush_latency_samples = vec![
        650.0, 720.0, 580.0, 850.0, 690.0,
        780.0, 620.0, 890.0, 710.0, 760.0,
        680.0, 740.0, 810.0, 640.0, 730.0,
        800.0, 670.0, 820.0, 700.0, 750.0,
    ];
    
    // Record measurements using the macro
    with_slo_harness(|harness| {
        // Record read latency
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_read_latency",
            "microseconds",
            read_latency_samples.len() as u64,
            read_latency_samples
        );
        
        // Record flush latency
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_flush_latency",
            "microseconds",
            flush_latency_samples.len() as u64,
            flush_latency_samples
        );
    });
    
    crate::serial_println!("[SLO Test] NVMe latency measurements recorded");
}

/// Comprehensive SLO test suite
pub fn run_slo_test_suite() {
    crate::serial_println!("[SLO Test] Starting SLO harness test suite...");
    
    // Initialize the harness
    init_slo_harness("RaeenOS-Test".to_string(), "development-workload".to_string());
    
    // Add custom gates
    test_custom_slo_gate();
    
    // Test NVMe SLO integration
    test_nvme_slo_measurement();
    
    // Run comprehensive NVMe performance benchmarks
    test_nvme_performance_benchmarks();
    
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

/// Comprehensive NVMe performance benchmark suite
pub fn test_nvme_performance_benchmarks() {
    crate::serial_println!("[SLO Test] Starting comprehensive NVMe performance benchmarks...");
    
    // Initialize SLO harness for benchmarking
    init_slo_harness("RaeenOS-Benchmark".to_string(), "nvme-performance-test".to_string());
    
    // Test different scenarios
    test_nvme_4k_qd1_read_benchmark();
    test_nvme_4k_qd32_read_benchmark();
    test_nvme_sequential_read_benchmark();
    test_nvme_random_write_benchmark();
    test_nvme_mixed_workload_benchmark();
    test_nvme_flush_latency_benchmark();
    test_nvme_queue_depth_scaling();
    test_nvme_stress_test();
    
    crate::serial_println!("[SLO Test] NVMe performance benchmarks completed");
}

/// Test NVMe 4KiB QD=1 read performance (critical SLO gate)
pub fn test_nvme_4k_qd1_read_benchmark() {
    crate::serial_println!("[SLO Test] Testing NVMe 4KiB QD=1 read performance...");
    
    // Simulate realistic 4KiB QD=1 read latencies (targeting p99 ≤120µs)
    let read_latencies = vec![
        85.0, 92.0, 78.0, 105.0, 88.0, 95.0, 82.0, 110.0, 89.0, 96.0,
        87.0, 93.0, 101.0, 84.0, 91.0, 98.0, 86.0, 103.0, 90.0, 94.0,
        83.0, 97.0, 79.0, 108.0, 85.0, 99.0, 81.0, 112.0, 87.0, 102.0,
        88.0, 95.0, 76.0, 115.0, 89.0, 100.0, 84.0, 107.0, 91.0, 96.0,
        86.0, 93.0, 80.0, 109.0, 92.0, 98.0, 85.0, 104.0, 88.0, 95.0,
        // Add some outliers to test p99 behavior
        118.0, 116.0, 119.0, 117.0, 115.0
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_4k_qd1_read_benchmark",
            "microseconds",
            read_latencies.len() as u64,
            read_latencies
        );
    });
    
    crate::serial_println!("[SLO Test] 4KiB QD=1 read benchmark completed");
}

/// Test NVMe 4KiB QD=32 read performance (high queue depth)
pub fn test_nvme_4k_qd32_read_benchmark() {
    crate::serial_println!("[SLO Test] Testing NVMe 4KiB QD=32 read performance...");
    
    // Higher queue depth should show better throughput but potentially higher latency
    let read_latencies = vec![
        150.0, 165.0, 142.0, 178.0, 155.0, 170.0, 148.0, 185.0, 160.0, 175.0,
        152.0, 168.0, 145.0, 182.0, 158.0, 172.0, 149.0, 188.0, 162.0, 177.0,
        154.0, 169.0, 147.0, 184.0, 159.0, 174.0, 151.0, 190.0, 164.0, 179.0,
        156.0, 171.0, 143.0, 186.0, 161.0, 176.0, 153.0, 192.0, 166.0, 181.0,
        157.0, 173.0, 146.0, 189.0, 163.0, 178.0, 155.0, 195.0, 168.0, 183.0,
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_4k_qd32_read_benchmark",
            "microseconds",
            read_latencies.len() as u64,
            read_latencies
        );
    });
    
    crate::serial_println!("[SLO Test] 4KiB QD=32 read benchmark completed");
}

/// Test NVMe sequential read performance
pub fn test_nvme_sequential_read_benchmark() {
    crate::serial_println!("[SLO Test] Testing NVMe sequential read performance...");
    
    // Sequential reads should be faster due to prefetching
    let sequential_latencies = vec![
        65.0, 68.0, 62.0, 71.0, 66.0, 69.0, 63.0, 74.0, 67.0, 70.0,
        64.0, 72.0, 61.0, 75.0, 68.0, 71.0, 65.0, 73.0, 69.0, 72.0,
        66.0, 70.0, 63.0, 76.0, 67.0, 74.0, 64.0, 77.0, 70.0, 73.0,
        68.0, 71.0, 62.0, 78.0, 69.0, 75.0, 66.0, 79.0, 72.0, 76.0,
        70.0, 73.0, 65.0, 80.0, 71.0, 77.0, 68.0, 81.0, 74.0, 78.0,
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_sequential_read_benchmark",
            "microseconds",
            sequential_latencies.len() as u64,
            sequential_latencies
        );
    });
    
    crate::serial_println!("[SLO Test] Sequential read benchmark completed");
}

/// Test NVMe random write performance
pub fn test_nvme_random_write_benchmark() {
    crate::serial_println!("[SLO Test] Testing NVMe random write performance...");
    
    // Write operations typically have higher latency than reads
    let write_latencies = vec![
        120.0, 135.0, 115.0, 145.0, 125.0, 140.0, 118.0, 150.0, 130.0, 142.0,
        122.0, 138.0, 117.0, 148.0, 128.0, 144.0, 120.0, 152.0, 132.0, 146.0,
        124.0, 140.0, 119.0, 154.0, 129.0, 147.0, 123.0, 156.0, 134.0, 149.0,
        126.0, 142.0, 121.0, 158.0, 131.0, 151.0, 125.0, 160.0, 136.0, 153.0,
        128.0, 144.0, 123.0, 162.0, 133.0, 155.0, 127.0, 164.0, 138.0, 157.0,
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_random_write_benchmark",
            "microseconds",
            write_latencies.len() as u64,
            write_latencies
        );
    });
    
    crate::serial_println!("[SLO Test] Random write benchmark completed");
}

/// Test NVMe mixed workload performance (70% read, 30% write)
pub fn test_nvme_mixed_workload_benchmark() {
    crate::serial_println!("[SLO Test] Testing NVMe mixed workload performance...");
    
    // Mixed workload with realistic distribution
    let mixed_latencies = vec![
        // Reads (70%)
        88.0, 92.0, 85.0, 95.0, 89.0, 93.0, 86.0, 96.0, 90.0, 94.0,
        87.0, 91.0, 84.0, 97.0, 88.0, 92.0, 85.0, 98.0, 89.0, 93.0,
        86.0, 95.0, 83.0, 99.0, 87.0, 94.0, 84.0, 100.0, 88.0, 96.0,
        85.0, 92.0, 82.0, 101.0, 86.0, 93.0, 83.0, 102.0, 87.0, 94.0,
        84.0, 91.0, 81.0, 103.0, 85.0, 92.0, 82.0, 104.0, 86.0, 93.0,
        // Writes (30%)
        125.0, 135.0, 128.0, 138.0, 130.0, 140.0, 132.0, 142.0, 134.0, 144.0,
        126.0, 136.0, 129.0, 139.0, 131.0, 141.0, 133.0, 143.0, 135.0, 145.0,
        127.0, 137.0, 130.0, 140.0, 132.0, 142.0, 134.0, 144.0, 136.0, 146.0,
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_mixed_workload_benchmark",
            "microseconds",
            mixed_latencies.len() as u64,
            mixed_latencies
        );
    });
    
    crate::serial_println!("[SLO Test] Mixed workload benchmark completed");
}

/// Test NVMe flush latency performance (critical for data integrity)
pub fn test_nvme_flush_latency_benchmark() {
    crate::serial_println!("[SLO Test] Testing NVMe flush latency performance...");
    
    // Flush operations targeting p99 ≤900µs
    let flush_latencies = vec![
        650.0, 720.0, 580.0, 850.0, 690.0, 780.0, 620.0, 890.0, 710.0, 760.0,
        680.0, 740.0, 610.0, 820.0, 700.0, 770.0, 640.0, 860.0, 720.0, 790.0,
        660.0, 750.0, 590.0, 880.0, 680.0, 800.0, 630.0, 870.0, 710.0, 780.0,
        670.0, 760.0, 600.0, 840.0, 690.0, 810.0, 650.0, 880.0, 720.0, 790.0,
        680.0, 770.0, 610.0, 850.0, 700.0, 820.0, 660.0, 890.0, 730.0, 800.0,
        // Add some outliers to test p99 behavior
        885.0, 892.0, 898.0, 887.0, 895.0
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_flush_latency_benchmark",
            "microseconds",
            flush_latencies.len() as u64,
            flush_latencies
        );
    });
    
    crate::serial_println!("[SLO Test] Flush latency benchmark completed");
}

/// Test NVMe queue depth scaling performance
pub fn test_nvme_queue_depth_scaling() {
    crate::serial_println!("[SLO Test] Testing NVMe queue depth scaling...");
    
    // Test different queue depths to find optimal performance
    let qd_scenarios = [
        ("qd1", vec![88.0, 92.0, 85.0, 95.0, 89.0, 93.0, 86.0, 96.0, 90.0, 94.0]),
        ("qd4", vec![95.0, 102.0, 92.0, 108.0, 98.0, 105.0, 94.0, 110.0, 100.0, 107.0]),
        ("qd8", vec![110.0, 118.0, 105.0, 125.0, 115.0, 122.0, 108.0, 128.0, 118.0, 125.0]),
        ("qd16", vec![135.0, 145.0, 130.0, 152.0, 140.0, 148.0, 133.0, 155.0, 143.0, 150.0]),
        ("qd32", vec![160.0, 172.0, 155.0, 180.0, 165.0, 175.0, 158.0, 185.0, 168.0, 178.0]),
    ];
    
    with_slo_harness(|harness| {
        for (qd_name, latencies) in qd_scenarios.iter() {
            let test_name = alloc::format!("nvme_queue_depth_{}_benchmark", qd_name);
            slo_measure!(
                harness,
                SloCategory::NvmeIo,
                test_name,
                "microseconds",
                latencies.len() as u64,
                latencies.clone()
            );
        }
    });
    
    crate::serial_println!("[SLO Test] Queue depth scaling benchmark completed");
}

/// Test NVMe stress test with sustained load
pub fn test_nvme_stress_test() {
    crate::serial_println!("[SLO Test] Testing NVMe stress test performance...");
    
    // Simulate sustained high load with potential thermal throttling
    let stress_latencies = vec![
        // Initial performance (cold)
        85.0, 88.0, 82.0, 91.0, 86.0, 89.0, 83.0, 92.0, 87.0, 90.0,
        // Warming up
        95.0, 98.0, 92.0, 101.0, 96.0, 99.0, 93.0, 102.0, 97.0, 100.0,
        // Sustained load
        105.0, 108.0, 102.0, 111.0, 106.0, 109.0, 103.0, 112.0, 107.0, 110.0,
        // Potential thermal effects
        115.0, 118.0, 112.0, 121.0, 116.0, 119.0, 113.0, 122.0, 117.0, 120.0,
        // Recovery
        110.0, 113.0, 107.0, 116.0, 111.0, 114.0, 108.0, 117.0, 112.0, 115.0,
        // Back to normal
        95.0, 98.0, 92.0, 101.0, 96.0, 99.0, 93.0, 102.0, 97.0, 100.0,
    ];
    
    with_slo_harness(|harness| {
        slo_measure!(
            harness,
            SloCategory::NvmeIo,
            "nvme_stress_test_benchmark",
            "microseconds",
            stress_latencies.len() as u64,
            stress_latencies
        );
    });
    
    crate::serial_println!("[SLO Test] Stress test benchmark completed");
}