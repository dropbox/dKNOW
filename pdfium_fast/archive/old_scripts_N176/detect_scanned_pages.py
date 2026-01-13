#!/usr/bin/env python3
"""
Detect scanned pages in PDF corpus using pdfium_cli smart mode detection logic.
Uses ctypes to call PDFium library directly for structure analysis.
"""

import sys
import ctypes
from pathlib import Path
from typing import List, Tuple

# Load PDFium library
lib_path = Path("out/Optimized-Shared/libpdfium.dylib")
if not lib_path.exists():
    print(f"Error: PDFium library not found at {lib_path}")
    sys.exit(1)

pdfium = ctypes.CDLL(str(lib_path))

# Define PDFium types
FPDF_DOCUMENT = ctypes.c_void_p
FPDF_PAGE = ctypes.c_void_p
FPDF_PAGEOBJECT = ctypes.c_void_p

# PDFium functions
pdfium.FPDF_InitLibrary.argtypes = []
pdfium.FPDF_InitLibrary.restype = None

pdfium.FPDF_LoadDocument.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
pdfium.FPDF_LoadDocument.restype = FPDF_DOCUMENT

pdfium.FPDF_GetPageCount.argtypes = [FPDF_DOCUMENT]
pdfium.FPDF_GetPageCount.restype = ctypes.c_int

pdfium.FPDF_LoadPage.argtypes = [FPDF_DOCUMENT, ctypes.c_int]
pdfium.FPDF_LoadPage.restype = FPDF_PAGE

pdfium.FPDFPage_CountObjects.argtypes = [FPDF_PAGE]
pdfium.FPDFPage_CountObjects.restype = ctypes.c_int

pdfium.FPDFPage_GetObject.argtypes = [FPDF_PAGE, ctypes.c_int]
pdfium.FPDFPage_GetObject.restype = FPDF_PAGEOBJECT

pdfium.FPDFPageObj_GetType.argtypes = [FPDF_PAGEOBJECT]
pdfium.FPDFPageObj_GetType.restype = ctypes.c_int

pdfium.FPDF_GetPageWidth.argtypes = [FPDF_PAGE]
pdfium.FPDF_GetPageWidth.restype = ctypes.c_double

pdfium.FPDF_GetPageHeight.argtypes = [FPDF_PAGE]
pdfium.FPDF_GetPageHeight.restype = ctypes.c_double

pdfium.FPDFPageObj_GetBounds.argtypes = [FPDF_PAGEOBJECT, ctypes.POINTER(ctypes.c_float),
                                          ctypes.POINTER(ctypes.c_float),
                                          ctypes.POINTER(ctypes.c_float),
                                          ctypes.POINTER(ctypes.c_float)]
pdfium.FPDFPageObj_GetBounds.restype = ctypes.c_int

pdfium.FPDF_ClosePage.argtypes = [FPDF_PAGE]
pdfium.FPDF_ClosePage.restype = None

pdfium.FPDF_CloseDocument.argtypes = [FPDF_DOCUMENT]
pdfium.FPDF_CloseDocument.restype = None

# Constants (from public/fpdf_edit.h)
FPDF_PAGEOBJ_UNKNOWN = 0
FPDF_PAGEOBJ_TEXT = 1
FPDF_PAGEOBJ_PATH = 2
FPDF_PAGEOBJ_IMAGE = 3
FPDF_PAGEOBJ_SHADING = 4
FPDF_PAGEOBJ_FORM = 5

