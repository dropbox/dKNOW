#!/usr/bin/env python3
"""
Validate image rendering against upstream pdfium_test

Compares our PNG renders vs upstream pdfium_test .ppm renders
Uses SSIM (Structural Similarity Index) for perceptual comparison
"""

import subprocess
import tempfile
from pathlib import Path
import os
import hashlib
import json
from datetime import datetime
import numpy as np
from skimage import io, metrics, transform
from PIL import Image

pdfium_root = Path(__file__).parent.parent.parent
pdfium_test = pdfium_root / 'out' / 'Optimized-Shared' / 'pdfium_test'
render_pages = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'render_pages'

# Test PDFs - 50 representative samples across different categories
TEST_PDFS = [
    # Arxiv (10) - Scientific papers
    'arxiv_001.pdf', 'arxiv_004.pdf', 'arxiv_007.pdf', 'arxiv_010.pdf', 'arxiv_012.pdf',
    'arxiv_014.pdf', 'arxiv_015.pdf', 'arxiv_016.pdf', 'arxiv_017.pdf', 'arxiv_018.pdf',

    # CC (10) - Common Crawl documents (various sizes)
    'cc_007_101p.pdf', 'cc_015_101p.pdf', 'cc_008_116p.pdf', 'cc_013_122p.pdf',
    'cc_009_188p.pdf', 'cc_010_206p.pdf', 'cc_003_162p.pdf', 'cc_004_291p.pdf',
    'cc_002_522p.pdf', 'cc_001_931p.pdf',

    # Edinet (10) - Japanese corporate reports
    'edinet_2025-06-24_1318_E01920_Makita Corporation.pdf',
    'edinet_2025-06-25_1608_E02628_KIMURATAN CORPORATION.pdf',
    'edinet_2025-06-24_0930_E03557_The Chiba Kogyo Bank Ltd.pdf',
    'edinet_2025-06-25_1027_E01750_DAIHEN CORPORATION.pdf',
    'edinet_2025-06-25_1357_E01684_SEIKO CORPORATION.pdf',
    'edinet_2025-06-25_1433_E01754_KAWADEN CORPORATION.pdf',
    'edinet_2025-06-25_1507_E01402_SUNCALL CORPORATION.pdf',
    'edinet_2025-06-25_1557_E05670_HIKARI HEIGHTS-VARUS COLTD.pdf',
    'edinet_2025-06-25_1634_E03116_AOKI Holdings Inc.pdf',
    'edinet_2025-06-24_1124_E05556_Saint Marc Holdings CoLtd.pdf',

    # Web (10) - Web-scraped PDFs
    'web_001.pdf', 'web_003.pdf', 'web_005.pdf', 'web_007.pdf', 'web_009.pdf',
    'web_011.pdf', 'web_012.pdf', 'web_013.pdf', 'web_014.pdf', 'web_015.pdf',

    # Various sizes (10) - Page count diversity
    '0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf',
    '0214pages_BZK65P6W74FNMD4O3A24LH3F2XLFH7V2.pdf',
    '0215pages_UAU4SPHOQHADJRFKILFDYETBGB6HXYU2.pdf',
    '0255pages_OKRV2XLZ2UGGRCBKHS3I5DA5TNLNVHWC.pdf',
    '0277pages_YCOBKTA46F7XEOWT4LS3BCPGDOMQSBWF.pdf',
    '0291pages_RGLWPWLY4JE6RPNCCHQ3LQWDFLPBGHXQ.pdf',
    '0309pages_7LD3RVJDZGTXF53CDLCI67YPWZQ5POOA.pdf',
    '0313pages_7DLRSEXVIADBZZUN4IJIACW5T23YBH7H.pdf',
    '0496pages_E3474JUEVRWQ3P2J2I3XBFKMMVZLLKWZ.pdf',
    '0821pages_LUNFJFH4KWZ3ZFNRO43WSMZPLM4OLB7C.pdf',
]

