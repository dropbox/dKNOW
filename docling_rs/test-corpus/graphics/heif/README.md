# HEIF/HEIC Test Files

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
