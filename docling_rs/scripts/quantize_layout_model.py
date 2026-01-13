#!/usr/bin/env python3
"""
INT8 Static Quantization for Layout Detection Model

Quantizes the RT-DETR layout detection ONNX model to INT8 for faster inference.
Uses static quantization with calibration data from PDF page renders.

Expected Results:
- Model size: ~164MB -> ~42MB (4x reduction)
- Inference speed: ~2x faster
- Accuracy loss: <1% mAP

Usage:
    python scripts/quantize_layout_model.py \
        --input crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx \
        --output crates/docling-pdf-ml/onnx_exports/layout_optimum/model_int8.onnx \
        --calibration-dir test-corpus/pdf/

Requirements:
    pip install onnxruntime onnx pillow pymupdf numpy
"""

import argparse
import os
import sys
from pathlib import Path
from typing import Iterator, Dict, Any
import numpy as np

try:
    import onnx
    from onnxruntime.quantization import (
        quantize_static,
        quantize_dynamic,
        CalibrationDataReader,
        QuantType,
        QuantFormat,
    )
except ImportError:
    print("ERROR: Missing dependencies. Install with:")
    print("  pip install onnxruntime onnx")
    sys.exit(1)

try:
    import fitz  # PyMuPDF for PDF rendering
except ImportError:
    fitz = None
    print("WARNING: pymupdf not installed. Will try to use pre-rendered images.")

try:
    from PIL import Image
except ImportError:
    print("ERROR: Missing pillow. Install with: pip install pillow")
    sys.exit(1)


# Model input configuration (from RT-DETR preprocessor)
TARGET_SIZE = 640
INPUT_NAME = "pixel_values"


def render_pdf_page(pdf_path: str, page_num: int = 0, dpi: int = 144) -> np.ndarray:
    """Render a PDF page to numpy array (RGB, HWC format)."""
    if fitz is None:
        raise ImportError("pymupdf not installed")

    doc = fitz.open(pdf_path)
    page = doc[page_num]

    # Render at specified DPI
    zoom = dpi / 72.0
    mat = fitz.Matrix(zoom, zoom)
    pix = page.get_pixmap(matrix=mat, colorspace=fitz.csRGB)

    # Convert to numpy
    img = np.frombuffer(pix.samples, dtype=np.uint8).reshape(pix.height, pix.width, 3)
    doc.close()

    return img


def preprocess_image(image: np.ndarray) -> np.ndarray:
    """
    Preprocess image for RT-DETR model.

    - Resize to 640x640
    - Rescale by 1/255 (no normalization)
    - Convert HWC to NCHW

    Args:
        image: RGB image in HWC format, uint8 [0-255]

    Returns:
        Preprocessed tensor in NCHW format, float32 [0-1]
    """
    # Resize to TARGET_SIZE x TARGET_SIZE using bilinear interpolation
    pil_img = Image.fromarray(image)
    pil_img = pil_img.resize((TARGET_SIZE, TARGET_SIZE), Image.Resampling.BILINEAR)
    resized = np.array(pil_img, dtype=np.float32)

    # Rescale to [0, 1]
    rescaled = resized / 255.0

    # Convert HWC to NCHW (batch of 1)
    nchw = rescaled.transpose(2, 0, 1)[np.newaxis, ...]

    return nchw.astype(np.float32)