def validate_pdf(pdf_name):
    """
    Validate a single PDF by comparing our render vs upstream pdfium_test

    Returns dict with validation results
    """
    pdf_path = pdfium_root / 'integration_tests' / 'pdfs' / 'benchmark' / pdf_name

    if not pdf_path.exists():
        return {'pdf': pdf_name, 'status': 'pdf_not_found', 'error': str(pdf_path)}

    with tempfile.TemporaryDirectory() as upstream_dir:
        with tempfile.TemporaryDirectory() as ours_dir:
            env = os.environ.copy()
            env['DYLD_LIBRARY_PATH'] = str(pdfium_test.parent)

            # Copy PDF to upstream directory so pdfium_test writes output files there
            import shutil
            upstream_pdf = Path(upstream_dir) / pdf_name
            shutil.copy2(pdf_path, upstream_pdf)

            # Generate upstream (pdfium_test creates .ppm files with --ppm flag)
            # Files are written to the same directory as the PDF
            # Use --scale=4.166666 to match 300 DPI (300/72 = 4.166666)
            print(f"  Generating upstream renders for {pdf_name}...", flush=True)
            try:
                result = subprocess.run(
                    [str(pdfium_test), '--ppm', '--scale=4.166666', pdf_name],
                    capture_output=True,
                    env=env,
                    timeout=300,
                    cwd=upstream_dir
                )
            except subprocess.TimeoutExpired:
                return {'pdf': pdf_name, 'status': 'upstream_timeout'}

            if result.returncode != 0:
                return {
                    'pdf': pdf_name,
                    'status': 'upstream_failed',
                    'error': result.stderr.decode('utf-8', errors='replace')
                }

            # Convert ppm to png (using macOS native sips)
            print(f"  Converting PPM to PNG...", flush=True)
            ppm_files = sorted(Path(upstream_dir).glob('*.ppm'))

            if not ppm_files:
                return {'pdf': pdf_name, 'status': 'no_ppm_files'}

            for ppm in ppm_files:
                png_path = ppm.with_suffix('.png')
                try:
                    subprocess.run(
                        ['sips', '-s', 'format', 'png', str(ppm), '--out', str(png_path)],
                        check=True,
                        capture_output=True,
                        timeout=60
                    )
                except subprocess.CalledProcessError as e:
                    return {
                        'pdf': pdf_name,
                        'status': 'conversion_failed',
                        'error': e.stderr.decode('utf-8', errors='replace')
                    }

            # Generate ours
            print(f"  Generating our renders for {pdf_name}...", flush=True)
            try:
                result = subprocess.run(
                    [str(render_pages), str(pdf_path), ours_dir, '1', '300'],
                    capture_output=True,
                    env=env,
                    timeout=300,
                    check=True
                )
            except subprocess.TimeoutExpired:
                return {'pdf': pdf_name, 'status': 'our_timeout'}
            except subprocess.CalledProcessError as e:
                return {
                    'pdf': pdf_name,
                    'status': 'our_failed',
                    'error': e.stderr.decode('utf-8', errors='replace')
                }

            # Compare using SSIM (Structural Similarity Index) for perceptual comparison
            # Upstream names: pdf_name.N.png (N is 0-based page number)
            # Our names: page_NNNN.png (NNNN is 0-based page number with leading zeros)

            ssim_scores = []
            low_similarity_pages = []
            comparison_errors = []

            # Build dictionaries mapping page number to file path
            upstream_pngs = {}
            for png_file in Path(upstream_dir).glob('*.png'):
                # Extract page number from filename like "arxiv_001.pdf.0.png"
                # Page number is between the last two dots
                parts = png_file.stem.split('.')
                if len(parts) >= 2:
                    try:
                        page_num = int(parts[-1])
                        upstream_pngs[page_num] = png_file
                    except ValueError:
                        pass

            our_pngs = {}
            for png_file in Path(ours_dir).glob('page_*.png'):
                # Extract page number from filename like "page_0000.png"
                try:
                    page_num = int(png_file.stem.split('_')[1])
                    our_pngs[page_num] = png_file
                except (ValueError, IndexError):
                    pass

            if len(upstream_pngs) != len(our_pngs):
                return {
                    'pdf': pdf_name,
                    'status': 'page_count_mismatch',
                    'upstream_pages': len(upstream_pngs),
                    'our_pages': len(our_pngs)
                }

            print(f"  Comparing {len(upstream_pngs)} pages using SSIM...", flush=True)
            for page_num in sorted(upstream_pngs.keys()):
                if page_num not in our_pngs:
                    comparison_errors.append({
                        'page': page_num + 1,
                        'error': 'missing_in_our_output'
                    })
                    continue

                try:
                    # Load images using PIL for better format handling
                    upstream_img = Image.open(upstream_pngs[page_num]).convert('RGB')
                    our_img = Image.open(our_pngs[page_num]).convert('RGB')

                    # Convert to numpy arrays
                    upstream_arr = np.array(upstream_img)
                    our_arr = np.array(our_img)

                    # Handle dimension differences by resizing to match upstream
                    if upstream_arr.shape != our_arr.shape:
                        # Resize our image to match upstream dimensions
                        our_arr = transform.resize(
                            our_arr,
                            upstream_arr.shape,
                            preserve_range=True,
                            anti_aliasing=True
                        ).astype(np.uint8)

                    # Compute SSIM on full image (data_range is 255 for 8-bit images)
                    ssim_score = metrics.structural_similarity(
                        upstream_arr,
                        our_arr,
                        channel_axis=2,  # RGB channels
                        data_range=255
                    )

                    ssim_scores.append(ssim_score)

                    # Track pages with low similarity (< 0.95 is concerning)
                    if ssim_score < 0.95:
                        low_similarity_pages.append({
                            'page': page_num + 1,
                            'ssim': ssim_score,
                            'upstream_shape': upstream_arr.shape,
                            'our_shape': np.array(Image.open(our_pngs[page_num]).convert('RGB')).shape
                        })

                except Exception as e:
                    comparison_errors.append({
                        'page': page_num + 1,
                        'error': f'comparison_failed: {str(e)}'
                    })

            # Calculate statistics
            if ssim_scores:
                mean_ssim = np.mean(ssim_scores)
                min_ssim = np.min(ssim_scores)
                max_ssim = np.max(ssim_scores)
            else:
                mean_ssim = min_ssim = max_ssim = 0.0

            return {
                'pdf': pdf_name,
                'status': 'compared',
                'total_pages': len(upstream_pngs),
                'compared_pages': len(ssim_scores),
                'ssim_mean': float(mean_ssim),
                'ssim_min': float(min_ssim),
                'ssim_max': float(max_ssim),
                'low_similarity_pages': low_similarity_pages if low_similarity_pages else None,
                'comparison_errors': comparison_errors if comparison_errors else None
            }

