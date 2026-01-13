#!/usr/bin/env python3
"""AI Verification of Test Outputs

Uses Claude API with vision capabilities to verify that test outputs are
semantically correct, not just structurally valid.

Usage:
    python scripts/ai_verify_outputs.py <input_file> <output_json> <operation>

Example:
    python scripts/ai_verify_outputs.py \
        test_edge_cases/image_test_dog.jpg \
        debug_output/stage_00_face_detection.json \
        face-detection

Requires:
    export ANTHROPIC_API_KEY="your-api-key"
"""

import anthropic
import base64
import json
import os
import sys
from pathlib import Path


def verify_vision_output(input_image_path, output_json_path, operation):
    """Use Claude to verify vision outputs are semantically correct"""

    if not os.environ.get("ANTHROPIC_API_KEY"):
        raise ValueError("ANTHROPIC_API_KEY environment variable not set")

    client = anthropic.Anthropic(api_key=os.environ["ANTHROPIC_API_KEY"])

    # Read input image
    with open(input_image_path, 'rb') as f:
        image_data = base64.standard_b64encode(f.read()).decode('utf-8')

    # Read output JSON
    with open(output_json_path) as f:
        output = json.load(f)

    # Determine image type
    ext = Path(input_image_path).suffix.lower()
    media_types = {
        '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg',
        '.png': 'image/png', '.webp': 'image/webp',
        '.gif': 'image/gif', '.bmp': 'image/bmp'
    }
    media_type = media_types.get(ext, 'image/jpeg')

    # Create verification prompt
    prompt = f"""Verify this {operation} output is semantically correct.

Input: {input_image_path}
Operation: {operation}
Output: {json.dumps(output, indent=2)}

Tasks:
1. Look at the image carefully
2. Check if the output matches what you see
3. Rate confidence 0.0-1.0 (1.0 = perfect match)
4. List any errors or suspicious findings

Respond in JSON:
{{
  "status": "CORRECT" | "SUSPICIOUS" | "INCORRECT",
  "confidence": 0.95,
  "findings": "What matches/doesn't match",
  "errors": ["any errors found"]
}}"""

    message = client.messages.create(
        model="claude-sonnet-4",
        max_tokens=2048,
        messages=[{
            "role": "user",
            "content": [
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type,
                        "data": image_data,
                    },
                },
                {
                    "type": "text",
                    "text": prompt
                }
            ],
        }]
    )

    return message.content[0].text


def verify_text_output(output_json_path, operation):
    """Use Claude to verify text outputs (transcription, etc.)"""

    if not os.environ.get("ANTHROPIC_API_KEY"):
        raise ValueError("ANTHROPIC_API_KEY environment variable not set")

    client = anthropic.Anthropic(api_key=os.environ["ANTHROPIC_API_KEY"])

    with open(output_json_path) as f:
        output = json.load(f)

    prompt = f"""Verify this {operation} output is semantically reasonable.

Operation: {operation}
Output: {json.dumps(output, indent=2)}

Check:
1. Does the structure make sense?
2. Are values plausible?
3. Any obvious errors?

Rate confidence 0.0-1.0.

Respond in JSON:
{{
  "status": "CORRECT" | "SUSPICIOUS" | "INCORRECT",
  "confidence": 0.95,
  "findings": "What looks correct/suspicious/incorrect",
  "errors": ["any errors found"]
}}"""

    message = client.messages.create(
        model="claude-sonnet-4",
        max_tokens=1024,
        messages=[{"role": "user", "content": prompt}]
    )

    return message.content[0].text


def main():
    if len(sys.argv) != 4:
        print(__doc__)
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2]
    operation = sys.argv[3]

    # Vision operations require image input
    vision_operations = [
        "face-detection", "object-detection", "ocr", "pose-estimation",
        "emotion-detection", "scene-detection", "action-recognition",
        "shot-classification", "smart-thumbnail", "duplicate-detection",
        "image-quality-assessment", "vision-embeddings", "keyframes"
    ]

    if operation in vision_operations:
        result = verify_vision_output(input_file, output_file, operation)
    else:
        result = verify_text_output(output_file, operation)

    print(result)


if __name__ == "__main__":
    main()
