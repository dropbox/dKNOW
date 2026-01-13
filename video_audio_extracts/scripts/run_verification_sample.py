#!/usr/bin/env python3
"""Run AI Verification on a Sample of Test Outputs

Creates a targeted sample of test outputs and verifies them with GPT-4 Vision.
Generates a comprehensive verification report.
"""

import subprocess
import json
import time
from pathlib import Path
from datetime import datetime

# Test cases to verify (carefully selected sample)
TEST_CASES = [
    # Object Detection
    ("test_files_objects_challenging/baboon.jpg", "object-detection", "Challenging: Baboon (non-standard object)"),
    ("test_files_objects_challenging/fruits.jpg", "object-detection", "Challenging: Multiple fruits"),
    ("test_files_objects_challenging/lena_opencv.jpg", "object-detection", "Standard: OpenCV test image"),

    # RAW Formats - Object Detection
    ("test_files_camera_raw/sony_a55.arw", "object-detection", "RAW: Sony ARW format"),
    ("test_files_camera_raw/canon_400d.cr2", "object-detection", "RAW: Canon CR2 format"),
    ("test_files_camera_raw/nikon_d80.nef", "object-detection", "RAW: Nikon NEF format"),

    # Different operations on same file
    ("test_files_camera_raw/sony_a55.arw", "ocr", "RAW: OCR on Sony ARW"),
    ("test_files_camera_raw/sony_a55.arw", "pose-estimation", "RAW: Pose estimation on Sony ARW"),
]

def run_extraction(input_file, operation):
    """Run video-extract debug mode on a test file"""
    cmd = [
        "./target/release/video-extract",
        "debug",
        "--ops", operation,
        input_file
    ]

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60,
            env={
                "PATH": f"{Path.home()}/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin",
                "PKG_CONFIG_PATH": "/opt/homebrew/lib/pkgconfig:/opt/homebrew/opt/ffmpeg/lib/pkgconfig"
            }
        )
        return result.returncode == 0
    except subprocess.TimeoutExpired:
        print(f"  TIMEOUT running {operation} on {input_file}")
        return False
    except Exception as e:
        print(f"  ERROR running {operation} on {input_file}: {e}")
        return False

def get_output_file(operation):
    """Get output filename for an operation"""
    # Map operation names to output file names
    operation_map = {
        "object-detection": "stage_00_object_detection.json",
        "face-detection": "stage_00_face_detection.json",
        "ocr": "stage_00_ocr.json",
        "pose-estimation": "stage_00_pose_estimation.json",
        "emotion-detection": "stage_00_emotion_detection.json",
        "action-recognition": "stage_00_action_recognition.json",
    }

    # Handle pipeline operations (keyframes;X)
    if ";" in operation:
        operation = operation.split(";")[-1]

    return operation_map.get(operation, f"stage_00_{operation.replace('-', '_')}.json")

def run_verification(input_file, output_file, operation):
    """Run AI verification on test output"""
    cmd = [
        "python3",
        "scripts/ai_verify_openai.py",
        input_file,
        output_file,
        operation
    ]

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60
        )

        # Parse JSON result
        output = result.stdout.strip()
        try:
            verification_result = json.loads(output)
            return verification_result
        except json.JSONDecodeError:
            return {
                "status": "ERROR",
                "confidence": 0.0,
                "findings": f"Failed to parse verification result: {output}",
                "errors": ["JSON parse error"],
                "warnings": []
            }
    except subprocess.TimeoutExpired:
        return {
            "status": "ERROR",
            "confidence": 0.0,
            "findings": "Verification timed out",
            "errors": ["Timeout"],
            "warnings": []
        }
    except Exception as e:
        return {
            "status": "ERROR",
            "confidence": 0.0,
            "findings": f"Verification failed: {str(e)}",
            "errors": [str(e)],
            "warnings": []
        }

