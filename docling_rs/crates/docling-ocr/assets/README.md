# OCR Model Assets

This directory contains the PaddleOCR PP-OCRv4 ONNX models required for OCR functionality.

## Required Models

The following files are required:

1. **det_model.onnx** (~4.5 MB) - Text detection model
2. **rec_model.onnx** (~10 MB) - Text recognition model
3. **ppocr_keys_v1.txt** (~26 KB) - Character dictionary for recognition

## Download Instructions

Run this script from the repository root to download the models:

```bash
#!/bin/bash
cd crates/docling-ocr/assets

# Detection model
curl -L -o det_model.onnx \
  "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.4.0/onnx/PP-OCRv4/det/ch_PP-OCRv4_det_infer.onnx"

# Recognition model
curl -L -o rec_model.onnx \
  "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v3.4.0/onnx/PP-OCRv4/rec/ch_PP-OCRv4_rec_infer.onnx"

# Character dictionary
curl -L -o ppocr_keys_v1.txt \
  "https://www.modelscope.cn/models/RapidAI/RapidOCR/resolve/v2.0.7/paddle/PP-OCRv4/rec/ch_PP-OCRv4_rec_infer/ppocr_keys_v1.txt"

echo "Models downloaded successfully!"
ls -lh
```

## Model Details

- **Source**: RapidAI/RapidOCR (ModelScope repository)
- **Version**: PP-OCRv4
- **Format**: ONNX (ONNXRuntime compatible)
- **Total Size**: ~14.5 MB
- **License**: Apache 2.0 (PaddleOCR license)

## Notes

- Models are embedded in the binary using `include_bytes!()` macro
- Models are **git-ignored** due to size - must download locally for development
- Alternative: Models can be downloaded at runtime (future enhancement)
- Chinese character support included (ppocr_keys_v1.txt contains 6622 characters)
