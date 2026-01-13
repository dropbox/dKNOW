#!/usr/bin/env python3
"""
Create synthetic scanned PDFs for testing smart mode.
Uses pypdf to create PDFs with embedded JPEG images.
"""

import sys
from pathlib import Path
from PIL import Image
import io

try:
    from pypdf import PdfWriter, PdfReader
    from pypdf.generic import NameObject, IndirectObject
except ImportError:
    print("Error: pypdf not installed. Run: pip install pypdf")
    sys.exit(1)

def create_test_image(width: int, height: int, text: str, output_path: Path):
    """Create a test JPEG image with text."""
    img = Image.new('RGB', (width, height), color='white')

    # Draw simple pattern to make it recognizable
    from PIL import ImageDraw, ImageFont
    draw = ImageDraw.Draw(img)

    # Draw colored rectangles
    draw.rectangle([10, 10, width-10, height-10], outline='blue', width=5)
    draw.rectangle([50, 50, width-50, height-50], fill='lightblue')

    # Add text
    try:
        # Try to use a larger font
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 48)
    except:
        font = ImageFont.load_default()

    text_bbox = draw.textbbox((0, 0), text, font=font)
    text_width = text_bbox[2] - text_bbox[0]
    text_height = text_bbox[3] - text_bbox[1]
    text_x = (width - text_width) // 2
    text_y = (height - text_height) // 2

    draw.text((text_x, text_y), text, fill='darkblue', font=font)

    # Save as JPEG
    img.save(output_path, 'JPEG', quality=85)
    return output_path

def create_scanned_pdf_simple(image_path: Path, output_pdf: Path, page_count: int = 1):
    """
    Create a PDF with embedded JPEG image(s) covering full page.
    This simulates a scanned document.
    """
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter
    from reportlab.lib.utils import ImageReader

    # Get image dimensions
    img = Image.open(image_path)
    img_width, img_height = img.size

    # Create PDF
    c = canvas.Canvas(str(output_pdf), pagesize=letter)
    page_width, page_height = letter

    for i in range(page_count):
        # Draw image to fill entire page
        c.drawImage(str(image_path), 0, 0, width=page_width, height=page_height,
                   preserveAspectRatio=False)
        c.showPage()

    c.save()
    return output_pdf

def create_mixed_pdf(image_path: Path, output_pdf: Path):
    """
    Create a PDF with mixed content: some scanned pages, some text pages.
    """
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter

    c = canvas.Canvas(str(output_pdf), pagesize=letter)
    page_width, page_height = letter

    # Page 1: Scanned (full-page image)
    c.drawImage(str(image_path), 0, 0, width=page_width, height=page_height,
               preserveAspectRatio=False)
    c.showPage()

    # Page 2: Text page
    c.setFont("Helvetica", 12)
    c.drawString(100, 750, "This is a text page with regular content.")
    c.drawString(100, 730, "It should NOT be detected as scanned.")
    c.showPage()

    # Page 3: Scanned (full-page image)
    c.drawImage(str(image_path), 0, 0, width=page_width, height=page_height,
               preserveAspectRatio=False)
    c.showPage()

    c.save()
    return output_pdf

def main():
    # Create output directory
    output_dir = Path("integration_tests/pdfs/scanned_test")
    output_dir.mkdir(parents=True, exist_ok=True)

    print("Creating synthetic scanned PDF test corpus...\n")

    # Create test images
    temp_dir = Path("/tmp/pdfium_test_images")
    temp_dir.mkdir(exist_ok=True)

    images = []
    for i, (width, height) in enumerate([(2480, 3508),  # A4 at 300 DPI
                                          (1654, 2339),  # A4 at 200 DPI
                                          (3508, 2480)], # A4 landscape at 300 DPI
                                         start=1):
        img_path = temp_dir / f"test_image_{i}.jpg"
        create_test_image(width, height, f"Test Image {i}\nScanned Page", img_path)
        images.append(img_path)
        print(f"Created test image: {img_path} ({width}x{height})")

    # Check if reportlab is available
    try:
        import reportlab
    except ImportError:
        print("\nError: reportlab not installed. Run: pip install reportlab")
        print("Cannot create test PDFs without reportlab.")
        sys.exit(1)

    # Create test PDFs
    test_pdfs = []

    # 1. Single-page scanned PDF
    pdf1 = output_dir / "scanned_single_page.pdf"
    create_scanned_pdf_simple(images[0], pdf1, page_count=1)
    test_pdfs.append(pdf1)
    print(f"✓ Created: {pdf1.name} (1 page, scanned)")

    # 2. Multi-page scanned PDF (5 pages)
    pdf2 = output_dir / "scanned_multi_page.pdf"
    create_scanned_pdf_simple(images[0], pdf2, page_count=5)
    test_pdfs.append(pdf2)
    print(f"✓ Created: {pdf2.name} (5 pages, all scanned)")

    # 3. High-resolution scanned PDF
    pdf3 = output_dir / "scanned_high_res.pdf"
    create_scanned_pdf_simple(images[0], pdf3, page_count=1)
    test_pdfs.append(pdf3)
    print(f"✓ Created: {pdf3.name} (1 page, 300 DPI)")

    # 4. Lower-resolution scanned PDF
    pdf4 = output_dir / "scanned_low_res.pdf"
    create_scanned_pdf_simple(images[1], pdf4, page_count=1)
    test_pdfs.append(pdf4)
    print(f"✓ Created: {pdf4.name} (1 page, 200 DPI)")

    # 5. Landscape scanned PDF
    pdf5 = output_dir / "scanned_landscape.pdf"
    create_scanned_pdf_simple(images[2], pdf5, page_count=1)
    test_pdfs.append(pdf5)
    print(f"✓ Created: {pdf5.name} (1 page, landscape)")

    # 6. Mixed content PDF (some scanned, some text)
    pdf6 = output_dir / "mixed_scanned_text.pdf"
    create_mixed_pdf(images[0], pdf6)
    test_pdfs.append(pdf6)
    print(f"✓ Created: {pdf6.name} (3 pages, mixed)")

    print(f"\n✅ Created {len(test_pdfs)} test PDFs in {output_dir}")
    print("\nTest corpus:")
    for pdf in test_pdfs:
        size_kb = pdf.stat().st_size / 1024
        print(f"  - {pdf.name} ({size_kb:.1f} KB)")

    # Verify with our detection script
    print("\n" + "="*60)
    print("Verifying detection on created PDFs...")
    print("="*60 + "\n")

    return test_pdfs

if __name__ == "__main__":
    test_pdfs = main()
