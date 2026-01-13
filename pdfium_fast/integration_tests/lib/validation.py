"""
Validation Utilities

Provides validation methods:
- Command execution
- Edit distance (Levenshtein)
- Text similarity
- JSONL field analysis
- Pixel-level comparison
- LLM-powered analysis (optional)
"""

import os
import json
import hashlib
import subprocess
from pathlib import Path
from typing import Tuple, Optional, List


def run_command(cmd: List[str], timeout: int = 300) -> subprocess.CompletedProcess:
    """
    Execute a command with timeout and capture output.

    Args:
        cmd: Command and arguments as list
        timeout: Timeout in seconds (default: 300)

    Returns:
        CompletedProcess with returncode, stdout, stderr
    """
    return subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=timeout
    )


def calculate_edit_distance(str1: str, str2: str, max_diff_per_page=50) -> int:
    """
    Calculate Levenshtein edit distance with performance optimization.

    Uses word count difference as fast pre-check. If files differ significantly,
    returns -1 (indicating "too different") to avoid expensive O(N×M) computation.

    Args:
        str1: First string
        str2: Second string
        max_diff_per_page: Max word count difference per page before skipping (default: 50)

    Returns:
        int: Edit distance, or -1 if files differ significantly (skip expensive calculation)
    """
    if str1 == str2:
        return 0

    # Fast pre-check: word count difference
    # If word counts differ significantly, files are different enough - skip expensive DP
    words1 = str1.split()
    words2 = str2.split()
    word_diff = abs(len(words1) - len(words2))

    # Estimate page count from text length (rough heuristic: ~2000 chars/page)
    estimated_pages = max(len(str1), len(str2)) / 2000
    if estimated_pages < 1:
        estimated_pages = 1

    word_diff_per_page = word_diff / estimated_pages

    if word_diff_per_page > max_diff_per_page:
        # Files differ significantly - return -1 to signal "too different to measure"
        return -1

    # Additional check: if strings very large and word diff significant, skip
    if (len(str1) > 100000 or len(str2) > 100000) and word_diff > 1000:
        return -1

    # Standard DP for small differences
    len1, len2 = len(str1), len(str2)
    dp = [[0] * (len2 + 1) for _ in range(len1 + 1)]

    for i in range(len1 + 1):
        dp[i][0] = i
    for j in range(len2 + 1):
        dp[0][j] = j

    for i in range(1, len1 + 1):
        for j in range(1, len2 + 1):
            if str1[i-1] == str2[j-1]:
                dp[i][j] = dp[i-1][j-1]
            else:
                dp[i][j] = 1 + min(dp[i-1][j], dp[i][j-1], dp[i-1][j-1])

    return dp[len1][len2]


def calculate_similarity(str1: str, str2: str) -> float:
    """Calculate similarity ratio (0.0-1.0)."""
    if str1 == str2:
        return 1.0

    import difflib
    return difflib.SequenceMatcher(None, str1, str2).ratio()


def calculate_image_md5(image_path: Path) -> str:
    """Calculate MD5 hash of image."""
    md5 = hashlib.md5()
    with open(image_path, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b''):
            md5.update(chunk)
    return md5.hexdigest()


def compare_images_pixel_level(expected_path: Path, actual_path: Path) -> Tuple[int, float]:
    """
    Compare images pixel-by-pixel.

    Returns: (pixel_diff_count, pixel_diff_percentage)
    """
    try:
        from PIL import Image
        import numpy as np

        img1 = Image.open(expected_path).convert('RGB')
        img2 = Image.open(actual_path).convert('RGB')

        if img1.size != img2.size:
            return -1, -1.0  # Different dimensions

        arr1 = np.array(img1)
        arr2 = np.array(img2)

        diff = np.any(arr1 != arr2, axis=-1)
        pixel_diff_count = np.sum(diff)
        total_pixels = arr1.shape[0] * arr1.shape[1]
        pixel_diff_pct = (pixel_diff_count / total_pixels) * 100

        return int(pixel_diff_count), float(pixel_diff_pct)
    except ImportError:
        return -2, -2.0  # PIL not available
    except Exception:
        return -3, -3.0  # Error


def analyze_text_with_llm(expected: str, actual: str, diff_lines: list) -> Optional[str]:
    """
    Use OpenAI to analyze text differences.

    Only called when there ARE differences (cost optimization).
    """
    try:
        import openai
    except ImportError:
        return None

    # Get API key
    api_key = os.environ.get('OPENAI_API_KEY')
    if not api_key:
        key_file = Path(__file__).parent.parent.parent / 'openai_api_key.txt'
        if key_file.exists():
            api_key = key_file.read_text().strip()

    if not api_key:
        return None

    # Prepare diff context (limit size)
    diff_context = "".join(diff_lines[:200])
    if len(diff_context) > 10000:
        diff_context = diff_context[:10000] + "\n... [truncated]"

    prompt = f"""Analyze PDF text extraction diff:

Expected: {len(expected)} chars
Actual: {len(actual)} chars

Diff:
{diff_context}

Identify:
1. Error types (missing spaces, extra spaces, transpositions)
2. Patterns (font-specific, line-break issues, Unicode)
3. Root causes
4. Fix recommendations

Be concise and technical."""

    try:
        client = openai.OpenAI(api_key=api_key)
        response = client.chat.completions.create(
            model="gpt-4o-mini",
            messages=[
                {"role": "system", "content": "Expert in PDF text extraction debugging."},
                {"role": "user", "content": prompt}
            ],
            max_tokens=800,
            temperature=0.3
        )

        return response.choices[0].message.content
    except Exception as e:
        return f"LLM analysis failed: {e}"


def analyze_image_with_llm(expected_path: Path, actual_path: Path) -> Optional[str]:
    """
    Use OpenAI Vision to analyze image differences.

    Only called when MD5s differ (cost optimization).
    """
    try:
        import openai
        import base64
    except ImportError:
        return None

    # Get API key
    api_key = os.environ.get('OPENAI_API_KEY')
    if not api_key:
        key_file = Path(__file__).parent.parent.parent / 'openai_api_key.txt'
        if key_file.exists():
            api_key = key_file.read_text().strip()

    if not api_key:
        return None

    # Encode images
    try:
        with open(expected_path, 'rb') as f:
            exp_b64 = base64.b64encode(f.read()).decode('utf-8')
        with open(actual_path, 'rb') as f:
            act_b64 = base64.b64encode(f.read()).decode('utf-8')
    except Exception as e:
        return f"Image encoding failed: {e}"

    prompt = """Compare these PDF page renderings:

Image 1: Expected (baseline)
Image 2: Actual (parallel rendering)

Identify visible differences, severity, and causes. Be specific."""

    try:
        client = openai.OpenAI(api_key=api_key)
        response = client.chat.completions.create(
            model="gpt-4o",
            messages=[{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {"url": f"data:image/png;base64,{exp_b64}"}},
                    {"type": "image_url", "image_url": {"url": f"data:image/png;base64,{act_b64}"}},
                ]
            }],
            max_tokens=800,
            temperature=0.3
        )

        return response.choices[0].message.content
    except Exception as e:
        return f"LLM vision failed: {e}"
