#!/usr/bin/env python3
"""
AI Visual Judge for DoclingViz

Uses Claude or GPT-4V to evaluate PDF extraction quality by analyzing
visualization screenshots with bounding box overlays.

Phases:
    Phase 2 (evaluate): Evaluate ML extraction quality
    Phase 3 (fix-loop): Auto-categorize issues and generate investigation tasks
    Phase 4 (validate/golden-set): Validate human corrections for golden set

Usage:
    # Phase 2: Single image evaluation
    python ai_visual_judge.py evaluate test-results/viz/page_001.png

    # Phase 2: Batch evaluation
    python ai_visual_judge.py evaluate test-results/viz/ --output test-results/ai_judge/

    # Phase 3: Auto-fix loop with issue categorization
    python ai_visual_judge.py fix-loop test-results/viz/ --output test-results/fix-loop/

    # Phase 3: With custom threshold
    python ai_visual_judge.py fix-loop test-results/viz/ --threshold 85 -v

    # Phase 4: Validate a single correction
    python ai_visual_judge.py validate screenshot.png original.json corrected.json

    # Phase 4: Process corrections for golden set
    python ai_visual_judge.py golden-set corrections/ --output golden/

    # Use GPT-4V instead of Claude
    python ai_visual_judge.py evaluate test-results/viz/ --openai

Requirements:
    pip install anthropic openai

Environment:
    ANTHROPIC_API_KEY or OPENAI_API_KEY depending on model choice
"""

import argparse
import base64
import json
import os
import sys
from pathlib import Path
from typing import Optional


def encode_image(path: Path) -> str:
    """Base64 encode image for API."""
    return base64.standard_b64encode(path.read_bytes()).decode()


def get_evaluation_prompt(json_data: Optional[dict], pdf_name: str) -> str:
    """Generate the evaluation prompt for the AI judge."""
    json_section = ""
    if json_data:
        json_section = f"""
JSON data for detected elements:
```json
{json.dumps(json_data, indent=2)}
```
"""

    return f"""You are evaluating a PDF extraction visualization for "{pdf_name}".

The image shows a PDF page with colored bounding boxes overlaid on detected elements:
- Blue boxes: Section headers
- Gray boxes: Regular text blocks
- Green boxes: Tables
- Yellow boxes: Pictures/figures
- Orange boxes: Captions
- Red boxes: Formulas/equations
- Purple boxes: Titles
- Light gray boxes: Page headers/footers
- Brown boxes: Footnotes
- Cyan boxes: List items
- Blue-gray boxes: Code blocks

Each box has a label showing: [reading order] [element type] [confidence %]
{json_section}
Evaluate the extraction quality on these criteria:

1. **Completeness** (0-25 points): Are all visible text regions, tables, and figures detected?
   - Check if any text paragraphs are missing boxes
   - Check if tables have complete coverage
   - Check if figures/images are detected

2. **Label Accuracy** (0-25 points): Are the element types correctly classified?
   - Is text labeled as "text", headers as "section", tables as "table"?
   - Are captions correctly identified near figures/tables?
   - Are page numbers/headers/footers correctly marked?

3. **Boundary Precision** (0-25 points): Do bounding boxes tightly fit the content?
   - Boxes should not be too large (including extra whitespace)
   - Boxes should not be too small (cutting off content)
   - Boxes should not significantly overlap

4. **Reading Order** (0-25 points): Does the numbered sequence make logical sense?
   - Should typically flow top-to-bottom, left-to-right
   - Multi-column layouts should be handled correctly
   - Captions should be ordered near their figures/tables

Provide your evaluation as JSON with this exact structure:
{{
  "overall_score": <0-100>,
  "completeness": {{
    "score": <0-25>,
    "issues": ["<specific issue 1>", "<specific issue 2>"]
  }},
  "label_accuracy": {{
    "score": <0-25>,
    "issues": ["<specific issue 1>", "<specific issue 2>"]
  }},
  "boundary_precision": {{
    "score": <0-25>,
    "issues": ["<specific issue 1>", "<specific issue 2>"]
  }},
  "reading_order": {{
    "score": <0-25>,
    "issues": ["<specific issue 1>", "<specific issue 2>"]
  }},
  "suggestions": ["<improvement suggestion 1>", "<improvement suggestion 2>"],
  "pass": <true if overall_score >= 90, false otherwise>
}}

Respond ONLY with the JSON object, no other text."""


def judge_with_claude(
    screenshot_path: Path,
    json_data: Optional[dict],
    pdf_name: str,
    model: str = "claude-sonnet-4-20250514",
) -> dict:
    """Use Claude to evaluate the extraction quality."""
    try:
        import anthropic
    except ImportError:
        print("Error: anthropic package not installed. Run: pip install anthropic")
        sys.exit(1)

    client = anthropic.Anthropic()
    prompt = get_evaluation_prompt(json_data, pdf_name)

    response = client.messages.create(
        model=model,
        max_tokens=2000,
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": encode_image(screenshot_path),
                        },
                    },
                    {"type": "text", "text": prompt},
                ],
            }
        ],
    )

    # Parse JSON from response
    response_text = response.content[0].text
    # Handle potential markdown code blocks
    if "```json" in response_text:
        response_text = response_text.split("```json")[1].split("```")[0]
    elif "```" in response_text:
        response_text = response_text.split("```")[1].split("```")[0]

    return json.loads(response_text.strip())


