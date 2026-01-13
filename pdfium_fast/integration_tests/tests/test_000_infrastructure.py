"""
Test 000: Infrastructure Verification

Verifies that all test files and baseline outputs exist and match their expected hashes.

This test MUST pass before running any other tests. It ensures:
1. All PDF test files exist and are uncorrupted
2. All baseline text outputs exist and match expected hashes
3. All baseline image outputs exist and match expected hashes
4. File manifest is up-to-date

RUN: pytest -v tests/test_000_infrastructure.py
"""

import pytest
import csv
import hashlib
from pathlib import Path
import sys

# Add lib to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'lib'))

from manifest_generator import ManifestGenerator


def compute_file_hash(filepath: Path) -> str:
    """Compute MD5 hash of a file."""
    md5_hash = hashlib.md5()
    with open(filepath, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b""):
            md5_hash.update(chunk)
    return md5_hash.hexdigest()


@pytest.fixture(scope="module")
def integration_tests_root():
    """Root directory of integration tests."""
    return Path(__file__).parent.parent


@pytest.fixture(scope="module")
def main_manifest_path(integration_tests_root):
    """Path to main file manifest."""
    return integration_tests_root / 'master_test_suite' / 'file_manifest.csv'


@pytest.fixture(scope="module")
def main_manifest_rows(main_manifest_path):
    """Load main manifest rows."""
    if not main_manifest_path.exists():
        pytest.skip(f"Main manifest not found: {main_manifest_path}\n"
                   f"Generate it with: cd integration_tests && python lib/manifest_generator.py generate-main")

    with open(main_manifest_path, 'r') as f:
        reader = csv.DictReader(f)
        return list(reader)


# ============================================================================
# Test 000.1: Main Manifest Exists and is Valid
# ============================================================================

@pytest.mark.infrastructure
@pytest.mark.smoke
def test_000_1_main_manifest_exists(main_manifest_path):
    """
    Verify main file manifest exists.

    META:
      id: infrastructure_001
      category: infrastructure
      level: smoke
      validates: Main manifest file exists and is readable
      impact: critical
    """
    assert main_manifest_path.exists(), \
        f"Main manifest not found: {main_manifest_path}\n" \
        f"Generate it with: cd integration_tests && python lib/manifest_generator.py generate-main"

    # Check it's not empty
    with open(main_manifest_path, 'r') as f:
        rows = list(csv.DictReader(f))
        assert len(rows) > 0, "Main manifest is empty"


@pytest.mark.infrastructure
@pytest.mark.smoke
def test_000_2_main_manifest_has_required_columns(main_manifest_rows):
    """
    Verify main manifest has all required columns.

    META:
      id: infrastructure_002
      category: infrastructure
      level: smoke
      validates: Manifest schema is correct
      impact: critical
    """
    required_columns = [
        'pdf_name',
        'pdf_path',
        'pdf_exists',
        'pdf_md5',
        'pdf_size',
        'pdf_pages',
        'text_baseline_path',
        'text_baseline_exists',
        'text_baseline_md5',
        'image_baseline_json_path',
        'image_baseline_json_exists'
    ]

    if not main_manifest_rows:
        pytest.skip("No rows in manifest")

    first_row = main_manifest_rows[0]

    for column in required_columns:
        assert column in first_row, f"Required column missing: {column}"


# ============================================================================
# Test 000.3: All PDFs Exist
# ============================================================================

def pytest_generate_tests(metafunc):
    """Parametrize tests with manifest rows."""
    if "manifest_row" in metafunc.fixturenames:
        # Load manifest
        integration_tests_root = Path(__file__).parent.parent
        main_manifest_path = integration_tests_root / 'master_test_suite' / 'file_manifest.csv'

        if not main_manifest_path.exists():
            pytest.skip("Main manifest not found")

        with open(main_manifest_path, 'r') as f:
            reader = csv.DictReader(f)
            rows = list(reader)

        metafunc.parametrize("manifest_row", rows, ids=lambda r: r['pdf_name'])


@pytest.mark.infrastructure
@pytest.mark.full
def test_000_3_pdf_exists(manifest_row, integration_tests_root):
    """
    Verify PDF test file exists.

    META:
      id: infrastructure_003
      category: infrastructure
      level: full
      validates: All PDF files exist and are readable
      impact: critical
    """
    pdf_path = integration_tests_root / manifest_row['pdf_path']

    assert pdf_path.exists(), f"PDF not found: {pdf_path}"
    assert pdf_path.stat().st_size > 0, f"PDF is empty: {pdf_path}"


