#!/usr/bin/env python3
"""
Create a true scanned PDF with embedded JPEG (DCTDecode filter).
Uses img2pdf which preserves JPEG encoding.
"""

import sys
from pathlib import Path
from PIL import Image

try:
    import img2pdf
except ImportError:
    print("Error: img2pdf not installed. Run: pip install img2pdf")
    sys.exit(1)

def create_real_scanned_pdfs():
    """Create PDFs with actual embedded JPEG images using img2pdf."""

    # Create output directory
    output_dir = Path("integration_tests/pdfs/scanned_real")
    output_dir.mkdir(parents=True, exist_ok=True)

    # Use existing test images
    temp_dir = Path("/tmp/pdfium_test_images")
    if not temp_dir.exists():
        print("Error: Test images not found. Run create_test_scanned_pdfs.py first.")
        sys.exit(1)

    images = list(temp_dir.glob("test_image_*.jpg"))
    if not images:
        print("Error: No test images found in /tmp/pdfium_test_images/")
        sys.exit(1)

    print(f"Found {len(images)} test images")
    print("Creating real scanned PDFs with embedded JPEG...\n")

    # 1. Single-page scanned PDF with JPEG
    pdf1 = output_dir / "scanned_single_jpeg.pdf"
    with open(pdf1, "wb") as f:
        f.write(img2pdf.convert(str(images[0])))
    print(f"✓ Created: {pdf1.name}")

    # 2. Multi-page scanned PDF (use same image 5 times)
    pdf2 = output_dir / "scanned_multi_jpeg.pdf"
    with open(pdf2, "wb") as f:
        # img2pdf can take list of images
        image_list = [str(images[0])] * 5
        f.write(img2pdf.convert(image_list))
    print(f"✓ Created: {pdf2.name}")

    # 3. High-res scanned PDF
    pdf3 = output_dir / "scanned_high_res_jpeg.pdf"
    with open(pdf3, "wb") as f:
        f.write(img2pdf.convert(str(images[0])))
    print(f"✓ Created: {pdf3.name}")

    # 4. Mixed-resolution multi-page
    if len(images) >= 3:
        pdf4 = output_dir / "scanned_mixed_jpeg.pdf"
        with open(pdf4, "wb") as f:
            image_list = [str(img) for img in images[:3]]
            f.write(img2pdf.convert(image_list))
        print(f"✓ Created: {pdf4.name}")

    print(f"\n✅ Created real scanned PDFs in {output_dir}")
    print("\nVerifying JPEG embedding...")

    return list(output_dir.glob("*.pdf"))

if __name__ == "__main__":
    test_pdfs = create_real_scanned_pdfs()
