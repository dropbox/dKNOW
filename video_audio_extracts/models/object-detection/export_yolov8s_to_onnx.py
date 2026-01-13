#!/usr/bin/env python3
"""Export YOLOv8s to ONNX format for object detection.

This script downloads and exports the YOLOv8s model from Ultralytics to ONNX format.
YOLOv8s is the small variant with better accuracy (75-80%) compared to YOLOv8n (60-70%).

Usage:
    python export_yolov8s_to_onnx.py

Output:
    yolov8s.onnx (approx 22 MB)
"""

from ultralytics import YOLO

def main():
    print("Loading YOLOv8s model...")
    model = YOLO('yolov8s.pt')  # Load the pre-trained model

    print("Exporting to ONNX format...")
    model.export(
        format='onnx',
        opset=12,  # ONNX opset version (12 is widely supported)
        simplify=True,  # Simplify the ONNX model
        dynamic=False,  # Use static shapes for better performance
        imgsz=640,  # Input image size (640x640 is standard for YOLO)
    )

    print("Export complete! Output: yolov8s.onnx")
    print("Model details:")
    print("  - Input: 640x640 RGB image")
    print("  - Output: COCO 80 classes")
    print("  - Size: ~22 MB")
    print("  - Accuracy: 75-80% (vs 60-70% for YOLOv8n)")
    print("  - Speed: 200 FPS (vs 300 FPS for YOLOv8n)")

if __name__ == "__main__":
    main()