class LayoutCalibrationDataReader(CalibrationDataReader):
    """
    Calibration data reader for layout model quantization.

    Reads PDF files from a directory, renders pages, and preprocesses them
    for the RT-DETR layout detection model.
    """

    def __init__(self, calibration_dir: str, max_samples: int = 100, pages_per_pdf: int = 3):
        """
        Initialize calibration data reader.

        Args:
            calibration_dir: Directory containing PDF files
            max_samples: Maximum number of calibration samples
            pages_per_pdf: Number of pages to sample from each PDF
        """
        self.calibration_dir = Path(calibration_dir)
        self.max_samples = max_samples
        self.pages_per_pdf = pages_per_pdf

        # Collect calibration data
        self.data = list(self._generate_data())
        self.index = 0

        print(f"Loaded {len(self.data)} calibration samples")

    def _generate_data(self) -> Iterator[Dict[str, np.ndarray]]:
        """Generate preprocessed calibration data from PDFs."""
        pdf_files = list(self.calibration_dir.glob("*.pdf"))

        if not pdf_files:
            print(f"WARNING: No PDF files found in {self.calibration_dir}")
            print("Generating synthetic calibration data...")
            for _ in range(min(self.max_samples, 50)):
                # Generate synthetic document-like images
                img = self._generate_synthetic_page()
                yield {INPUT_NAME: preprocess_image(img)}
            return

        samples = 0
        for pdf_path in pdf_files:
            if samples >= self.max_samples:
                break

            try:
                doc = fitz.open(str(pdf_path)) if fitz else None
                if doc is None:
                    continue

                num_pages = min(len(doc), self.pages_per_pdf)

                for page_idx in range(num_pages):
                    if samples >= self.max_samples:
                        break

                    try:
                        img = render_pdf_page(str(pdf_path), page_idx)
                        preprocessed = preprocess_image(img)
                        yield {INPUT_NAME: preprocessed}
                        samples += 1
                    except Exception as e:
                        print(f"  Warning: Failed to render {pdf_path} page {page_idx}: {e}")

                doc.close()
            except Exception as e:
                print(f"  Warning: Failed to open {pdf_path}: {e}")

        print(f"  Generated {samples} samples from {len(pdf_files)} PDFs")

    def _generate_synthetic_page(self) -> np.ndarray:
        """Generate a synthetic document-like page for calibration."""
        # Create a white page with document-like features
        height, width = 842, 595  # A4 page at 72 DPI
        img = np.ones((height, width, 3), dtype=np.uint8) * 255

        # Add some text-like gray areas (simulate text blocks)
        np.random.seed(None)  # Different each time
        num_blocks = np.random.randint(3, 8)

        for _ in range(num_blocks):
            y = np.random.randint(50, height - 100)
            x = np.random.randint(50, width - 200)
            h = np.random.randint(20, 150)
            w = np.random.randint(100, width - x - 50)

            # Simulate text as gray lines
            gray_value = np.random.randint(30, 100)
            for line_y in range(y, min(y + h, height), 15):
                line_h = min(10, height - line_y)
                img[line_y:line_y+line_h, x:x+w] = gray_value

        return img

    def get_next(self) -> Dict[str, np.ndarray] | None:
        """Get next calibration sample."""
        if self.index >= len(self.data):
            return None
        result = self.data[self.index]
        self.index += 1
        return result

    def rewind(self):
        """Rewind to beginning of calibration data."""
        self.index = 0


def quantize_model_static(
    input_path: str,
    output_path: str,
    calibration_dir: str,
    max_samples: int = 100
) -> None:
    """
    Quantize ONNX model using static INT8 quantization.

    Args:
        input_path: Path to input ONNX model (FP32)
        output_path: Path to output quantized model (INT8)
        calibration_dir: Directory containing PDF files for calibration
        max_samples: Maximum calibration samples
    """
    print(f"\n=== Static INT8 Quantization ===")
    print(f"Input:  {input_path}")
    print(f"Output: {output_path}")
    print(f"Calibration: {calibration_dir}")

    # Create calibration data reader
    calib_reader = LayoutCalibrationDataReader(
        calibration_dir,
        max_samples=max_samples
    )

    if len(calib_reader.data) == 0:
        print("ERROR: No calibration data available")
        sys.exit(1)

    print(f"\nRunning static quantization with {len(calib_reader.data)} samples...")

    # Perform static quantization
    quantize_static(
        model_input=input_path,
        model_output=output_path,
        calibration_data_reader=calib_reader,
        quant_format=QuantFormat.QDQ,  # Quantize-Dequantize format (better accuracy)
        per_channel=True,  # Per-channel quantization (better accuracy)
        weight_type=QuantType.QInt8,
        activation_type=QuantType.QInt8,
        extra_options={
            "ActivationSymmetric": False,  # Asymmetric for activations
            "WeightSymmetric": True,  # Symmetric for weights
            "CalibTensorRangeSymmetric": False,
        }
    )

    print(f"\nQuantization complete!")

    # Report size reduction
    input_size = Path(input_path).stat().st_size / (1024 * 1024)
    output_size = Path(output_path).stat().st_size / (1024 * 1024)
    reduction = (1 - output_size / input_size) * 100

    print(f"\n=== Results ===")
    print(f"Original size:  {input_size:.1f} MB")
    print(f"Quantized size: {output_size:.1f} MB")
    print(f"Size reduction: {reduction:.1f}%")
    print(f"Compression:    {input_size/output_size:.1f}x")


