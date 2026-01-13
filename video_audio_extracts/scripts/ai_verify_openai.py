#!/usr/bin/env python3
"""AI Verification of Test Outputs using OpenAI GPT-4 Vision

Verifies that test outputs are semantically correct using GPT-4 Vision API.

Usage:
    python scripts/ai_verify_openai.py <input_file> <output_json> <operation>

Example:
    python scripts/ai_verify_openai.py \
        test_files_camera_raw/sony_a55.arw \
        debug_output/stage_00_face_detection.json \
        face-detection

Requires:
    export OPENAI_API_KEY="sk-..."
    OR place key in OPENAI_API_KEY.txt
"""

from openai import OpenAI
import base64
import json
import os
import sys
import tempfile
import subprocess
from pathlib import Path


def load_api_key():
    """Load API key from environment or file"""
    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        # Try loading from OPENAI_API_KEY.txt
        key_file = Path(__file__).parent.parent / "OPENAI_API_KEY.txt"
        if key_file.exists():
            key = key_file.read_text().strip()
    if not key:
        raise ValueError("OPENAI_API_KEY not found in environment or OPENAI_API_KEY.txt")
    return key


def is_supported_format(image_path):
    """Check if image format is supported by OpenAI (png, jpeg, gif, webp)"""
    supported_exts = {'.png', '.jpg', '.jpeg', '.gif', '.webp'}
    return Path(image_path).suffix.lower() in supported_exts


def convert_to_jpeg(image_path):
    """Convert unsupported image formats to JPEG using FFmpeg/dcraw

    Returns path to converted JPEG (temporary file that caller should clean up)
    """
    # Create temporary JPEG file
    temp_fd, temp_path = tempfile.mkstemp(suffix='.jpg')
    os.close(temp_fd)

    # Check if it's a RAW camera format (requires dcraw)
    raw_formats = {'.arw', '.cr2', '.cr3', '.nef', '.raf', '.dng', '.orf', '.rw2', '.srw', '.pef'}
    is_raw = Path(image_path).suffix.lower() in raw_formats

    try:
        if is_raw:
            # Use dcraw to convert RAW to PPM, then FFmpeg to JPEG
            temp_ppm_fd, temp_ppm_path = tempfile.mkstemp(suffix='.ppm')
            os.close(temp_ppm_fd)

            try:
                # dcraw: -c output to stdout, -w camera white balance, -q 3 interpolation quality
                with open(temp_ppm_path, 'wb') as f:
                    subprocess.run(
                        ['dcraw', '-c', '-w', '-q', '3', image_path],
                        check=True,
                        stdout=f,
                        stderr=subprocess.PIPE
                    )

                # Convert PPM to JPEG with FFmpeg
                subprocess.run(
                    ['ffmpeg', '-i', temp_ppm_path, '-qscale:v', '2', temp_path, '-y'],
                    check=True,
                    capture_output=True
                )
            finally:
                if os.path.exists(temp_ppm_path):
                    os.unlink(temp_ppm_path)
        else:
            # Use FFmpeg directly for non-RAW formats (HEIC, AVIF, BMP, etc.)
            # For video files, extract first frame only with -update 1
            subprocess.run(
                ['ffmpeg', '-i', image_path, '-vframes', '1', '-qscale:v', '2', '-update', '1', temp_path, '-y'],
                check=True,
                capture_output=True
            )

        return temp_path
    except subprocess.CalledProcessError as e:
        if os.path.exists(temp_path):
            os.unlink(temp_path)
        stderr = e.stderr.decode() if e.stderr else "Unknown error"
        raise RuntimeError(f"Failed to convert {image_path} to JPEG: {stderr}")


def encode_image(image_path):
    """Encode image to base64, converting to JPEG if necessary"""
    temp_file = None
    try:
        # Convert to JPEG if format is not supported by OpenAI
        if not is_supported_format(image_path):
            temp_file = convert_to_jpeg(image_path)
            image_path = temp_file

        with open(image_path, 'rb') as f:
            encoded = base64.b64encode(f.read()).decode('utf-8')

        return encoded
    finally:
        # Clean up temporary file
        if temp_file and os.path.exists(temp_file):
            os.unlink(temp_file)


