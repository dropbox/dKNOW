#!/usr/bin/env python3
"""
Export DocLayout-YOLO model to ONNX format.

Usage:
    # Install dependencies first:
    pip install torch ultralytics huggingface_hub onnx onnxruntime

    # Run export:
    python scripts/export_doclayout_yolo.py

    # Verify ONNX model:
    python scripts/export_doclayout_yolo.py --verify
"""

import argparse
import sys
from pathlib import Path

def main():
    parser = argparse.ArgumentParser(description="Export DocLayout-YOLO to ONNX")
    parser.add_argument("--verify", action="store_true", help="Verify ONNX export")
    parser.add_argument("--output", type=str, default="models/doclayout_yolo_doclaynet.onnx",
                       help="Output ONNX path")
    parser.add_argument("--model", type=str,
                       default="juliozhao/DocLayout-YOLO-DocLayNet-Docsynth300K_pretrained",
                       help="HuggingFace model ID")
    args = parser.parse_args()

    try:
        import torch
        from huggingface_hub import hf_hub_download
    except ImportError:
        print("ERROR: Missing dependencies. Install with:")
        print("  pip install torch huggingface_hub")
        sys.exit(1)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"Downloading model from: {args.model}")

    # Download the PyTorch model from HuggingFace
    # The model filename follows the pattern: doclayout_yolo_doclaynet_imgsz1120_docsynth_pretrain.pt
    try:
        model_path = hf_hub_download(
            repo_id=args.model,
            filename="doclayout_yolo_doclaynet_imgsz1120_docsynth_pretrain.pt",
            local_dir="models/cache",
        )
        print(f"Downloaded model to: {model_path}")
    except Exception as e:
        print(f"ERROR downloading model: {e}")
        print("\nAlternative: Download manually from HuggingFace:")
        print(f"  https://huggingface.co/{args.model}")
        sys.exit(1)

    # Try to load with doclayout_yolo first (native library)
    exported = False
    try:
        from doclayout_yolo import YOLOv10
        print("\nLoading model with doclayout_yolo native library...")
        model = YOLOv10(model_path)

        # Export to ONNX (use static sizes - dynamic export fails)
        print(f"\nExporting to ONNX: {output_path}")
        model.export(format="onnx", imgsz=1120, dynamic=False, simplify=False)

        # Move the exported file
        exported_path = Path(model_path).with_suffix(".onnx")
        if exported_path.exists():
            import shutil
            shutil.move(str(exported_path), str(output_path))
            print(f"\nONNX model saved to: {output_path}")
            exported = True
        else:
            print(f"WARNING: Expected ONNX file not found at: {exported_path}")

    except ImportError:
        print("doclayout_yolo not installed, trying ultralytics...")
    except Exception as e:
        print(f"ERROR with doclayout_yolo export: {e}")

    # Fallback to ultralytics
    if not exported:
        try:
            from ultralytics import YOLO
            print("\nLoading model with ultralytics...")
            model = YOLO(model_path)

            # Export to ONNX
            print(f"\nExporting to ONNX: {output_path}")
            model.export(format="onnx", imgsz=1120, dynamic=True)

            # Move the exported file
            exported_path = Path(model_path).with_suffix(".onnx")
            if exported_path.exists():
                import shutil
                shutil.move(str(exported_path), str(output_path))
                print(f"\nONNX model saved to: {output_path}")
                exported = True
            else:
                print(f"WARNING: Expected ONNX file not found at: {exported_path}")

        except ImportError:
            print("ultralytics not installed.")
        except Exception as e:
            print(f"ERROR with ultralytics export: {e}")

    # Final fallback: direct torch export
    if not exported:
        print("Trying direct torch export...")
        export_with_torch(model_path, output_path)

    if args.verify and output_path.exists():
        verify_onnx(output_path)


