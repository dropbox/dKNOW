#!/usr/bin/env python3
"""LLM-as-Judge evaluation for PDF extraction quality.

Evaluates extracted markdown against annotated page visualizations using OpenAI GPT-4o.
"""
import os
import sys
import json
import base64
import re
from pathlib import Path

# Check for API key
api_key = os.environ.get('OPENAI_API_KEY')
if not api_key:
    print("Error: OPENAI_API_KEY not set. Run: source .env")
    sys.exit(1)

try:
    import openai
except ImportError:
    print("Installing openai package...")
    os.system("pip install openai -q")
    import openai

client = openai.OpenAI(api_key=api_key)

def encode_image(image_path: str) -> str:
    """Encode image to base64."""
    with open(image_path, "rb") as f:
        return base64.b64encode(f.read()).decode("utf-8")

def extract_page_text(markdown_path: str, page_num: int) -> str:
    """Extract text for a specific page from markdown (uses page comments)."""
    content = Path(markdown_path).read_text()

    # Docling emits page markers using 1-indexed PDF page numbers:
    # `<!-- page 1 -->`, `<!-- page 2 -->`, ...
    target_page_no = page_num + 1

    lines = content.splitlines()
    marker_re = re.compile(r"^<!--\s*page\s+(\d+)\s*-->$")
    markers: list[tuple[int, int]] = []

    for idx, line in enumerate(lines):
        match = marker_re.match(line.strip())
        if match:
            markers.append((int(match.group(1)), idx))

    if markers:
        markers.sort(key=lambda x: x[1])
        start_idx: int | None = None
        end_idx = len(lines)

        for i, (page_no, idx) in enumerate(markers):
            if page_no == target_page_no:
                start_idx = idx + 1  # skip the marker line itself
                if i + 1 < len(markers):
                    end_idx = markers[i + 1][1]
                break

        if start_idx is not None:
            extracted = "\n".join(lines[start_idx:end_idx]).strip()
            if extracted:
                return extracted

    # If no page markers, return first ~2000 chars for page 0
    if "<!-- page " not in content and page_num == 0:
        return content[:2000]

    # Fallback: divide content roughly by page count (prefer marker-derived count if present)
    total_pages_guess = max((p for p, _ in markers), default=36)
    chars_per_page = max(1, len(content) // total_pages_guess)
    start = page_num * chars_per_page
    end = (page_num + 1) * chars_per_page
    return content[start:end]

def evaluate_page(image_path: str, extracted_text: str, page_num: int) -> dict:
    """Evaluate a single page using GPT-4o vision."""
    base64_image = encode_image(image_path)

    prompt = f"""You are evaluating PDF text extraction quality. Be ACCURATE - do not claim text is missing unless you've searched the extracted text carefully.

Compare the annotated PDF page image to the extracted text below.

EXTRACTED TEXT (Page {page_num}):
```
{extracted_text[:4000]}
```

CRITICAL INSTRUCTIONS:
- Before claiming ANY text is "missing", SEARCH the extracted text above carefully
- Text may appear in different order or formatting than the image - that's OK for completeness
- Only score completeness lower if text ACTUALLY does not appear anywhere above
- Minor formatting differences (extra spaces, different bullet style) are OK
- Reading order issues only matter if the flow is completely wrong

Evaluate on these criteria (1-10 scale):
1. COMPLETENESS (9-10 if all text present, even with format changes): Is the text content captured?
2. ACCURACY (9-10 if text correct even with minor OCR issues): Are words spelled correctly?
3. STRUCTURE (8-10 if headers/paragraphs recognizable): Is document structure preserved?
4. READING_ORDER (8-10 if generally readable top-to-bottom): Is the flow logical?

Return JSON only:
{{"completeness": N, "accuracy": N, "structure": N, "reading_order": N, "overall": N, "issues": "specific issues found - be precise about what is actually wrong"}}"""

    response = client.chat.completions.create(
        model="gpt-4o",
        messages=[
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:image/png;base64,{base64_image}",
                            "detail": "high"
                        }
                    }
                ]
            }
        ],
        max_tokens=500
    )

    result_text = response.choices[0].message.content
    # Extract JSON from response
    try:
        # Handle markdown code blocks
        if "```json" in result_text:
            result_text = result_text.split("```json")[1].split("```")[0]
        elif "```" in result_text:
            result_text = result_text.split("```")[1].split("```")[0]
        return json.loads(result_text.strip())
    except json.JSONDecodeError:
        return {"error": result_text, "overall": 5}