def judge_with_openai(
    screenshot_path: Path,
    json_data: Optional[dict],
    pdf_name: str,
    model: str = "gpt-4o",
) -> dict:
    """Use GPT-4V to evaluate the extraction quality."""
    try:
        import openai
    except ImportError:
        print("Error: openai package not installed. Run: pip install openai")
        sys.exit(1)

    client = openai.OpenAI()
    prompt = get_evaluation_prompt(json_data, pdf_name)

    response = client.chat.completions.create(
        model=model,
        max_tokens=2000,
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:image/png;base64,{encode_image(screenshot_path)}"
                        },
                    },
                    {"type": "text", "text": prompt},
                ],
            }
        ],
    )

    # Parse JSON from response
    response_text = response.choices[0].message.content
    # Handle potential markdown code blocks
    if "```json" in response_text:
        response_text = response_text.split("```json")[1].split("```")[0]
    elif "```" in response_text:
        response_text = response_text.split("```")[1].split("```")[0]

    return json.loads(response_text.strip())


def judge_extraction(
    screenshot_path: Path,
    json_path: Optional[Path],
    use_openai: bool = False,
    model: Optional[str] = None,
) -> dict:
    """Evaluate extraction quality using AI vision model."""
    # Load JSON sidecar if available
    json_data = None
    if json_path and json_path.exists():
        json_data = json.loads(json_path.read_text())

    # Extract PDF name from JSON or filename
    pdf_name = "unknown.pdf"
    if json_data and "pdf" in json_data:
        pdf_name = json_data["pdf"]
    else:
        # Try to extract from filename pattern: {stem}_page_{N}_{stage}.png
        stem = screenshot_path.stem
        if "_page_" in stem:
            pdf_name = stem.split("_page_")[0] + ".pdf"

    if use_openai:
        return judge_with_openai(
            screenshot_path, json_data, pdf_name, model or "gpt-4o"
        )
    else:
        return judge_with_claude(
            screenshot_path, json_data, pdf_name, model or "claude-sonnet-4-20250514"
        )


# =============================================================================
# Phase 3: Auto-Fix Loop - Issue Categorization and Investigation Tasks
# =============================================================================

# Issue type handlers - maps issue types to descriptions and investigation hints
ISSUE_HANDLERS = {
    "missed_element": {
        "description": "ML missed detecting this element",
        "investigation": "Check OCR or layout detection confidence threshold",
        "files": ["crates/docling-pdf-ml/src/models/layout_predictor/"],
    },
    "wrong_label": {
        "description": "Element detected but incorrectly classified",
        "investigation": "Classification model confusion, may need fine-tuning or class mapping review",
        "files": ["crates/docling-pdf-ml/src/models/layout_predictor/"],
    },
    "bbox_too_large": {
        "description": "Bounding box includes too much whitespace or adjacent content",
        "investigation": "Post-processing bbox shrinking or clustering parameters",
        "files": ["crates/docling-pdf-ml/src/pipeline/bbox_adjust.rs"],
    },
    "bbox_too_small": {
        "description": "Bounding box clips content or is undersized",
        "investigation": "Check padding parameters or clustering algorithm",
        "files": ["crates/docling-pdf-ml/src/pipeline/bbox_adjust.rs"],
    },
    "wrong_reading_order": {
        "description": "Reading order sequence is incorrect",
        "investigation": "Reading order algorithm issue - check column detection or spatial sorting",
        "files": ["crates/docling-pdf-ml/src/pipeline/reading_order.rs"],
    },
    "table_structure_wrong": {
        "description": "Table cells or structure incorrectly detected",
        "investigation": "TableFormer prediction issue - check OTSL parsing for complex headers",
        "files": ["crates/docling-pdf-ml/src/models/table_structure/"],
    },
    "merged_elements": {
        "description": "Separate elements incorrectly merged into one",
        "investigation": "Over-aggressive clustering - adjust merge threshold",
        "files": ["crates/docling-pdf-ml/src/pipeline/cell_assignment.rs"],
    },
    "split_elements": {
        "description": "Single element incorrectly split into multiple",
        "investigation": "Under-aggressive clustering - adjust split threshold",
        "files": ["crates/docling-pdf-ml/src/pipeline/cell_assignment.rs"],
    },
    "ocr_error": {
        "description": "OCR text extraction error",
        "investigation": "Check OCR model or preprocessing",
        "files": ["crates/docling-pdf-ml/src/ocr/"],
    },
    "overlap_issue": {
        "description": "Bounding boxes have significant overlap",
        "investigation": "Check NMS (non-maximum suppression) or deduplication logic",
        "files": ["crates/docling-pdf-ml/src/pipeline/"],
    },
}


