#!/usr/bin/env python3
"""Test MLX LoRA save function."""

import json
from pathlib import Path

import mlx.core as mx
import numpy as np
import sys
sys.path.insert(0, str(Path(__file__).parent))
from train_xtr_mlx import T5Encoder, save_lora_weights

# Load model with LoRA
print("Loading model with LoRA...")
model = T5Encoder.from_pretrained("google/xtr-base-en", use_lora=True, lora_r=8, lora_alpha=16)

# Test save
output_path = Path("checkpoints/xtr-mlx-test-save")
output_path.mkdir(parents=True, exist_ok=True)

print("Saving LoRA weights...")
save_lora_weights(model, output_path)

# Verify
weights_path = output_path / "lora_weights.npz"
if weights_path.exists():
    loaded = dict(np.load(weights_path))
    print(f"Loaded {len(loaded)} weights from saved file:")
    for name, arr in list(loaded.items())[:6]:
        print(f"  {name}: {arr.shape}")
else:
    print("ERROR: Save failed")
