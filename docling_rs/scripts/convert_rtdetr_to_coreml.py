#!/usr/bin/env python3
"""
Convert RT-DETR Layout Model from PyTorch/HuggingFace to CoreML for ANE acceleration.

This script attempts direct PyTorch → CoreML conversion, bypassing the blocked
ONNX → CoreML path (coremltools 8.0+ dropped ONNX support).

Usage:
    source .venv_coreml/bin/activate
    python scripts/convert_rtdetr_to_coreml.py

Expected blockers (documented in WORKER_DIRECTIVE.md):
    - Deformable attention may not convert
    - May need to simplify model architecture
    - Alternative: Export with torch.export() (PyTorch 2.0+)

N=3496: Initial attempt at PyTorch → CoreML conversion

FINDINGS (N=3496):
    RT-DETR CoreML conversion is BLOCKED by unsupported operations:

    1. GridSample (opset 16) - Used for deformable attention in RT-DETR backbone
       - onnx2torch: NotImplementedError: Converter is not implemented (GridSample)
       - CoreML: No direct support for this spatial transformation operation

    2. DequantizeLinear (INT8 model) - Quantization ops need special handling

    CONCLUSION:
    - RT-DETR cannot be converted to CoreML via PyTorch or ONNX pathways
    - The deformable attention mechanism is incompatible with CoreML
    - YOLO might convert better (simpler architecture without deformable attention)

    RECOMMENDATION:
    - Use ONNX Runtime INT8 on CPU (best current option: 116ms)
    - Try YOLO → CoreML conversion instead (simpler architecture)
    - Wait for coremltools/onnx2torch to support GridSample
"""

import os
import sys
from pathlib import Path

