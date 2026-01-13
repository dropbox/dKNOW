#!/usr/bin/env python3
"""
LLM Quality Testing Script for Docling Formats
Tests a format by converting a sample file and evaluating quality with GPT-4
"""

import sys
import os
import subprocess
import json
from openai import OpenAI
from pathlib import Path

# Load .env file if it exists
env_file = Path(__file__).parent / ".env"
if env_file.exists():
    with open(env_file) as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith('#') and '=' in line:
                key, value = line.split('=', 1)
                os.environ[key] = value

def run_docling_conversion(input_file):
    """Run docling binary on input file and capture markdown output"""
    # Use pre-built binary with proper environment setup
    env = os.environ.copy()
    cargo_bin = os.path.expanduser("~/.cargo/bin")
    env["PATH"] = f"{cargo_bin}:{env.get('PATH', '')}"
    env["DYLD_LIBRARY_PATH"] = f"{os.path.expanduser('~/.local/lib')}:{env.get('DYLD_LIBRARY_PATH', '')}"

    binary_path = "/Users/ayates/docling_rs/target/release/docling"

    result = subprocess.run(
        [binary_path, "convert", "--backend", "rust", input_file],
        capture_output=True,
        text=True,
        cwd="/Users/ayates/docling_rs",
        env=env
    )

    if result.returncode != 0:
        print(f"Error running docling: {result.stderr}")
        return None

    return result.stdout

def evaluate_quality(input_file, markdown_output, format_type):
    """Use GPT-4 to evaluate the quality of the markdown output"""

    # Load API key from environment
    api_key = os.getenv("OPENAI_API_KEY")
    if not api_key:
        print("Error: OPENAI_API_KEY not set")
        return None

    client = OpenAI(api_key=api_key)

    # Create evaluation prompt
    prompt = f"""You are evaluating the quality of a document conversion from {format_type.upper()} to Markdown.

Input file: {os.path.basename(input_file)}
Format: {format_type.upper()}

Converted Markdown Output (total length: {len(markdown_output)} chars):
```markdown
{markdown_output[:30000]}
```
{f'(Showing first 30000 of {len(markdown_output)} chars)' if len(markdown_output) > 30000 else ''}

Please evaluate this conversion on a scale of 0-100% based on:
1. **Completeness** (30%): Is text content preserved? Are sections complete?
   Note: Figures/images cannot be embedded in markdown - this is expected and not a defect.
2. **Accuracy** (30%): Is the content correct? No errors or corruptions?
3. **Structure** (20%): Does it maintain logical document structure with proper headings?
4. **Formatting** (20%): Is markdown formatting appropriate and readable?
   Note: Some DOI/URL formatting differences are acceptable.

Provide your response as JSON:
{{
  "score": <0-100>,
  "completeness": <0-100>,
  "accuracy": <0-100>,
  "structure": <0-100>,
  "formatting": <0-100>,
  "strengths": ["list", "of", "strengths"],
  "weaknesses": ["list", "of", "issues"],
  "recommendation": "brief recommendation"
}}

Be objective and constructive. Focus on concrete issues."""

    try:
        response = client.chat.completions.create(
            model="gpt-4o",
            messages=[
                {"role": "system", "content": "You are a document quality evaluator. Provide objective, detailed evaluations in JSON format."},
                {"role": "user", "content": prompt}
            ],
            temperature=0.3,  # Lower temperature for more consistent evaluations
        )

        # Parse JSON response
        response_text = response.choices[0].message.content
        # Extract JSON from markdown code blocks if present
        if "```json" in response_text:
            response_text = response_text.split("```json")[1].split("```")[0]
        elif "```" in response_text:
            response_text = response_text.split("```")[1].split("```")[0]

        result = json.loads(response_text.strip())
        return result

    except Exception as e:
        print(f"Error calling OpenAI API: {e}")
        return None

def main():
    if len(sys.argv) < 2:
        print("Usage: python test_format_quality.py <input_file>")
        print("Example: python test_format_quality.py test-corpus/jats/elife_sample_02.nxml")
        sys.exit(1)

    input_file = sys.argv[1]

    # Detect format from extension
    ext = os.path.splitext(input_file)[1].lower()
    format_map = {
        '.nxml': 'jats',
        '.ipynb': 'ipynb',
        '.kml': 'kml',
        '.kmz': 'kmz',
        '.ics': 'ics',
        '.epub': 'epub',
        '.bmp': 'bmp',
        '.gif': 'gif',
        '.heic': 'heif',
        '.heif': 'heif',
        '.avif': 'avif',
    }

    format_type = format_map.get(ext, ext[1:])

    print(f"Testing {format_type.upper()} format quality...")
    print(f"Input: {input_file}")
    print()

    # Step 1: Convert with docling
    print("Step 1: Running docling conversion...")
    markdown = run_docling_conversion(input_file)
    if markdown is None:
        print("Conversion failed!")
        sys.exit(1)

    print(f"Output length: {len(markdown)} chars")
    print()

    # Step 2: Evaluate quality with LLM
    print("Step 2: Evaluating quality with GPT-4...")
    evaluation = evaluate_quality(input_file, markdown, format_type)
    if evaluation is None:
        print("Evaluation failed!")
        sys.exit(1)

    # Step 3: Display results
    print()
    print("=" * 70)
    print(f"QUALITY EVALUATION RESULTS - {format_type.upper()}")
    print("=" * 70)
    print()
    print(f"Overall Score: {evaluation['score']}%")
    print()
    print("Component Scores:")
    print(f"  Completeness: {evaluation['completeness']}%")
    print(f"  Accuracy:     {evaluation['accuracy']}%")
    print(f"  Structure:    {evaluation['structure']}%")
    print(f"  Formatting:   {evaluation['formatting']}%")
    print()

    if evaluation.get('strengths'):
        print("Strengths:")
        for s in evaluation['strengths']:
            print(f"  ✓ {s}")
        print()

    if evaluation.get('weaknesses'):
        print("Weaknesses:")
        for w in evaluation['weaknesses']:
            print(f"  ✗ {w}")
        print()

    if evaluation.get('recommendation'):
        print(f"Recommendation: {evaluation['recommendation']}")
    print()

    # Pass/Fail
    if evaluation['score'] >= 95:
        print("✅ PASS (≥95%)")
    else:
        print(f"❌ FAIL (need {95 - evaluation['score']}% more to pass)")

    print("=" * 70)

if __name__ == "__main__":
    main()