def quantize_model_dynamic(
    input_path: str,
    output_path: str
) -> None:
    """
    Quantize ONNX model using dynamic INT8 quantization.

    Dynamic quantization is faster (no calibration) but less accurate.
    Used as fallback when no calibration data available.

    Args:
        input_path: Path to input ONNX model (FP32)
        output_path: Path to output quantized model (INT8)
    """
    print(f"\n=== Dynamic INT8 Quantization ===")
    print(f"Input:  {input_path}")
    print(f"Output: {output_path}")
    print("(No calibration data required)")

    quantize_dynamic(
        model_input=input_path,
        model_output=output_path,
        weight_type=QuantType.QInt8,
        extra_options={
            "WeightSymmetric": True,
        }
    )

    print(f"\nQuantization complete!")

    # Report size reduction
    input_size = Path(input_path).stat().st_size / (1024 * 1024)
    output_size = Path(output_path).stat().st_size / (1024 * 1024)
    reduction = (1 - output_size / input_size) * 100

    print(f"\n=== Results ===")
    print(f"Original size:  {input_size:.1f} MB")
    print(f"Quantized size: {output_size:.1f} MB")
    print(f"Size reduction: {reduction:.1f}%")
    print(f"Compression:    {input_size/output_size:.1f}x")


def verify_model(model_path: str) -> bool:
    """Verify ONNX model is valid."""
    try:
        model = onnx.load(model_path)
        onnx.checker.check_model(model)
        print(f"✓ Model {model_path} is valid")
        return True
    except Exception as e:
        print(f"✗ Model validation failed: {e}")
        return False


def main():
    parser = argparse.ArgumentParser(
        description="Quantize RT-DETR layout detection model to INT8",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    # Static quantization (recommended)
    python scripts/quantize_layout_model.py \\
        --input crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx \\
        --output crates/docling-pdf-ml/onnx_exports/layout_optimum/model_int8.onnx \\
        --calibration-dir test-corpus/pdf/

    # Dynamic quantization (no calibration, less accurate)
    python scripts/quantize_layout_model.py \\
        --input crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx \\
        --output crates/docling-pdf-ml/onnx_exports/layout_optimum/model_int8_dynamic.onnx \\
        --dynamic

    # Verify quantized model
    python scripts/quantize_layout_model.py \\
        --verify crates/docling-pdf-ml/onnx_exports/layout_optimum/model_int8.onnx
"""
    )

    parser.add_argument(
        "--input", "-i",
        type=str,
        help="Path to input ONNX model (FP32)"
    )

    parser.add_argument(
        "--output", "-o",
        type=str,
        help="Path to output quantized model"
    )

    parser.add_argument(
        "--calibration-dir", "-c",
        type=str,
        default="test-corpus/pdf/",
        help="Directory containing PDF files for calibration (default: test-corpus/pdf/)"
    )

    parser.add_argument(
        "--max-samples", "-n",
        type=int,
        default=100,
        help="Maximum calibration samples (default: 100)"
    )

    parser.add_argument(
        "--dynamic", "-d",
        action="store_true",
        help="Use dynamic quantization instead of static"
    )

    parser.add_argument(
        "--verify", "-v",
        type=str,
        help="Verify an existing ONNX model"
    )

    args = parser.parse_args()

    # Verify mode
    if args.verify:
        success = verify_model(args.verify)
        sys.exit(0 if success else 1)

    # Quantization mode
    if not args.input or not args.output:
        parser.error("--input and --output are required for quantization")

    if not Path(args.input).exists():
        print(f"ERROR: Input model not found: {args.input}")
        sys.exit(1)

    # Create output directory if needed
    output_dir = Path(args.output).parent
    output_dir.mkdir(parents=True, exist_ok=True)

    # Verify input model
    print("Verifying input model...")
    if not verify_model(args.input):
        sys.exit(1)

    # Run quantization
    if args.dynamic:
        quantize_model_dynamic(args.input, args.output)
    else:
        quantize_model_static(
            args.input,
            args.output,
            args.calibration_dir,
            args.max_samples
        )

    # Verify output model
    print("\nVerifying output model...")
    if not verify_model(args.output):
        sys.exit(1)

    print("\n✓ Quantization successful!")
    print(f"  Quantized model: {args.output}")
    print(f"\nNext steps:")
    print(f"  1. Benchmark: cargo bench --bench layout_optimization")
    print(f"  2. Test accuracy: Run integration tests with quantized model")


if __name__ == "__main__":
    main()
