#!/usr/bin/env python3
"""
Convert ONNX models to CoreML format for Apple Neural Engine (ANE) acceleration.

Usage:
    python scripts/convert_to_coreml.py --input model.onnx --output model.mlpackage
    python scripts/convert_to_coreml.py --all  # Convert all models

Requirements:
    pip install coremltools onnx

Supported models:
    - Layout (RT-DETR): onnx_exports/layout_optimum/model.onnx
    - RapidOCR Detection: models/rapidocr/ch_PP-OCRv4_det_infer.onnx
    - RapidOCR Recognition: models/rapidocr/ch_PP-OCRv4_rec_infer.onnx
    - RapidOCR Classification: models/rapidocr/ch_ppocr_mobile_v2.0_cls_infer.onnx
    - TableFormer (Microsoft): onnx_exports/tableformer/table_structure_model.onnx
"""

import argparse
import os
import sys
from pathlib import Path

import coremltools as ct

# Model definitions with their configurations
MODELS = {
    "layout": {
        "onnx_path": "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx",
        "output_name": "layout.mlpackage",
        "description": "RT-DETR Layout Detection Model",
        # Dynamic input shape - CoreML will handle
    },
    "ocr_det": {
        "onnx_path": "crates/docling-pdf-ml/models/rapidocr/ch_PP-OCRv4_det_infer.onnx",
        "output_name": "ocr_detection.mlpackage",
        "description": "RapidOCR Text Detection Model (PP-OCRv4)",
    },
    "ocr_rec": {
        "onnx_path": "crates/docling-pdf-ml/models/rapidocr/ch_PP-OCRv4_rec_infer.onnx",
        "output_name": "ocr_recognition.mlpackage",
        "description": "RapidOCR Text Recognition Model (PP-OCRv4)",
    },
    "ocr_cls": {
        "onnx_path": "crates/docling-pdf-ml/models/rapidocr/ch_ppocr_mobile_v2.0_cls_infer.onnx",
        "output_name": "ocr_classification.mlpackage",
        "description": "RapidOCR Text Angle Classification Model",
    },
    "tableformer": {
        "onnx_path": "crates/docling-pdf-ml/onnx_exports/tableformer/table_structure_model.onnx",
        "output_name": "tableformer_ms.mlpackage",
        "description": "Microsoft Table Transformer Model",
    },
}


def convert_onnx_to_coreml(
    onnx_path: str,
    output_path: str,
    compute_units: str = "ALL",
    minimum_deployment_target: str = "macOS13",
) -> bool:
    """
    Convert an ONNX model to CoreML format.

    Args:
        onnx_path: Path to the input ONNX model
        output_path: Path for the output .mlpackage
        compute_units: Target compute units - "ALL", "CPU_AND_NE", "CPU_AND_GPU", "CPU_ONLY"
        minimum_deployment_target: Minimum macOS version (e.g., "macOS13" for Ventura)

    Returns:
        True if conversion succeeded, False otherwise
    """
    if not os.path.exists(onnx_path):
        print(f"ERROR: ONNX model not found: {onnx_path}")
        return False

    print(f"Converting: {onnx_path}")
    print(f"  -> Output: {output_path}")
    print(f"  -> Compute units: {compute_units}")
    print(f"  -> Target: {minimum_deployment_target}")

    try:
        # Map string to ct.ComputeUnit enum
        compute_unit_map = {
            "ALL": ct.ComputeUnit.ALL,
            "CPU_AND_NE": ct.ComputeUnit.CPU_AND_NE,
            "CPU_AND_GPU": ct.ComputeUnit.CPU_AND_GPU,
            "CPU_ONLY": ct.ComputeUnit.CPU_ONLY,
        }

        # Map string to ct.target enum (handle different coremltools versions)
        target_map = {
            "macOS13": ct.target.macOS13,
            "macOS14": ct.target.macOS14,
        }
        # macOS15 was added in later coremltools versions
        if hasattr(ct.target, 'macOS15'):
            target_map["macOS15"] = ct.target.macOS15

        # First load the ONNX model info
        import onnx
        onnx_model = onnx.load(onnx_path)
        print(f"  ONNX opset version: {onnx_model.opset_import[0].version}")

        # Check coremltools version for API selection
        ct_version = tuple(int(x) for x in ct.__version__.split('.')[:2])
        print(f"  coremltools version: {ct.__version__}")

        # coremltools < 8.0 supports ONNX directly
        if ct_version < (8, 0):
            print("  Using direct ONNX conversion (coremltools < 8.0)")
            model = ct.converters.onnx.convert(
                model=onnx_path,
                compute_units=compute_unit_map.get(compute_units, ct.ComputeUnit.ALL),
                minimum_deployment_target=target_map.get(
                    minimum_deployment_target, ct.target.macOS13
                ),
            )
        else:
            # coremltools 8.0+ doesn't support ONNX directly
            # Need to convert ONNX -> PyTorch -> CoreML
            # Get input shape from ONNX model
            input_name = onnx_model.graph.input[0].name
            input_shape = []
            for dim in onnx_model.graph.input[0].type.tensor_type.shape.dim:
                if dim.dim_value:
                    input_shape.append(dim.dim_value)
                else:
                    input_shape.append(1)  # Default dynamic dim to 1
            print(f"  Input: {input_name} shape={input_shape}")

            try:
                import torch
                from onnx2torch import convert as onnx2torch_convert

                print("  Converting ONNX to PyTorch...")
                torch_model = onnx2torch_convert(onnx_path)
                torch_model.eval()

                # Create example input for tracing
                example_input = torch.randn(input_shape)

                print("  Tracing PyTorch model...")
                traced_model = torch.jit.trace(torch_model, example_input)

                # Convert traced model to CoreML
                model = ct.convert(
                    traced_model,
                    source="pytorch",
                    convert_to="mlprogram",
                    inputs=[ct.TensorType(name=input_name, shape=input_shape)],
                    compute_units=compute_unit_map.get(compute_units, ct.ComputeUnit.ALL),
                    minimum_deployment_target=target_map.get(
                        minimum_deployment_target, ct.target.macOS13
                    ),
                )

            except ImportError:
                print("  WARNING: onnx2torch not installed")
                raise ValueError(
                    "coremltools 8.0+ doesn't support ONNX directly. "
                    "Either use coremltools < 8.0 or install onnx2torch: pip install onnx2torch"
                )

        # Save the model
        output_dir = os.path.dirname(output_path)
        if output_dir:
            os.makedirs(output_dir, exist_ok=True)

        model.save(output_path)
        print(f"  SUCCESS: Saved to {output_path}")

        # Print model info
        spec = model.get_spec()
        print(f"  Inputs: {[inp.name for inp in spec.description.input]}")
        print(f"  Outputs: {[out.name for out in spec.description.output]}")

        return True

    except Exception as e:
        print(f"  ERROR: Conversion failed: {e}")
        return False


