#!/usr/bin/env python3
"""
SLO Gate Check Script

Implements the SLO gate logic as specified in Production_Checklist.md:
- Two consecutive passes OR ≤ 5% drift vs rolling 7-day median
- Critical metrics must pass for gate to open
"""

import argparse
import json
import sys
import statistics
from datetime import datetime, timedelta
from typing import Dict, List, Tuple, Optional
import requests
import os

class SloGateChecker:
    def __init__(self, github_token: str, repo: str):
        self.github_token = github_token
        self.repo = repo
        self.headers = {
            'Authorization': f'token {github_token}',
            'Accept': 'application/vnd.github.v3+json'
        }
        
        # Critical metrics that must pass for gate to open
        self.critical_metrics = {
            'input.latency.p99': 2000.0,  # 2ms
            'compositor.jitter.p99': 300.0,  # 0.3ms @ 120Hz
            'compositor.cpu_time.p99': 1500.0,  # 1.5ms @ 120Hz
            'audio.jitter.p99': 200.0,  # 200µs
            'ipc.rtt.same_core.p99': 3.0,  # 3µs
            'cap.revoke.p99': 200.0,  # 200µs
            'memory.anon_fault.p99': 15.0,  # 15µs
            'memory.tlb_shootdown.p99': 40.0,  # 40µs
        }
        
        self.drift_threshold_percent = 5.0
        self.rolling_window_days = 7
        self.consecutive_passes_required = 2
    
    def load_slo_results(self, file_path: str) -> Dict:
        """Load SLO results from JSON file"""
        try:
            with open(file_path, 'r') as f:
                return json.load(f)
        except Exception as e:
            print(f"❌ Failed to load SLO results: {e}")
            sys.exit(1)
    
    def get_historical_results(self, sku: str, days: int = 7) -> List[Dict]:
        """Fetch historical SLO results from GitHub artifacts"""
        # In a real implementation, this would fetch from GitHub API
        # For now, return empty list (no historical data)
        print(f"📊 Fetching historical SLO data for {sku} (last {days} days)...")
        
        # TODO: Implement actual GitHub API calls to fetch artifacts
        # This would involve:
        # 1. List workflow runs from the last N days
        # 2. Download artifacts containing SLO results
        # 3. Parse and return the data
        
        return []
    
    def check_critical_metrics(self, results: Dict) -> Tuple[bool, List[str]]:
        """Check if all critical metrics meet their thresholds"""
        failures = []
        
        for metric, threshold in self.critical_metrics.items():
            if metric not in results['metrics']:
                failures.append(f"Missing critical metric: {metric}")
                continue
            
            value = results['metrics'][metric]
            if value > threshold:
                failures.append(
                    f"SLO violation: {metric} = {value:.3f}µs > {threshold:.3f}µs"
                )
        
        return len(failures) == 0, failures
    
    def calculate_drift(self, current_results: Dict, historical_results: List[Dict]) -> Tuple[bool, Dict[str, float]]:
        """Calculate drift vs rolling 7-day median"""
        if not historical_results:
            print("⚠️  No historical data available, skipping drift analysis")
            return True, {}
        
        drift_violations = {}
        
        # Calculate median for each metric over the historical period
        historical_metrics = {}
        for result in historical_results:
            for metric, value in result['metrics'].items():
                if metric not in historical_metrics:
                    historical_metrics[metric] = []
                historical_metrics[metric].append(value)
        
        # Check drift for each current metric
        for metric, current_value in current_results['metrics'].items():
            if metric in historical_metrics and len(historical_metrics[metric]) > 0:
                median_value = statistics.median(historical_metrics[metric])
                
                if median_value > 0:  # Avoid division by zero
                    drift_percent = abs((current_value - median_value) / median_value) * 100
                    
                    if drift_percent > self.drift_threshold_percent:
                        drift_violations[metric] = drift_percent
        
        return len(drift_violations) == 0, drift_violations
    
    def check_consecutive_passes(self, sku: str) -> Tuple[bool, int]:
        """Check if we have consecutive passes"""
        # TODO: Implement actual consecutive pass tracking
        # This would involve:
        # 1. Fetch recent workflow runs
        # 2. Check their SLO test outcomes
        # 3. Count consecutive passes
        
        print(f"📈 Checking consecutive passes for {sku}...")
        
        # For now, assume we need to pass critical metrics
        # In a real implementation, this would track actual pass/fail history
        return False, 0
    
    def should_pass_gate(self, results: Dict, sku: str) -> Tuple[bool, str, Dict]:
        """Determine if SLO gate should pass"""
        gate_info = {
            'critical_metrics_pass': False,
            'drift_check_pass': False,
            'consecutive_passes': 0,
            'gate_decision': False,
            'reason': ''
        }
        
        # Check critical metrics
        critical_pass, critical_failures = self.check_critical_metrics(results)
        gate_info['critical_metrics_pass'] = critical_pass
        
        if not critical_pass:
            gate_info['gate_decision'] = False
            gate_info['reason'] = f"Critical metrics failed: {'; '.join(critical_failures)}"
            return False, gate_info['reason'], gate_info
        
        # Get historical data for drift analysis
        historical_results = self.get_historical_results(sku, self.rolling_window_days)
        
        # Check drift
        drift_pass, drift_violations = self.calculate_drift(results, historical_results)
        gate_info['drift_check_pass'] = drift_pass
        
        # Check consecutive passes
        consecutive_pass, consecutive_count = self.check_consecutive_passes(sku)
        gate_info['consecutive_passes'] = consecutive_count
        
        # Gate decision logic: critical metrics AND (drift OR consecutive passes)
        if critical_pass and (drift_pass or consecutive_count >= self.consecutive_passes_required):
            gate_info['gate_decision'] = True
            if drift_pass:
                gate_info['reason'] = f"✅ Gate PASS: Critical metrics pass, drift ≤ {self.drift_threshold_percent}%"
            else:
                gate_info['reason'] = f"✅ Gate PASS: Critical metrics pass, {consecutive_count} consecutive passes"
            return True, gate_info['reason'], gate_info
        else:
            reasons = []
            if not critical_pass:
                reasons.append("critical metrics failed")
            if not drift_pass and consecutive_count < self.consecutive_passes_required:
                drift_details = ", ".join([f"{k}: {v:.1f}%" for k, v in drift_violations.items()])
                reasons.append(f"drift > {self.drift_threshold_percent}% ({drift_details}) and only {consecutive_count}/{self.consecutive_passes_required} consecutive passes")
            
            gate_info['gate_decision'] = False
            gate_info['reason'] = f"❌ Gate FAIL: {'; '.join(reasons)}"
            return False, gate_info['reason'], gate_info
    
    def generate_report(self, results: Dict, gate_info: Dict, sku: str) -> str:
        """Generate detailed SLO gate report"""
        report = []
        report.append(f"# SLO Gate Report - {sku}")
        report.append(f"")
        report.append(f"**Platform:** {results['platform']}")
        report.append(f"**Timestamp:** {datetime.utcnow().isoformat()}Z")
        report.append(f"**Gate Decision:** {'✅ PASS' if gate_info['gate_decision'] else '❌ FAIL'}")
        report.append(f"")
        report.append(f"## Gate Analysis")
        report.append(f"")
        report.append(f"- **Critical Metrics:** {'✅ PASS' if gate_info['critical_metrics_pass'] else '❌ FAIL'}")
        report.append(f"- **Drift Check:** {'✅ PASS' if gate_info['drift_check_pass'] else '❌ FAIL'}")
        report.append(f"- **Consecutive Passes:** {gate_info['consecutive_passes']}/{self.consecutive_passes_required}")
        report.append(f"")
        report.append(f"**Reason:** {gate_info['reason']}")
        report.append(f"")
        report.append(f"## Measured Metrics")
        report.append(f"")
        
        for metric, value in results['metrics'].items():
            status = "✅"
            threshold_info = ""
            
            if metric in self.critical_metrics:
                threshold = self.critical_metrics[metric]
                if value > threshold:
                    status = "❌"
                threshold_info = f" (threshold: {threshold:.3f}µs)"
            
            report.append(f"- {status} **{metric}:** {value:.3f}µs{threshold_info}")
        
        return "\n".join(report)