@pytest.mark.infrastructure
@pytest.mark.full
def test_000_4_pdf_hash_matches(manifest_row, integration_tests_root):
    """
    Verify PDF file hash matches manifest.

    META:
      id: infrastructure_004
      category: infrastructure
      level: full
      validates: PDF files are uncorrupted and match expected content
      impact: critical
    """
    pdf_path = integration_tests_root / manifest_row['pdf_path']

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_path}")

    expected_md5 = manifest_row['pdf_md5']
    if not expected_md5:
        pytest.skip("No MD5 in manifest")

    current_md5 = compute_file_hash(pdf_path)

    assert current_md5 == expected_md5, \
        f"PDF hash mismatch: {manifest_row['pdf_name']}\n" \
        f"  Expected: {expected_md5}\n" \
        f"  Current:  {current_md5}\n" \
        f"  This indicates the PDF file has been modified since manifest generation."


# ============================================================================
# Test 000.5: Text Baselines Exist
# ============================================================================

@pytest.mark.infrastructure
@pytest.mark.full
def test_000_5_text_baseline_exists(manifest_row, integration_tests_root):
    """
    Verify text baseline exists for each PDF.

    META:
      id: infrastructure_005
      category: infrastructure
      level: full
      validates: All text baselines exist
      impact: critical
    """
    text_baseline_exists = manifest_row['text_baseline_exists'] == 'True'

    if not text_baseline_exists:
        text_path = integration_tests_root / manifest_row['text_baseline_path']
        pytest.fail(
            f"Text baseline missing: {manifest_row['pdf_name']}\n"
            f"  Expected: {text_path}\n"
            f"  Generate with: cd integration_tests && python lib/baseline_generator.py {manifest_row['pdf_name']}"
        )

    text_path = integration_tests_root / manifest_row['text_baseline_path']
    assert text_path.exists(), f"Text baseline file missing: {text_path}"


@pytest.mark.infrastructure
@pytest.mark.full
def test_000_6_text_baseline_hash_matches(manifest_row, integration_tests_root):
    """
    Verify text baseline hash matches manifest.

    META:
      id: infrastructure_006
      category: infrastructure
      level: full
      validates: Text baselines are uncorrupted
      impact: critical
    """
    text_baseline_exists = manifest_row['text_baseline_exists'] == 'True'
    if not text_baseline_exists:
        pytest.skip("Text baseline does not exist in manifest")

    text_path = integration_tests_root / manifest_row['text_baseline_path']
    if not text_path.exists():
        pytest.skip(f"Text baseline not found: {text_path}")

    expected_md5 = manifest_row['text_baseline_md5']
    if not expected_md5:
        pytest.skip("No MD5 in manifest")

    current_md5 = compute_file_hash(text_path)

    assert current_md5 == expected_md5, \
        f"Text baseline hash mismatch: {manifest_row['pdf_name']}\n" \
        f"  Expected: {expected_md5}\n" \
        f"  Current:  {current_md5}\n" \
        f"  This indicates the baseline has been modified."


# ============================================================================
# Test 000.7: Image Baselines Exist
# ============================================================================

@pytest.mark.infrastructure
@pytest.mark.full
@pytest.mark.image
def test_000_7_image_baseline_json_exists(manifest_row, integration_tests_root):
    """
    Verify image baseline JSON exists for each PDF.

    META:
      id: infrastructure_007
      category: infrastructure
      level: full
      validates: All image baseline metadata files exist
      impact: high
    """
    image_json_exists = manifest_row['image_baseline_json_exists'] == 'True'

    if not image_json_exists:
        json_path = integration_tests_root / manifest_row['image_baseline_json_path']
        pytest.fail(
            f"Image baseline JSON missing: {manifest_row['pdf_name']}\n"
            f"  Expected: {json_path}\n"
            f"  Generate with: cd integration_tests && python lib/baseline_generator.py --images-only"
        )

    json_path = integration_tests_root / manifest_row['image_baseline_json_path']
    assert json_path.exists(), f"Image baseline JSON missing: {json_path}"


