#!/usr/bin/env python3
"""
Download pdfium_fast test PDFs from Dropbox with proper handling.

This handles Dropbox's redirect behavior and downloads the actual file.
"""

import sys
import os
import tarfile
from pathlib import Path

def download_with_requests(url, output_path):
    """Download using requests library with proper redirect handling."""
    try:
        import requests
    except ImportError:
        print("requests library not found. Installing...")
        import subprocess
        subprocess.check_call([sys.executable, "-m", "pip", "install", "requests"])
        import requests

    # Force direct download by changing dl=0 to dl=1
    download_url = url.replace('dl=0', 'dl=1')

    print(f"Downloading from Dropbox...")
    print(f"URL: {download_url}")
    print()

    # Use streaming to show progress
    response = requests.get(download_url, stream=True, allow_redirects=True)
    response.raise_for_status()

    # Check if we got HTML instead of the file
    content_type = response.headers.get('content-type', '')
    if 'text/html' in content_type:
        print("✗ Received HTML page instead of file")
        print()
        print("Dropbox link may not support automated downloads.")
        print("Please download manually:")
        print(f"  1. Visit: {url}")
        print(f"  2. Click 'Download' button")
        print(f"  3. Save to: {output_path.parent}/")
        print(f"  4. Run: tar xzf {output_path.name}")
        return False

    total_size = int(response.headers.get('content-length', 0))
    block_size = 1024 * 1024  # 1MB
    downloaded = 0

    with open(output_path, 'wb') as f:
        for chunk in response.iter_content(chunk_size=block_size):
            if chunk:
                f.write(chunk)
                downloaded += len(chunk)
                if total_size > 0:
                    percent = downloaded * 100 // total_size
                    mb_down = downloaded / (1024 * 1024)
                    mb_total = total_size / (1024 * 1024)
                    print(f"\rProgress: {percent}% ({mb_down:.1f}/{mb_total:.1f} MB)", end='', flush=True)

    print()  # New line after progress
    return True

def main():
    # GitHub Release (requires repo access for private repos)
    GITHUB_URL = "https://github.com/ayates_dbx/pdfium_fast/releases/download/test-pdfs-v1/pdfium_test_pdfs.tar.gz"

    script_dir = Path(__file__).parent.resolve()
    os.chdir(script_dir)

    print("=" * 60)
    print("PDFium Fast - Test PDF Download")
    print("=" * 60)
    print()

    # Check if already exists
    pdfs_dir = script_dir / "pdfs"
    if pdfs_dir.exists():
        pdf_count = len(list(pdfs_dir.rglob("*.pdf")))
        if pdf_count > 100:
            print(f"✓ Test PDFs already exist ({pdf_count} PDFs)")
            return 0

    archive_path = script_dir / "pdfium_test_pdfs.tar.gz"

    # Try download
    success = download_with_requests(GITHUB_URL, archive_path)

    if not success:
        return 1

    # Verify size
    size_mb = archive_path.stat().st_size / (1024 * 1024)
    print(f"✓ Downloaded: {size_mb:.1f} MB")

    if size_mb < 100:
        print("✗ File too small - likely not the actual archive")
        archive_path.unlink()
        return 1

    print()
    print("Extracting...")

    try:
        with tarfile.open(archive_path, 'r:gz') as tar:
            tar.extractall(script_dir)
        print("✓ Extraction complete")
    except Exception as e:
        print(f"✗ Extraction failed: {e}")
        return 1

    # Cleanup
    archive_path.unlink()

    # Verify
    pdf_count = len(list(pdfs_dir.rglob("*.pdf")))
    print()
    print("=" * 60)
    print("✓ Success!")
    print("=" * 60)
    print(f"PDFs: {pdf_count}")
    print()
    print("Run tests:")
    print("  pytest -m smoke")
    print("  pytest -m extended")
    print()

    return 0

if __name__ == "__main__":
    sys.exit(main())