def main():
    print("=" * 60)
    print("RT-DETR PyTorch → CoreML Direct Conversion")
    print("=" * 60)

    # Check coremltools version
    import coremltools as ct
    print(f"coremltools version: {ct.__version__}")

    import torch
    print(f"PyTorch version: {torch.__version__}")

    # Output directory
    output_dir = Path("models/coreml")
    output_dir.mkdir(parents=True, exist_ok=True)

    # Try multiple approaches

    # =========================================================================
    # APPROACH 1: Load from HuggingFace and trace
    # =========================================================================
    print("\n" + "=" * 60)
    print("APPROACH 1: HuggingFace Model + torch.jit.trace")
    print("=" * 60)

    try:
        from transformers import AutoModelForObjectDetection, AutoConfig

        # Try loading the docling RT-DETR model
        # Model ID from docling: ds4sd/docling-layout-hf
        model_id = "ds4sd/docling-layout-hf"
        print(f"Loading model: {model_id}")

        config = AutoConfig.from_pretrained(model_id, trust_remote_code=True)
        print(f"Model config: {config.model_type}")

        model = AutoModelForObjectDetection.from_pretrained(
            model_id,
            trust_remote_code=True
        )
        model.eval()
        print(f"Model loaded: {type(model).__name__}")

        # Create dummy input
        dummy_input = torch.randn(1, 3, 640, 640)
        print(f"Dummy input shape: {dummy_input.shape}")

        # Trace the model
        print("Tracing model with torch.jit.trace...")
        with torch.no_grad():
            traced = torch.jit.trace(model, dummy_input)
        print("Tracing successful!")

        # Convert to CoreML
        print("Converting to CoreML...")
        mlmodel = ct.convert(
            traced,
            source="pytorch",
            convert_to="mlprogram",
            inputs=[ct.TensorType(name="pixel_values", shape=(1, 3, 640, 640))],
            compute_units=ct.ComputeUnit.CPU_AND_NE,
            minimum_deployment_target=ct.target.macOS13,
        )

        output_path = output_dir / "rtdetr_layout.mlpackage"
        mlmodel.save(str(output_path))
        print(f"SUCCESS: Model saved to {output_path}")

        # Verify model info
        spec = mlmodel.get_spec()
        print(f"Inputs: {[inp.name for inp in spec.description.input]}")
        print(f"Outputs: {[out.name for out in spec.description.output]}")

        return True

    except Exception as e:
        print(f"APPROACH 1 FAILED: {e}")
        import traceback
        traceback.print_exc()

    # =========================================================================
    # APPROACH 2: Use torch.export (PyTorch 2.0+ ExportedProgram)
    # =========================================================================
    print("\n" + "=" * 60)
    print("APPROACH 2: torch.export (PyTorch 2.0+)")
    print("=" * 60)

    try:
        from transformers import AutoModelForObjectDetection

        model_id = "ds4sd/docling-layout-hf"
        model = AutoModelForObjectDetection.from_pretrained(
            model_id,
            trust_remote_code=True
        )
        model.eval()

        dummy_input = torch.randn(1, 3, 640, 640)

        print("Using torch.export()...")
        # ExportedProgram is the new format in PyTorch 2.0+
        exported = torch.export.export(model, (dummy_input,))
        print(f"Export successful: {type(exported)}")

        print("Converting ExportedProgram to CoreML...")
        mlmodel = ct.convert(
            exported,
            source="pytorch",
            convert_to="mlprogram",
            inputs=[ct.TensorType(name="pixel_values", shape=(1, 3, 640, 640))],
            compute_units=ct.ComputeUnit.CPU_AND_NE,
            minimum_deployment_target=ct.target.macOS13,
        )

        output_path = output_dir / "rtdetr_layout_export.mlpackage"
        mlmodel.save(str(output_path))
        print(f"SUCCESS: Model saved to {output_path}")
        return True

    except Exception as e:
        print(f"APPROACH 2 FAILED: {e}")
        import traceback
        traceback.print_exc()

    # =========================================================================
    # APPROACH 3: Convert ONNX model using onnx2torch then to CoreML
    # =========================================================================
    print("\n" + "=" * 60)
    print("APPROACH 3: ONNX → PyTorch → CoreML (via onnx2torch)")
    print("=" * 60)

    try:
        # Check if onnx2torch is available
        try:
            from onnx2torch import convert as onnx2torch_convert
        except ImportError:
            print("onnx2torch not installed. Installing...")
            import subprocess
            subprocess.run([sys.executable, "-m", "pip", "install", "onnx2torch"], check=True)
            from onnx2torch import convert as onnx2torch_convert

        import onnx

        # Find our ONNX model
        onnx_paths = [
            "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx",
            "crates/docling-pdf-ml/onnx_exports/layout_optimum/model_int8.onnx",
        ]

        onnx_path = None
        for p in onnx_paths:
            if os.path.exists(p):
                onnx_path = p
                break

        if not onnx_path:
            print("No ONNX model found at expected paths")
            raise FileNotFoundError("ONNX model not found")

        print(f"Loading ONNX model: {onnx_path}")

        # Load and convert ONNX to PyTorch
        print("Converting ONNX to PyTorch...")
        torch_model = onnx2torch_convert(onnx_path)
        torch_model.eval()
        print(f"Converted to PyTorch: {type(torch_model)}")

        # Get input shape from ONNX model
        onnx_model = onnx.load(onnx_path)
        input_info = onnx_model.graph.input[0]
        input_name = input_info.name
        input_shape = []
        for dim in input_info.type.tensor_type.shape.dim:
            if dim.dim_value:
                input_shape.append(dim.dim_value)
            else:
                input_shape.append(1)
        print(f"Input: {input_name} shape={input_shape}")

        # Create example input for tracing
        example_input = torch.randn(input_shape)

        print("Tracing PyTorch model...")
        traced_model = torch.jit.trace(torch_model, example_input)

        # Convert to CoreML
        print("Converting to CoreML...")
        mlmodel = ct.convert(
            traced_model,
            source="pytorch",
            convert_to="mlprogram",
            inputs=[ct.TensorType(name=input_name, shape=input_shape)],
            compute_units=ct.ComputeUnit.CPU_AND_NE,
            minimum_deployment_target=ct.target.macOS13,
        )

        output_path = output_dir / "rtdetr_layout_onnx2torch.mlpackage"
        mlmodel.save(str(output_path))
        print(f"SUCCESS: Model saved to {output_path}")
        return True

    except Exception as e:
        print(f"APPROACH 3 FAILED: {e}")
        import traceback
        traceback.print_exc()

    # =========================================================================
    # APPROACH 4: Simplified forward pass (skip deformable attention)
    # =========================================================================
    print("\n" + "=" * 60)
    print("APPROACH 4: Simplified Model (skip complex ops)")
    print("=" * 60)

    print("NOTE: This would require modifying the model architecture")
    print("to remove deformable attention and other unsupported operations.")
    print("This is a significant undertaking and may degrade accuracy.")
    print("Skipping for now - would need architecture-specific implementation.")

    # =========================================================================
    # Summary
    # =========================================================================
    print("\n" + "=" * 60)
    print("CONVERSION SUMMARY")
    print("=" * 60)
    print("All approaches failed. CoreML conversion remains blocked.")
    print("\nPossible reasons:")
    print("1. RT-DETR uses deformable attention (complex spatial operations)")
    print("2. Dynamic shapes not supported in CoreML")
    print("3. Certain ONNX operators don't have CoreML equivalents")
    print("\nNext steps:")
    print("1. Try a simpler model architecture (YOLO is simpler than RT-DETR)")
    print("2. Wait for coremltools to add better transformer support")
    print("3. Use ONNX Runtime CPU backend (current best: INT8 @ 116ms)")

    return False


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