@pytest.mark.infrastructure
@pytest.mark.corpus
@pytest.mark.image
def test_000_9_all_page_images_exist(manifest_row, integration_tests_root):
    """
    Verify all page image baselines exist for each PDF (PPM MD5 format).

    META:
      id: infrastructure_009
      category: infrastructure
      level: extended
      validates: PPM image baselines (JSON with MD5 hashes) exist
      impact: high
      duration: 10m (checks all image baselines)
    """
    pdf_stem = Path(manifest_row['pdf_name']).stem

    # Check for PPM JSON baseline (new format: MD5 hashes only)
    ppm_baseline_dir = integration_tests_root / 'baselines' / 'upstream' / 'images_ppm'
    ppm_baseline_path = ppm_baseline_dir / f'{pdf_stem}.json'

    if not ppm_baseline_path.exists():
        pytest.skip(f"PPM baseline not found: {ppm_baseline_path}")

    # Load PPM baseline
    import json
    baseline_data = json.loads(ppm_baseline_path.read_text())

    # Validate baseline structure
    assert 'format' in baseline_data, f"Missing 'format' key in {ppm_baseline_path}"
    assert baseline_data['format'] == 'ppm', f"Expected format 'ppm', got '{baseline_data['format']}'"
    assert 'dpi' in baseline_data, f"Missing 'dpi' key in {ppm_baseline_path}"
    assert baseline_data['dpi'] == 300, f"Expected DPI 300, got {baseline_data['dpi']}"
    assert 'pages' in baseline_data, f"Missing 'pages' key in {ppm_baseline_path}"

    pages = baseline_data['pages']
    if not pages:
        pytest.skip(f"No pages in baseline: {ppm_baseline_path}")

    # Verify all page MD5 hashes are non-empty
    invalid_pages = []
    for page_num, md5_hash in pages.items():
        if not md5_hash or len(md5_hash) != 32:
            invalid_pages.append(f"Page {page_num}: invalid MD5 '{md5_hash}'")

    if invalid_pages:
        pytest.fail(
            f"Invalid MD5 hashes in baseline for {manifest_row['pdf_name']}:\n" +
            "\n".join(f"  - {page}" for page in invalid_pages[:10]) +
            (f"\n  ... and {len(invalid_pages) - 10} more" if len(invalid_pages) > 10 else "")
        )


# ============================================================================
# Test 000.10: Summary Statistics
# ============================================================================

@pytest.mark.infrastructure
@pytest.mark.smoke
def test_000_10_manifest_summary(main_manifest_rows, integration_tests_root):
    """
    Display summary statistics for manifest coverage.

    META:
      id: infrastructure_010
      category: infrastructure
      level: smoke
      validates: Overall test infrastructure health
      impact: informational
    """
    stats = {
        'total_pdfs': len(main_manifest_rows),
        'pdfs_exist': 0,
        'text_baselines_exist': 0,
        'image_jsons_exist': 0,
        'total_pages': 0,
    }

    for row in main_manifest_rows:
        if row['pdf_exists'] == 'True':
            stats['pdfs_exist'] += 1

        if row['text_baseline_exists'] == 'True':
            stats['text_baselines_exist'] += 1

        if row['image_baseline_json_exists'] == 'True':
            stats['image_jsons_exist'] += 1

        if row['pdf_pages'] and row['pdf_pages'] != '-1':
            stats['total_pages'] += int(row['pdf_pages'])

    # Print summary
    print("\n" + "=" * 60)
    print("TEST INFRASTRUCTURE SUMMARY")
    print("=" * 60)
    print(f"Total PDFs in master list: {stats['total_pdfs']}")
    print(f"PDFs exist: {stats['pdfs_exist']} ({stats['pdfs_exist']/stats['total_pdfs']*100:.1f}%)")
    print(f"Text baselines exist: {stats['text_baselines_exist']} ({stats['text_baselines_exist']/stats['total_pdfs']*100:.1f}%)")
    print(f"Image JSONs exist: {stats['image_jsons_exist']} ({stats['image_jsons_exist']/stats['total_pdfs']*100:.1f}%)")
    print(f"Total pages: {stats['total_pages']}")
    print("=" * 60)

    # All PDFs must exist
    assert stats['pdfs_exist'] == stats['total_pdfs'], \
        f"Not all PDFs exist: {stats['pdfs_exist']}/{stats['total_pdfs']}"

    # Warn if baselines are incomplete
    if stats['text_baselines_exist'] < stats['total_pdfs']:
        pytest.fail(
            f"Text baselines incomplete: {stats['text_baselines_exist']}/{stats['total_pdfs']}\n"
            f"Generate with: cd integration_tests && python lib/baseline_generator.py --text-only"
        )

    if stats['image_jsons_exist'] < stats['total_pdfs']:
        print(f"\nWarning: Image baselines incomplete: {stats['image_jsons_exist']}/{stats['total_pdfs']}")
        print(f"Generate with: cd integration_tests && python lib/baseline_generator.py --images-only")