def categorize_issue(issue_text: str, category: str) -> str:
    """
    Categorize an issue string into a specific issue type.

    Args:
        issue_text: The issue description from AI evaluation
        category: The category it came from (completeness, label_accuracy, etc.)

    Returns:
        Issue type string (e.g., "missed_element", "wrong_label")
    """
    issue_lower = issue_text.lower()

    # Keyword-based categorization
    if category == "completeness":
        if any(kw in issue_lower for kw in ["miss", "not detect", "undetect", "no box"]):
            return "missed_element"
        if any(kw in issue_lower for kw in ["merge", "combined", "grouped"]):
            return "merged_elements"
        if any(kw in issue_lower for kw in ["split", "broken", "fragmented"]):
            return "split_elements"
        return "missed_element"  # Default for completeness issues

    elif category == "label_accuracy":
        if any(kw in issue_lower for kw in ["wrong", "incorrect", "mislabel", "should be"]):
            return "wrong_label"
        if any(kw in issue_lower for kw in ["table", "cell"]):
            return "table_structure_wrong"
        return "wrong_label"  # Default for label issues

    elif category == "boundary_precision":
        if any(kw in issue_lower for kw in ["large", "extend", "whitespace", "extra"]):
            return "bbox_too_large"
        if any(kw in issue_lower for kw in ["small", "clip", "cut", "missing part"]):
            return "bbox_too_small"
        if any(kw in issue_lower for kw in ["overlap", "intersect"]):
            return "overlap_issue"
        return "bbox_too_large"  # Default for boundary issues

    elif category == "reading_order":
        return "wrong_reading_order"

    # Fallback based on content
    if "table" in issue_lower:
        return "table_structure_wrong"
    if "ocr" in issue_lower or "text" in issue_lower:
        return "ocr_error"

    return "missed_element"  # Ultimate fallback


def extract_and_categorize_issues(eval_result: dict) -> list:
    """
    Extract all issues from an evaluation result and categorize them.

    Returns:
        List of categorized issue dicts with type, description, severity, and metadata
    """
    categorized = []
    categories = ["completeness", "label_accuracy", "boundary_precision", "reading_order"]

    for cat in categories:
        if cat not in eval_result:
            continue

        cat_data = eval_result[cat]
        score = cat_data.get("score", 25)
        issues = cat_data.get("issues", [])

        # Determine severity based on score
        if score < 10:
            severity = "critical"
        elif score < 18:
            severity = "high"
        elif score < 22:
            severity = "medium"
        else:
            severity = "low"

        for issue_text in issues:
            issue_type = categorize_issue(issue_text, cat)
            handler = ISSUE_HANDLERS.get(issue_type, {})

            categorized.append({
                "type": issue_type,
                "severity": severity,
                "category": cat,
                "description": issue_text,
                "investigation_hint": handler.get("investigation", ""),
                "relevant_files": handler.get("files", []),
            })

    return categorized


def generate_investigation_task(
    pdf_path: str,
    page: int,
    score: int,
    issues: list,
    screenshot_path: Optional[str] = None,
) -> dict:
    """
    Generate an investigation task JSON for a failed evaluation.

    Args:
        pdf_path: Path to the PDF that failed
        page: Page number
        score: Overall score
        issues: List of categorized issues

    Returns:
        Investigation task dict for tracking
    """
    # Group issues by type and count
    type_counts = {}
    for issue in issues:
        itype = issue["type"]
        type_counts[itype] = type_counts.get(itype, 0) + 1

    # Find the primary issue (most frequent high-severity)
    severity_order = {"critical": 0, "high": 1, "medium": 2, "low": 3}
    primary_issue = None
    for issue in sorted(issues, key=lambda x: (severity_order.get(x["severity"], 4), -type_counts.get(x["type"], 0))):
        primary_issue = issue
        break

    if not primary_issue:
        primary_type = "unknown"
        primary_files = []
    else:
        primary_type = primary_issue["type"]
        primary_files = primary_issue.get("relevant_files", [])

    # Build task
    task = {
        "task_id": f"fix_{Path(pdf_path).stem}_p{page}_{primary_type}",
        "type": "investigation",
        "priority": "high" if score < 70 else "medium" if score < 85 else "low",
        "status": "open",
        "document": pdf_path,
        "page": page,
        "score": score,
        "primary_issue_type": primary_type,
        "issue_summary": {
            "total_issues": len(issues),
            "by_type": type_counts,
            "by_severity": {
                "critical": sum(1 for i in issues if i["severity"] == "critical"),
                "high": sum(1 for i in issues if i["severity"] == "high"),
                "medium": sum(1 for i in issues if i["severity"] == "medium"),
                "low": sum(1 for i in issues if i["severity"] == "low"),
            },
        },
        "issues": issues,
        "suggested_investigation": {
            "files": primary_files,
            "description": ISSUE_HANDLERS.get(primary_type, {}).get("investigation", "Manual investigation required"),
        },
        "test_command": f"dlviz-screenshot {pdf_path} --page {page} --stage reading-order -v",
    }

    if screenshot_path:
        task["screenshot"] = screenshot_path

    return task