def is_scanned_page(page: FPDF_PAGE, debug: bool = False) -> Tuple[bool, str]:
    """
    Check if page is a scanned page (single image covering ≥95% of page).
    Returns (is_scanned, reason).
    """
    # Count objects
    obj_count = pdfium.FPDFPage_CountObjects(page)
    if debug:
        print(f"    DEBUG: obj_count={obj_count}")
    if obj_count != 1:
        return False, f"obj_count={obj_count}"

    # Get object type
    obj = pdfium.FPDFPage_GetObject(page, 0)
    if not obj:
        if debug:
            print(f"    DEBUG: obj is NULL")
        return False, "obj_null"

    obj_type = pdfium.FPDFPageObj_GetType(obj)
    if debug:
        print(f"    DEBUG: obj_type={obj_type} (IMAGE=3)")
    if obj_type != FPDF_PAGEOBJ_IMAGE:
        return False, f"obj_type={obj_type}"

    # Get page dimensions
    page_width = pdfium.FPDF_GetPageWidth(page)
    page_height = pdfium.FPDF_GetPageHeight(page)
    page_area = page_width * page_height
    if debug:
        print(f"    DEBUG: page={page_width:.1f}x{page_height:.1f}, area={page_area:.1f}")

    # Get object bounds
    left = ctypes.c_float()
    bottom = ctypes.c_float()
    right = ctypes.c_float()
    top = ctypes.c_float()

    result = pdfium.FPDFPageObj_GetBounds(obj, ctypes.byref(left), ctypes.byref(bottom),
                                          ctypes.byref(right), ctypes.byref(top))
    if not result:
        if debug:
            print(f"    DEBUG: GetBounds failed")
        return False, "bounds_error"

    obj_width = right.value - left.value
    obj_height = top.value - bottom.value
    obj_area = obj_width * obj_height
    coverage = obj_area / page_area if page_area > 0 else 0

    if debug:
        print(f"    DEBUG: obj={obj_width:.1f}x{obj_height:.1f}, area={obj_area:.1f}")
        print(f"    DEBUG: coverage={coverage:.2%}")

    # Check coverage
    if coverage >= 0.95:
        return True, f"coverage={coverage:.2%}"
    else:
        return False, f"coverage={coverage:.2%}"

def analyze_pdf(pdf_path: Path, debug: bool = False) -> dict:
    """Analyze a PDF and return scanned page statistics."""
    # Load document
    doc = pdfium.FPDF_LoadDocument(str(pdf_path).encode('utf-8'), None)
    if not doc:
        return {"error": "Failed to load PDF"}

    try:
        page_count = pdfium.FPDF_GetPageCount(doc)
        scanned_pages = []

        # Check first 10 pages (faster for large PDFs)
        check_count = min(page_count, 10)
        for i in range(check_count):
            page = pdfium.FPDF_LoadPage(doc, i)
            if page:
                is_scanned, reason = is_scanned_page(page, debug=debug)
                if is_scanned:
                    scanned_pages.append(i)
                elif debug:
                    print(f"  Page {i}: NOT scanned ({reason})")
                pdfium.FPDF_ClosePage(page)

        return {
            "page_count": page_count,
            "checked_pages": check_count,
            "scanned_pages": scanned_pages,
            "scanned_ratio": len(scanned_pages) / check_count if check_count > 0 else 0
        }
    finally:
        pdfium.FPDF_CloseDocument(doc)

def main():
    # Initialize PDFium
    pdfium.FPDF_InitLibrary()

    # Check which directory to scan
    if len(sys.argv) > 1:
        pdf_dir = Path(sys.argv[1])
    else:
        # Default to scanned_test if it exists, otherwise benchmark
        scanned_dir = Path("integration_tests/pdfs/scanned_test")
        if scanned_dir.exists():
            pdf_dir = scanned_dir
        else:
            pdf_dir = Path("integration_tests/pdfs/benchmark")

    pdfs = sorted(pdf_dir.glob("*.pdf"))

    # Enable debug mode for scanned_test directory
    debug_mode = (pdf_dir.name == "scanned_test")

    print(f"Scanning {len(pdfs)} PDFs in {pdf_dir} for scanned pages...")
    if debug_mode:
        print("(Debug mode enabled)\n")
    else:
        print()

    candidates = []
    for i, pdf_path in enumerate(pdfs):
        if len(pdfs) > 20 and i % 20 == 0:
            print(f"Progress: {i}/{len(pdfs)}...", file=sys.stderr)

        if debug_mode:
            print(f"\nAnalyzing: {pdf_path.name}")

        result = analyze_pdf(pdf_path, debug=debug_mode)
        if "error" in result:
            print(f"  {pdf_path.name}: ERROR", file=sys.stderr)
            continue

        # Report all results for scanned_test, only hits for others
        if pdf_dir.name == "scanned_test" or result["scanned_pages"]:
            candidates.append((pdf_path.name, result))

    print(f"\nFound {len(candidates)} PDFs with scanned pages:\n")

    for name, result in candidates:
        ratio = result["scanned_ratio"]
        scanned_count = len(result["scanned_pages"])
        checked = result["checked_pages"]
        total = result["page_count"]
        print(f"  {name}")
        print(f"    Total pages: {total}")
        print(f"    Scanned: {scanned_count}/{checked} checked ({ratio:.0%})")
        if result['scanned_pages']:
            print(f"    Scanned page indices: {result['scanned_pages']}")
        print()

if __name__ == "__main__":
    main()
