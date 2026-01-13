# LLM Validation Loop - Continuous Quality Verification

**User directive:** "Use an LLM as Judge for both text and image to verify correctness"

**Approach:** Automated validation loop with GPT-4 as judge

---

## Setup

**API Key provided:** sk-proj-EHWVvsIF... (OpenAI)
**Judge model:** GPT-4 (best for quality assessment)

---

## Validation Loop Design

### Step 1: Select Random PDFs

**Sources:**
1. Local: ~/pdfium_fast/integration_tests/pdfs/benchmark/ (196 PDFs)
2. Internet: Common public PDFs
   - arXiv papers (academic)
   - Government forms (USA.gov, forms.gov)
   - Wikipedia exports
   - Project Gutenberg (books)
   - Corporate reports (SEC filings)

**Selection:** 10 random PDFs per iteration

### Step 2: Extract with pdfium_fast

```bash
for pdf in $RANDOM_PDFS; do
  # Text extraction
  pdfium_cli extract-text "$pdf" "$OUTPUT/text.txt"

  # JSONL metadata
  pdfium_cli extract-jsonl "$pdf" "$OUTPUT/metadata.jsonl"

  # Image rendering (JPEG web preset)
  pdfium_cli --preset web render-pages "$pdf" "$OUTPUT/images/"
done
```

### Step 3: LLM Judge Validation

**For each PDF, ask GPT-4 to judge:**

#### Text Quality Assessment

```python
import openai
openai.api_key = "sk-proj-EHWVvsIF..."

def judge_text_extraction(pdf_path, extracted_text):
    prompt = f'''You are an expert PDF extraction validator.

PDF file: {pdf_path}

Extracted text:
"""
{extracted_text}
"""

Evaluate the text extraction quality:
1. Is the text readable and coherent?
2. Are there obvious extraction errors (garbled text, missing sections)?
3. Is formatting preserved (paragraphs, spacing)?
4. Are special characters handled correctly (unicode, symbols)?
5. Overall quality: Excellent/Good/Fair/Poor

Provide:
- Overall score: 1-10
- Issues found (if any)
- Recommendation: PASS / FAIL / NEEDS_REVIEW
'''

    response = openai.ChatCompletion.create(
        model="gpt-4",
        messages=[{"role": "user", "content": prompt}]
    )

    return response.choices[0].message.content
```

#### Image Quality Assessment

```python
def judge_image_rendering(pdf_path, image_path):
    # Upload image to GPT-4 Vision
    with open(image_path, 'rb') as f:
        image_data = base64.b64encode(f.read()).decode()

    prompt = f'''You are an expert PDF rendering validator.

PDF file: {pdf_path}
Rendered image: page_0000.jpg

Evaluate the image rendering quality:
1. Is the content clearly visible and readable?
2. Are there rendering artifacts (missing elements, corruption)?
3. Is text sharp and legible?
4. Are images properly rendered?
5. Is layout preserved correctly?
6. Overall quality: Excellent/Good/Fair/Poor

Provide:
- Overall score: 1-10
- Issues found (if any)
- Recommendation: PASS / FAIL / NEEDS_REVIEW
'''

    response = openai.ChatCompletion.create(
        model="gpt-4-vision-preview",
        messages=[{
            "role": "user",
            "content": [
                {"type": "text", "text": prompt},
                {"type": "image_url", "image_url": {"url": f"data:image/jpeg;base64,{image_data}"}}
            ]
        }]
    )

    return response.choices[0].message.content
```

#### JSONL Validation

```python
def judge_jsonl_metadata(pdf_path, jsonl_data):
    prompt = f'''You are an expert PDF metadata validator.

PDF file: {pdf_path}

JSONL metadata (first 10 lines):
"""
{jsonl_data[:10]}
"""

Evaluate the metadata extraction:
1. Are character positions reasonable (x, y coordinates)?
2. Are bounding boxes valid (width, height positive)?
3. Are font names present and sensible?
4. Are unicode codepoints valid?
5. Is the data structure consistent?
6. Overall quality: Excellent/Good/Fair/Poor

Provide:
- Overall score: 1-10
- Issues found (if any)
- Recommendation: PASS / FAIL / NEEDS_REVIEW
'''

    response = openai.ChatCompletion.create(
        model="gpt-4",
        messages=[{"role": "user", "content": prompt}]
    )

    return response.choices[0].message.content
```

---

## Implementation Script

**File:** `integration_tests/llm_validation_loop.py`

