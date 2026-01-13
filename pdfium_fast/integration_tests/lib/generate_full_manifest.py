#!/usr/bin/env python3
"""
Generate complete PDF manifest for all 452 PDFs with markers.

Run: python lib/generate_full_manifest.py

Output: master_test_suite/pdf_manifest.csv (452 rows)
"""

import csv
import hashlib
from pathlib import Path
from datetime import datetime

# Standard 60-PDF set (curated for comprehensive testing)
STANDARD_60_SET = {
    "0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf",
    "0106pages_IYAFRX3M262EGRMQJHT7MCODUP5ZGYRW.pdf",
    "0109pages_NRCGAGBLTXTXEKXNLQ4L4NFA6ZQPCWE5.pdf",
    "0124pages_HXTX4CFAYHW5LKUFDCEHIITIMEYCJ7MW.pdf",
    "0130pages_ZJJJ6P4UAGH7LKLACPT5P437FB5F3MYF.pdf",
    "0134pages_LYZMN7YTFC6ULBL6TP4WQQRK356GB36T.pdf",
    "0150pages_44LQBJ56XNS2C6VVWKOP2YUNKCODKTEA.pdf",
    "0159pages_5XPIPZJEUIFPFOXGLU26NFQSU4GSJMF3.pdf",
    "0169pages_CLEQSRWNHVWUAHLTNGPYHMYOWEC4JR6S.pdf",
    "0171pages_USTMTDJBOQ327J2XVWE76TPUOKIBJ4S5.pdf",
    "0172pages_DNJIAKOPLDZDQCXT2PDA6V6DGL2MQL2H.pdf",
    "0192pages_C46Z2Q4AMEZJVTRV2XO434YLMVHCB46O.pdf",
    "0192pages_ULFRJCOWYIDOOTY7KWUPC6NU7UTTCLSO.pdf",
    "0193pages_CADINJMTO23ZTKPBNZ6TUOQT4BFPZC2C.pdf",
    "0201pages_RYDFB4ZZNNBE6LLDSY4CFGWQQ7U3KSAA.pdf",
    "0214pages_BZK65P6W74FNMD4O3A24LH3F2XLFH7V2.pdf",
    "0215pages_UAU4SPHOQHADJRFKILFDYETBGB6HXYU2.pdf",
    "0255pages_OKRV2XLZ2UGGRCBKHS3I5DA5TNLNVHWC.pdf",
    "0277pages_YCOBKTA46F7XEOWT4LS3BCPGDOMQSBWF.pdf",
    "0291pages_RGLWPWLY4JE6RPNCCHQ3LQWDFLPBGHXQ.pdf",
    "0309pages_7LD3RVJDZGTXF53CDLCI67YPWZQ5POOA.pdf",
    "0313pages_7DLRSEXVIADBZZUN4IJIACW5T23YBH7H.pdf",
    "0496pages_E3474JUEVRWQ3P2J2I3XBFKMMVZLLKWZ.pdf",
    "0569pages_QXQ2QSHOPBTSXLDGKKM4TYMR4R7QODHB.pdf",
    "0821pages_LUNFJFH4KWZ3ZFNRO43WSMZPLM4OLB7C.pdf",
    "1931pages_7ZNNFJGHOEFFP6I4OARCZGH3GPPDNDXC.pdf",
    "arxiv_001.pdf", "arxiv_005.pdf", "arxiv_009.pdf", "arxiv_014.pdf",
    "arxiv_018.pdf", "arxiv_022.pdf", "arxiv_026.pdf", "arxiv_030.pdf",
    "arxiv_034.pdf", "arxiv_038.pdf",
    "cc_001_931p.pdf", "cc_003_162p.pdf", "cc_005_172p.pdf", "cc_007_101p.pdf",
    "cc_009_188p.pdf", "cc_011_222p.pdf", "cc_013_122p.pdf", "cc_015_101p.pdf",
    "cc_017_145p.pdf", "cc_019_198p.pdf",
    "edinet_2025-06-23_1701_E04341_Konoike Transport CoLtd.pdf",
    "edinet_2025-06-24_1318_E01920_Makita Corporation.pdf",
    "edinet_2025-06-25_1507_E01402_SUNCALL CORPORATION.pdf",
    "edinet_2025-06-26_0909_E00982_LTT Bio-Pharma Co Ltd.pdf",
    "edinet_2025-06-26_1500_E01974_NKK SWITCHESCOLTD.pdf",
    "edinet_2025-06-26_1713_E00771_OSAKA SODA COLTD.pdf",
    "edinet_2025-06-27_1330_E02927_TAKACHIHO KOHEKI COLTD.pdf",
    "edinet_2025-07-11_1533_E04884_KYOWA ENGINEERING CONSULTANTS.pdf",
    "edinet_2025-08-08_1325_E35294_TECHNOFLEX CORPORATION.pdf",
    "edinet_2025-08-08_1603_E03724_J Trust CoLtd.pdf",
    "web_001.pdf", "web_012.pdf", "web_023.pdf", "web_034.pdf"
}

# Smoke fast set (< 1 minute, 10 PDFs)
SMOKE_FAST_SET = {
    "arxiv_001.pdf",      # 10 pages
    "arxiv_005.pdf",      # 12 pages
    "cc_007_101p.pdf",    # 101 pages
    "cc_015_101p.pdf",    # 101 pages
    "web_001.pdf",        # ~20 pages
    "0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf",  # 100 pages
}


