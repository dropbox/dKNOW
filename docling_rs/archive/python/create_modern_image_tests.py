#!/usr/bin/env python3
"""
Create test files for HEIF/HEIC and AVIF formats.
Since these formats require special libraries, we'll create simple test images
using PIL and document where to get real test files.
"""

from PIL import Image, ImageDraw, ImageFont
import os

def create_test_image_png(text, filename, size=(800, 600)):
    """Create a simple PNG test image with text"""
    img = Image.new('RGB', size, color='white')
    draw = ImageDraw.Draw(img)

    # Try to use a system font, fallback to default
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 60)
    except:
        font = ImageFont.load_default()

    # Draw text
    draw.text((50, 250), text, fill='black', font=font)

    # Save as PNG
    img.save(filename)
    print(f"Created: {filename}")

def create_heif_placeholders():
    """Create placeholder PNG files for HEIF conversion"""
    heif_dir = "test-corpus/graphics/heif"
    os.makedirs(heif_dir, exist_ok=True)

    test_files = [
        ("simple_text.png", "Simple HEIF Test"),
        ("photo_sample.png", "Photo Quality Test"),
        ("high_compression.png", "High Compression"),
        ("transparency_test.png", "Transparency Test"),
        ("large_image.png", "Large Image Test"),
    ]

    for filename, text in test_files:
        create_test_image_png(text, os.path.join(heif_dir, filename))

    # Create README
    readme_content = """# HEIF/HEIC Test Files

These PNG placeholders should be converted to HEIF/HEIC format.

## Conversion Instructions

### Option 1: Using sips (macOS)
```bash
for f in *.png; do
    sips -s format heic "$f" --out "${f%.png}.heic"
done
```

### Option 2: Using ImageMagick with HEIF support
```bash
for f in *.png; do
    magick convert "$f" "${f%.png}.heic"
done
```

### Option 3: Download sample HEIF files
- https://github.com/strukturag/libheif/tree/master/examples
- Apple iOS photos (iPhone 7 and later)

## Test File Descriptions

1. **simple_text.png/heic** - Simple text for OCR testing
2. **photo_sample.png/heic** - Photo-quality image
3. **high_compression.png/heic** - Tests high compression ratios
4. **transparency_test.png/heic** - Tests alpha channel (if supported)
5. **large_image.png/heic** - Large file size test

## Expected Behavior

docling should:
- Detect HEIF/HEIC format from file extension (.heif, .heic)
- Load image using libheif-rs
- Process through OCR pipeline for text extraction
- Handle errors gracefully for corrupted files
"""

    with open(os.path.join(heif_dir, "README.md"), "w") as f:
        f.write(readme_content)
    print(f"Created: {heif_dir}/README.md")

def create_avif_placeholders():
    """Create placeholder PNG files for AVIF conversion"""
    avif_dir = "test-corpus/graphics/avif"
    os.makedirs(avif_dir, exist_ok=True)

    test_files = [
        ("simple_text.png", "Simple AVIF Test"),
        ("photo_sample.png", "AV1 Compression"),
        ("hdr_sample.png", "HDR Image Test"),
        ("animation_frame.png", "Animation Test"),
        ("web_optimized.png", "Web Optimized"),
    ]

    for filename, text in test_files:
        create_test_image_png(text, os.path.join(avif_dir, filename))

    # Create README
    readme_content = """# AVIF Test Files

These PNG placeholders should be converted to AVIF format.

## Conversion Instructions

### Option 1: Using cavif (Rust tool)
```bash
cargo install cavif
for f in *.png; do
    cavif --quality 80 "$f" -o "${f%.png}.avif"
done
```

### Option 2: Using avifenc (from libavif)
```bash
brew install libavif
for f in *.png; do
    avifenc "$f" "${f%.png}.avif"
done
```

### Option 3: Using ImageMagick with AVIF support
```bash
for f in *.png; do
    magick convert "$f" "${f%.png}.avif"
done
```

### Option 4: Download sample AVIF files
- https://github.com/AOMediaCodec/av1-avif/tree/master/testFiles
- Netflix AVIF samples
- AV1 codec sample files

## Test File Descriptions

1. **simple_text.png/avif** - Simple text for OCR testing
2. **photo_sample.png/avif** - Photo with AV1 compression
3. **hdr_sample.png/avif** - HDR image (if supported)
4. **animation_frame.png/avif** - Single frame from animation
5. **web_optimized.png/avif** - Web-optimized compression

## Expected Behavior

docling should:
- Detect AVIF format from file extension (.avif)
- Load image using image crate with AVIF feature
- Process through OCR pipeline for text extraction
- Handle errors gracefully for corrupted files

## AVIF Advantages

- Better compression than JPEG (up to 50% smaller)
- Supports HDR and wide color gamut
- Royalty-free (based on AV1 codec)
- Increasingly supported by browsers
"""

    with open(os.path.join(avif_dir, "README.md"), "w") as f:
        f.write(readme_content)
    print(f"Created: {avif_dir}/README.md")

if __name__ == "__main__":
    print("Creating HEIF test placeholders...")
    create_heif_placeholders()

    print("\nCreating AVIF test placeholders...")
    create_avif_placeholders()

    print("\nDone! PNG placeholders created.")
    print("To convert to actual HEIF/AVIF files, follow instructions in README.md files.")
