#!/usr/bin/env python3
"""
Generate Expected Outputs for All 452 PDFs

Generates baseline outputs using Rust extraction tools with upstream PDFium binary:
- Per-page text files (committed to git)
- Full text file (committed to git)
- JSONL for page 0 (extract_text_jsonl tool)
- PNG + JPG images (metadata only, images NOT committed)
- Per-PDF manifest.json

Usage:
    python lib/generate_expected_outputs.py [--pdf PDF_NAME] [--workers N] [--dry-run]

Options:
    --pdf PDF_NAME    Generate for single PDF (default: all 452)
    --workers N       Number of parallel workers (default: 4)
    --dry-run         Show what would be generated without generating
"""

import subprocess
import tempfile
import hashlib
import json
import csv
import argparse
from pathlib import Path
from typing import Dict, List, Tuple
from datetime import datetime
import os
import sys
from multiprocessing import Pool, Manager, cpu_count

# Add lib directory to path
sys.path.insert(0, str(Path(__file__).parent))


def _worker_process_pdf(args):
    """Worker function for multiprocessing - must be at module level for pickling."""
    pdf_row, generator_config, idx, total_pdfs = args

    # Create a generator instance in this worker process
    integration_tests_root = Path(generator_config['integration_tests_root'])
    generator = ExpectedOutputGenerator(integration_tests_root)

    try:
        jsonl_only = generator_config.get('jsonl_only', False)
        result = generator.generate_for_pdf(pdf_row, dry_run=False, jsonl_only=jsonl_only)
        status = '✓' if result else '✗'
    except Exception as e:
        print(f"[{idx}/{total_pdfs}] {pdf_row['pdf_name']}: ✗ (Exception: {e})")
        return False

    print(f"[{idx}/{total_pdfs}] {pdf_row['pdf_name']}: {status}")
    return result