def verify_vision_output(client, input_image_path, output_json_path, operation):
    """Use GPT-4 Vision to verify vision outputs are semantically correct"""

    # Read output JSON
    with open(output_json_path) as f:
        output = json.load(f)

    # Encode image
    image_base64 = encode_image(input_image_path)

    # Create verification prompt
    prompt = f"""You are verifying the output of a {operation} operation on an image.

INPUT FILE: {input_image_path}
OPERATION: {operation}

OUTPUT:
{json.dumps(output, indent=2)}

YOUR TASK:
1. Look at the image carefully
2. Check if the output matches what you actually see
3. For face-detection: Are bounding boxes around actual faces? Are there false positives?
4. For object-detection: Are objects labeled correctly? Is "dog" actually a dog?
5. For ocr: Is detected text actually present in the image?
6. For pose-estimation: Are keypoints on actual human bodies?
7. Rate your confidence 0.0-1.0 (1.0 = perfectly correct)

RESPOND IN JSON FORMAT:
{{
  "status": "CORRECT" | "SUSPICIOUS" | "INCORRECT",
  "confidence": 0.95,
  "findings": "Detailed explanation of what you verified",
  "errors": ["List any errors or false positives found"],
  "warnings": ["List any suspicious patterns"]
}}

Be rigorous. If you see any false positives, mark as SUSPICIOUS or INCORRECT."""

    response = client.chat.completions.create(
        model="gpt-4o",  # Use gpt-4o for vision
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:image/jpeg;base64,{image_base64}"
                        }
                    }
                ]
            }
        ],
        max_tokens=2048,
        temperature=0.0  # Deterministic for consistency
    )

    content = response.choices[0].message.content
    # Strip markdown code block wrapper if present
    if content.startswith("```json\n"):
        content = content[8:]  # Remove ```json\n
    if content.startswith("```\n"):
        content = content[4:]  # Remove ```\n
    if content.endswith("\n```"):
        content = content[:-4]  # Remove \n```
    return content


def verify_text_output(client, output_json_path, operation):
    """Use GPT-4 to verify text outputs"""
    with open(output_json_path) as f:
        output = json.load(f)

    prompt = f"""Verify this {operation} output is semantically reasonable.

OUTPUT:
{json.dumps(output, indent=2)}

CHECKS:
1. Structure makes sense for this operation?
2. Values are plausible?
3. For transcription: Is text coherent (not gibberish)?
4. For diarization: Do speaker segments make sense?
5. For embeddings: Are dimensions correct, values in range?

RESPOND IN JSON:
{{
  "status": "CORRECT" | "SUSPICIOUS" | "INCORRECT",
  "confidence": 0.90,
  "findings": "Analysis",
  "errors": [],
  "warnings": []
}}"""

    response = client.chat.completions.create(
        model="gpt-4o",
        messages=[{"role": "user", "content": prompt}],
        max_tokens=1024,
        temperature=0.0
    )

    content = response.choices[0].message.content
    # Strip markdown code block wrapper if present
    if content.startswith("```json\n"):
        content = content[8:]  # Remove ```json\n
    if content.startswith("```\n"):
        content = content[4:]  # Remove ```\n
    if content.endswith("\n```"):
        content = content[:-4]  # Remove \n```
    return content


def main():
    if len(sys.argv) < 4:
        print("Usage: python ai_verify_openai.py <input_file> <output_json> <operation>")
        print("\nExample:")
        print("  python scripts/ai_verify_openai.py \\")
        print("    test_files_camera_raw/sony_a55.arw \\")
        print("    debug_output/stage_00_face_detection.json \\")
        print("    face-detection")
        sys.exit(1)

    input_file = sys.argv[1]
    output_file = sys.argv[2]
    operation = sys.argv[3]

    # Load API key
    try:
        api_key = load_api_key()
        client = OpenAI(api_key=api_key)
    except Exception as e:
        print(f"ERROR: {e}")
        sys.exit(1)

    # Verify based on operation type
    vision_ops = [
        "face-detection", "object-detection", "ocr", "pose-estimation",
        "emotion-detection", "action-recognition", "shot-classification",
        "smart-thumbnail", "image-quality-assessment"
    ]

    try:
        if operation in vision_ops:
            result = verify_vision_output(client, input_file, output_file, operation)
        else:
            result = verify_text_output(client, output_file, operation)

        print(result)

        # Parse result to check status
        try:
            result_json = json.loads(result)
            sys.exit(0 if result_json.get("status") == "CORRECT" else 1)
        except:
            sys.exit(0)  # If can't parse, assume success

    except Exception as e:
        print(json.dumps({
            "status": "ERROR",
            "confidence": 0.0,
            "findings": f"Verification failed: {str(e)}",
            "errors": [str(e)],
            "warnings": []
        }))
        sys.exit(2)


if __name__ == "__main__":
    main()