def convert_all_models(output_dir: str = "models/coreml") -> dict:
    """
    Convert all known ONNX models to CoreML format.

    Args:
        output_dir: Directory to save converted models

    Returns:
        Dict of {model_name: success_bool}
    """
    results = {}
    os.makedirs(output_dir, exist_ok=True)

    for name, config in MODELS.items():
        print(f"\n{'='*60}")
        print(f"Converting: {name} - {config['description']}")
        print(f"{'='*60}")

        output_path = os.path.join(output_dir, config["output_name"])
        success = convert_onnx_to_coreml(
            onnx_path=config["onnx_path"],
            output_path=output_path,
        )
        results[name] = success

    # Summary
    print(f"\n{'='*60}")
    print("CONVERSION SUMMARY")
    print(f"{'='*60}")
    for name, success in results.items():
        status = "SUCCESS" if success else "FAILED"
        print(f"  {name}: {status}")

    succeeded = sum(1 for s in results.values() if s)
    total = len(results)
    print(f"\nTotal: {succeeded}/{total} models converted successfully")

    return results


def main():
    parser = argparse.ArgumentParser(
        description="Convert ONNX models to CoreML format for ANE acceleration"
    )
    parser.add_argument("--input", "-i", help="Input ONNX model path")
    parser.add_argument("--output", "-o", help="Output .mlpackage path")
    parser.add_argument(
        "--all", action="store_true", help="Convert all known models"
    )
    parser.add_argument(
        "--compute-units",
        choices=["ALL", "CPU_AND_NE", "CPU_AND_GPU", "CPU_ONLY"],
        default="ALL",
        help="Target compute units (default: ALL)",
    )
    parser.add_argument(
        "--target",
        choices=["macOS13", "macOS14", "macOS15"],
        default="macOS13",
        help="Minimum deployment target (default: macOS13)",
    )
    parser.add_argument(
        "--output-dir",
        default="models/coreml",
        help="Output directory for --all (default: models/coreml)",
    )

    args = parser.parse_args()

    if args.all:
        results = convert_all_models(output_dir=args.output_dir)
        sys.exit(0 if all(results.values()) else 1)
    elif args.input:
        if not args.output:
            # Generate output path from input
            input_path = Path(args.input)
            args.output = str(input_path.with_suffix(".mlpackage"))

        success = convert_onnx_to_coreml(
            onnx_path=args.input,
            output_path=args.output,
            compute_units=args.compute_units,
            minimum_deployment_target=args.target,
        )
        sys.exit(0 if success else 1)
    else:
        parser.print_help()
        print("\nAvailable models for --all conversion:")
        for name, config in MODELS.items():
            print(f"  {name}: {config['description']}")
            print(f"    ONNX: {config['onnx_path']}")
        sys.exit(1)


if __name__ == "__main__":
    main()
