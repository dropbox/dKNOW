#!/usr/bin/env python3
"""
Export DocLayout-YOLO model to CoreML format using ultralytics.

This script properly exports the model with trained weights and NMS included.

Usage:
    # Create virtualenv with ultralytics
    /opt/homebrew/bin/python3.12 -m venv .venv_ultralytics
    source .venv_ultralytics/bin/activate
    pip install torch ultralytics coremltools

    # Run export
    python scripts/export_doclayout_to_coreml.py
"""

import argparse
import sys
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description="Export DocLayout-YOLO to CoreML")
    parser.add_argument(
        "--input",
        type=str,
        default="models/doclayout_yolo_doclaynet_imgsz1120_from_scratch.pt",
        help="Input PyTorch model path",
    )
    parser.add_argument(
        "--output",
        type=str,
        default="models/doclayout_yolo_doclaynet.mlmodel",
        help="Output CoreML model path",
    )
    parser.add_argument(
        "--imgsz",
        type=int,
        default=1120,
        help="Input image size (default: 1120)",
    )
    parser.add_argument(
        "--nms",
        action="store_true",
        default=True,
        help="Include NMS in the model (default: True)",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="Verify the exported model",
    )
    args = parser.parse_args()

    # Check dependencies
    try:
        import torch
        import coremltools as ct

        print(f"PyTorch version: {torch.__version__}")
        print(f"coremltools version: {ct.__version__}")
    except ImportError as e:
        print(f"ERROR: Missing dependency: {e}")
        print("\nInstall with:")
        print("  pip install torch ultralytics coremltools")
        sys.exit(1)

    input_path = Path(args.input)
    output_path = Path(args.output)

    if not input_path.exists():
        print(f"ERROR: Input model not found: {input_path}")
        sys.exit(1)

    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Try using ultralytics first (recommended)
    success = export_with_ultralytics(input_path, output_path, args.imgsz, args.nms)

    if not success:
        print("\nTrying direct coremltools conversion...")
        success = export_with_coremltools(input_path, output_path, args.imgsz)

    if success and args.verify:
        verify_coreml(output_path)

    if success:
        print(f"\n✅ CoreML model exported to: {output_path}")
    else:
        print(f"\n❌ Export failed")
        sys.exit(1)


def export_with_ultralytics(input_path: Path, output_path: Path, imgsz: int, nms: bool) -> bool:
    """Export using ultralytics library (recommended approach)."""
    try:
        from ultralytics import YOLO

        print(f"\n=== Exporting with ultralytics ===")
        print(f"Loading model: {input_path}")

        # Load the model
        model = YOLO(str(input_path))

        # Get model info
        print(f"Model type: {type(model)}")
        print(f"Model task: {model.task}")

        # Export to CoreML
        print(f"\nExporting to CoreML (imgsz={imgsz}, nms={nms})...")

        # ultralytics export returns the path to the exported model
        exported = model.export(
            format="coreml",
            imgsz=imgsz,
            nms=nms,
            half=False,  # Keep FP32 for accuracy
        )

        print(f"Exported model path: {exported}")

        # Move to desired location if different
        exported_path = Path(exported)
        if exported_path.exists() and exported_path != output_path:
            import shutil

            # Handle both .mlmodel and .mlpackage
            if exported_path.is_dir():
                # .mlpackage is a directory
                if output_path.exists():
                    shutil.rmtree(output_path)
                shutil.move(str(exported_path), str(output_path))
            else:
                shutil.move(str(exported_path), str(output_path))
            print(f"Moved to: {output_path}")
            return True
        elif exported_path.exists():
            return True

        # Check for alternative output locations
        for ext in [".mlmodel", ".mlpackage"]:
            alt_path = input_path.with_suffix(ext)
            if alt_path.exists():
                import shutil
                shutil.move(str(alt_path), str(output_path))
                print(f"Found and moved from: {alt_path}")
                return True

        return False

    except ImportError:
        print("ultralytics not installed")
        return False
    except Exception as e:
        print(f"ERROR: {e}")
        import traceback
        traceback.print_exc()
        return False


def export_with_coremltools(input_path: Path, output_path: Path, imgsz: int) -> bool:
    """Export using coremltools directly (fallback approach)."""
    try:
        import torch
        import coremltools as ct

        print(f"\n=== Exporting with coremltools ===")
        print(f"Loading PyTorch model: {input_path}")

        # Load checkpoint
        checkpoint = torch.load(str(input_path), map_location="cpu", weights_only=False)

        # Extract model
        if isinstance(checkpoint, dict):
            if "model" in checkpoint:
                model = checkpoint["model"]
            elif "state_dict" in checkpoint:
                # Need to reconstruct model from state dict
                print("ERROR: Cannot reconstruct model from state_dict alone")
                return False
            else:
                print(f"Unexpected checkpoint structure. Keys: {checkpoint.keys()}")
                return False
        else:
            model = checkpoint

        # For YOLO models, the model might be wrapped
        if hasattr(model, "model"):
            model = model.model

        model.eval()

        # Create dummy input
        dummy_input = torch.randn(1, 3, imgsz, imgsz)

        print(f"Tracing model with input shape: {dummy_input.shape}")

        # Trace the model
        with torch.no_grad():
            traced = torch.jit.trace(model, dummy_input)

        print("Converting to CoreML...")

        # Convert to CoreML
        mlmodel = ct.convert(
            traced,
            inputs=[ct.TensorType(name="images", shape=(1, 3, imgsz, imgsz))],
            compute_units=ct.ComputeUnit.CPU_AND_NE,  # Use ANE when available
            minimum_deployment_target=ct.target.macOS13,
        )

        # Save
        mlmodel.save(str(output_path))
        print(f"Saved to: {output_path}")
        return True

    except Exception as e:
        print(f"ERROR: {e}")
        import traceback
        traceback.print_exc()
        return False


def verify_coreml(model_path: Path):
    """Verify the exported CoreML model."""
    try:
        import coremltools as ct
        import numpy as np

        print(f"\n=== Verifying CoreML model ===")
        print(f"Loading: {model_path}")

        model = ct.models.MLModel(str(model_path))
        spec = model.get_spec()

        # Print model info
        print(f"\nModel inputs:")
        for inp in spec.description.input:
            shape = list(inp.type.multiArrayType.shape)
            print(f"  {inp.name}: {shape}")

        print(f"\nModel outputs:")
        for out in spec.description.output:
            shape = list(out.type.multiArrayType.shape)
            print(f"  {out.name}: {shape}")

        # Test inference
        print("\nRunning test inference...")
        dummy_input = np.random.randn(1, 3, 1120, 1120).astype(np.float32)

        import time
        start = time.time()
        predictions = model.predict({"images": dummy_input})
        elapsed = (time.time() - start) * 1000

        print(f"Inference time: {elapsed:.2f}ms")
        for name, value in predictions.items():
            if hasattr(value, "shape"):
                print(f"Output '{name}': shape={value.shape}, dtype={value.dtype}")
                # Check value range
                print(f"  min={value.min():.4f}, max={value.max():.4f}, mean={value.mean():.4f}")
            else:
                print(f"Output '{name}': {type(value)}")

    except Exception as e:
        print(f"Verification error: {e}")


if __name__ == "__main__":
    main()
