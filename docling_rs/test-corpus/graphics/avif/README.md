# AVIF Test Files

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
