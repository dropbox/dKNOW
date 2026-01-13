#!/usr/bin/env python3
"""
Debug script to inspect PDF structure and understand why scanned page detection might fail.
"""

import sys
import ctypes
from pathlib import Path

# Load PDFium library
lib_path = Path("out/Optimized-Shared/libpdfium.dylib")
pdfium = ctypes.CDLL(str(lib_path))

# Define types and functions
FPDF_DOCUMENT = ctypes.c_void_p
FPDF_PAGE = ctypes.c_void_p
FPDF_PAGEOBJECT = ctypes.c_void_p

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

# Constants
FPDF_PAGEOBJ_TEXT = 1
FPDF_PAGEOBJ_PATH = 2
FPDF_PAGEOBJ_IMAGE = 3
FPDF_PAGEOBJ_SHADING = 4
FPDF_PAGEOBJ_FORM = 5

OBJ_TYPE_NAMES = {
    1: "TEXT",
    2: "PATH",
    3: "IMAGE",
    4: "SHADING",
    5: "FORM"
}

def debug_page(page: FPDF_PAGE, page_num: int):
    """Debug a single page."""
    print(f"\n{'='*60}")
    print(f"Page {page_num}")
    print(f"{'='*60}")

    # Get page dimensions
    page_width = pdfium.FPDF_GetPageWidth(page)
    page_height = pdfium.FPDF_GetPageHeight(page)
    page_area = page_width * page_height

    print(f"Page size: {page_width:.2f} x {page_height:.2f}")
    print(f"Page area: {page_area:.2f}")

    # Count objects
    obj_count = pdfium.FPDFPage_CountObjects(page)
    print(f"Object count: {obj_count}")

    # Inspect each object
    for i in range(obj_count):
        obj = pdfium.FPDFPage_GetObject(page, i)
        obj_type = pdfium.FPDFPageObj_GetType(obj)
        type_name = OBJ_TYPE_NAMES.get(obj_type, f"UNKNOWN({obj_type})")

        print(f"\n  Object {i}: {type_name}")

        # Get bounds
        left = ctypes.c_float()
        bottom = ctypes.c_float()
        right = ctypes.c_float()
        top = ctypes.c_float()

        result = pdfium.FPDFPageObj_GetBounds(obj, ctypes.byref(left), ctypes.byref(bottom),
                                              ctypes.byref(right), ctypes.byref(top))

        if result:
            obj_width = right.value - left.value
            obj_height = top.value - bottom.value
            obj_area = obj_width * obj_height
            coverage = (obj_area / page_area * 100) if page_area > 0 else 0

            print(f"    Position: ({left.value:.2f}, {bottom.value:.2f}) to ({right.value:.2f}, {top.value:.2f})")
            print(f"    Size: {obj_width:.2f} x {obj_height:.2f}")
            print(f"    Area: {obj_area:.2f}")
            print(f"    Coverage: {coverage:.1f}%")
        else:
            print(f"    ERROR: Could not get bounds")

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 debug_pdf_structure.py <pdf_file>")
        sys.exit(1)

    pdf_path = Path(sys.argv[1])
    if not pdf_path.exists():
        print(f"Error: {pdf_path} not found")
        sys.exit(1)

    # Initialize PDFium
    pdfium.FPDF_InitLibrary()

    # Load document
    doc = pdfium.FPDF_LoadDocument(str(pdf_path).encode('utf-8'), None)
    if not doc:
        print(f"Error: Failed to load {pdf_path}")
        sys.exit(1)

    try:
        page_count = pdfium.FPDF_GetPageCount(doc)
        print(f"\nPDF: {pdf_path.name}")
        print(f"Total pages: {page_count}")

        # Debug first 3 pages
        for i in range(min(page_count, 3)):
            page = pdfium.FPDF_LoadPage(doc, i)
            if page:
                debug_page(page, i)
                pdfium.FPDF_ClosePage(page)

    finally:
        pdfium.FPDF_CloseDocument(doc)

if __name__ == "__main__":
    main()