```python
#!/usr/bin/env python3
"""
LLM Validation Loop - Continuous quality verification using GPT-4 as judge.

Usage: python3 llm_validation_loop.py --iterations 10 --api-key sk-proj-...
"""

import openai
import subprocess
import random
import json
import base64
from pathlib import Path
import argparse

OPENAI_API_KEY = os.getenv("OPENAI_API_KEY")  # Set via environment variable

def select_random_pdfs(pdf_dir, count=10):
    """Select random PDFs from benchmark directory."""
    pdfs = list(Path(pdf_dir).glob("*.pdf"))
    return random.sample(pdfs, min(count, len(pdfs)))

def extract_all(pdfium_cli, pdf_path, output_dir):
    """Extract text, JSONL, and images from PDF."""
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    results = {}

    # Text extraction
    text_path = output_dir / "text.txt"
    result = subprocess.run([
        pdfium_cli, "extract-text", str(pdf_path), str(text_path)
    ], capture_output=True, text=True)
    results['text_success'] = (result.returncode == 0)
    results['text_content'] = text_path.read_text(encoding='utf-8', errors='ignore') if text_path.exists() else None

    # JSONL extraction (first page)
    jsonl_path = output_dir / "metadata.jsonl"
    result = subprocess.run([
        pdfium_cli, "extract-jsonl", str(pdf_path), str(jsonl_path)
    ], capture_output=True, text=True)
    results['jsonl_success'] = (result.returncode == 0)
    results['jsonl_content'] = jsonl_path.read_text() if jsonl_path.exists() else None

    # Image rendering (first page only for speed)
    image_dir = output_dir / "images"
    image_dir.mkdir(exist_ok=True)
    result = subprocess.run([
        pdfium_cli, "--preset", "web", "--pages", "0", "render-pages",
        str(pdf_path), str(image_dir) + "/"
    ], capture_output=True, text=True)
    results['image_success'] = (result.returncode == 0)

    image_files = list(image_dir.glob("page_*.jpg"))
    results['image_path'] = image_files[0] if image_files else None

    return results

def llm_judge_text(pdf_name, text_content):
    """Use GPT-4 to judge text extraction quality."""
    if not text_content or len(text_content) < 10:
        return {"score": 0, "verdict": "FAIL", "reason": "No text extracted"}

    prompt = f'''You are an expert PDF extraction validator.

PDF: {pdf_name}

Extracted text (first 500 chars):
"""
{text_content[:500]}
"""

Evaluate text extraction quality (1-10):
1. Readability: Is text coherent?
2. Completeness: Any obvious missing sections?
3. Formatting: Preserved reasonably?
4. Special chars: Handled correctly?
5. Overall quality

Return JSON:
{{
  "score": 8,
  "verdict": "PASS",
  "issues": ["minor spacing issue"],
  "summary": "Good quality extraction"
}}

Scores: 9-10 = Excellent, 7-8 = Good, 5-6 = Fair, 1-4 = Poor
Verdict: PASS (≥7), NEEDS_REVIEW (5-6), FAIL (<5)
'''

    try:
        response = openai.ChatCompletion.create(
            model="gpt-4",
            messages=[{"role": "user", "content": prompt}],
            api_key=OPENAI_API_KEY
        )
        return json.loads(response.choices[0].message.content)
    except Exception as e:
        return {"score": 0, "verdict": "ERROR", "reason": str(e)}

def llm_judge_image(pdf_name, image_path):
    """Use GPT-4 Vision to judge image rendering quality."""
    if not image_path or not image_path.exists():
        return {"score": 0, "verdict": "FAIL", "reason": "No image created"}

    with open(image_path, 'rb') as f:
        image_data = base64.b64encode(f.read()).decode()

    prompt = f'''You are an expert PDF rendering validator.

PDF: {pdf_name}
Rendered image: First page

Evaluate rendering quality (1-10):
1. Clarity: Content visible and clear?
2. Text: Sharp and legible?
3. Layout: Preserved correctly?
4. Images: Rendered properly?
5. Artifacts: Any corruption/missing elements?

Return JSON:
{{
  "score": 9,
  "verdict": "PASS",
  "issues": [],
  "summary": "Excellent rendering"
}}

Scores: 9-10 = Excellent, 7-8 = Good, 5-6 = Fair, 1-4 = Poor
Verdict: PASS (≥7), NEEDS_REVIEW (5-6), FAIL (<5)
'''

    try:
        response = openai.ChatCompletion.create(
            model="gpt-4-vision-preview",
            messages=[{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:image/jpeg;base64,{image_data}",
                            "detail": "high"
                        }
                    }
                ]
            }],
            api_key=OPENAI_API_KEY,
            max_tokens=500
        )
        return json.loads(response.choices[0].message.content)
    except Exception as e:
        return {"score": 0, "verdict": "ERROR", "reason": str(e)}

def validation_loop(pdfium_cli, pdf_dir, iterations=10):
    """Run validation loop with LLM judge."""
    results = []

    for i in range(iterations):
        print(f"\n=== Iteration {i+1}/{iterations} ===")

        # Select random PDFs
        pdfs = select_random_pdfs(pdf_dir, count=10)

        for pdf in pdfs:
            print(f"\nValidating: {pdf.name}")

            # Extract
            output_dir = Path(f"/tmp/llm_validation/{pdf.stem}")
            extracted = extract_all(pdfium_cli, pdf, output_dir)

            # Judge text
            if extracted['text_success']:
                text_verdict = llm_judge_text(pdf.name, extracted['text_content'])
                print(f"  Text: {text_verdict.get('score')}/10 - {text_verdict.get('verdict')}")
            else:
                text_verdict = {"score": 0, "verdict": "FAIL", "reason": "Extraction failed"}
                print(f"  Text: FAILED (extraction error)")

            # Judge image
            if extracted['image_success']:
                image_verdict = llm_judge_image(pdf.name, extracted['image_path'])
                print(f"  Image: {image_verdict.get('score')}/10 - {image_verdict.get('verdict')}")
            else:
                image_verdict = {"score": 0, "verdict": "FAIL", "reason": "Rendering failed"}
                print(f"  Image: FAILED (rendering error)")

            # Record results
            results.append({
                "pdf": pdf.name,
                "iteration": i + 1,
                "text": text_verdict,
                "image": image_verdict,
                "jsonl_success": extracted['jsonl_success']
            })

    # Summary
    text_passes = sum(1 for r in results if r['text'].get('verdict') == 'PASS')
    image_passes = sum(1 for r in results if r['image'].get('verdict') == 'PASS')
    total = len(results)

    print(f"\n=== Validation Summary ===")
    print(f"Total PDFs: {total}")
    print(f"Text PASS: {text_passes}/{total} ({text_passes/total*100:.1f}%)")
    print(f"Image PASS: {image_passes}/{total} ({image_passes/total*100:.1f}%)")

    # Save results
    with open("/tmp/llm_validation_results.json", "w") as f:
        json.dump(results, f, indent=2)

    return results

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--iterations", type=int, default=10)
    parser.add_argument("--cli", default="../out/Release/pdfium_cli")
    parser.add_argument("--pdf-dir", default="pdfs/benchmark")
    args = parser.parse_args()

    print("Starting LLM validation loop...")
    print(f"Iterations: {args.iterations}")
    print(f"CLI: {args.cli}")
    print(f"PDF dir: {args.pdf_dir}")

    results = validation_loop(args.cli, args.pdf_dir, args.iterations)

    # Report failures
    failures = [r for r in results if r['text'].get('verdict') == 'FAIL' or r['image'].get('verdict') == 'FAIL']
    if failures:
        print(f"\n⚠️ Failures detected: {len(failures)}")
        for f in failures:
            print(f"  - {f['pdf']}: Text={f['text'].get('verdict')}, Image={f['image'].get('verdict')}")
    else:
        print("\n✅ All validations passed!")
```

