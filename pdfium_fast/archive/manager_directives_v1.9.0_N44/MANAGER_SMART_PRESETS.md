# MANAGER: Add Smart Presets (Better than Raw DPI)

**To:** WORKER0 (N=37+)
**User feedback:** "Too many options - how do I choose DPI?"

---

## Problem

**Current (v1.8.0):**
```bash
./pdfium_cli --dpi 72 --format jpg render-pages input.pdf output/
```

**User must know:**
- What DPI to use?
- When to use JPEG vs PNG?
- What quality setting?

**This is TOO COMPLEX for most users.**

---

## Solution: Smart Presets

### Add --preset Flag (v1.9.0 Feature)

**Simple interface:**
```bash
# Default (high quality)
./pdfium_cli render-pages input.pdf output/

# Web preview (smaller, faster)
./pdfium_cli --preset web render-pages input.pdf output/

# Thumbnails (smallest, fastest)
./pdfium_cli --preset thumbnail render-pages input.pdf output/

# Print quality
./pdfium_cli --preset print render-pages input.pdf output/
```

### Preset Definitions

**Default (no preset):**
- Format: PNG
- DPI: 300
- Quality: Lossless
- Use case: Maximum quality

**--preset web:**
- Format: JPEG
- DPI: 150
- Quality: 85
- Max dimension: 2048px
- Use case: Web display, email
- **Speed: 1.8x faster, 10x smaller files**

**--preset thumbnail:**
- Format: JPEG
- DPI: 72 (or fixed width)
- Quality: 80
- Max dimension: 1024px
- Use case: Image previews, galleries
- **Speed: 2.3x faster, 20x smaller files**

**--preset print:**
- Format: PNG (lossless)
- DPI: 300
- Use case: High-quality printing

---

## Implementation (N=37-39)

**File:** `examples/pdfium_cli.cpp`

```cpp
enum class RenderPreset {
  DEFAULT,
  WEB,
  THUMBNAIL,
  PRINT
};

struct PresetConfig {
  const char* format;
  int dpi;
  int jpeg_quality;
  int max_dimension;
};

const PresetConfig PRESETS[] = {
  {"png", 300, 90, 0},        // DEFAULT
  {"jpg", 150, 85, 2048},     // WEB
  {"jpg", 72, 80, 1024},      // THUMBNAIL
  {"png", 300, 90, 0},        // PRINT
};

// Parse --preset flag
RenderPreset preset = RenderPreset::DEFAULT;
if (arg == "--preset") {
  if (next_arg == "web") preset = RenderPreset::WEB;
  else if (next_arg == "thumbnail") preset = RenderPreset::THUMBNAIL;
  else if (next_arg == "print") preset = RenderPreset::PRINT;
}

// Apply preset
auto config = PRESETS[static_cast<int>(preset)];
if (format.empty()) format = config.format;
if (dpi == 300.0) dpi = config.dpi;
if (jpeg_quality == 90) jpeg_quality = config.jpeg_quality;

// Optionally: Downscale if image > max_dimension
if (config.max_dimension > 0 && width > config.max_dimension) {
  double scale = config.max_dimension / (double)width;
  width *= scale;
  height *= scale;
}
```

**Help text:**
```cpp
fprintf(stderr, "  --preset MODE     Rendering preset: web|thumbnail|print\n");
fprintf(stderr, "\nPresets:\n");
fprintf(stderr, "  default:    300 DPI PNG (maximum quality)\n");
fprintf(stderr, "  web:        150 DPI JPEG q85 (web display, 1.8x faster)\n");
fprintf(stderr, "  thumbnail:  72 DPI JPEG q80 max 1024px (2.3x faster)\n");
fprintf(stderr, "  print:      300 DPI PNG (high-quality printing)\n");
```

**Examples:**
```cpp
fprintf(stderr, "  ./pdfium_cli --preset web render-pages input.pdf output/\n");
fprintf(stderr, "  ./pdfium_cli --preset thumbnail render-pages input.pdf thumbs/\n");
```

---

## User Experience Comparison

**Before (complex):**
```bash
# User thinks: "What DPI? What format? What quality?"
./pdfium_cli --dpi 150 --format jpg --quality 85 render-pages input.pdf output/
```

**After (simple):**
```bash
# User thinks: "I want web images"
./pdfium_cli --preset web render-pages input.pdf output/
```

**Advanced users can still use flags:**
```bash
# Override preset
./pdfium_cli --preset web --dpi 200 render-pages input.pdf output/
```

---

## WORKER: Implement Smart Presets (N=37-39)

**Priority:** HIGH (better UX than raw DPI)

1. Add preset parsing
2. Apply preset defaults
3. Document presets in help
4. Test all 3 presets

**Then do RGB/BGR and jemalloc (N=40-42)**

**This makes the tool EASIER to use while still being FASTER.**
