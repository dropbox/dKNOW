"""
Manifest Generator - Complete Test File Verification System

Generates CSV manifests with MD5 hashes, file sizes, timestamps for:
- All PDF test files
- Text baselines (per PDF)
- JSONL baselines (per PDF)
- Image baselines (per page)

Enables test_000_infrastructure.py to verify all files exist and match hashes.
"""

import hashlib
import json
import csv
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Tuple, Optional
import subprocess
import os


class ManifestGenerator:
    """Generates and validates file manifests for test infrastructure."""

    def __init__(self, integration_tests_root: Path):
        self.root = Path(integration_tests_root)
        self.pdfs_dir = self.root / 'pdfs'
        self.baselines_dir = self.root / 'baselines' / 'upstream'

        # Manifest files
        self.main_manifest = self.root / 'master_test_suite' / 'file_manifest.csv'
        self.image_manifests_dir = self.root / 'master_test_suite' / 'image_manifests'
        self.image_manifests_dir.mkdir(parents=True, exist_ok=True)

    def get_file_metadata(self, filepath: Path) -> Dict:
        """Get MD5, size, and timestamp for a file."""
        if not filepath.exists():
            return {
                'exists': False,
                'md5': None,
                'size': None,
                'modified': None
            }

        # Compute MD5
        md5_hash = hashlib.md5()
        with open(filepath, 'rb') as f:
            for chunk in iter(lambda: f.read(8192), b""):
                md5_hash.update(chunk)

        stat = filepath.stat()

        return {
            'exists': True,
            'md5': md5_hash.hexdigest(),
            'size': stat.st_size,
            'modified': datetime.fromtimestamp(stat.st_mtime).isoformat()
        }

    def get_pdf_page_count(self, pdf_path: Path) -> int:
        """Get page count from PDF using pdfium_test or fallback."""
        # Try to extract from filename first (e.g., "0100pages_...pdf")
        if pdf_path.stem.startswith('0') and 'pages_' in pdf_path.stem:
            try:
                pages_str = pdf_path.stem.split('pages_')[0].lstrip('0')
                return int(pages_str) if pages_str else 0
            except ValueError:
                pass

        # Try using pdfium_test to count pages
        pdfium_root = self.root.parent
        pdfium_test = pdfium_root / 'out' / 'Optimized-Shared' / 'pdfium_test'

        if pdfium_test.exists():
            try:
                # pdfium_test prints page count
                result = subprocess.run(
                    [str(pdfium_test), str(pdf_path)],
                    capture_output=True,
                    text=True,
                    timeout=30
                )
                # Look for "Document has X pages"
                for line in result.stdout.split('\n'):
                    if 'has' in line and 'pages' in line.lower():
                        parts = line.split()
                        for i, part in enumerate(parts):
                            if 'page' in part.lower() and i > 0:
                                try:
                                    return int(parts[i-1])
                                except ValueError:
                                    pass
            except (subprocess.TimeoutExpired, Exception) as e:
                print(f"  Warning: Could not get page count for {pdf_path.name}: {e}")

        # Fallback: return -1 to indicate unknown
        return -1

    def load_master_pdf_list(self) -> List[str]:
        """
        Load PDF list from existing manifest or discover from directories.

        Priority:
        1. If manifest exists, use PDFs from manifest
        2. Otherwise, discover all PDFs in pdfs/benchmark directory
        """
        # Try loading from existing manifest first
        if self.main_manifest.exists():
            pdfs = []
            with open(self.main_manifest, 'r') as f:
                reader = csv.DictReader(f)
                for row in reader:
                    pdfs.append(row['pdf_name'])
            return pdfs

        # Fallback: discover PDFs from benchmark directory
        pdfs = []
        benchmark_dir = self.pdfs_dir / 'benchmark'
        if benchmark_dir.exists():
            for pdf_file in sorted(benchmark_dir.glob('*.pdf')):
                pdfs.append(pdf_file.name)

        return pdfs

    def find_pdf_path(self, pdf_name: str) -> Optional[Path]:
        """Find PDF in benchmark or edge_cases directories."""
        for subdir in ['benchmark', 'edge_cases']:
            pdf_path = self.pdfs_dir / subdir / pdf_name
            if pdf_path.exists():
                return pdf_path
        return None

    def get_pdf_category(self, pdf_name: str) -> str:
        """Determine PDF category from filename."""
        if pdf_name.startswith('arxiv_'):
            return 'arxiv'
        elif pdf_name.startswith('cc_'):
            return 'cc'
        elif pdf_name.startswith('edinet_'):
            return 'edinet'
        elif pdf_name.startswith('web_'):
            return 'web'
        elif 'pages_' in pdf_name:
            return 'pages'
        else:
            return 'other'

    def get_pdf_size_class(self, page_count: int) -> str:
        """Determine PDF size class from page count."""
        if page_count < 0:
            return 'unknown'
        elif page_count < 100:
            return 'small'
        elif page_count < 200:
            return 'medium'
        else:
            return 'large'

    def generate_main_manifest(self, pdf_list: Optional[List[str]] = None) -> Dict:
        """
        Generate main manifest CSV with all PDFs and their expected outputs.

        Returns dict with statistics.
        """
        if pdf_list is None:
            pdf_list = self.load_master_pdf_list()

        if not pdf_list:
            raise ValueError("No PDFs in master list")

        print(f"\nGenerating main manifest for {len(pdf_list)} PDFs...")

        rows = []
        stats = {
            'total_pdfs': len(pdf_list),
            'found_pdfs': 0,
            'missing_pdfs': 0,
            'has_text_baseline': 0,
            'has_jsonl_baseline': 0,
            'has_image_baseline': 0
        }

        for pdf_name in pdf_list:
            pdf_stem = Path(pdf_name).stem
            pdf_path = self.find_pdf_path(pdf_name)

            if not pdf_path:
                print(f"  ✗ PDF not found: {pdf_name}")
                stats['missing_pdfs'] += 1
                continue

            stats['found_pdfs'] += 1

            # Get PDF metadata
            pdf_meta = self.get_file_metadata(pdf_path)
            page_count = self.get_pdf_page_count(pdf_path)

            # Get baseline metadata
            text_baseline = self.baselines_dir / 'text' / f'{pdf_stem}.txt'
            jsonl_baseline = self.baselines_dir / 'jsonl' / f'{pdf_stem}.jsonl'
            image_baseline_json = self.baselines_dir / 'images' / f'{pdf_stem}.json'

            text_meta = self.get_file_metadata(text_baseline)
            jsonl_meta = self.get_file_metadata(jsonl_baseline)
            image_meta = self.get_file_metadata(image_baseline_json)

            if text_meta['exists']:
                stats['has_text_baseline'] += 1
            if jsonl_meta['exists']:
                stats['has_jsonl_baseline'] += 1
            if image_meta['exists']:
                stats['has_image_baseline'] += 1

            # PDF metadata for markers
            category = self.get_pdf_category(pdf_name)
            size_class = self.get_pdf_size_class(page_count)

            row = {
                # PDF info
                'pdf_name': pdf_name,
                'pdf_path': str(pdf_path.relative_to(self.root)),
                'pdf_exists': pdf_meta['exists'],
                'pdf_md5': pdf_meta['md5'],
                'pdf_size': pdf_meta['size'],
                'pdf_modified': pdf_meta['modified'],
                'pdf_pages': page_count,
                'pdf_category': category,
                'pdf_size_class': size_class,

                # Text baseline
                'text_baseline_path': str(text_baseline.relative_to(self.root)),
                'text_baseline_exists': text_meta['exists'],
                'text_baseline_md5': text_meta['md5'],
                'text_baseline_size': text_meta['size'],
                'text_baseline_modified': text_meta['modified'],

                # JSONL baseline
                'jsonl_baseline_path': str(jsonl_baseline.relative_to(self.root)),
                'jsonl_baseline_exists': jsonl_meta['exists'],
                'jsonl_baseline_md5': jsonl_meta['md5'],
                'jsonl_baseline_size': jsonl_meta['size'],
                'jsonl_baseline_modified': jsonl_meta['modified'],

                # Image baseline JSON
                'image_baseline_json_path': str(image_baseline_json.relative_to(self.root)),
                'image_baseline_json_exists': image_meta['exists'],
                'image_baseline_json_md5': image_meta['md5'],
                'image_baseline_json_size': image_meta['size'],
                'image_baseline_json_modified': image_meta['modified'],
            }

            rows.append(row)

        # Write CSV
        if rows:
            with open(self.main_manifest, 'w', newline='') as f:
                writer = csv.DictWriter(f, fieldnames=rows[0].keys())
                writer.writeheader()
                writer.writerows(rows)

            print(f"  ✓ Main manifest written: {self.main_manifest}")
            print(f"    Rows: {len(rows)}")

        return stats

    def generate_image_manifest(self, pdf_name: str, page_count: int) -> Dict:
        """
        Generate per-PDF image manifest with all page images.

        Returns dict with statistics.
        """
        pdf_stem = Path(pdf_name).stem
        images_dir = self.baselines_dir / 'images' / pdf_stem

        manifest_file = self.image_manifests_dir / f'{pdf_stem}_images.csv'

        rows = []
        stats = {
            'pdf': pdf_name,
            'expected_pages': page_count,
            'found_pages': 0,
            'missing_pages': 0
        }

        for page_num in range(page_count):
            # Expected image filename pattern: <pdf_stem>_page_<N>.png
            image_file = images_dir / f'{pdf_stem}_page_{page_num:04d}.png'

            meta = self.get_file_metadata(image_file)

            if meta['exists']:
                stats['found_pages'] += 1
            else:
                stats['missing_pages'] += 1

            row = {
                'pdf_name': pdf_name,
                'page_number': page_num,
                'image_path': str(image_file.relative_to(self.root)),
                'image_exists': meta['exists'],
                'image_md5': meta['md5'],
                'image_size': meta['size'],
                'image_modified': meta['modified']
            }

            rows.append(row)

        # Write CSV
        if rows:
            with open(manifest_file, 'w', newline='') as f:
                writer = csv.DictWriter(f, fieldnames=rows[0].keys())
                writer.writeheader()
                writer.writerows(rows)

        return stats

    def generate_all_image_manifests(self) -> Dict:
        """Generate image manifests for all PDFs in main manifest."""
        print("\nGenerating per-PDF image manifests...")

        if not self.main_manifest.exists():
            raise FileNotFoundError(f"Main manifest not found: {self.main_manifest}")

        # Read main manifest
        with open(self.main_manifest, 'r') as f:
            reader = csv.DictReader(f)
            pdfs = list(reader)

        total_stats = {
            'total_pdfs': len(pdfs),
            'processed': 0,
            'skipped_no_pages': 0,
            'total_expected_pages': 0,
            'total_found_pages': 0,
            'total_missing_pages': 0
        }

        for row in pdfs:
            pdf_name = row['pdf_name']
            page_count = int(row['pdf_pages']) if row['pdf_pages'] and row['pdf_pages'] != '-1' else 0

            if page_count <= 0:
                total_stats['skipped_no_pages'] += 1
                continue

            stats = self.generate_image_manifest(pdf_name, page_count)

            total_stats['processed'] += 1
            total_stats['total_expected_pages'] += stats['expected_pages']
            total_stats['total_found_pages'] += stats['found_pages']
            total_stats['total_missing_pages'] += stats['missing_pages']

            if stats['found_pages'] == stats['expected_pages']:
                status = "✓"
            elif stats['found_pages'] == 0:
                status = "✗"
            else:
                status = "⚠"

            print(f"  {status} {pdf_name}: {stats['found_pages']}/{stats['expected_pages']} pages")

        print(f"\n  ✓ Generated {total_stats['processed']} image manifests")
        print(f"    Total pages: {total_stats['total_expected_pages']}")
        print(f"    Found: {total_stats['total_found_pages']}")
        print(f"    Missing: {total_stats['total_missing_pages']}")

        return total_stats

    def verify_manifest(self) -> Tuple[bool, Dict]:
        """
        Verify all files in main manifest exist and match hashes.

        Returns (all_valid, stats_dict)
        """
        if not self.main_manifest.exists():
            return False, {'error': 'Main manifest not found'}

        print("\nVerifying main manifest...")

        with open(self.main_manifest, 'r') as f:
            reader = csv.DictReader(f)
            rows = list(reader)

        stats = {
            'total_files': 0,
            'valid_files': 0,
            'missing_files': 0,
            'hash_mismatches': 0,
            'errors': []
        }

        for row in rows:
            # Check PDF
            for file_type in ['pdf', 'text_baseline', 'jsonl_baseline', 'image_baseline_json']:
                path_key = f'{file_type}_path'
                exists_key = f'{file_type}_exists'
                md5_key = f'{file_type}_md5'

                if path_key not in row:
                    continue

                stats['total_files'] += 1

                expected_exists = row[exists_key] == 'True'
                expected_md5 = row[md5_key]

                if not expected_exists:
                    # File was not expected to exist in manifest
                    continue

                file_path = self.root / row[path_key]
                current_meta = self.get_file_metadata(file_path)

                if not current_meta['exists']:
                    stats['missing_files'] += 1
                    stats['errors'].append(f"Missing: {row[path_key]}")
                elif current_meta['md5'] != expected_md5:
                    stats['hash_mismatches'] += 1
                    stats['errors'].append(
                        f"Hash mismatch: {row[path_key]}\n"
                        f"  Expected: {expected_md5}\n"
                        f"  Current:  {current_meta['md5']}"
                    )
                else:
                    stats['valid_files'] += 1

        all_valid = stats['missing_files'] == 0 and stats['hash_mismatches'] == 0

        print(f"  Total files checked: {stats['total_files']}")
        print(f"  Valid: {stats['valid_files']}")
        print(f"  Missing: {stats['missing_files']}")
        print(f"  Hash mismatches: {stats['hash_mismatches']}")

        if not all_valid and stats['errors']:
            print("\n  Errors:")
            for error in stats['errors'][:10]:  # Show first 10 errors
                print(f"    {error}")
            if len(stats['errors']) > 10:
                print(f"    ... and {len(stats['errors']) - 10} more errors")

        return all_valid, stats