def main():
    print("=" * 80)
    print("AI VERIFICATION SAMPLE - N=156")
    print("=" * 80)
    print(f"Start time: {datetime.now().isoformat()}")
    print(f"Total test cases: {len(TEST_CASES)}")
    print()

    results = []
    stats = {
        "total": 0,
        "correct": 0,
        "suspicious": 0,
        "incorrect": 0,
        "error": 0
    }

    for idx, (input_file, operation, description) in enumerate(TEST_CASES, 1):
        print(f"[{idx}/{len(TEST_CASES)}] {description}")
        print(f"  Input: {input_file}")
        print(f"  Operation: {operation}")

        # Check if input file exists
        if not Path(input_file).exists():
            print(f"  SKIP: Input file not found")
            print()
            continue

        # Run extraction
        print(f"  Running extraction...")
        if not run_extraction(input_file, operation):
            print(f"  ERROR: Extraction failed")
            stats["error"] += 1
            stats["total"] += 1
            results.append({
                "test_number": idx,
                "description": description,
                "input_file": input_file,
                "operation": operation,
                "status": "ERROR",
                "confidence": 0.0,
                "findings": "Extraction failed",
                "errors": ["Extraction command failed"],
                "warnings": []
            })
            print()
            continue

        # Get output file path
        output_file = f"debug_output/{get_output_file(operation)}"
        if not Path(output_file).exists():
            print(f"  ERROR: Output file not found: {output_file}")
            stats["error"] += 1
            stats["total"] += 1
            results.append({
                "test_number": idx,
                "description": description,
                "input_file": input_file,
                "operation": operation,
                "status": "ERROR",
                "confidence": 0.0,
                "findings": "Output file not found",
                "errors": ["Output file not found"],
                "warnings": []
            })
            print()
            continue

        # Run verification
        print(f"  Verifying with GPT-4 Vision...")
        verification = run_verification(input_file, output_file, operation)

        status = verification.get("status", "ERROR")
        confidence = verification.get("confidence", 0.0)

        print(f"  Result: {status} (confidence: {confidence})")

        # Update stats
        stats["total"] += 1
        if status == "CORRECT":
            stats["correct"] += 1
        elif status == "SUSPICIOUS":
            stats["suspicious"] += 1
        elif status == "INCORRECT":
            stats["incorrect"] += 1
        else:
            stats["error"] += 1

        # Store result
        results.append({
            "test_number": idx,
            "description": description,
            "input_file": input_file,
            "operation": operation,
            **verification
        })

        print()

        # Rate limit: 20 second delay between API calls
        if idx < len(TEST_CASES):
            print("  Waiting 20s (API rate limit)...")
            time.sleep(20)

    # Generate report
    print("=" * 80)
    print("GENERATING REPORT")
    print("=" * 80)

    success_rate = 100.0 * stats["correct"] / stats["total"] if stats["total"] > 0 else 0.0

    report = f"""# AI Verification Report - N=156

**Date:** {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}
**Branch:** main
**Commit:** N=156
**MANAGER Directive:** MANAGER_FINAL_DIRECTIVE_100_PERCENT.md (Objective 3)

---

## Executive Summary

AI verification using GPT-4 Vision on {stats["total"]} carefully selected test cases.

**Success Rate:** {success_rate:.1f}% ({stats["correct"]}/{stats["total"]} CORRECT)

| Status | Count | Percentage |
|--------|-------|------------|
| ✅ CORRECT | {stats["correct"]} | {100.0 * stats["correct"] / stats["total"]:.1f}% |
| ⚠️ SUSPICIOUS | {stats["suspicious"]} | {100.0 * stats["suspicious"] / stats["total"]:.1f}% |
| ❌ INCORRECT | {stats["incorrect"]} | {100.0 * stats["incorrect"] / stats["total"]:.1f}% |
| 🔴 ERROR | {stats["error"]} | {100.0 * stats["error"] / stats["total"]:.1f}% |
| **TOTAL** | **{stats["total"]}** | **100%** |

---

## Key Findings

"""

    # Add findings section
    if stats["incorrect"] > 0 or stats["suspicious"] > 0:
        report += "### Issues Identified\n\n"
        for result in results:
            if result["status"] in ["INCORRECT", "SUSPICIOUS"]:
                report += f"**{result['description']}**\n"
                report += f"- Status: {result['status']}\n"
                report += f"- Input: `{result['input_file']}`\n"
                report += f"- Operation: {result['operation']}\n"
                report += f"- Findings: {result['findings']}\n"
                if result.get("errors"):
                    report += f"- Errors: {', '.join(result['errors'])}\n"
                report += "\n"

    report += """---

## Detailed Results

"""

    # Add detailed results
    for result in results:
        status_emoji = {
            "CORRECT": "✅",
            "SUSPICIOUS": "⚠️",
            "INCORRECT": "❌",
            "ERROR": "🔴"
        }.get(result["status"], "❓")

        report += f"""### Test #{result['test_number']}: {result['description']}

{status_emoji} **Status:** {result['status']} (Confidence: {result['confidence']})

- **Input:** `{result['input_file']}`
- **Operation:** `{result['operation']}`
- **Findings:** {result['findings']}

"""

        if result.get("errors"):
            report += f"**Errors:**\n"
            for error in result["errors"]:
                report += f"- {error}\n"
            report += "\n"

        if result.get("warnings"):
            report += f"**Warnings:**\n"
            for warning in result["warnings"]:
                report += f"- {warning}\n"
            report += "\n"

        report += "---\n\n"

    # Add conclusion
    report += f"""## Conclusion

**Verification Status:** {"✅ PASSED" if success_rate >= 85.0 else "⚠️ NEEDS IMPROVEMENT"}

"""

    if success_rate >= 85.0:
        report += f"System achieves {success_rate:.1f}% success rate, exceeding the 85% target. "
        report += "Test outputs are semantically correct and meet quality standards.\n\n"
    else:
        report += f"System achieves {success_rate:.1f}% success rate, below the 85% target. "
        report += "Issues identified require investigation and potential fixes.\n\n"

    if stats["incorrect"] > 0:
        report += f"**Action Required:** {stats['incorrect']} test(s) returned INCORRECT results. "
        report += "These represent real accuracy issues that should be investigated.\n\n"

    if stats["suspicious"] > 0:
        report += f"**Review Recommended:** {stats['suspicious']} test(s) returned SUSPICIOUS results. "
        report += "These may indicate edge cases or model limitations.\n\n"

    report += """---

## Methodology

- **Sample Size:** Carefully selected challenging test cases
- **Verification Tool:** GPT-4 Vision (gpt-4o model)
- **Temperature:** 0.0 (deterministic)
- **API Rate Limit:** 20 second delay between requests
- **Focus:** RAW formats, challenging images, diverse operations

---

**Next Steps:**
1. Review INCORRECT cases for potential model issues
2. Investigate SUSPICIOUS cases for edge case handling
3. Consider expanding verification to additional test cases
4. Document known limitations (e.g., COCO dataset class constraints)
"""

    # Save report
    report_path = "AI_VERIFICATION_REPORT.md"
    with open(report_path, 'w') as f:
        f.write(report)

    print(f"Report saved to: {report_path}")
    print()
    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"Total tests: {stats['total']}")
    print(f"CORRECT: {stats['correct']} ({100.0 * stats['correct'] / stats['total']:.1f}%)")
    print(f"SUSPICIOUS: {stats['suspicious']} ({100.0 * stats['suspicious'] / stats['total']:.1f}%)")
    print(f"INCORRECT: {stats['incorrect']} ({100.0 * stats['incorrect'] / stats['total']:.1f}%)")
    print(f"ERROR: {stats['error']} ({100.0 * stats['error'] / stats['total']:.1f}%)")
    print(f"Success Rate: {success_rate:.1f}%")
    print()

    if success_rate >= 85.0:
        print("✅ PASSED: Success rate meets 85% target")
    else:
        print("⚠️ NEEDS IMPROVEMENT: Success rate below 85% target")

    print("=" * 80)

if __name__ == "__main__":
    main()