def export_with_torch(model_path: str, output_path: Path):
    """Export using TorchScript tracing approach for ONNX export."""
    import torch
    import torch.jit
    import warnings

    print(f"\nLoading PyTorch checkpoint: {model_path}")

    # Load checkpoint with weights_only=False (required for PyTorch 2.6+)
    # This model is from HuggingFace and trusted
    checkpoint = torch.load(model_path, map_location="cpu", weights_only=False)

    # Check what's in the checkpoint
    print(f"Checkpoint keys: {checkpoint.keys() if isinstance(checkpoint, dict) else 'raw tensor'}")

    if isinstance(checkpoint, dict):
        if "model" in checkpoint:
            model_state = checkpoint["model"]
            print(f"Model state type: {type(model_state)}")

            # For YOLO models, the model is typically stored directly
            if hasattr(model_state, "model"):
                model = model_state.model
            else:
                model = model_state
        else:
            print("WARNING: Unexpected checkpoint structure")
            print(f"Available keys: {list(checkpoint.keys())}")
            return
    else:
        model = checkpoint

    # Set to eval mode
    if hasattr(model, "eval"):
        model.eval()

    # Create dummy input (1120x1120 RGB image)
    dummy_input = torch.randn(1, 3, 1120, 1120)

    print(f"\nTrying TorchScript trace then ONNX export...")

    # Try using trace-based export (works better for YOLO models)
    try:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            # Trace the model
            traced = torch.jit.trace(model, dummy_input)

        print(f"Exporting traced model to ONNX: {output_path}")
        torch.onnx.export(
            traced,
            dummy_input,
            str(output_path),
            export_params=True,
            opset_version=14,
            do_constant_folding=True,
            input_names=["images"],
            output_names=["output"],
        )
        print(f"\nONNX model saved to: {output_path}")
        return
    except Exception as e:
        print(f"TorchScript trace failed: {e}")

    # Fallback: Try using dynamo_export flag
    print(f"\nTrying dynamo_export=False (legacy export)...")
    try:
        torch.onnx.export(
            model,
            dummy_input,
            str(output_path),
            export_params=True,
            opset_version=14,
            do_constant_folding=True,
            input_names=["images"],
            output_names=["output"],
            dynamo=False,  # Use legacy exporter
        )
        print(f"\nONNX model saved to: {output_path}")
    except Exception as e:
        print(f"Legacy ONNX export also failed: {e}")


def verify_onnx(onnx_path: Path):
    """Verify ONNX model."""
    print(f"\n## Verifying ONNX model: {onnx_path}")

    try:
        import onnx
        import onnxruntime as ort
        import numpy as np
    except ImportError:
        print("WARNING: onnx/onnxruntime not installed, skipping verification")
        return

    # Check ONNX model
    model = onnx.load(str(onnx_path))
    onnx.checker.check_model(model)
    print("ONNX model is valid")

    # Print model info
    print(f"\nModel inputs:")
    for inp in model.graph.input:
        shape = [d.dim_value if d.dim_value else d.dim_param for d in inp.type.tensor_type.shape.dim]
        print(f"  {inp.name}: {shape}")

    print(f"\nModel outputs:")
    for out in model.graph.output:
        shape = [d.dim_value if d.dim_value else d.dim_param for d in out.type.tensor_type.shape.dim]
        print(f"  {out.name}: {shape}")

    # Test inference
    print("\nTesting inference...")
    session = ort.InferenceSession(str(onnx_path))

    # Create test input
    input_name = session.get_inputs()[0].name
    dummy_input = np.random.randn(1, 3, 1120, 1120).astype(np.float32)

    import time
    start = time.time()
    outputs = session.run(None, {input_name: dummy_input})
    elapsed = (time.time() - start) * 1000

    print(f"Inference time: {elapsed:.2f}ms")
    print(f"Output shapes: {[o.shape for o in outputs]}")
    print("\nVerification complete!")


if __name__ == "__main__":
    main()