def run_fix_loop(
    input_dir: Path,
    output_dir: Path,
    use_openai: bool = False,
    model: Optional[str] = None,
    verbose: bool = False,
    threshold: int = 90,
) -> dict:
    """
    Run the auto-fix loop: evaluate all visualizations, categorize issues,
    and generate investigation tasks for failures.

    Args:
        input_dir: Directory containing visualization PNGs
        output_dir: Directory for output (tasks, reports)
        use_openai: Use OpenAI instead of Claude
        model: Specific model to use
        verbose: Show detailed output
        threshold: Pass threshold (default 90)

    Returns:
        Summary with tasks and statistics
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    tasks_dir = output_dir / "tasks"
    tasks_dir.mkdir(exist_ok=True)

    all_tasks = []
    all_results = []
    issue_stats = {}

    png_files = sorted(input_dir.glob("*.png"))

    if not png_files:
        print(f"No PNG files found in {input_dir}")
        return {"total": 0, "passed": 0, "failed": 0, "tasks": []}

    print(f"Found {len(png_files)} visualization(s) to evaluate")
    print(f"Pass threshold: {threshold}/100")
    print()

    for i, png_path in enumerate(png_files, 1):
        json_path = png_path.with_suffix(".json")

        print(f"[{i}/{len(png_files)}] {png_path.name}...", end=" ", flush=True)

        try:
            result = judge_extraction(png_path, json_path, use_openai, model)
            result["file"] = png_path.name
            all_results.append(result)

            score = result.get("overall_score", 0)
            passed = score >= threshold

            status = "PASS" if passed else "FAIL"
            print(f"{score}/100 - {status}")

            if not passed:
                # Categorize issues
                categorized_issues = extract_and_categorize_issues(result)

                # Update global stats
                for issue in categorized_issues:
                    itype = issue["type"]
                    issue_stats[itype] = issue_stats.get(itype, 0) + 1

                # Generate investigation task
                pdf_name = "unknown.pdf"
                page_num = 0
                # Parse from filename pattern: {stem}_page_{N}_{stage}.png
                stem = png_path.stem
                if "_page_" in stem:
                    parts = stem.split("_page_")
                    pdf_name = parts[0] + ".pdf"
                    try:
                        page_num = int(parts[1].split("_")[0])
                    except (ValueError, IndexError):
                        pass

                task = generate_investigation_task(
                    pdf_name,
                    page_num,
                    score,
                    categorized_issues,
                    str(png_path),
                )
                all_tasks.append(task)

                # Save individual task file
                task_path = tasks_dir / f"{task['task_id']}.json"
                task_path.write_text(json.dumps(task, indent=2))

                if verbose:
                    print(f"  Created task: {task['task_id']}")
                    print(f"  Primary issue: {task['primary_issue_type']}")
                    for issue in categorized_issues[:3]:
                        print(f"    - [{issue['severity']}] {issue['type']}: {issue['description'][:60]}...")

        except Exception as e:
            print(f"ERROR: {e}")
            all_results.append({
                "file": png_path.name,
                "overall_score": 0,
                "pass": False,
                "error": str(e),
            })

    # Generate summary
    scores = [r.get("overall_score", 0) for r in all_results if "error" not in r]
    summary = {
        "total": len(all_results),
        "passed": sum(1 for r in all_results if r.get("overall_score", 0) >= threshold),
        "failed": sum(1 for r in all_results if r.get("overall_score", 0) < threshold and "error" not in r),
        "errors": sum(1 for r in all_results if "error" in r),
        "avg_score": sum(scores) / len(scores) if scores else 0,
        "threshold": threshold,
        "issue_statistics": issue_stats,
        "tasks_generated": len(all_tasks),
        "tasks": all_tasks,
        "results": all_results,
    }

    # Save summary
    summary_path = output_dir / "fix_loop_summary.json"
    summary_path.write_text(json.dumps(summary, indent=2))

    # Save tasks index
    tasks_index_path = output_dir / "tasks_index.json"
    tasks_index = {
        "generated": str(Path.cwd() / output_dir),
        "total_tasks": len(all_tasks),
        "by_priority": {
            "high": sum(1 for t in all_tasks if t["priority"] == "high"),
            "medium": sum(1 for t in all_tasks if t["priority"] == "medium"),
            "low": sum(1 for t in all_tasks if t["priority"] == "low"),
        },
        "by_issue_type": issue_stats,
        "tasks": [{"id": t["task_id"], "priority": t["priority"], "score": t["score"]} for t in all_tasks],
    }
    tasks_index_path.write_text(json.dumps(tasks_index, indent=2))

    # Print summary
    print()
    print("=" * 70)
    print("AUTO-FIX LOOP SUMMARY")
    print("=" * 70)
    print(f"Total evaluated:     {summary['total']}")
    print(f"Passed (>={threshold}):       {summary['passed']}")
    print(f"Failed (<{threshold}):        {summary['failed']}")
    if summary["errors"]:
        print(f"Errors:              {summary['errors']}")
    print(f"Average score:       {summary['avg_score']:.1f}/100")
    print()

    if all_tasks:
        print("INVESTIGATION TASKS GENERATED")
        print("-" * 70)
        print(f"Total tasks:         {len(all_tasks)}")
        print(f"High priority:       {tasks_index['by_priority']['high']}")
        print(f"Medium priority:     {tasks_index['by_priority']['medium']}")
        print(f"Low priority:        {tasks_index['by_priority']['low']}")
        print()
        print("Issues by Type:")
        for itype, count in sorted(issue_stats.items(), key=lambda x: -x[1]):
            handler = ISSUE_HANDLERS.get(itype, {})
            print(f"  {itype}: {count}")
            if verbose and handler:
                print(f"    → {handler.get('investigation', 'N/A')}")
        print()

    print(f"Tasks saved to:      {tasks_dir}/")
    print(f"Summary saved to:    {summary_path}")
    print(f"Tasks index:         {tasks_index_path}")

    return summary


def cmd_fix_loop(args):
    """Handle 'fix-loop' command for auto-fix iteration."""
    summary = run_fix_loop(
        args.input,
        args.output,
        use_openai=args.openai,
        model=args.model,
        verbose=args.verbose,
        threshold=args.threshold,
    )
    # Exit with failure if there are tasks to address
    sys.exit(0 if summary["tasks_generated"] == 0 else 1)


# =============================================================================
# Phase 4: Golden Set Builder - Correction Validation
# =============================================================================


def compute_correction_diff(original: dict, corrected: dict) -> dict:
    """
    Compute the difference between original detection and human correction.

    Returns a diff structure showing what changed.
    """
    diff = {
        "added_elements": [],
        "removed_elements": [],
        "modified_elements": [],
        "unchanged_count": 0,
    }

    orig_elements = {e.get("id", i): e for i, e in enumerate(original.get("elements", []))}
    corr_elements = {e.get("id", i): e for i, e in enumerate(corrected.get("elements", []))}

    orig_ids = set(orig_elements.keys())
    corr_ids = set(corr_elements.keys())

    # Find added elements
    for eid in corr_ids - orig_ids:
        diff["added_elements"].append(corr_elements[eid])

    # Find removed elements
    for eid in orig_ids - corr_ids:
        diff["removed_elements"].append(orig_elements[eid])

    # Find modified elements
    for eid in orig_ids & corr_ids:
        orig_elem = orig_elements[eid]
        corr_elem = corr_elements[eid]

        changes = {}
        for key in set(orig_elem.keys()) | set(corr_elem.keys()):
            if orig_elem.get(key) != corr_elem.get(key):
                changes[key] = {
                    "original": orig_elem.get(key),
                    "corrected": corr_elem.get(key),
                }

        if changes:
            diff["modified_elements"].append({
                "id": eid,
                "original": orig_elem,
                "corrected": corr_elem,
                "changes": changes,
            })
        else:
            diff["unchanged_count"] += 1

    return diff


def get_validation_prompt(
    pdf_name: str,
    correction_diff: dict,
    corrected_json: dict,
) -> str:
    """Generate the prompt for AI correction validation."""
    return f"""You are validating a human correction to a PDF extraction for "{pdf_name}".

