#!/usr/bin/env python3
"""Export YOLOv8x-pose model to ONNX format for pose estimation.

This script downloads YOLOv8x-pose.pt from Ultralytics and exports it to ONNX format.

Requirements:
    pip3 install ultralytics

Usage:
    python3 models/pose-estimation/export_yolov8x_pose_to_onnx.py

Output:
    models/pose-estimation/yolov8x-pose.onnx (~260MB)
    models/pose-estimation/yolov8x-pose.pt (~131MB)
"""

from pathlib import Path
from ultralytics import YOLO

def main():
    model_dir = Path(__file__).parent
    model_dir.mkdir(exist_ok=True)

    pt_path = model_dir / "yolov8x-pose.pt"
    onnx_path = model_dir / "yolov8x-pose.onnx"

    print(f"Downloading YOLOv8x-pose model...")
    model = YOLO('yolov8x-pose.pt')

    print(f"Exporting to ONNX: {onnx_path}")
    model.export(
        format='onnx',
        opset=12,
        simplify=True,
        dynamic=False,
        imgsz=640
    )

    # Move the exported ONNX file to the correct location
    exported_onnx = Path('yolov8x-pose.onnx')
    if exported_onnx.exists():
        exported_onnx.rename(onnx_path)

    print(f"✅ Export complete!")
    print(f"  PyTorch model: {pt_path} (~131MB)")
    print(f"  ONNX model: {onnx_path} (~260MB)")
    print(f"\nModel details:")
    print(f"  - 17 keypoints (COCO format)")
    print(f"  - Input: 640×640 RGB")
    print(f"  - BEST accuracy for pose estimation")

if __name__ == "__main__":
    main()