# Run validation
print("="*80)
print("Image Validation vs Upstream pdfium_test")
print("="*80)
print(f"Start time: {datetime.now().isoformat()}")
print(f"Testing {len(TEST_PDFS)} PDFs")
print()

results = []
for i, pdf in enumerate(TEST_PDFS, 1):
    print(f"[{i}/{len(TEST_PDFS)}] {pdf}")
    result = validate_pdf(pdf)
    results.append(result)

    if result['status'] == 'compared':
        mean_ssim = result['ssim_mean']
        min_ssim = result['ssim_min']
        pages_compared = result['compared_pages']
        total_pages = result['total_pages']

        if mean_ssim >= 0.99:
            status = '✓'
        elif mean_ssim >= 0.95:
            status = '○'
        else:
            status = '✗'

        print(f"  {status} SSIM: mean={mean_ssim:.4f}, min={min_ssim:.4f}, pages={pages_compared}/{total_pages}")

        if result.get('low_similarity_pages'):
            print(f"     Low similarity: {len(result['low_similarity_pages'])} pages")
        if result.get('comparison_errors'):
            print(f"     Errors: {len(result['comparison_errors'])} pages")
    else:
        print(f"  ✗ {result['status']}")
    print()

# Save detailed results to JSON
results_file = pdfium_root / 'integration_tests' / 'image_validation_results.json'
with open(results_file, 'w') as f:
    json.dump({
        'timestamp': datetime.now().isoformat(),
        'pdfium_test': str(pdfium_test),
        'render_pages': str(render_pages),
        'results': results
    }, f, indent=2)

print()
print("="*80)
print("Image Validation Summary")
print("="*80)

# Calculate statistics
compared = [r for r in results if r['status'] == 'compared']
failed = [r for r in results if r['status'] != 'compared']

if compared:
    total_pages = sum(r['compared_pages'] for r in compared)
    all_ssim_scores = []

    for r in compared:
        # Weight by number of pages for overall average
        all_ssim_scores.extend([r['ssim_mean']] * r['compared_pages'])

    overall_mean_ssim = np.mean(all_ssim_scores) if all_ssim_scores else 0.0
    overall_min_ssim = min(r['ssim_min'] for r in compared)
    overall_max_ssim = max(r['ssim_max'] for r in compared)

    pdfs_excellent = sum(1 for r in compared if r['ssim_mean'] >= 0.99)
    pdfs_good = sum(1 for r in compared if 0.95 <= r['ssim_mean'] < 0.99)
    pdfs_poor = sum(1 for r in compared if r['ssim_mean'] < 0.95)

    print(f"PDFs compared: {len(compared)}/{len(TEST_PDFS)}")
    print(f"Total pages compared: {total_pages}")
    print()
    print(f"Overall SSIM Statistics:")
    print(f"  Mean: {overall_mean_ssim:.4f}")
    print(f"  Min:  {overall_min_ssim:.4f}")
    print(f"  Max:  {overall_max_ssim:.4f}")
    print()
    print(f"Quality Distribution:")
    print(f"  Excellent (≥0.99): {pdfs_excellent} PDFs")
    print(f"  Good (0.95-0.99):  {pdfs_good} PDFs")
    print(f"  Poor (<0.95):      {pdfs_poor} PDFs")

    if pdfs_poor > 0:
        print()
        print("PDFs with low similarity:")
        for r in compared:
            if r['ssim_mean'] < 0.95:
                print(f"  {r['pdf']}: mean={r['ssim_mean']:.4f}, min={r['ssim_min']:.4f}")

    # Report low similarity pages
    total_low_sim_pages = sum(len(r.get('low_similarity_pages', [])) for r in compared)
    if total_low_sim_pages > 0:
        print()
        print(f"Total pages with low similarity (<0.95): {total_low_sim_pages}")
        print("Details:")
        for r in compared:
            if r.get('low_similarity_pages'):
                print(f"  {r['pdf']}:")
                for page_info in r['low_similarity_pages'][:5]:  # Show first 5
                    print(f"    Page {page_info['page']}: SSIM={page_info['ssim']:.4f}")
                if len(r['low_similarity_pages']) > 5:
                    print(f"    ... and {len(r['low_similarity_pages']) - 5} more")

if failed:
    print()
    print(f"Failed validations: {len(failed)}")
    for r in failed:
        print(f"  {r['pdf']}: {r['status']}")

print()
print(f"Detailed results saved to: {results_file}")
print(f"End time: {datetime.now().isoformat()}")
