# CLIP-Based Logo Detection Setup

This directory contains the CLIP-based logo detection database using similarity search instead of YOLOv8 training.

## Approach

Instead of training a custom YOLOv8 model on logo datasets (20-40 commits estimated), we use CLIP embeddings for zero-shot logo detection via similarity search (5-8 commits).

### Advantages
- **No training required**: Uses pre-trained CLIP model (already available: 578MB)
- **Easily extensible**: Add new logos by just adding images
- **Fast implementation**: Reuses existing embeddings infrastructure
- **Good accuracy**: CLIP is trained on 400M image-text pairs

### How It Works
1. **Logo Database**: Extract CLIP embeddings for 50-200 brand logos
2. **Detection**: Extract CLIP embeddings from input image regions
3. **Similarity Search**: Compare region embeddings to logo database (cosine similarity)
4. **Threshold**: Return matches with similarity > 0.75

## Directory Structure

```
models/logo-detection/clip_database/
├── SETUP.md                    # This file
├── logos/                      # Logo images organized by category
│   ├── tech/                   # Technology brands
│   │   ├── apple.png
│   │   ├── google.png
│   │   ├── microsoft.png
│   │   └── ...
│   ├── sportswear/             # Sports & apparel brands
│   │   ├── nike.png
│   │   ├── adidas.png
│   │   └── ...
│   ├── food/                   # Food & beverage brands
│   │   ├── coca_cola.png
│   │   ├── mcdonalds.png
│   │   └── ...
│   └── automotive/             # Automotive brands
│       ├── tesla.png
│       ├── bmw.png
│       └── ...
└── logo_database.json          # Generated: Logo metadata + CLIP embeddings
```

## Setup Instructions

### 1. Download Logo Images

Download logo images (PNG format preferred, clean logos on transparent or white background):

**Option A: Wikimedia Commons** (free, licensed)
- https://commons.wikimedia.org/wiki/Category:Company_logos
- Download SVG/PNG versions
- Convert SVG to PNG: `convert logo.svg -resize 512x512 logo.png`

**Option B: Company Websites** (fair use for detection)
- Visit company press kits/media pages
- Download official logo files
- Ensure proper licensing for your use case

**Recommended Logos** (50-100 logos):

**Tech (15 logos)**:
- Apple, Google, Microsoft, Amazon, Meta, Netflix, Tesla, Nvidia, Intel, AMD, Samsung, Sony, IBM, Oracle, Adobe

**Sportswear (10 logos)**:
- Nike, Adidas, Puma, Under Armour, Reebok, New Balance, Converse, Vans, Asics, Fila

**Food & Beverage (15 logos)**:
- Coca-Cola, Pepsi, McDonald's, Starbucks, Burger King, KFC, Subway, Dunkin', Red Bull, Monster, Nestle, Unilever, P&G, Heinz, Kraft

**Automotive (10 logos)**:
- Tesla, BMW, Mercedes, Audi, Toyota, Honda, Ford, Chevrolet, Volkswagen, Hyundai

**Retail (10 logos)**:
- Walmart, Target, Amazon, Costco, IKEA, Home Depot, Best Buy, Walgreens, CVS, Kroger

**Fashion (10 logos)**:
- Gucci, Louis Vuitton, Chanel, Prada, Zara, H&M, Gap, Uniqlo, Levi's, North Face

**Total: 70 logos** (good starting point)

### 2. Organize Logo Files

Create category directories and place logos:

```bash
cd models/logo-detection/clip_database/logos

# Create category directories
mkdir -p tech sportswear food automotive retail fashion

# Place logo images in respective directories
# File naming: lowercase brand name, underscores for spaces
# Example: apple.png, coca_cola.png, louis_vuitton.png
```

**Image Requirements**:
- Format: PNG, JPG, or WebP
- Resolution: 512x512 recommended (CLIP resizes to 224x224)
- Background: Transparent or solid color
- Content: Clean logo without text (or with minimal text)

### 3. Build Logo Database

Run the build tool to extract CLIP embeddings:

```bash
cargo run --release -p build_logo_database -- \\
    models/logo-detection/clip_database/logos \\
    models/embeddings/clip_vit_b32.onnx \\
    models/logo-detection/clip_database/logo_database.json
```

**Output**: `logo_database.json` (JSON file with logo metadata and CLIP embeddings)

Example structure:
```json
{
  "model": "clip-vit-b32",
  "embedding_dim": 512,
  "logos": [
    {
      "id": "tech_apple",
      "brand": "apple",
      "category": "tech",
      "image_path": "tech/apple.png",
      "embedding": [0.123, -0.456, ..., 0.789]  // 512-dim vector
    },
    ...
  ]
}
```

**Expected output**: 70 logos × 512 floats × 4 bytes = ~140 KB (plus metadata)

### 4. Test Logo Detection

Once the database is built, test logo detection:

```bash
# Test with an image containing known logos
cargo run --release --bin video-extract -- \\
    --file test_images/brands.jpg \\
    --operation logo-detection \\
    --confidence 0.75
```

## Performance Expectations

- **Database size**: ~140 KB (70 logos) to ~400 KB (200 logos)
- **Embedding extraction**: ~100ms per image region (224x224 CLIP inference)
- **Similarity search**: ~1ms per region (70 logos × 512-dim cosine similarity)
- **Total per image**: ~100-200ms (depends on number of regions to check)

## Accuracy Expectations

- **Well-known logos**: 85-95% accuracy (Apple, Nike, Coca-Cola, etc.)
- **Similar logos**: 70-85% accuracy (may confuse similar brands)
- **Partial logos**: 60-80% accuracy (depends on visibility)
- **Distorted logos**: 50-70% accuracy (rotation, occlusion, low resolution)

## Limitations

1. **Region extraction**: Requires pre-processing to extract candidate regions (sliding window or object detection)
2. **Computational cost**: CLIP inference for each region (~100ms per region)
3. **Scale variance**: CLIP may struggle with very small or very large logos
4. **Color variance**: White logos on white backgrounds may not be detected
5. **Perspective distortion**: Logos on curved surfaces may reduce accuracy

## Future Improvements

1. **Region proposals**: Use lightweight object detector (YOLOv8n) to propose candidate regions
2. **Multi-scale detection**: Run detection at multiple image scales
3. **Logo variants**: Add multiple versions of each logo (different colors, orientations)
4. **Brand text detection**: Combine logo detection with OCR for brand names
5. **Fine-tuned CLIP**: Fine-tune CLIP on logo-specific data for better accuracy

## License Considerations

**IMPORTANT**: Logo detection involves intellectual property. Usage restrictions may apply:

1. **Logo images**: Ensure you have rights to use logo images (fair use, press kit, etc.)
2. **Brand trademarks**: Some brands prohibit unauthorized logo detection
3. **Commercial use**: May require permission from brand owners for commercial applications

**Recommended Use Cases** (generally acceptable):
- Academic research
- Content moderation (removing counterfeit/unauthorized logos)
- Brand monitoring (with brand authorization)
- Advertising analysis (aggregate statistics)

**Use Cases Requiring Legal Review**:
- Commercial logo recognition services
- Competitive analysis without authorization
- Trademark enforcement tools

**This implementation does NOT include logo images**. Users must provide their own logo images and ensure compliance with applicable licenses and intellectual property laws.