def compute_md5(filepath):
    """Compute MD5 hash of file."""
    md5 = hashlib.md5()
    with open(filepath, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b""):
            md5.update(chunk)
    return md5.hexdigest()


def get_page_count(pdf_path):
    """Get page count from filename or return -1."""
    # Try to extract from filename
    stem = pdf_path.stem
    if stem.startswith('0') and 'pages_' in stem:
        try:
            pages_str = stem.split('pages_')[0].lstrip('0')
            return int(pages_str) if pages_str else 0
        except ValueError:
            pass

    # Check for _NNNp suffix (e.g., cc_001_931p.pdf)
    if '_' in stem:
        parts = stem.split('_')
        for part in parts:
            if part.endswith('p'):
                try:
                    return int(part[:-1])
                except ValueError:
                    pass

    return -1


def get_pdf_subcategory(pdf_name, parent_dir):
    """Determine subcategory from filename."""
    if pdf_name.startswith('arxiv_'):
        return 'arxiv'
    elif pdf_name.startswith('cc_'):
        return 'cc'
    elif pdf_name.startswith('edinet_'):
        return 'edinet'
    elif pdf_name.startswith('web_'):
        return 'web'
    elif pdf_name.startswith('japanese_'):
        return 'japanese'
    elif 'pages_' in pdf_name:
        return 'pages'
    else:
        return parent_dir  # 'benchmark' or 'edge_cases'


def get_size_class(page_count):
    """Determine size class."""
    if page_count < 0:
        return 'unknown'
    elif page_count < 100:
        return 'small'
    elif page_count < 200:
        return 'medium'
    else:
        return 'large'


def assign_markers(pdf_name, subcategory, size_class):
    """Assign pytest markers to PDF."""
    markers = ['full']  # All PDFs are in full suite

    # Add standard_60_set if in curated list
    if pdf_name in STANDARD_60_SET:
        markers.append('standard_60_set')

    # Add smoke_fast if in fast smoke set
    if pdf_name in SMOKE_FAST_SET:
        markers.append('smoke_fast')

    # Add batch_bulk (all PDFs)
    markers.append('batch_bulk')

    # Add subcategory
    markers.append(subcategory)

    # Add size class
    if size_class != 'unknown':
        markers.append(f'{size_class}_pdf')

    return ','.join(markers)


def main():
    integration_tests = Path(__file__).parent.parent
    pdfs_dir = integration_tests / 'pdfs'

    # Discover all PDFs
    all_pdfs = []

    for subdir in ['benchmark', 'edge_cases']:
        pdf_subdir = pdfs_dir / subdir
        if not pdf_subdir.exists():
            continue

        for pdf_file in sorted(pdf_subdir.glob('*.pdf')):
            # Get metadata
            md5 = compute_md5(pdf_file)
            size_bytes = pdf_file.stat().st_size
            page_count = get_page_count(pdf_file)
            subcategory = get_pdf_subcategory(pdf_file.name, subdir)
            size_class = get_size_class(page_count)
            markers = assign_markers(pdf_file.name, subcategory, size_class)

            # Check if baselines exist
            text_baseline_path = integration_tests / 'baselines' / 'upstream' / 'text' / f'{pdf_file.stem}.txt'
            jsonl_baseline_path = integration_tests / 'baselines' / 'upstream' / 'jsonl' / f'{pdf_file.stem}.jsonl'
            image_baseline_json_path = integration_tests / 'baselines' / 'upstream' / 'images' / f'{pdf_file.stem}.json'

            row = {
                'pdf_name': pdf_file.name,
                'pdf_path': str(pdf_file.relative_to(integration_tests)),
                'pdf_md5': md5,
                'pdf_bytes': size_bytes,
                'pdf_pages': page_count if page_count > 0 else 'unknown',
                'pdf_category': subdir,
                'pdf_subcategory': subcategory,
                'pdf_size_class': size_class,
                'markers': markers,
                'expected_outputs_dir': f'master_test_suite/expected_outputs/{subcategory}/{pdf_file.stem}',
                'manifest_json_path': f'master_test_suite/expected_outputs/{subcategory}/{pdf_file.stem}/manifest.json',
                'text_baseline_path': f'baselines/upstream/text/{pdf_file.stem}.txt',
                'text_baseline_exists': str(text_baseline_path.exists()),
                'jsonl_baseline_path': f'baselines/upstream/jsonl/{pdf_file.stem}.jsonl',
                'jsonl_baseline_exists': str(jsonl_baseline_path.exists()),
                'image_baseline_json_path': f'baselines/upstream/images/{pdf_file.stem}.json',
                'image_baseline_json_exists': str(image_baseline_json_path.exists())
            }

            all_pdfs.append(row)

    # Write manifest
    manifest_path = integration_tests / 'master_test_suite' / 'pdf_manifest.csv'

    with open(manifest_path, 'w', newline='') as f:
        fieldnames = list(all_pdfs[0].keys())
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(all_pdfs)

    print(f"✓ Generated manifest: {manifest_path}")
    print(f"  Total PDFs: {len(all_pdfs)}")
    print(f"  Benchmark: {len([p for p in all_pdfs if p['pdf_category'] == 'benchmark'])}")
    print(f"  Edge cases: {len([p for p in all_pdfs if p['pdf_category'] == 'edge_cases'])}")
    print(f"  standard_60_set: {len([p for p in all_pdfs if 'standard_60_set' in p['markers']])}")
    print(f"  smoke_fast: {len([p for p in all_pdfs if 'smoke_fast' in p['markers']])}")


if __name__ == '__main__':
    main()