A human reviewed the ML extraction results and made corrections. Your job is to determine if these corrections are accurate.

## Correction Summary

**Added elements:** {len(correction_diff["added_elements"])}
**Removed elements:** {len(correction_diff["removed_elements"])}
**Modified elements:** {len(correction_diff["modified_elements"])}
**Unchanged elements:** {correction_diff["unchanged_count"]}

## Detailed Changes

### Added Elements (human added these, ML missed them):
```json
{json.dumps(correction_diff["added_elements"], indent=2) if correction_diff["added_elements"] else "None"}
```

### Removed Elements (human deleted these, ML false positives):
```json
{json.dumps(correction_diff["removed_elements"], indent=2) if correction_diff["removed_elements"] else "None"}
```

### Modified Elements (human changed these):
```json
{json.dumps(correction_diff["modified_elements"], indent=2) if correction_diff["modified_elements"] else "None"}
```

## Your Task

Look at the PDF page image and evaluate each correction:

1. **For added elements:** Is there actually content at the specified location that the ML missed?
2. **For removed elements:** Was the ML detection actually a false positive (no real content there)?
3. **For modified elements:** Is the human's change (label, bbox, etc.) more accurate than the ML's?

Respond with JSON:
{{
  "verdict": "CORRECT" | "INCORRECT" | "AMBIGUOUS",
  "confidence": <0.0-1.0>,
  "analysis": {{
    "added_elements_valid": <count of valid additions>,
    "added_elements_invalid": <count of invalid additions>,
    "removed_elements_valid": <count of valid removals>,
    "removed_elements_invalid": <count of invalid removals>,
    "modified_elements_valid": <count of valid modifications>,
    "modified_elements_invalid": <count of invalid modifications>
  }},
  "issues": [
    {{"type": "invalid_addition" | "invalid_removal" | "invalid_modification", "element_id": <id>, "reason": "<explanation>"}}
  ],
  "recommendation": "add_to_golden_set" | "reject" | "needs_review",
  "reasoning": "<brief explanation of your verdict>"
}}

Rules:
- CORRECT: All or nearly all corrections are valid (>90% correct)
- INCORRECT: Most corrections are wrong (>50% wrong)
- AMBIGUOUS: Mixed or unclear, needs human expert review

