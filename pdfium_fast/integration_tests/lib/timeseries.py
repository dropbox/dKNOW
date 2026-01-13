"""
Time Series Analysis for Test Telemetry

Analyzes testing/telemetry/runs.csv to generate:
- Performance trends over time
- Statistical summaries
- Regression detection
- Comparative analysis (git commits, worker counts, PDFs)
- Exportable reports (JSON, CSV, plots)
"""

import pandas as pd
import json
from pathlib import Path
from datetime import datetime, timedelta
from typing import Dict, List, Optional


class TimeSeriesAnalyzer:
    """Comprehensive time series analysis for test telemetry."""

    def __init__(self, telemetry_dir: Path):
        self.telemetry_dir = Path(telemetry_dir)
        self.csv_path = self.telemetry_dir / 'runs.csv'
        self.stats_dir = self.telemetry_dir / 'statistics'
        self.stats_dir.mkdir(exist_ok=True)

        if not self.csv_path.exists():
            print(f"No telemetry data: {self.csv_path}")
            return

        # Load data
        self.df = pd.read_csv(self.csv_path)
        self.df['timestamp'] = pd.to_datetime(self.df['timestamp'])

    # ========================================================================
    # Time Series Generation
    # ========================================================================

    def generate_performance_timeseries(self, test_id: str = None, days: int = 30) -> pd.DataFrame:
        """
        Generate time series of performance metrics.

        Returns DataFrame with columns:
          - timestamp
          - test_id
          - worker_count
          - pages_per_sec
          - speedup_vs_1w
          - edit_distance
          - git_commit_short
          - load_avg_1m
        """
        if not hasattr(self, 'df'):
            return pd.DataFrame()

        # Filter by time
        cutoff = pd.Timestamp.utcnow() - pd.Timedelta(days=days)
        df = self.df[self.df['timestamp'] >= cutoff].copy()

        # Filter by test if specified
        if test_id:
            df = df[df['test_id'] == test_id]

        # Select relevant columns
        cols = [
            'timestamp', 'test_id', 'test_name', 'pdf_name',
            'worker_count', 'pages_per_sec', 'speedup_vs_1w',
            'text_edit_distance', 'text_similarity',
            'git_commit_short', 'git_branch',
            'load_avg_1m', 'cpu_temp_c',
            'result', 'duration_sec'
        ]

        # Only include columns that exist
        cols = [c for c in cols if c in df.columns]

        return df[cols].sort_values('timestamp')

    def generate_aggregate_stats(self, group_by: str = 'test_id') -> pd.DataFrame:
        """
        Generate aggregate statistics grouped by specified field.

        Args:
            group_by: 'test_id' | 'pdf_name' | 'worker_count' | 'git_commit_short'

        Returns:
            DataFrame with aggregate statistics (mean, std, min, max, count)
        """
        if not hasattr(self, 'df'):
            return pd.DataFrame()

        # Numeric columns to aggregate
        numeric_cols = [
            'duration_sec', 'pages_per_sec', 'speedup_vs_1w',
            'text_edit_distance', 'text_similarity',
            'pixel_diff_pct', 'load_avg_1m'
        ]

        # Only include columns that exist
        numeric_cols = [c for c in numeric_cols if c in self.df.columns]

        # Group and aggregate
        grouped = self.df.groupby(group_by)[numeric_cols].agg(['mean', 'std', 'min', 'max', 'count'])

        # Flatten column names
        grouped.columns = ['_'.join(col).strip() for col in grouped.columns.values]

        # Add pass rate
        result_df = self.df.groupby(group_by).agg({
            'passed': 'sum',
            'failed': 'sum',
            'skipped': 'sum',
        })

        result_df['total_runs'] = result_df['passed'] + result_df['failed'] + result_df['skipped']
        result_df['pass_rate'] = result_df['passed'] / result_df['total_runs']

        return pd.concat([grouped, result_df], axis=1)

    def generate_trend_analysis(self, metric: str = 'pages_per_sec', window: int = 10) -> Dict:
        """
        Analyze trends for a specific metric.

        Args:
            metric: Column name to analyze
            window: Rolling window size for trend detection

        Returns:
            Dict with trend analysis results
        """
        if not hasattr(self, 'df'):
            return {}

        if metric not in self.df.columns:
            return {'error': f'Metric {metric} not found'}

        # Drop NaN values
        data = self.df[['timestamp', 'test_id', metric]].dropna()

        if len(data) < window:
            return {'error': f'Insufficient data (need {window} runs, have {len(data)})'}

        # Group by test_id and analyze each
        trends = {}

        for test_id in data['test_id'].unique():
            if not test_id:
                continue

            test_data = data[data['test_id'] == test_id].sort_values('timestamp')

            if len(test_data) < window:
                continue

            # Calculate rolling mean
            test_data['rolling_mean'] = test_data[metric].rolling(window=window).mean()

            # Recent vs previous
            recent_mean = test_data[metric].tail(window).mean()
            if len(test_data) >= window * 2:
                previous_mean = test_data[metric].tail(window * 2).head(window).mean()
                change_pct = ((recent_mean - previous_mean) / previous_mean) * 100 if previous_mean > 0 else 0
            else:
                previous_mean = None
                change_pct = None

            # Linear regression (simple trend)
            values = test_data[metric].values
            x = list(range(len(values)))
            if len(x) >= 2:
                # Simple linear fit
                n = len(x)
                sum_x = sum(x)
                sum_y = sum(values)
                sum_xy = sum(x[i] * values[i] for i in range(n))
                sum_x2 = sum(xi**2 for xi in x)

                slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x**2) if (n * sum_x2 - sum_x**2) != 0 else 0

                if slope > 0.1:
                    trend_direction = 'improving'
                elif slope < -0.1:
                    trend_direction = 'regressing'
                else:
                    trend_direction = 'stable'
            else:
                slope = 0
                trend_direction = 'insufficient_data'

            trends[test_id] = {
                'current_mean': round(recent_mean, 2),
                'previous_mean': round(previous_mean, 2) if previous_mean else None,
                'change_pct': round(change_pct, 2) if change_pct else None,
                'slope': round(slope, 4),
                'trend': trend_direction,
                'sample_size': len(test_data),
                'std': round(test_data[metric].std(), 2),
                'min': round(test_data[metric].min(), 2),
                'max': round(test_data[metric].max(), 2),
            }

        return trends

    def detect_regressions(self, metric: str = 'pages_per_sec', threshold: float = 0.05, window: int = 10) -> List[Dict]:
        """
        Detect performance regressions.

        Args:
            metric: Metric to analyze
            threshold: Regression threshold (0.05 = 5%)
            window: Window size for comparison

        Returns:
            List of regressions: [{'test_id': ..., 'change_pct': ..., ...}]
        """
        trends = self.generate_trend_analysis(metric, window)

        # Handle error case
        if 'error' in trends:
            return []

        regressions = []
        for test_id, trend_data in trends.items():
            # Skip if trend_data is not a dict
            if not isinstance(trend_data, dict):
                continue

            if trend_data.get('change_pct') and trend_data['change_pct'] < -threshold * 100:
                regressions.append({
                    'test_id': test_id,
                    'change_pct': trend_data['change_pct'],
                    'current': trend_data['current_mean'],
                    'previous': trend_data['previous_mean'],
                    'severity': 'critical' if trend_data['change_pct'] < -10 else 'warning',
                })

        return sorted(regressions, key=lambda x: x['change_pct'])

    # ========================================================================
    # Comparative Analysis
    # ========================================================================

    def compare_git_commits(self, commit1: str, commit2: str, metric: str = 'pages_per_sec') -> Dict:
        """
        Compare performance between two git commits.

        Returns:
            Dict with comparison results
        """
        if not hasattr(self, 'df'):
            return {}

        data1 = self.df[self.df['git_commit_short'] == commit1]
        data2 = self.df[self.df['git_commit_short'] == commit2]

        if len(data1) == 0 or len(data2) == 0:
            return {'error': 'No data for one or both commits'}

        mean1 = data1[metric].mean()
        mean2 = data2[metric].mean()
        change_pct = ((mean2 - mean1) / mean1) * 100 if mean1 > 0 else 0

        return {
            'commit1': commit1,
            'commit2': commit2,
            'metric': metric,
            'commit1_mean': round(mean1, 2),
            'commit2_mean': round(mean2, 2),
            'change_pct': round(change_pct, 2),
            'commit1_samples': len(data1),
            'commit2_samples': len(data2),
        }

    def compare_worker_counts(self) -> pd.DataFrame:
        """
        Compare performance across different worker counts.

        Returns:
            DataFrame with mean performance per worker count
        """
        if not hasattr(self, 'df'):
            return pd.DataFrame()

        # Group by worker count
        worker_stats = self.df.groupby('worker_count').agg({
            'pages_per_sec': ['mean', 'std', 'count'],
            'duration_sec': ['mean', 'std'],
            'text_edit_distance': ['mean', 'max'],
            'passed': 'sum',
            'failed': 'sum',
        })

        return worker_stats

    # ========================================================================
    # Export & Reporting
    # ========================================================================

    def export_timeseries_csv(self, filename: str = 'timeseries.csv', days: int = 30):
        """Export time series data to CSV."""
        ts = self.generate_performance_timeseries(days=days)

        output_path = self.stats_dir / filename
        ts.to_csv(output_path, index=False)

        print(f"✓ Exported time series to: {output_path}")
        print(f"  Rows: {len(ts)}")
        print(f"  Columns: {len(ts.columns)}")

    def export_summary_json(self, filename: str = 'summary.json'):
        """Export summary statistics to JSON."""
        summary = {
            'generated_at': datetime.utcnow().isoformat() + 'Z',
            'total_runs': len(self.df) if hasattr(self, 'df') else 0,
            'date_range': {
                'first': str(self.df['timestamp'].min()) if hasattr(self, 'df') else None,
                'last': str(self.df['timestamp'].max()) if hasattr(self, 'df') else None,
            },
            'trends': self.generate_trend_analysis() if hasattr(self, 'df') else {},
            'regressions': self.detect_regressions() if hasattr(self, 'df') else [],
        }

        output_path = self.stats_dir / filename
        with open(output_path, 'w') as f:
            json.dump(summary, f, indent=2, default=str)

        print(f"✓ Exported summary to: {output_path}")

    def generate_markdown_report(self, filename: str = 'report.md', days: int = 7):
        """Generate comprehensive markdown report."""
        if not hasattr(self, 'df'):
            return

        output_path = self.stats_dir / filename

        cutoff = pd.Timestamp.utcnow() - pd.Timedelta(days=days)
        recent = self.df[self.df['timestamp'] >= cutoff]

        with open(output_path, 'w') as f:
            f.write(f"# Test Performance Report\n\n")
            f.write(f"**Generated:** {datetime.utcnow().isoformat()}Z\n\n")
            f.write(f"**Period:** Last {days} days\n\n")
            f.write(f"---\n\n")

            # Summary
            f.write(f"## Summary\n\n")
            f.write(f"- **Total runs:** {len(recent)}\n")
            f.write(f"- **Passed:** {recent['passed'].sum()} ({recent['passed'].sum()/len(recent)*100:.1f}%)\n")
            f.write(f"- **Failed:** {recent['failed'].sum()} ({recent['failed'].sum()/len(recent)*100:.1f}%)\n")
            f.write(f"- **Skipped:** {recent['skipped'].sum()}\n\n")

            # Performance by worker count
            f.write(f"## Performance by Worker Count\n\n")
            worker_stats = recent.groupby('worker_count')['pages_per_sec'].agg(['mean', 'std', 'count'])
            f.write(worker_stats.to_markdown() + "\n\n")

            # Trends
            f.write(f"## Performance Trends\n\n")
            trends = self.generate_trend_analysis()

            if 'error' in trends:
                f.write(f"Insufficient data for trend analysis\n\n")
            else:
                for test_id, trend in trends.items():
                    if not isinstance(trend, dict):
                        continue
                    f.write(f"### {test_id}\n\n")
                    f.write(f"- **Current:** {trend.get('current_mean', 'N/A')} pps\n")
                    if trend.get('previous_mean'):
                        f.write(f"- **Previous:** {trend['previous_mean']} pps\n")
                        f.write(f"- **Change:** {trend.get('change_pct', 0):+.1f}%\n")
                    f.write(f"- **Trend:** {trend.get('trend', 'N/A')}\n")
                    f.write(f"- **Std dev:** {trend.get('std', 'N/A')}\n\n")

            # Regressions
            regressions = self.detect_regressions()
            if regressions:
                f.write(f"## ⚠️ Regressions Detected\n\n")
                for reg in regressions:
                    f.write(f"### {reg['test_id']}\n\n")
                    f.write(f"- **Change:** {reg['change_pct']:.1f}% slower\n")
                    f.write(f"- **Current:** {reg['current']:.1f} pps\n")
                    f.write(f"- **Previous:** {reg['previous']:.1f} pps\n")
                    f.write(f"- **Severity:** {reg['severity']}\n\n")

            # Top failures
            if recent['failed'].sum() > 0:
                f.write(f"## Most Common Failures\n\n")
                failures = recent[recent['result'] == 'failed']
                top_failures = failures['test_name'].value_counts().head(10)
                for test_name, count in top_failures.items():
                    f.write(f"- **{test_name}:** {count} failures\n")
                f.write("\n")

        print(f"✓ Generated markdown report: {output_path}")

    # ========================================================================
    # Visualization (requires matplotlib - optional)
    # ========================================================================

    def plot_performance_over_time(self, test_id: str = None, metric: str = 'pages_per_sec', days: int = 30):
        """
        Plot performance metric over time.

        Saves plot to testing/telemetry/statistics/
        """
        try:
            import matplotlib.pyplot as plt
            import matplotlib.dates as mdates
        except ImportError:
            print("matplotlib not installed - skipping plot")
            return

        if not hasattr(self, 'df'):
            return

        cutoff = pd.Timestamp.utcnow() - pd.Timedelta(days=days)
        df = self.df[self.df['timestamp'] >= cutoff].copy()

        if test_id:
            df = df[df['test_id'] == test_id]

        if metric not in df.columns:
            print(f"Metric {metric} not found")
            return

        # Drop NaN
        df = df[['timestamp', 'test_id', metric]].dropna()

        if len(df) == 0:
            print("No data to plot")
            return

        # Create plot
        fig, ax = plt.subplots(figsize=(12, 6))

        # Plot each test_id
        for tid in df['test_id'].unique():
            if not tid:
                continue
            test_data = df[df['test_id'] == tid]
            ax.plot(test_data['timestamp'], test_data[metric], marker='o', label=tid)

        ax.set_xlabel('Time')
        ax.set_ylabel(metric.replace('_', ' ').title())
        ax.set_title(f'{metric.replace("_", " ").title()} Over Time')
        ax.legend()
        ax.grid(True, alpha=0.3)

        # Format x-axis
        ax.xaxis.set_major_formatter(mdates.DateFormatter('%m-%d %H:%M'))
        plt.xticks(rotation=45)

        plt.tight_layout()

        # Save
        output_path = self.stats_dir / f'{metric}_timeseries.png'
        plt.savefig(output_path, dpi=150)
        plt.close()

        print(f"✓ Saved plot: {output_path}")

    # ========================================================================
    # Dashboard
    # ========================================================================

    def generate_dashboard(self, days: int = 7):
        """Generate comprehensive dashboard with all analyses."""
        print(f"\n{'='*80}")
        print(f"TEST PERFORMANCE DASHBOARD (last {days} days)")
        print(f"{'='*80}\n")

        if not hasattr(self, 'df'):
            print("No telemetry data available")
            return

        cutoff = pd.Timestamp.utcnow() - pd.Timedelta(days=days)
        recent = self.df[self.df['timestamp'] >= cutoff]

        # Overall summary
        print(f"**Overall Summary**")
        print(f"  Total runs: {len(recent)}")
        print(f"  Passed: {recent['passed'].sum()} ({recent['passed'].sum()/len(recent)*100:.1f}%)")
        print(f"  Failed: {recent['failed'].sum()}")
        print(f"  Date range: {recent['timestamp'].min()} to {recent['timestamp'].max()}")
        print()

        # Performance by worker count
        print(f"**Performance by Worker Count**")
        for worker in sorted(recent['worker_count'].dropna().unique()):
            worker_data = recent[recent['worker_count'] == worker]
            mean_pps = worker_data['pages_per_sec'].mean()
            std_pps = worker_data['pages_per_sec'].std()
            count = len(worker_data)
            print(f"  {int(worker):2d} workers: {mean_pps:6.1f} ± {std_pps:4.1f} pps ({count} runs)")
        print()

        # Trends
        print(f"**Performance Trends**")
        trends = self.generate_trend_analysis(days=days)
        for test_id, trend in list(trends.items())[:5]:  # Top 5
            print(f"  {test_id}:")
            print(f"    Current: {trend['current_mean']:.1f} pps")
            if trend['previous_mean']:
                print(f"    Change: {trend['change_pct']:+.1f}%")
            print(f"    Trend: {trend['trend']}")
        print()

        # Regressions
        regressions = self.detect_regressions()
        if regressions:
            print(f"**⚠️  Regressions Detected: {len(regressions)}**")
            for reg in regressions[:3]:  # Top 3
                print(f"  {reg['test_id']}: {reg['change_pct']:.1f}% slower ({reg['severity']})")
            print()
        else:
            print(f"**✓ No Regressions Detected**\n")

        # Export all data
        self.export_timeseries_csv(f'timeseries_{days}days.csv', days=days)
        self.export_summary_json(f'summary_{days}days.json')
        self.generate_markdown_report(f'report_{days}days.md', days=days)

        print(f"\n{'='*80}")
        print(f"Dashboard files saved to: {self.stats_dir}/")
        print(f"{'='*80}\n")