def main():
    """CLI for manifest generation."""
    import sys

    integration_tests_root = Path(__file__).parent.parent
    generator = ManifestGenerator(integration_tests_root)

    command = sys.argv[1] if len(sys.argv) > 1 else 'generate-all'

    if command == 'generate-main':
        stats = generator.generate_main_manifest()
        print("\nMain Manifest Statistics:")
        for key, value in stats.items():
            print(f"  {key}: {value}")

    elif command == 'generate-images':
        stats = generator.generate_all_image_manifests()
        print("\nImage Manifest Statistics:")
        for key, value in stats.items():
            print(f"  {key}: {value}")

    elif command == 'generate-all':
        print("=" * 60)
        print("MANIFEST GENERATION")
        print("=" * 60)

        main_stats = generator.generate_main_manifest()
        print("\nMain Manifest Statistics:")
        for key, value in main_stats.items():
            print(f"  {key}: {value}")

        try:
            image_stats = generator.generate_all_image_manifests()
            print("\nImage Manifest Statistics:")
            for key, value in image_stats.items():
                print(f"  {key}: {value}")
        except Exception as e:
            print(f"\nWarning: Could not generate image manifests: {e}")

    elif command == 'verify':
        all_valid, stats = generator.verify_manifest()
        print("\nVerification Result:")
        print(f"  All valid: {all_valid}")
        sys.exit(0 if all_valid else 1)

    else:
        print("Usage:")
        print("  python manifest_generator.py generate-main    # Generate main manifest only")
        print("  python manifest_generator.py generate-images  # Generate image manifests only")
        print("  python manifest_generator.py generate-all     # Generate all manifests")
        print("  python manifest_generator.py verify           # Verify all files match manifest")
        sys.exit(1)


if __name__ == '__main__':
    main()
