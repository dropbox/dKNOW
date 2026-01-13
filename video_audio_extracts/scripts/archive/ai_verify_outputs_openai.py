#!/usr/bin/env python3
"""AI Verification of Test Outputs using OpenAI GPT-4 Vision

Uses GPT-4 Vision API to verify that test outputs are semantically correct,
not just structurally valid.

Usage:
    python scripts/ai_verify_outputs_openai.py <input_file> <output_json> <operation>

Example:
    python scripts/ai_verify_outputs_openai.py \
        test_edge_cases/image_test_dog.jpg \
        debug_output/stage_00_face_detection.json \
        face-detection

Requires:
    export OPENAI_API_KEY="sk-proj-..."
    pip install openai
"""

from openai import OpenAI
import base64
import json
import os
import sys
from pathlib import Path


def verify_vision_output(input_image_path, output_json_path, operation):
    """Use GPT-4 Vision to verify vision outputs are semantically correct"""

    if not os.environ.get("OPENAI_API_KEY"):
        raise ValueError("OPENAI_API_KEY environment variable not set")

    client = OpenAI(api_key=os.environ["OPENAI_API_KEY"])

    # Read input image
    with open(input_image_path, 'rb') as f:
        image_data = base64.b64encode(f.read()).decode('utf-8')

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
3. For face-detection: Are bounding boxes around actual faces?
4. For object-detection: Are objects labeled correctly?
5. For ocr: Is text detection accurate?
6. For pose-estimation: Are keypoints on actual body parts?
7. For emotion-detection: Do emotions match facial expressions?
8. Rate confidence 0.0-1.0 (1.0 = perfect match)
9. List any errors or suspicious findings

Respond in JSON format:
{{
  "status": "CORRECT" | "SUSPICIOUS" | "INCORRECT",
  "confidence": 0.95,
  "findings": "What matches or doesn't match",
  "errors": ["list any errors found"]
}}"""

    # Call GPT-4 Vision (gpt-4o has vision capabilities built-in)
    response = client.chat.completions.create(
        model="gpt-4o",  # gpt-4o has vision capabilities
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:{media_type};base64,{image_data}"
                        }
                    },
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]
            }
        ],
        max_tokens=2048
    )

    content = response.choices[0].message.content

    # Strip markdown code blocks if present (GPT-4 often wraps JSON in ```json ... ```)
    if content.startswith("```json"):
        content = content[7:]  # Remove ```json
    if content.startswith("```"):
        content = content[3:]  # Remove ```
    if content.endswith("```"):
        content = content[:-3]  # Remove trailing ```
    content = content.strip()

    return content


def verify_text_output(output_json_path, operation):
    """Use GPT-4 to verify text outputs (transcription, etc.)"""

    if not os.environ.get("OPENAI_API_KEY"):
        raise ValueError("OPENAI_API_KEY environment variable not set")

    client = OpenAI(api_key=os.environ["OPENAI_API_KEY"])

    with open(output_json_path) as f:
        output = json.load(f)

    prompt = f"""Verify this {operation} output is semantically reasonable.

Operation: {operation}
Output: {json.dumps(output, indent=2)}

Check:
1. Does the structure make sense?
2. Are values plausible?
3. For transcription: Is text coherent and plausible?
4. For diarization: Do speaker segments make sense?
5. For audio-classification: Are event labels reasonable?
6. For profanity-detection: Are flagged words actually profane?
7. Any obvious errors?

Rate confidence 0.0-1.0.

Respond in JSON format:
{{
  "status": "CORRECT" | "SUSPICIOUS" | "INCORRECT",
  "confidence": 0.90,
  "findings": "What looks correct/suspicious/incorrect",
  "errors": ["list any errors found"]
}}"""

    response = client.chat.completions.create(
        model="gpt-4o",
        messages=[{"role": "user", "content": prompt}],
        max_tokens=1024
    )

    content = response.choices[0].message.content

    # Strip markdown code blocks if present
    if content.startswith("```json"):
        content = content[7:]
    if content.startswith("```"):
        content = content[3:]
    if content.endswith("```"):
        content = content[:-3]
    content = content.strip()

    return content


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
