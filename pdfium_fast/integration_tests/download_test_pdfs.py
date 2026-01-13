#!/usr/bin/env python3
"""
Download pdfium_fast test PDF corpus from GitHub Releases.

This script downloads the complete test PDF corpus (1.4GB compressed, 1.5GB uncompressed)
required for running the integration test suite.

Usage:
    cd integration_tests
    python3 download_test_pdfs.py

Requirements:
    - Python 3.6+
    - 2GB free disk space
"""

import os
import sys
import tarfile
from pathlib import Path
import urllib.request
import urllib.error

# GitHub Release URL
GITHUB_RELEASE_URL = "https://github.com/dropbox/dKNOW/pdfium_fast/releases/download/test-pdfs-v1/pdfium_test_pdfs.tar.gz"
ARCHIVE_NAME = "pdfium_test_pdfs.tar.gz"
EXPECTED_SIZE_MB = 1400

def main():
    script_dir = Path(__file__).parent.resolve()
    os.chdir(script_dir)

    print("=" * 60)
    print("PDFium Fast - Test PDF Download")
    print("=" * 60)
    print()

    # Check if PDFs already exist
    pdfs_dir = script_dir / "pdfs"
    if pdfs_dir.exists():
        pdf_count = len(list(pdfs_dir.rglob("*.pdf")))
        if pdf_count > 100:
            print(f"✓ Test PDFs already exist ({pdf_count} PDFs found)")
            print()
            print("To re-download, remove the pdfs/ directory first:")
            print("  rm -rf pdfs/")
            print("  python3 download_test_pdfs.py")
            return 0

    print(f"Downloading test PDF corpus...")
    print(f"Source: GitHub Releases")
    print(f"URL: {GITHUB_RELEASE_URL}")
    print(f"Size: ~{EXPECTED_SIZE_MB}MB compressed")
    print()
    print("This may take 5-10 minutes depending on your connection...")
    print()

    archive_path = script_dir / ARCHIVE_NAME

    # Download with urllib (built-in, no dependencies)
    try:
        def progress_hook(block_num, block_size, total_size):
            if total_size > 0:
                downloaded = block_num * block_size
                percent = min(100, downloaded * 100 // total_size)
                mb_downloaded = downloaded / (1024 * 1024)
                mb_total = total_size / (1024 * 1024)
                print(f"\rDownloading: {percent}% ({mb_downloaded:.1f}/{mb_total:.1f} MB)", end='', flush=True)

        urllib.request.urlretrieve(GITHUB_RELEASE_URL, archive_path, progress_hook)
        print()  # New line after progress

    except urllib.error.URLError as e:
        print(f"\n✗ Download failed: {e}")
        print()
        if "404" in str(e):
            print("This is a PRIVATE repository. Authentication required.")
            print()
            print("METHOD 1: Use GitHub CLI (gh) - Recommended")
            print("  1. Install gh: brew install gh")
            print("  2. Authenticate: gh auth login")
            print("  3. Download:")
            print(f"     cd {script_dir}")
            print("     gh release download test-pdfs-v1 --repo dropbox/dKNOW/pdfium_fast")
            print(f"     tar xzf {ARCHIVE_NAME}")
            print()
            print("METHOD 2: Manual download via browser")
            print("  1. Visit: https://github.com/dropbox/dKNOW/pdfium_fast/releases")
            print("  2. Log in to GitHub")
            print("  3. Find the 'test-pdfs-v1' release")
            print("  4. Download 'pdfium_test_pdfs.tar.gz'")
            print(f"  5. Save to: {script_dir}/")
            print(f"  6. Run: tar xzf {ARCHIVE_NAME}")
        else:
            print("Alternative download methods:")
            print("  1. Download manually from GitHub:")
            print(f"     Visit: https://github.com/dropbox/dKNOW/pdfium_fast/releases")
            print(f"     Find: test-pdfs-v1 release")
            print(f"     Download: pdfium_test_pdfs.tar.gz")
            print("  2. Extract to: integration_tests/")
            print(f"  3. Run: tar xzf {ARCHIVE_NAME}")
        return 1

    # Verify download
    downloaded_size = archive_path.stat().st_size / (1024 * 1024)
    print(f"✓ Downloaded: {downloaded_size:.1f} MB")

    if downloaded_size < 100:
        print()
        print("✗ Downloaded file is too small (likely an error page)")
        print()
        print("Please download manually:")
        print("  1. Visit: https://github.com/dropbox/dKNOW/pdfium_fast/releases")
        print("  2. Find the 'test-pdfs-v1' release")
        print("  3. Download 'pdfium_test_pdfs.tar.gz'")
        print(f"  4. Save to: {script_dir}/")
        print(f"  5. Run: tar xzf {ARCHIVE_NAME}")
        archive_path.unlink()
        return 1

    print()
    print("Extracting archive...")

    try:
        with tarfile.open(archive_path, 'r:gz') as tar:
            tar.extractall(script_dir)
    except Exception as e:
        print(f"\n✗ Extraction failed: {e}")
        return 1

    print("✓ Extraction complete")

    # Clean up
    print()
    print("Cleaning up...")
    archive_path.unlink()

    # Verify
    pdf_count = len(list(pdfs_dir.rglob("*.pdf")))

    print()
    print("=" * 60)
    print("✓ Download complete!")
    print("=" * 60)
    print()
    print(f"PDFs extracted: {pdf_count}")
    print(f"Location: {pdfs_dir}/")
    print()
    print("Next steps:")
    print("  1. Run smoke tests: pytest -m smoke")
    print("  2. Run full suite: pytest -m extended")
    print()

    return 0

if __name__ == "__main__":
    sys.exit(main())
