"""
Statistics Analyzer for Telemetry Data

Analyzes testing/telemetry/runs.csv to:
- Show performance trends
- Detect regressions
- Generate reports
- Query historical data
"""

import pandas as pd
from pathlib import Path
from datetime import datetime, timedelta


class StatisticsAnalyzer:
    """Analyze telemetry CSV data."""

    def __init__(self, telemetry_dir: Path):
        self.telemetry_dir = Path(telemetry_dir)
        self.csv_path = self.telemetry_dir / 'runs.csv'

        if not self.csv_path.exists():
            print(f"No telemetry data found: {self.csv_path}")
            print("Run tests first: pytest")
            return

        # Load data
        self.df = pd.read_csv(self.csv_path)
        self.df['timestamp'] = pd.to_datetime(self.df['timestamp'])

    def show_trends(self, days=30):
        """Show performance trends over time."""
        if not hasattr(self, 'df'):
            return

        # Make cutoff timezone-aware to match CSV timestamps (UTC)
        cutoff = pd.Timestamp.utcnow() - pd.Timedelta(days=days)
        recent = self.df[self.df['timestamp'] >= cutoff]

        print(f"\n{'='*70}")
        print(f"PERFORMANCE TRENDS (last {days} days)")
        print(f"{'='*70}\n")

        # Group by test_id
        for test_id in recent['test_id'].unique():
            if not test_id:
                continue

            test_data = recent[recent['test_id'] == test_id]

            # Get performance metrics
            if 'pages_per_sec' in test_data.columns:
                pps_data = test_data['pages_per_sec'].dropna()
                if len(pps_data) > 0:
                    mean_pps = pps_data.mean()
                    std_pps = pps_data.std()
                    trend = self._calculate_trend(pps_data.values)

                    print(f"{test_id}:")
                    print(f"  Mean: {mean_pps:.1f} pps")
                    print(f"  Std: {std_pps:.1f}")
                    print(f"  Trend: {trend}")
                    print()

    def check_regression(self, window=10, threshold=0.05):
        """Check for performance regressions."""
        if not hasattr(self, 'df'):
            return

        print(f"\n{'='*70}")
        print(f"REGRESSION DETECTION")
        print(f"{'='*70}\n")

        # Group by test_id
        for test_id in self.df['test_id'].unique():
            if not test_id:
                continue

            test_data = self.df[self.df['test_id'] == test_id].sort_values('timestamp')

            if 'pages_per_sec' in test_data.columns and len(test_data) >= window * 2:
                pps = test_data['pages_per_sec'].dropna()

                if len(pps) >= window * 2:
                    recent_mean = pps.tail(window).mean()
                    previous_mean = pps.tail(window * 2).head(window).mean()

                    change = (recent_mean - previous_mean) / previous_mean

                    if change < -threshold:
                        print(f"⚠️  REGRESSION: {test_id}")
                        print(f"    {change*100:.1f}% slower")
                        print(f"    Previous: {previous_mean:.1f} pps")
                        print(f"    Recent: {recent_mean:.1f} pps")
                        print()
                    elif change > threshold:
                        print(f"✅ IMPROVEMENT: {test_id}")
                        print(f"    {change*100:.1f}% faster")
                        print()

    def generate_report(self, days=7):
        """Generate comprehensive report."""
        if not hasattr(self, 'df'):
            return

        # Make cutoff timezone-aware to match CSV timestamps (UTC)
        cutoff = pd.Timestamp.utcnow() - pd.Timedelta(days=days)
        recent = self.df[self.df['timestamp'] >= cutoff]

        print(f"\n{'='*70}")
        print(f"TEST REPORT (last {days} days)")
        print(f"{'='*70}\n")

        # Summary
        total_runs = len(recent)
        passed = (recent['result'] == 'passed').sum()
        failed = (recent['result'] == 'failed').sum()
        skipped = (recent['result'] == 'skipped').sum()

        print(f"Total runs: {total_runs}")
        print(f"  Passed: {passed} ({passed/total_runs*100:.1f}%)")
        print(f"  Failed: {failed} ({failed/total_runs*100:.1f}%)")
        print(f"  Skipped: {skipped} ({skipped/total_runs*100:.1f}%)")
        print()

        # Performance by worker count
        if 'worker_count' in recent.columns and 'pages_per_sec' in recent.columns:
            print("Performance by worker count:")
            for workers in sorted(recent['worker_count'].dropna().unique()):
                worker_data = recent[recent['worker_count'] == workers]
                mean_pps = worker_data['pages_per_sec'].mean()
                print(f"  {int(workers):2d} workers: {mean_pps:.1f} pps")
            print()

        # Most common failures
        if failed > 0:
            failures = recent[recent['result'] == 'failed']
            print("Most common failures:")
            for test in failures['test_name'].value_counts().head(5).items():
                print(f"  {test[0]}: {test[1]} failures")
            print()

    def interactive_query(self):
        """Interactive CSV query."""
        if not hasattr(self, 'df'):
            return

        print(f"\n{'='*70}")
        print(f"TELEMETRY DATA QUERY")
        print(f"{'='*70}\n")

        print(f"CSV: {self.csv_path}")
        print(f"Total rows: {len(self.df)}")
        print(f"Columns: {len(self.df.columns)}")
        print()

        print("Available fields:")
        for col in sorted(self.df.columns):
            print(f"  - {col}")
        print()

        print("Example queries:")
        print("  df[df['worker_count'] == 4]")
        print("  df[df['result'] == 'failed']")
        print("  df[df['test_category'] == 'correctness']")
        print("  df.groupby('worker_count')['pages_per_sec'].mean()")
        print()

        # Open interactive Python shell with df loaded
        print("Starting interactive mode (type 'exit()' to quit)...")
        import code
        code.interact(local={'df': self.df, 'pd': pd})

    def _calculate_trend(self, values):
        """Calculate trend direction."""
        if len(values) < 2:
            return "insufficient data"

        # Simple linear regression
        x = list(range(len(values)))
        y = values

        n = len(x)
        sum_x = sum(x)
        sum_y = sum(y)
        sum_xy = sum(x[i] * y[i] for i in range(n))
        sum_x2 = sum(x[i]**2 for i in range(n))

        slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x**2)

        if slope > 0.5:
            return "↑ improving"
        elif slope < -0.5:
            return "↓ regressing"
        else:
            return "→ stable"