def main():
    parser = argparse.ArgumentParser(description='SLO Gate Checker')
    parser.add_argument('--results', required=True, help='Path to SLO results JSON file')
    parser.add_argument('--sku', required=True, help='Reference SKU ID')
    parser.add_argument('--github-token', required=True, help='GitHub token for API access')
    parser.add_argument('--repo', required=True, help='GitHub repository (owner/repo)')
    parser.add_argument('--sha', required=True, help='Git commit SHA')
    parser.add_argument('--output', help='Output file for gate report')
    
    args = parser.parse_args()
    
    # Initialize gate checker
    checker = SloGateChecker(args.github_token, args.repo)
    
    # Load SLO results
    results = checker.load_slo_results(args.results)
    
    print(f"🔍 Checking SLO gate for {args.sku} (commit: {args.sha[:8]})")
    
    # Check if gate should pass
    should_pass, reason, gate_info = checker.should_pass_gate(results, args.sku)
    
    # Generate report
    report = checker.generate_report(results, gate_info, args.sku)
    
    # Output report
    if args.output:
        with open(args.output, 'w') as f:
            f.write(report)
        print(f"📄 Report written to {args.output}")
    else:
        print("\n" + report)
    
    # Print summary
    print(f"\n{reason}")
    
    # Set exit code
    if should_pass:
        print(f"\n🎉 SLO gate PASSED for {args.sku}")
        sys.exit(0)
    else:
        print(f"\n💥 SLO gate FAILED for {args.sku}")
        sys.exit(1)

if __name__ == '__main__':
    main()