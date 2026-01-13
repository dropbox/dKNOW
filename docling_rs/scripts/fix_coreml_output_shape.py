#!/usr/bin/env python3
"""
Fix CoreML model output shape from dynamic to fixed [1, 15, 25725].

This resolves the coreml-rs error:
"Unable to copy Float32 1 × 15 × 25725 array into Float32  array"

Usage:
    source .venv_coreml_sys/bin/activate
    python scripts/fix_coreml_output_shape.py
"""

import coremltools as ct
from coremltools.models import MLModel
import numpy as np

def fix_output_shape():
    input_path = "models/doclayout_yolo_doclaynet.mlmodel"
    output_path = "models/doclayout_yolo_doclaynet_fixed.mlmodel"

    print(f"Loading model: {input_path}")
    model = ct.models.MLModel(input_path)

    # Get model spec
    spec = model.get_spec()

    print(f"\nOriginal model info:")
    print(f"  Input: {spec.description.input[0]}")
    print(f"  Output: {spec.description.output[0]}")

    # Update output shape to fixed [1, 15, 25725]
    output_desc = spec.description.output[0]
    output_name = output_desc.name

    # Clear the multiarray shape and set fixed dimensions
    output_desc.type.multiArrayType.ClearField('shape')
    output_desc.type.multiArrayType.shape.append(1)
    output_desc.type.multiArrayType.shape.append(15)
    output_desc.type.multiArrayType.shape.append(25725)

    print(f"\nFixed output shape: {list(output_desc.type.multiArrayType.shape)}")

    # Save the fixed model
    fixed_model = ct.models.MLModel(spec)
    fixed_model.save(output_path)
    print(f"\nSaved fixed model to: {output_path}")

    # Verify the fix
    print("\nVerifying fixed model...")
    verify_model = ct.models.MLModel(output_path)
    verify_spec = verify_model.get_spec()
    verify_output = verify_spec.description.output[0]
    print(f"  Output shape: {list(verify_output.type.multiArrayType.shape)}")

    return output_path

if __name__ == "__main__":
    fix_output_shape()