Respond ONLY with the JSON object."""


def validate_correction_with_claude(
    screenshot_path: Path,
    correction_diff: dict,
    corrected_json: dict,
    pdf_name: str,
    model: str = "claude-sonnet-4-20250514",
) -> dict:
    """Use Claude to validate a human correction."""
    try:
        import anthropic
    except ImportError:
        print("Error: anthropic package not installed. Run: pip install anthropic")
        sys.exit(1)

    client = anthropic.Anthropic()
    prompt = get_validation_prompt(pdf_name, correction_diff, corrected_json)

    response = client.messages.create(
        model=model,
        max_tokens=2000,
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": encode_image(screenshot_path),
                        },
                    },
                    {"type": "text", "text": prompt},
                ],
            }
        ],
    )

    response_text = response.content[0].text
    if "```json" in response_text:
        response_text = response_text.split("```json")[1].split("```")[0]
    elif "```" in response_text:
        response_text = response_text.split("```")[1].split("```")[0]

    return json.loads(response_text.strip())


def validate_correction_with_openai(
    screenshot_path: Path,
    correction_diff: dict,
    corrected_json: dict,
    pdf_name: str,
    model: str = "gpt-4o",
) -> dict:
    """Use GPT-4V to validate a human correction."""
    try:
        import openai
    except ImportError:
        print("Error: openai package not installed. Run: pip install openai")
        sys.exit(1)

    client = openai.OpenAI()
    prompt = get_validation_prompt(pdf_name, correction_diff, corrected_json)

    response = client.chat.completions.create(
        model=model,
        max_tokens=2000,
        messages=[
            {
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:image/png;base64,{encode_image(screenshot_path)}"
                        },
                    },
                    {"type": "text", "text": prompt},
                ],
            }
        ],
    )

    response_text = response.choices[0].message.content
    if "```json" in response_text:
        response_text = response_text.split("```json")[1].split("```")[0]
    elif "```" in response_text:
        response_text = response_text.split("```")[1].split("```")[0]

    return json.loads(response_text.strip())


def validate_correction(
    screenshot_path: Path,
    original_json_path: Path,
    corrected_json_path: Path,
    use_openai: bool = False,
    model: Optional[str] = None,
) -> dict:
    """
    Validate a human correction against the original ML detection.

    Args:
        screenshot_path: Path to visualization PNG
        original_json_path: Path to original ML detection JSON
        corrected_json_path: Path to human-corrected JSON
        use_openai: Use OpenAI instead of Claude
        model: Specific model to use

    Returns:
        Validation result with verdict, confidence, and details
    """
    original = json.loads(original_json_path.read_text())
    corrected = json.loads(corrected_json_path.read_text())

    # Compute what changed
    diff = compute_correction_diff(original, corrected)

    # Extract PDF name
    pdf_name = corrected.get("pdf", original.get("pdf", "unknown.pdf"))

    # Ask AI to validate
    if use_openai:
        result = validate_correction_with_openai(
            screenshot_path, diff, corrected, pdf_name, model or "gpt-4o"
        )
    else:
        result = validate_correction_with_claude(
            screenshot_path, diff, corrected, pdf_name, model or "claude-sonnet-4-20250514"
        )

    # Add metadata
    result["original_path"] = str(original_json_path)
    result["corrected_path"] = str(corrected_json_path)
    result["diff_summary"] = {
        "added": len(diff["added_elements"]),
        "removed": len(diff["removed_elements"]),
        "modified": len(diff["modified_elements"]),
        "unchanged": diff["unchanged_count"],
    }

    return result


def process_golden_set(
    corrections_dir: Path,
    output_dir: Path,
    use_openai: bool = False,
    model: Optional[str] = None,
    verbose: bool = False,
) -> dict:
    """
    Process a directory of corrections and validate them for golden set inclusion.

    Expected directory structure:
        corrections_dir/
            page_001/
                screenshot.png
                original.json
                corrected.json
            page_002/
                ...

    Returns:
        Summary with approved, rejected, and needs_review counts
    """
    output_dir.mkdir(parents=True, exist_ok=True)

    results = {
        "approved": [],
        "rejected": [],
        "needs_review": [],
        "errors": [],
    }

    # Find all correction directories
    correction_dirs = [d for d in corrections_dir.iterdir() if d.is_dir()]

    if not correction_dirs:
        print(f"No correction directories found in {corrections_dir}")
        return results

    print(f"Found {len(correction_dirs)} correction(s) to validate")
    print()

    for i, corr_dir in enumerate(sorted(correction_dirs), 1):
        screenshot = corr_dir / "screenshot.png"
        original = corr_dir / "original.json"
        corrected = corr_dir / "corrected.json"

        print(f"[{i}/{len(correction_dirs)}] {corr_dir.name}...", end=" ", flush=True)

        # Check required files exist
        if not screenshot.exists():
            print("SKIP (no screenshot.png)")
            continue
        if not original.exists():
            print("SKIP (no original.json)")
            continue
        if not corrected.exists():
            print("SKIP (no corrected.json)")
            continue

        try:
            result = validate_correction(
                screenshot, original, corrected, use_openai, model
            )
            result["correction_dir"] = str(corr_dir)

            verdict = result.get("verdict", "AMBIGUOUS")
            recommendation = result.get("recommendation", "needs_review")
            confidence = result.get("confidence", 0)

            print(f"{verdict} ({confidence:.0%}) - {recommendation}")

            if recommendation == "add_to_golden_set":
                results["approved"].append(result)
                # Copy corrected.json to golden set
                golden_path = output_dir / "golden" / f"{corr_dir.name}.json"
                golden_path.parent.mkdir(parents=True, exist_ok=True)
                import shutil
                shutil.copy(corrected, golden_path)
            elif recommendation == "reject":
                results["rejected"].append(result)
            else:
                results["needs_review"].append(result)

            if verbose and recommendation != "add_to_golden_set":
                issues = result.get("issues", [])
                if issues:
                    print("    Issues:")
                    for issue in issues[:3]:
                        print(f"      - [{issue.get('type')}] {issue.get('reason', '')}")

        except Exception as e:
            print(f"ERROR: {e}")
            results["errors"].append({
                "correction_dir": str(corr_dir),
                "error": str(e),
            })

    # Write results
    results_path = output_dir / "golden_set_validation.json"
    summary = {
        "total": len(correction_dirs),
        "approved": len(results["approved"]),
        "rejected": len(results["rejected"]),
        "needs_review": len(results["needs_review"]),
        "errors": len(results["errors"]),
        "details": results,
    }
    results_path.write_text(json.dumps(summary, indent=2))

    # Print summary
    print()
    print("=" * 60)
    print("GOLDEN SET VALIDATION SUMMARY")
    print("=" * 60)
    print(f"Total processed:   {summary['total']}")
    print(f"Approved:          {summary['approved']}")
    print(f"Rejected:          {summary['rejected']}")
    print(f"Needs review:      {summary['needs_review']}")
    if summary["errors"]:
        print(f"Errors:            {summary['errors']}")
    print()
    print(f"Golden set saved to: {output_dir / 'golden'}")
    print(f"Full results:        {results_path}")

    return summary


# =============================================================================
# Phase 2: Batch Evaluation
# =============================================================================


def run_batch_evaluation(
    input_dir: Path,
    output_dir: Path,
    use_openai: bool = False,
    model: Optional[str] = None,
    verbose: bool = False,
) -> dict:
    """Run AI judge on all PNG files in directory."""
    output_dir.mkdir(parents=True, exist_ok=True)

    results = []
    png_files = sorted(input_dir.glob("*.png"))

    if not png_files:
        print(f"No PNG files found in {input_dir}")
        return {"total": 0, "passed": 0, "failed": 0, "avg_score": 0, "results": []}

    print(f"Found {len(png_files)} visualization(s) to evaluate")
    print()

    for i, png_path in enumerate(png_files, 1):
        json_path = png_path.with_suffix(".json")

        print(f"[{i}/{len(png_files)}] {png_path.name}...", end=" ", flush=True)

        try:
            result = judge_extraction(png_path, json_path, use_openai, model)
            result["file"] = png_path.name
            results.append(result)

            status = "PASS" if result.get("pass", False) else "FAIL"
            score = result.get("overall_score", 0)
            print(f"{score}/100 - {status}")

            if verbose and not result.get("pass", False):
                print("  Issues:")
                for category in [
                    "completeness",
                    "label_accuracy",
                    "boundary_precision",
                    "reading_order",
                ]:
                    if category in result:
                        issues = result[category].get("issues", [])
                        if issues:
                            print(f"    {category}:")
                            for issue in issues[:2]:  # Show max 2 issues per category
                                print(f"      - {issue}")

        except Exception as e:
            print(f"ERROR: {e}")
            results.append(
                {
                    "file": png_path.name,
                    "overall_score": 0,
                    "pass": False,
                    "error": str(e),
                }
            )

    # Calculate summary
    scores = [r.get("overall_score", 0) for r in results if "error" not in r]
    summary = {
        "total": len(results),
        "passed": sum(1 for r in results if r.get("pass", False)),
        "failed": sum(1 for r in results if not r.get("pass", False)),
        "errors": sum(1 for r in results if "error" in r),
        "avg_score": sum(scores) / len(scores) if scores else 0,
        "results": results,
    }

    # Write detailed results
    results_path = output_dir / "ai_judge_results.json"
    results_path.write_text(json.dumps(summary, indent=2))

    # Print summary
    print()
    print("=" * 60)
    print("EVALUATION SUMMARY")
    print("=" * 60)
    print(f"Total evaluated:  {summary['total']}")
    print(f"Passed (>=90):    {summary['passed']}")
    print(f"Failed (<90):     {summary['failed']}")
    if summary["errors"]:
        print(f"Errors:           {summary['errors']}")
    print(f"Average score:    {summary['avg_score']:.1f}/100")
    print()
    print(f"Results saved to: {results_path}")

    return summary


def cmd_evaluate(args):
    """Handle 'evaluate' command for quality evaluation."""
    if args.input.is_dir():
        summary = run_batch_evaluation(
            args.input,
            args.output,
            use_openai=args.openai,
            model=args.model,
            verbose=args.verbose,
        )
        sys.exit(0 if summary["failed"] == 0 else 1)
    elif args.input.is_file() and args.input.suffix.lower() == ".png":
        json_path = args.input.with_suffix(".json")
        result = judge_extraction(
            args.input, json_path, use_openai=args.openai, model=args.model
        )
        print(json.dumps(result, indent=2))
        status = "PASS" if result.get("pass", False) else "FAIL"
        print(f"\nOverall: {result.get('overall_score', 0)}/100 - {status}")
        sys.exit(0 if result.get("pass", False) else 1)
    else:
        print(f"Error: {args.input} is not a PNG file or directory")
        sys.exit(1)


def cmd_validate(args):
    """Handle 'validate' command for single correction validation."""
    result = validate_correction(
        args.screenshot,
        args.original,
        args.corrected,
        use_openai=args.openai,
        model=args.model,
    )
    print(json.dumps(result, indent=2))
    print()
    print(f"Verdict: {result.get('verdict', 'UNKNOWN')}")
    print(f"Confidence: {result.get('confidence', 0):.0%}")
    print(f"Recommendation: {result.get('recommendation', 'unknown')}")
    sys.exit(0 if result.get("recommendation") == "add_to_golden_set" else 1)


def cmd_golden_set(args):
    """Handle 'golden-set' command for batch correction validation."""
    summary = process_golden_set(
        args.corrections_dir,
        args.output,
        use_openai=args.openai,
        model=args.model,
        verbose=args.verbose,
    )
    sys.exit(0 if summary["rejected"] == 0 and summary["errors"] == 0 else 1)


def main():
    parser = argparse.ArgumentParser(
        description="AI Visual Judge for DoclingViz extraction quality",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Commands:
  evaluate     Evaluate ML extraction quality (Phase 2)
  fix-loop     Auto-categorize issues and generate investigation tasks (Phase 3)
  validate     Validate a single human correction (Phase 4)
  golden-set   Process and validate corrections for golden set (Phase 4)

Examples:
  # Phase 2: Evaluate extraction quality
  python ai_visual_judge.py evaluate test-results/viz/

  # Phase 3: Run auto-fix loop (categorize issues, generate tasks)
  python ai_visual_judge.py fix-loop test-results/viz/ -v

  # Phase 3: With custom threshold (default 90)
  python ai_visual_judge.py fix-loop test-results/viz/ --threshold 85

  # Phase 4: Validate a single correction
  python ai_visual_judge.py validate screenshot.png original.json corrected.json

  # Phase 4: Process corrections for golden set
  python ai_visual_judge.py golden-set corrections/ --output golden/

Issue Types (Phase 3):
  missed_element      - ML didn't detect this element
  wrong_label         - Element detected but incorrectly classified
  bbox_too_large      - Bounding box includes extra whitespace
  bbox_too_small      - Bounding box clips content
  wrong_reading_order - Reading order sequence incorrect
  table_structure_wrong - Table cells/structure incorrect
  merged_elements     - Separate elements incorrectly merged
  split_elements      - Single element incorrectly split
  ocr_error          - OCR text extraction error
  overlap_issue      - Bounding boxes significantly overlap
""",
    )

    # Common arguments
    parser.add_argument(
        "--model",
        "-m",
        type=str,
        help="Model to use (default: claude-sonnet-4-20250514 or gpt-4o)",
    )
    parser.add_argument(
        "--openai",
        action="store_true",
        help="Use OpenAI GPT-4V instead of Claude",
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Show detailed information",
    )

    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # Evaluate subcommand
    eval_parser = subparsers.add_parser(
        "evaluate", help="Evaluate ML extraction quality"
    )
    eval_parser.add_argument(
        "input",
        type=Path,
        help="Input PNG file or directory containing visualization PNGs",
    )
    eval_parser.add_argument(
        "--output",
        "-o",
        type=Path,
        default=Path("test-results/ai_judge"),
        help="Output directory for results",
    )
    eval_parser.set_defaults(func=cmd_evaluate)

    # Validate subcommand
    validate_parser = subparsers.add_parser(
        "validate", help="Validate a single human correction"
    )
    validate_parser.add_argument(
        "screenshot",
        type=Path,
        help="Path to visualization screenshot PNG",
    )
    validate_parser.add_argument(
        "original",
        type=Path,
        help="Path to original ML detection JSON",
    )
    validate_parser.add_argument(
        "corrected",
        type=Path,
        help="Path to human-corrected JSON",
    )
    validate_parser.set_defaults(func=cmd_validate)

    # Golden-set subcommand
    golden_parser = subparsers.add_parser(
        "golden-set", help="Process and validate corrections for golden set"
    )
    golden_parser.add_argument(
        "corrections_dir",
        type=Path,
        help="Directory containing correction subdirectories",
    )
    golden_parser.add_argument(
        "--output",
        "-o",
        type=Path,
        default=Path("test-results/golden"),
        help="Output directory for golden set and results",
    )
    golden_parser.set_defaults(func=cmd_golden_set)

    # Fix-loop subcommand (Phase 3)
    fix_parser = subparsers.add_parser(
        "fix-loop", help="Run auto-fix loop: evaluate, categorize issues, generate tasks"
    )
    fix_parser.add_argument(
        "input",
        type=Path,
        help="Directory containing visualization PNGs",
    )
    fix_parser.add_argument(
        "--output",
        "-o",
        type=Path,
        default=Path("test-results/fix-loop"),
        help="Output directory for tasks and reports",
    )
    fix_parser.add_argument(
        "--threshold",
        "-t",
        type=int,
        default=90,
        help="Pass threshold score (default: 90)",
    )
    fix_parser.set_defaults(func=cmd_fix_loop)

    args = parser.parse_args()

    # Backwards compatibility: if no subcommand, treat first arg as input for evaluate
    if args.command is None:
        # Check if there's an input-like positional argument
        if len(sys.argv) > 1 and not sys.argv[1].startswith("-"):
            # Old-style invocation: python ai_visual_judge.py <input>
            old_parser = argparse.ArgumentParser()
            old_parser.add_argument("input", type=Path)
            old_parser.add_argument("--output", "-o", type=Path, default=Path("test-results/ai_judge"))
            old_parser.add_argument("--model", "-m", type=str)
            old_parser.add_argument("--openai", action="store_true")
            old_parser.add_argument("--verbose", "-v", action="store_true")
            args = old_parser.parse_args()
            args.func = cmd_evaluate
        else:
            parser.print_help()
            sys.exit(1)

    # Check for API keys
    if args.openai:
        if not os.environ.get("OPENAI_API_KEY"):
            print("Error: OPENAI_API_KEY environment variable not set")
            print("Set it with: export OPENAI_API_KEY=sk-...")
            sys.exit(1)
    else:
        if not os.environ.get("ANTHROPIC_API_KEY"):
            print("Error: ANTHROPIC_API_KEY environment variable not set")
            print("Set it with: export ANTHROPIC_API_KEY=sk-ant-...")
            sys.exit(1)

    # Run the command
    args.func(args)


if __name__ == "__main__":
    main()