def main():
    viz_dir = Path("/tmp/viz_output")
    markdown_path = Path("/tmp/mamba_output.md")

    if not markdown_path.exists():
        print(f"Error: {markdown_path} not found. Run docling convert first.")
        sys.exit(1)

    # Sample pages: diverse selection across document
    sample_pages = [0, 1, 5, 10, 17, 25, 35]  # 0-indexed (7 pages)

    print(f"Evaluating {len(sample_pages)} pages using GPT-4o vision...")
    print("=" * 60)

    results = []
    for page_num in sample_pages:
        # Find image file (format: 2312.00752_page_000_reading.png)
        image_files = list(viz_dir.glob(f"*_page_{page_num:03d}_*.png"))
        if not image_files:
            print(f"Page {page_num}: No visualization found, skipping")
            continue

        image_path = image_files[0]
        page_text = extract_page_text(str(markdown_path), page_num)

        print(f"\nPage {page_num}: Evaluating...")
        result = evaluate_page(str(image_path), page_text, page_num)
        result['page'] = page_num
        results.append(result)

        if 'error' in result:
            print(f"  Error: {result['error'][:100]}")
        else:
            print(f"  Completeness: {result.get('completeness', 'N/A')}/10")
            print(f"  Accuracy:     {result.get('accuracy', 'N/A')}/10")
            print(f"  Structure:    {result.get('structure', 'N/A')}/10")
            print(f"  Reading Order:{result.get('reading_order', 'N/A')}/10")
            print(f"  Overall:      {result.get('overall', 'N/A')}/10")
            if result.get('issues'):
                print(f"  Issues: {result['issues']}")

    # Calculate averages
    valid_results = [r for r in results if 'error' not in r]
    if valid_results:
        print("\n" + "=" * 60)
        print("SUMMARY")
        print("=" * 60)
        avg_overall = sum(r['overall'] for r in valid_results) / len(valid_results)
        avg_completeness = sum(r.get('completeness', 0) for r in valid_results) / len(valid_results)
        avg_accuracy = sum(r.get('accuracy', 0) for r in valid_results) / len(valid_results)
        avg_structure = sum(r.get('structure', 0) for r in valid_results) / len(valid_results)
        avg_reading = sum(r.get('reading_order', 0) for r in valid_results) / len(valid_results)

        print(f"Pages evaluated: {len(valid_results)}")
        print(f"Average Completeness: {avg_completeness:.1f}/10")
        print(f"Average Accuracy:     {avg_accuracy:.1f}/10")
        print(f"Average Structure:    {avg_structure:.1f}/10")
        print(f"Average Reading Order:{avg_reading:.1f}/10")
        print(f"Average Overall:      {avg_overall:.1f}/10 ({avg_overall*10:.0f}%)")

        # Save results
        output_path = Path("reports/llm_judge_results.json")
        output_path.parent.mkdir(exist_ok=True)
        with open(output_path, 'w') as f:
            json.dump({
                "pdf": "test-corpus/pdf/2312.00752.pdf",
                "pages_evaluated": sample_pages,
                "results": results,
                "averages": {
                    "completeness": avg_completeness,
                    "accuracy": avg_accuracy,
                    "structure": avg_structure,
                    "reading_order": avg_reading,
                    "overall": avg_overall
                }
            }, f, indent=2)
        print(f"\nResults saved to: {output_path}")

if __name__ == "__main__":
    main()
