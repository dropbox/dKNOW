#!/usr/bin/env python3
"""
Regenerate JSONL files for all PDFs that have text baselines but no JSONL.

Only regenerates JSONL - doesn't touch text or images.
Fast: ~30-60 seconds per PDF (only page 0)
"""

import subprocess
import json
from pathlib import Path
import os
import hashlib
import sys

# Setup paths
root = Path(__file__).parent.parent
pdfium_root = root.parent
jsonl_tool = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'extract_text_jsonl'
baseline_lib = pdfium_root / 'out' / 'Release' / 'libpdfium.dylib'
pdf_dir = root / 'pdfs'

# Verify tool exists
if not jsonl_tool.exists():
    print(f"ERROR: Tool not found: {jsonl_tool}")
    print("Build it with: cd rust && cargo build --release --examples")
    sys.exit(1)

if not baseline_lib.exists():
    print(f"ERROR: Library not found: {baseline_lib}")
    sys.exit(1)

env = os.environ.copy()
env['DYLD_LIBRARY_PATH'] = str(baseline_lib.parent)

# Find all PDFs with manifests
manifest_files = sorted(root.glob('master_test_suite/expected_outputs/*/*/manifest.json'))

success = 0
errors = 0
skipped = 0

print(f"Found {len(manifest_files)} manifests")
print(f"Tool: {jsonl_tool}")
print(f"Library: {baseline_lib}")
print(f"=" * 70)

for i, manifest_path in enumerate(manifest_files):
    with open(manifest_path) as f:
        manifest = json.load(f)

    pdf_name = manifest['pdf']

    # Skip if page_count is 0 (check both 'page_count' and 'pdf_pages')
    page_count = manifest.get('page_count', manifest.get('pdf_pages', 0))
    if page_count == 0:
        print(f"[{i+1}/{len(manifest_files)}] SKIP: {pdf_name} (0 pages)")
        skipped += 1
        continue

    # Skip if already has JSONL with pages AND the file has content (> 0 bytes)
    jsonl_pages = manifest.get('jsonl', {}).get('pages')
    if jsonl_pages:
        # Check if page 0 JSONL file has content
        page_0_bytes = jsonl_pages[0].get('bytes', 0) if len(jsonl_pages) > 0 else 0
        if page_0_bytes > 0:
            print(f"[{i+1}/{len(manifest_files)}] SKIP: {pdf_name} (already has JSONL)")
            skipped += 1
            continue
        # If 0 bytes, regenerate it

    # Find PDF file - search in multiple directories
    pdf_path = None
    for subdir in ['benchmark', 'edge_cases', 'arxiv', 'cc', 'edinet', 'japanese', 'pages', 'web']:
        candidate = pdf_dir / subdir / pdf_name
        if candidate.exists():
            pdf_path = candidate
            break

    # Try without subdirectory
    if not pdf_path:
        candidate = pdf_dir / pdf_name
        if candidate.exists():
            pdf_path = candidate

    if not pdf_path or not pdf_path.exists():
        print(f"[{i+1}/{len(manifest_files)}] SKIP: {pdf_name} (PDF not found in pdfs/)")
        skipped += 1
        continue

    # Regenerate JSONL
    jsonl_dir = manifest_path.parent / 'jsonl'
    jsonl_dir.mkdir(exist_ok=True)
    jsonl_file = jsonl_dir / 'page_0000.jsonl'

    try:
        result = subprocess.run(
            [str(jsonl_tool), str(pdf_path), str(jsonl_file), '0'],
            capture_output=True,
            env=env,
            timeout=120
        )

        if result.returncode == 0 and jsonl_file.exists():
            # Update manifest
            line_count = sum(1 for _ in open(jsonl_file, 'rb'))
            file_size = jsonl_file.stat().st_size
            file_md5 = hashlib.md5(open(jsonl_file, 'rb').read()).hexdigest()

            manifest['jsonl'] = {
                "note": "Character-level metadata for page 0",
                "pages": [{
                    "page": 0,
                    "path": "jsonl/page_0000.jsonl",
                    "md5": file_md5,
                    "bytes": file_size,
                    "lines": line_count,
                    "char_count": line_count
                }]
            }

            with open(manifest_path, 'w') as f:
                json.dump(manifest, f, indent=2)

            print(f"[{i+1}/{len(manifest_files)}] ✓ {pdf_name} ({line_count} chars, {file_size} bytes)")
            success += 1
        else:
            stderr = result.stderr.decode('utf-8', errors='ignore') if result.stderr else ''
            print(f"[{i+1}/{len(manifest_files)}] ✗ {pdf_name} (extraction failed, exit={result.returncode})")
            if stderr:
                print(f"    Error: {stderr[:200]}")
            errors += 1

    except subprocess.TimeoutExpired:
        print(f"[{i+1}/{len(manifest_files)}] ✗ {pdf_name} (timeout after 120s)")
        errors += 1
    except Exception as e:
        print(f"[{i+1}/{len(manifest_files)}] ✗ {pdf_name} (error: {e})")
        errors += 1

print(f"\n{'='*70}")
print(f"JSONL Regeneration Complete")
print(f"  Success: {success}")
print(f"  Errors: {errors}")
print(f"  Skipped: {skipped}")
print(f"  Total: {len(manifest_files)}")
print(f"{'='*70}")
