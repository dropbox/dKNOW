"""
Baseline Management System

Loads and saves expected outcomes for correctness validation:
- Upstream text baselines (from official PDFium)
- Upstream image baselines (MD5 hashes per page, PPM format)
- Optimized 1-worker JSONL (for debugging)

Baselines stored in:
  testing/baselines/upstream/text/*.txt
  testing/baselines/upstream/images_ppm/*.json (PPM format with MD5 hashes)
  testing/baselines/optimized_1w/jsonl/*.jsonl
"""

import json
import subprocess
from pathlib import Path
from typing import Optional, Dict


class BaselineManager:
    """Manage expected outcomes (baselines) for testing."""

    def __init__(self, testing_root: Path):
        self.testing_root = Path(testing_root)
        self.upstream_text_dir = self.testing_root / 'baselines' / 'upstream' / 'text'
        self.upstream_images_dir = self.testing_root / 'baselines' / 'upstream' / 'images_ppm'  # Changed to PPM format
        self.optimized_jsonl_dir = self.testing_root / 'baselines' / 'optimized_1w' / 'jsonl'

        # Ensure directories exist
        self.upstream_text_dir.mkdir(parents=True, exist_ok=True)
        self.upstream_images_dir.mkdir(parents=True, exist_ok=True)
        self.optimized_jsonl_dir.mkdir(parents=True, exist_ok=True)

    # ========================================================================
    # Load Baselines
    # ========================================================================

    def load_text_baseline(self, pdf_name: str) -> Optional[str]:
        """
        Load upstream text baseline for PDF with MD5 verification.

        Args:
            pdf_name: PDF filename (e.g., "arxiv_001.pdf")

        Returns:
            Text content from upstream, or None if not found

        Raises:
            ValueError: If MD5 checksum doesn't match (baseline corrupted)
        """
        basename = Path(pdf_name).stem
        baseline_path = self.upstream_text_dir / f'{basename}.txt'
        md5_path = self.upstream_text_dir / f'{basename}.txt.md5'

        if not baseline_path.exists():
            return None

        # Check if empty (not yet generated)
        if baseline_path.stat().st_size == 0:
            return None

        # Load text content (UTF-32 LE format from Rust tools)
        # Use surrogatepass to handle surrogate pairs if present
        text_content = baseline_path.read_text(encoding='utf-32-le', errors='surrogatepass')

        # Verify MD5 if MD5 file exists
        if md5_path.exists():
            stored_md5 = md5_path.read_text().strip()

            # Compute MD5 of raw file bytes (not decoded text)
            import hashlib
            computed_md5 = hashlib.md5(baseline_path.read_bytes()).hexdigest()

            if computed_md5 != stored_md5:
                raise ValueError(
                    f"MD5 mismatch for {pdf_name} baseline!\n"
                    f"  Expected: {stored_md5}\n"
                    f"  Got:      {computed_md5}\n"
                    f"  Baseline file may be corrupted or modified."
                )

        return text_content

    def load_image_baseline(self, pdf_name: str) -> Optional[Dict[str, str]]:
        """
        Load upstream image MD5 hashes for PDF (PPM format).

        Args:
            pdf_name: PDF filename

        Returns:
            Dict of {page_num: md5_hash}, or None if not found

        Example:
            {"0": "a1b2c3...", "1": "b2c3d4...", "2": "c3d4e5..."}
        """
        basename = Path(pdf_name).stem
        baseline_path = self.upstream_images_dir / f'{basename}.json'

        if not baseline_path.exists():
            return None

        if baseline_path.stat().st_size == 0:
            return None

        # PPM baseline format: {"pdf_name": "...", "format": "ppm", "dpi": 300, "pages": {...}}
        baseline_data = json.loads(baseline_path.read_text())
        return baseline_data.get("pages", {})

    def load_jsonl_baseline(self, pdf_name: str) -> Optional[list]:
        """
        Load optimized 1-worker JSONL debug output.

        Args:
            pdf_name: PDF filename

        Returns:
            List of JSONL records (one per character), or None
        """
        basename = Path(pdf_name).stem
        baseline_path = self.optimized_jsonl_dir / f'{basename}.jsonl'

        if not baseline_path.exists():
            return None

        if baseline_path.stat().st_size == 0:
            return None

        records = []
        with open(baseline_path) as f:
            for line in f:
                if line.strip():
                    records.append(json.loads(line))

        return records

    def get_text_baseline(self, pdf_name: str) -> Optional[str]:
        """Alias for load_text_baseline() for backward compatibility."""
        return self.load_text_baseline(pdf_name)

    def has_text_baseline(self, pdf_name: str) -> bool:
        """Check if text baseline exists and is non-empty."""
        baseline = self.load_text_baseline(pdf_name)
        return baseline is not None and len(baseline) > 0

    def has_image_baseline(self, pdf_name: str) -> bool:
        """Check if image baseline exists and is non-empty."""
        baseline = self.load_image_baseline(pdf_name)
        return baseline is not None and len(baseline) > 0

    # ========================================================================
    # Save Baselines (for generation scripts)
    # ========================================================================

    def save_text_baseline(self, pdf_name: str, text_content: str):
        """Save upstream text baseline with MD5 hash."""
        import hashlib

        basename = Path(pdf_name).stem
        baseline_path = self.upstream_text_dir / f'{basename}.txt'
        md5_path = self.upstream_text_dir / f'{basename}.txt.md5'

        # Save text content
        baseline_path.write_text(text_content)

        # Compute and save MD5
        md5_hash = hashlib.md5(text_content.encode('utf-8')).hexdigest()
        md5_path.write_text(md5_hash + '\n')

    def save_image_baseline(self, pdf_name: str, page_hashes: Dict[str, str]):
        """
        Save upstream image MD5 hashes.

        Args:
            pdf_name: PDF filename
            page_hashes: Dict of {page_num_str: md5_hash}
        """
        basename = Path(pdf_name).stem
        baseline_path = self.upstream_images_dir / f'{basename}.json'
        baseline_path.write_text(json.dumps(page_hashes, indent=2))

    def save_jsonl_baseline(self, pdf_name: str, jsonl_records: list):
        """Save optimized 1-worker JSONL debug output."""
        basename = Path(pdf_name).stem
        baseline_path = self.optimized_jsonl_dir / f'{basename}.jsonl'

        with open(baseline_path, 'w') as f:
            for record in jsonl_records:
                f.write(json.dumps(record) + '\n')

    # ========================================================================
    # Statistics
    # ========================================================================

    def count_baselines(self) -> Dict[str, int]:
        """Count how many baselines exist."""
        text_count = sum(1 for p in self.upstream_text_dir.glob('*.txt') if p.stat().st_size > 0)
        image_count = sum(1 for p in self.upstream_images_dir.glob('*.json') if p.stat().st_size > 0)
        jsonl_count = sum(1 for p in self.optimized_jsonl_dir.glob('*.jsonl') if p.stat().st_size > 0)

        return {
            'upstream_text': text_count,
            'upstream_images': image_count,
            'optimized_jsonl': jsonl_count,
        }

    def list_missing_baselines(self, pdf_list: list) -> Dict[str, list]:
        """
        Check which baselines are missing for given PDF list.

        Returns:
            {'text': [missing pdfs], 'images': [missing pdfs]}
        """
        missing_text = []
        missing_images = []

        for pdf_name in pdf_list:
            if not self.has_text_baseline(pdf_name):
                missing_text.append(pdf_name)
            if not self.has_image_baseline(pdf_name):
                missing_images.append(pdf_name)

        return {
            'text': missing_text,
            'images': missing_images,
        }