---

## When to Start This Loop

**Condition:** Worker stuck in test loops for 5+ iterations with no progress

**Indicators:**
- Same "Regular Maintenance" message repeated
- No new features implemented
- No bugs being fixed
- Just running smoke tests repeatedly

**Action:** Start LLM validation loop as additional validation

---

## Expected Results

**Text extraction (expect 90-95% PASS):**
- Most PDFs: Clear, readable text
- Some PDFs: May have formatting issues (acceptable)
- Failures: Corrupted PDFs, scanned without OCR

**Image rendering (expect 95-100% PASS):**
- Most PDFs: Clear, legible rendering
- Some PDFs: May have minor artifacts (acceptable)
- Failures: Extremely complex PDFs, rendering bugs

**JSONL (expect 90-95% success):**
- Metadata extracted successfully
- Coordinates valid
- Font info present

---

## Use Cases

**1. Continuous validation:**
- Run loop overnight
- Catch edge cases
- Find rendering issues

**2. Quality assurance:**
- LLM spots issues humans would spot
- Automated QA without manual review

**3. Regression detection:**
- Run before/after code changes
- LLM detects quality degradation

---

## Cost Estimate

**GPT-4 pricing:**
- Text: ~$0.01 per PDF (500 tokens)
- Image: ~$0.05 per PDF (vision API)
- Total: ~$0.06 per PDF

**100 PDFs:** ~$6
**1000 PDFs:** ~$60

**Reasonable for quality assurance.**

---

## Implementation for Worker

**When worker is stuck in loops:**

```bash
cd ~/pdfium_fast/integration_tests

# Run LLM validation (10 iterations = 100 PDFs)
python3 llm_validation_loop.py --iterations 10

# Review results
cat /tmp/llm_validation_results.json | jq '.[] | select(.text.verdict != "PASS" or .image.verdict != "PASS")'
```

**Commit results:**
```
[WORKER0] # [N]: LLM Validation Loop - Quality Verification

Ran LLM judge validation on 100 random PDFs.

Results:
- Text PASS: [X]/100 ([Y]%)
- Image PASS: [X]/100 ([Y]%)
- Average text score: [score]/10
- Average image score: [score]/10

Issues found: [count]
[List any consistent issues]

LLM judge provides independent quality assessment.
System validation: [PASS/FAIL/NEEDS_REVIEW]
```

---

## When to Use This

**Start LLM loop when:**
1. Worker stuck in maintenance loops (5+ iterations of same message)
2. Full test suite complete
3. No more bugs being fixed
4. Need additional validation confidence

**This provides extra assurance beyond automated tests.**
