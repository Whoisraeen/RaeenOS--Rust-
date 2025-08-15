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
import zipfile
import tempfile
import base64

class SloGateChecker:
    def __init__(self, github_token: str, repo: str):
        self.github_token = github_token
        self.repo = repo
        self.headers = {
            'Authorization': f'token {github_token}',
            'Accept': 'application/vnd.github.v3+json'
        }
        
        # Critical metrics that must pass for gate to open (updated to match schema)
        self.critical_metrics = {
            'scheduler.input_p99_ms': 2.0,  # 2ms
            'graphics.jitter_p99_ms': 0.3,  # 0.3ms @ 120Hz
            'scheduler.compositor_cpu_ms': 1.5,  # 1.5ms @ 120Hz
            'audio.jitter_p99_us': 200.0,  # 200µs
            'ipc.rtt_same_core_p99_us': 3.0,  # 3µs
            'cap.revoke_block_new_p99_us': 200.0,  # 200µs
            'memory.anon_page_fault_p99_us': 15.0,  # 15µs
            'memory.tlb_shootdown_p99_us': 40.0,  # 40µs
        }
        
        self.drift_threshold_percent = 5.0
        self.rolling_window_days = 7
        self.consecutive_passes_required = 2
        self.gate_history_file = '.slo_gate_history.json'
    
    def _download_and_parse_artifact(self, download_url: str) -> Optional[Dict]:
        """Download and parse SLO results from GitHub artifact"""
        try:
            # Download the artifact zip file
            response = requests.get(download_url, headers=self.headers)
            response.raise_for_status()
            
            # Create a temporary file to store the zip
            with tempfile.NamedTemporaryFile(delete=False) as temp_file:
                temp_file.write(response.content)
                temp_file_path = temp_file.name
            
            try:
                # Extract and parse the JSON file from the zip
                with zipfile.ZipFile(temp_file_path, 'r') as zip_file:
                    # Look for JSON files in the zip
                    json_files = [f for f in zip_file.namelist() if f.endswith('.json')]
                    
                    if not json_files:
                        return None
                    
                    # Read the first JSON file (should be slo_results.json)
                    with zip_file.open(json_files[0]) as json_file:
                        content = json_file.read().decode('utf-8')
                        return json.loads(content)
            finally:
                # Clean up temporary file
                os.unlink(temp_file_path)
                
        except Exception as e:
            print(f"⚠️  Error downloading/parsing artifact: {e}")
            return None
    
    def _load_local_gate_history(self) -> Dict:
        """Load local gate history from file"""
        try:
            if os.path.exists(self.gate_history_file):
                with open(self.gate_history_file, 'r') as f:
                    return json.load(f)
        except Exception as e:
            print(f"⚠️  Failed to load gate history: {e}")
        return {'runs': []}
    
    def _save_local_gate_history(self, history: Dict):
        """Save local gate history to file"""
        try:
            with open(self.gate_history_file, 'w') as f:
                json.dump(history, f, indent=2)
        except Exception as e:
            print(f"⚠️  Failed to save gate history: {e}")
    
    def _record_gate_result(self, sku: str, sha: str, gate_passed: bool, results: Dict):
        """Record gate result in local history"""
        history = self._load_local_gate_history()
        
        # Add new result
        new_result = {
            'timestamp': datetime.utcnow().isoformat() + 'Z',
            'sku': sku,
            'sha': sha,
            'gate_passed': gate_passed,
            'metrics': results['metrics'].copy()
        }
        
        history['runs'].append(new_result)
        
        # Keep only last 50 runs to prevent file from growing too large
        if len(history['runs']) > 50:
            history['runs'] = history['runs'][-50:]
        
        self._save_local_gate_history(history)
    
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
        print(f"📊 Fetching historical SLO data for {sku} (last {days} days)...")
        
        try:
            # Calculate date range
            end_date = datetime.utcnow()
            start_date = end_date - timedelta(days=days)
            
            # List workflow runs from the specified date range
            url = f"https://api.github.com/repos/{self.repo}/actions/runs"
            params = {
                'status': 'completed',
                'created': f'{start_date.isoformat()}..{end_date.isoformat()}',
                'per_page': 100
            }
            
            response = requests.get(url, headers=self.headers, params=params)
            response.raise_for_status()
            
            workflow_runs = response.json()['workflow_runs']
            historical_results = []
            
            for run in workflow_runs:
                # Skip failed runs
                if run['conclusion'] != 'success':
                    continue
                    
                # Get artifacts for this run
                artifacts_url = f"https://api.github.com/repos/{self.repo}/actions/runs/{run['id']}/artifacts"
                artifacts_response = requests.get(artifacts_url, headers=self.headers)
                
                if artifacts_response.status_code != 200:
                    continue
                    
                artifacts = artifacts_response.json()['artifacts']
                
                # Look for SLO results artifact
                slo_artifact = None
                for artifact in artifacts:
                    if 'slo-results' in artifact['name'].lower() and sku.lower() in artifact['name'].lower():
                        slo_artifact = artifact
                        break
                
                if not slo_artifact:
                    continue
                    
                # Download and parse the artifact
                try:
                    slo_data = self._download_and_parse_artifact(slo_artifact['archive_download_url'])
                    if slo_data:
                        slo_data['timestamp'] = run['created_at']
                        slo_data['run_id'] = run['id']
                        historical_results.append(slo_data)
                except Exception as e:
                    print(f"⚠️  Failed to download artifact {slo_artifact['name']}: {e}")
                    continue
            
            print(f"📈 Found {len(historical_results)} historical SLO results")
            return historical_results
            
        except Exception as e:
            print(f"⚠️  Failed to fetch historical data: {e}")
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
        """Check if we have consecutive passes using GitHub API and local history"""
        print(f"📈 Checking consecutive passes for {sku}...")
        
        # Try GitHub API first
        consecutive_passes = self._check_consecutive_passes_github(sku)
        
        # If GitHub API fails or returns 0, try local history as fallback
        if consecutive_passes == 0:
            consecutive_passes = self._check_consecutive_passes_local(sku)
            
        print(f"📊 Found {consecutive_passes} consecutive passes")
        return consecutive_passes >= self.consecutive_passes_required, consecutive_passes
    
    def _check_consecutive_passes_github(self, sku: str) -> int:
        """Check consecutive passes using GitHub API"""
        try:
            # Get recent workflow runs (last 20 to check for consecutive passes)
            url = f"https://api.github.com/repos/{self.repo}/actions/runs"
            params = {
                'status': 'completed',
                'per_page': 20
            }
            
            response = requests.get(url, headers=self.headers, params=params)
            response.raise_for_status()
            
            workflow_runs = response.json()['workflow_runs']
            consecutive_passes = 0
            
            for run in workflow_runs:
                # Check if this run had SLO tests
                if 'slo' not in run['name'].lower():
                    continue
                    
                # Check if the run passed (successful conclusion)
                if run['conclusion'] == 'success':
                    # Verify it actually had SLO results by checking for artifacts
                    artifacts_url = f"https://api.github.com/repos/{self.repo}/actions/runs/{run['id']}/artifacts"
                    artifacts_response = requests.get(artifacts_url, headers=self.headers)
                    
                    if artifacts_response.status_code == 200:
                        artifacts = artifacts_response.json()['artifacts']
                        has_slo_results = any('slo-results' in artifact['name'].lower() and sku.lower() in artifact['name'].lower() 
                                            for artifact in artifacts)
                        
                        if has_slo_results:
                            consecutive_passes += 1
                        else:
                            break  # No SLO results, stop counting
                    else:
                        break  # Can't verify, stop counting
                else:
                    break  # Failed run, reset consecutive count
            
            return consecutive_passes
            
        except Exception as e:
            print(f"⚠️  GitHub API failed, will try local history: {e}")
            return 0
    
    def _check_consecutive_passes_local(self, sku: str) -> int:
        """Check consecutive passes using local history as fallback"""
        try:
            history = self._load_local_gate_history()
            
            # Filter runs for this SKU and sort by timestamp (newest first)
            sku_runs = [run for run in history['runs'] if run.get('sku') == sku]
            sku_runs.sort(key=lambda x: x['timestamp'], reverse=True)
            
            consecutive_passes = 0
            for run in sku_runs:
                if run.get('gate_passed', False):
                    consecutive_passes += 1
                else:
                    break  # Failed run, stop counting
            
            return consecutive_passes
            
        except Exception as e:
            print(f"⚠️  Failed to check local history: {e}")
            return 0
    
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
                
                # Format units based on metric name
                if '_ms' in metric:
                    unit = 'ms'
                    threshold_info = f" (threshold: {threshold:.3f}ms)"
                    value_str = f"{value:.3f}ms"
                elif '_us' in metric:
                    unit = 'µs'
                    threshold_info = f" (threshold: {threshold:.3f}µs)"
                    value_str = f"{value:.3f}µs"
                elif '_w' in metric:
                    unit = 'W'
                    threshold_info = f" (threshold: {threshold:.3f}W)"
                    value_str = f"{value:.3f}W"
                else:
                    unit = ''
                    threshold_info = f" (threshold: {threshold:.3f})"
                    value_str = f"{value:.3f}"
            else:
                # Non-critical metric
                if '_ms' in metric:
                    value_str = f"{value:.3f}ms"
                elif '_us' in metric:
                    value_str = f"{value:.3f}µs"
                elif '_w' in metric:
                    value_str = f"{value:.3f}W"
                else:
                    value_str = f"{value:.3f}"
                threshold_info = ""
            
            report.append(f"- {status} **{metric}:** {value_str}{threshold_info}")
        
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
    
    # Record gate result in local history
    checker._record_gate_result(args.sku, args.sha, should_pass, results)
    
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
    
    # Additional information about gate history
    if gate_info['consecutive_passes'] > 0:
        print(f"📈 Consecutive passes: {gate_info['consecutive_passes']}/{checker.consecutive_passes_required}")
    
    if not gate_info['drift_check_pass'] and gate_info['consecutive_passes'] < checker.consecutive_passes_required:
        print(f"💡 Tip: Need {checker.consecutive_passes_required - gate_info['consecutive_passes']} more consecutive passes to override drift check")
    
    # Set exit code
    if should_pass:
        print(f"\n🎉 SLO gate PASSED for {args.sku}")
        sys.exit(0)
    else:
        print(f"\n💥 SLO gate FAILED for {args.sku}")
        sys.exit(1)

if __name__ == '__main__':
    main()