class ExpectedOutputGenerator:
    """Generates expected outputs for test suite."""

    def __init__(self, integration_tests_root: Path):
        self.root = Path(integration_tests_root)
        self.pdfium_root = self.root.parent

        # Find baseline binary and Rust tools
        self.baseline_lib = self.pdfium_root / 'out' / 'Release' / 'libpdfium.dylib'
        self.extract_text_bin = self.pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'extract_text'
        self.extract_text_jsonl_bin = self.pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'extract_text_jsonl'
        self.render_pages_bin = self.pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'render_pages'

        # Verify binaries exist
        if not self.baseline_lib.exists():
            raise FileNotFoundError(f"Baseline PDFium library not found: {self.baseline_lib}")
        if not self.extract_text_bin.exists():
            raise FileNotFoundError(f"extract_text binary not found: {self.extract_text_bin}")
        if not self.extract_text_jsonl_bin.exists():
            raise FileNotFoundError(f"extract_text_jsonl binary not found: {self.extract_text_jsonl_bin}")
        if not self.render_pages_bin.exists():
            raise FileNotFoundError(f"render_pages binary not found: {self.render_pages_bin}")

        # Verify baseline binary MD5
        baseline_md5 = self.compute_md5(self.baseline_lib)
        expected_md5 = "00cd20f999bf60b1f779249dbec8ceaa"
        if not baseline_md5.startswith(expected_md5[:12]):
            print(f"WARNING: Baseline binary MD5 mismatch!")
            print(f"  Expected: {expected_md5}")
            print(f"  Actual: {baseline_md5}")
            print(f"  Continuing anyway...")

        print(f"Using baseline library: {self.baseline_lib}")
        print(f"  MD5: {baseline_md5}")
        print(f"Using extract_text: {self.extract_text_bin}")
        print(f"Using render_pages: {self.render_pages_bin}")

        # Output directories
        self.expected_outputs_dir = self.root / 'master_test_suite' / 'expected_outputs'
        self.expected_outputs_dir.mkdir(parents=True, exist_ok=True)

        # PDF manifest
        self.manifest_file = self.root / 'master_test_suite' / 'pdf_manifest.csv'

    def compute_md5(self, filepath: Path) -> str:
        """Compute MD5 hash of file."""
        md5_hash = hashlib.md5()
        with open(filepath, 'rb') as f:
            for chunk in iter(lambda: f.read(8192), b""):
                md5_hash.update(chunk)
        return md5_hash.hexdigest()

    def compute_md5_bytes(self, data: bytes) -> str:
        """Compute MD5 hash of bytes."""
        return hashlib.md5(data).hexdigest()

    def get_page_count(self, pdf_path: Path) -> int:
        """Get page count from PDF using extract_text tool."""
        try:
            # Run extract_text with a temp output to get page count from output
            with tempfile.NamedTemporaryFile(suffix='.txt', delete=True) as tmp:
                env = os.environ.copy()
                env['DYLD_LIBRARY_PATH'] = str(self.baseline_lib.parent)

                result = subprocess.run(
                    [str(self.extract_text_bin), str(pdf_path), tmp.name, '1'],
                    capture_output=True,
                    text=True,
                    env=env,
                    timeout=60
                )

                if result.returncode != 0:
                    print(f"  Warning: Could not get page count: {result.stderr}")
                    return 0

                # Count BOMs in output (each page has a BOM)
                output = Path(tmp.name).read_bytes()
                bom = b'\xff\xfe\x00\x00'
                page_count = output.count(bom)
                return max(page_count, 1)  # At least 1 page

        except Exception as e:
            print(f"  Warning: Could not get page count: {e}")
            return 0

    def extract_text_per_page(self, pdf_path: Path, output_dir: Path) -> List[Dict]:
        """Extract text per page using Rust extract_text tool."""
        text_dir = output_dir / 'text'
        text_dir.mkdir(parents=True, exist_ok=True)

        # Extract to temp file
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as tmp:
            tmp_path = Path(tmp.name)

        try:
            env = os.environ.copy()
            env['DYLD_LIBRARY_PATH'] = str(self.baseline_lib.parent)

            # Force single-threaded for deterministic output
            result = subprocess.run(
                [str(self.extract_text_bin), str(pdf_path), str(tmp_path), '--workers', '1'],
                capture_output=True,
                env=env,
                timeout=600
            )

            if result.returncode != 0:
                raise RuntimeError(f"Text extraction failed: {result.stderr.decode()}")

            # Read full output
            full_text = tmp_path.read_bytes()

            # Split by BOM (each page starts with BOM)
            bom = b'\xff\xfe\x00\x00'
            pages = full_text.split(bom)

            # First element is empty (file starts with BOM), rest are pages
            # NOTE: Keep blank pages (don't filter with 'if page')
            pages = [bom + page for page in pages[1:]]

            # Save per-page files
            page_metadata = []
            for page_num, page_text in enumerate(pages):
                page_file = text_dir / f'page_{page_num:04d}.txt'
                page_file.write_bytes(page_text)

                page_meta = {
                    "page": page_num,
                    "path": f"text/page_{page_num:04d}.txt",
                    "md5": self.compute_md5_bytes(page_text),
                    "bytes": len(page_text),
                    "chars": (len(page_text) - 4) // 4  # Subtract BOM, divide by 4 (UTF-32)
                }
                page_metadata.append(page_meta)

            # Save full text (concatenation of all pages)
            full_file = text_dir / 'full.txt'
            full_file.write_bytes(full_text)

            return page_metadata

        finally:
            # Clean up temp file
            if tmp_path.exists():
                tmp_path.unlink()

    def render_images(self, pdf_path: Path, output_dir: Path, page_count: int) -> List[Dict]:
        """Render images and save metadata only (delete actual images)."""
        images_dir = output_dir / 'images'
        images_dir.mkdir(parents=True, exist_ok=True)

        # Create temp directory for rendering
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_output = Path(tmpdir)

            env = os.environ.copy()
            env['DYLD_LIBRARY_PATH'] = str(self.baseline_lib.parent)

            # Force single-threaded for deterministic output
            result = subprocess.run(
                [str(self.render_pages_bin), str(pdf_path), str(tmp_output), '1', '300'],
                capture_output=True,
                env=env,
                timeout=1200
            )

            if result.returncode != 0:
                raise RuntimeError(f"Image rendering failed: {result.stderr.decode()}")

            # Process each page's images
            page_metadata = []
            for page_num in range(page_count):
                png_file = tmp_output / f'page_{page_num:04d}.png'

                if not png_file.exists():
                    print(f"  Warning: PNG not found for page {page_num}")
                    continue

                # Read PNG data
                png_data = png_file.read_bytes()

                # Get image dimensions using PIL
                try:
                    from PIL import Image
                    img = Image.open(png_file)
                    width_px, height_px = img.size
                except ImportError:
                    # Fallback if PIL not available
                    width_px, height_px = 0, 0

                # Convert PNG to JPG (quality 85)
                jpg_file = tmp_output / f'page_{page_num:04d}.jpg'
                if width_px > 0:
                    img_rgb = img.convert('RGB')
                    img_rgb.save(jpg_file, 'JPEG', quality=85)
                    jpg_data = jpg_file.read_bytes()
                else:
                    jpg_data = b''

                # Save metadata (NOT images)
                page_meta = {
                    "page": page_num,
                    "png": {
                        "path": f"images/page_{page_num:04d}.png",
                        "md5": self.compute_md5_bytes(png_data),
                        "bytes": len(png_data),
                        "width_px": width_px,
                        "height_px": height_px
                    },
                    "jpg": {
                        "path": f"images/page_{page_num:04d}.jpg",
                        "md5": self.compute_md5_bytes(jpg_data) if jpg_data else "",
                        "bytes": len(jpg_data),
                        "quality": 85,
                        "width_px": width_px,
                        "height_px": height_px
                    }
                }
                page_metadata.append(page_meta)

            return page_metadata

    def generate_jsonl_page0(self, pdf_path: Path, output_dir: Path) -> Dict:
        """Generate JSONL for page 0 using extract_text_jsonl tool."""
        jsonl_dir = output_dir / 'jsonl'
        jsonl_dir.mkdir(parents=True, exist_ok=True)

        jsonl_file = jsonl_dir / 'page_0000.jsonl'

        # Set library path
        env = os.environ.copy()
        env['DYLD_LIBRARY_PATH'] = str(self.baseline_lib.parent)

        # Extract JSONL for page 0
        result = subprocess.run(
            [str(self.extract_text_jsonl_bin), str(pdf_path), str(jsonl_file), '0'],
            capture_output=True,
            env=env,
            timeout=120
        )

        if result.returncode != 0:
            raise RuntimeError(f"JSONL extraction failed: {result.stderr.decode()}")

        # Count lines and compute MD5
        line_count = sum(1 for _ in open(jsonl_file, 'rb'))
        file_size = jsonl_file.stat().st_size
        file_md5 = self.compute_md5(jsonl_file)

        return {
            "note": "Character-level metadata for page 0",
            "pages": [{
                "page": 0,
                "path": "jsonl/page_0000.jsonl",
                "md5": file_md5,
                "bytes": file_size,
                "lines": line_count,
                "char_count": line_count  # Each line is one character
            }]
        }

    def generate_manifest(self, pdf_path: Path, pdf_row: Dict, text_meta: List[Dict],
                          image_meta: List[Dict], jsonl_meta: Dict) -> Dict:
        """Generate per-PDF manifest.json."""
        # ALWAYS use actual extracted page count (not CSV - CSV may be wrong)
        pdf_pages = len(text_meta)

        try:
            pdf_bytes = int(pdf_row['pdf_bytes'])
        except (ValueError, KeyError):
            pdf_bytes = pdf_path.stat().st_size if pdf_path.exists() else 0

        manifest = {
            "pdf": pdf_row['pdf_name'],
            "pdf_path": pdf_row['pdf_path'],
            "pdf_md5": pdf_row['pdf_md5'],
            "pdf_bytes": pdf_bytes,
            "pdf_pages": pdf_pages,
            "pdf_category": pdf_row['pdf_category'],
            "pdf_subcategory": pdf_row.get('pdf_subcategory', ''),
            "pdf_size_class": pdf_row['pdf_size_class'],

            "generated_by": "lib/generate_expected_outputs.py",
            "baseline_binary": str(self.baseline_lib),
            "baseline_binary_md5": self.compute_md5(self.baseline_lib),
            "generated_date": datetime.now().isoformat(),

            "text": {
                "full": {
                    "path": "text/full.txt",
                    "md5": self.compute_md5(self.root / pdf_row['expected_outputs_dir'] / 'text' / 'full.txt'),
                    "bytes": (self.root / pdf_row['expected_outputs_dir'] / 'text' / 'full.txt').stat().st_size,
                    "chars": sum(m['chars'] for m in text_meta)
                },
                "pages": text_meta
            },

            "jsonl": jsonl_meta,

            "images": {
                "formats": ["png", "jpg"],
                "dpi": 300,
                "note": "Images not committed - metadata only. Regenerate with: python lib/regenerate_images.py",
                "pages": image_meta
            }
        }

        return manifest

    def generate_for_pdf(self, pdf_row: Dict, dry_run: bool = False, jsonl_only: bool = False) -> bool:
        """Generate all expected outputs for a single PDF."""
        pdf_name = pdf_row['pdf_name']
        # Use pdf_path from manifest (includes correct subdirectory)
        pdf_path = self.root / pdf_row['pdf_path']

        if not pdf_path.exists():
            print(f"  ERROR: PDF not found: {pdf_path}")
            return False

        print(f"\n[{pdf_name}]")
        print(f"  Category: {pdf_row['pdf_category']}")
        print(f"  Pages: {pdf_row['pdf_pages']}")
        # Handle 'unknown' for pdf_bytes
        try:
            pdf_bytes_kb = int(pdf_row['pdf_bytes']) / 1024
            print(f"  Size: {pdf_bytes_kb:.1f} KB")
        except (ValueError, TypeError):
            print(f"  Size: {pdf_row['pdf_bytes']}")

        if dry_run:
            print(f"  [DRY RUN] Would generate outputs")
            return True

        # Create output directory
        output_dir = self.root / pdf_row['expected_outputs_dir']
        output_dir.mkdir(parents=True, exist_ok=True)

        try:
            if jsonl_only:
                # JSONL-only mode: skip text and image generation
                # Get page count from manifest (or extract minimal info)
                manifest_file = output_dir / 'manifest.json'
                if manifest_file.exists():
                    manifest = json.loads(manifest_file.read_text())
                    page_count = manifest.get('pdf_pages', 0)
                else:
                    # Try to parse page count, default to 0 if unknown
                    try:
                        page_count = int(pdf_row['pdf_pages'])
                    except (ValueError, TypeError):
                        page_count = 0

                # Generate JSONL for page 0 only
                if page_count > 0:
                    print(f"  Generating JSONL for page 0...")
                    jsonl_meta = self.generate_jsonl_page0(pdf_path, output_dir)
                    if jsonl_meta['pages']:
                        char_count = jsonl_meta['pages'][0]['char_count']
                        print(f"    ✓ {char_count} characters with metadata")
                else:
                    print(f"  Generating JSONL for page 0...")
                    print(f"    ✓ 0 characters with metadata (0-page PDF)")
                    jsonl_meta = {'pages': [], 'total_lines': 0}

                # Update manifest with new JSONL metadata
                if manifest_file.exists():
                    manifest = json.loads(manifest_file.read_text())
                    manifest['jsonl'] = jsonl_meta
                    manifest_file.write_text(json.dumps(manifest, indent=2))
                    print(f"    ✓ Manifest updated")
                else:
                    print(f"    WARNING: No existing manifest found, skipping manifest update")

                return True

            # Normal mode: generate all outputs
            # 1. Extract text per page
            print(f"  Extracting text per page...")
            text_meta = self.extract_text_per_page(pdf_path, output_dir)
            print(f"    ✓ {len(text_meta)} pages extracted")

            # 2. Generate JSONL for page 0 (skip for 0-page PDFs)
            if len(text_meta) > 0:
                print(f"  Generating JSONL for page 0...")
                jsonl_meta = self.generate_jsonl_page0(pdf_path, output_dir)
                if jsonl_meta['pages']:
                    char_count = jsonl_meta['pages'][0]['char_count']
                    print(f"    ✓ {char_count} characters with metadata")
            else:
                # 0-page PDF: Create empty JSONL metadata
                print(f"  Generating JSONL for page 0...")
                print(f"    ✓ 0 characters with metadata (0-page PDF)")
                jsonl_meta = {'pages': [], 'total_lines': 0}

            # 3. Render images (metadata only)
            print(f"  Rendering images (PNG + JPG)...")
            page_count = len(text_meta)
            image_meta = self.render_images(pdf_path, output_dir, page_count)
            print(f"    ✓ {len(image_meta)} pages rendered (metadata saved)")

            # 4. Generate manifest.json
            manifest = self.generate_manifest(pdf_path, pdf_row, text_meta, image_meta, jsonl_meta)
            manifest_file = output_dir / 'manifest.json'
            manifest_file.write_text(json.dumps(manifest, indent=2))
            print(f"    ✓ Manifest saved")

            return True

        except Exception as e:
            print(f"  ERROR: {e}")
            import traceback
            traceback.print_exc()
            return False

    def load_pdf_manifest(self) -> List[Dict]:
        """Load PDF manifest CSV."""
        pdfs = []
        with open(self.manifest_file, 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                pdfs.append(row)
        return pdfs

    def generate_all(self, dry_run: bool = False, pdf_filter: str = None, workers: int = 4, jsonl_only: bool = False):
        """Generate expected outputs for all PDFs in manifest."""
        pdfs = self.load_pdf_manifest()

        if pdf_filter:
            pdfs = [p for p in pdfs if p['pdf_name'] == pdf_filter]
            if not pdfs:
                print(f"ERROR: PDF not found in manifest: {pdf_filter}")
                return

        print(f"=" * 80)
        print(f"Expected Output Generation")
        print(f"=" * 80)
        print(f"PDFs: {len(pdfs)}")
        print(f"Workers: {workers}")
        print(f"Dry run: {dry_run}")
        print(f"JSONL only: {jsonl_only}")
        print(f"=" * 80)

        if workers == 1 or dry_run:
            # Single-threaded execution
            success_count = 0
            fail_count = 0

            for i, pdf_row in enumerate(pdfs, 1):
                print(f"\n[{i}/{len(pdfs)}]", end=" ")

                if self.generate_for_pdf(pdf_row, dry_run=dry_run, jsonl_only=jsonl_only):
                    success_count += 1
                else:
                    fail_count += 1
        else:
            # Multi-process execution
            success_count, fail_count = self._generate_all_parallel(pdfs, workers, jsonl_only=jsonl_only)

        print(f"\n" + "=" * 80)
        print(f"COMPLETE")
        print(f"  Success: {success_count}")
        print(f"  Failed: {fail_count}")
        print(f"=" * 80)

    def _generate_all_parallel(self, pdfs: List[Dict], workers: int, jsonl_only: bool = False) -> Tuple[int, int]:
        """Generate outputs using multiprocessing."""
        # Prepare arguments for worker processes
        # Each worker needs: (pdf_row, generator_config, progress_info)
        total_pdfs = len(pdfs)

        # Serialize generator configuration for workers
        generator_config = {
            'integration_tests_root': str(self.root),
            'baseline_lib': str(self.baseline_lib),
            'extract_text_bin': str(self.extract_text_bin),
            'render_pages_bin': str(self.render_pages_bin),
            'jsonl_only': jsonl_only,
        }

        # Create work items
        work_items = []
        for i, pdf_row in enumerate(pdfs, 1):
            work_items.append((pdf_row, generator_config, i, total_pdfs))

        # Create pool and map work
        with Pool(processes=workers) as pool:
            results = pool.map(_worker_process_pdf, work_items)

        success_count = sum(1 for r in results if r)
        fail_count = sum(1 for r in results if not r)

        return success_count, fail_count


def main():
    parser = argparse.ArgumentParser(description='Generate expected outputs for test suite')
    parser.add_argument('--pdf', help='Generate for single PDF only')
    parser.add_argument('--workers', type=int, default=4, help='Number of parallel workers (default: 4)')
    parser.add_argument('--dry-run', action='store_true', help='Show what would be generated')
    parser.add_argument('--jsonl-only', action='store_true', help='Only regenerate JSONL outputs')

    args = parser.parse_args()

    # Get integration_tests root
    script_dir = Path(__file__).parent
    integration_tests_root = script_dir.parent

    generator = ExpectedOutputGenerator(integration_tests_root)
    generator.generate_all(dry_run=args.dry_run, pdf_filter=args.pdf, workers=args.workers, jsonl_only=args.jsonl_only)


if __name__ == '__main__':
    main()
