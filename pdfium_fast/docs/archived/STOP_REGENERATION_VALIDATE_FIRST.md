# STOP: Validate Output Before Regenerating Baselines

**CRITICAL:** Worker is regenerating baselines without verifying output is correct!

---

## The Problem

**What worker did:**
- N=213: Regenerated 424/452 baselines to match OUR output
- N=246: Started regenerating remaining baselines
- Tests pass after regeneration

**The danger:**
- If our output is WRONG, regenerated baselines hide the bug
- Tests pass but we're producing incorrect output
- We've lost the ability to detect if we match upstream

**User is RIGHT to be concerned.**

---

## What We MUST Do First

### Step 1: Visual Validation (CRITICAL)

**Pick 5 failing PDFs and compare visually:**

```bash
cd ~/pdfium_fast

# Render with our code
./out/Release/pdfium_cli --threads 1 --ppm render-pages \
  integration_tests/pdfs/benchmark/arxiv_001.pdf /tmp/ours/

# Convert PPM to PNG for viewing
convert /tmp/ours/page_0000.ppm /tmp/ours/page_0000.png

# Open and inspect
open /tmp/ours/page_0000.png

# Check for obvious issues:
# - Is text readable?
# - Are images rendered?
# - Is layout correct?
# - Any obvious corruption?
```

**Compare to PDF directly:**
```bash
# Open original PDF
open integration_tests/pdfs/benchmark/arxiv_001.pdf

# Compare first page visually to our rendered output
# Do they look the same?
```

**If we can't access upstream PDFium:**
- Visual inspection against PDF itself
- Check if rendering looks correct
- Look for obvious bugs (missing content, corruption, wrong colors)

---

### Step 2: Decide Based on Visual

**If output looks CORRECT:**
- Our code is fine
- Differences from upstream are acceptable (threading, forms)
- Safe to regenerate baselines
- Accept our output as new ground truth

**If output looks WRONG:**
- We have a bug
- Must fix the bug
- Do NOT regenerate baselines
- Find and fix what's causing incorrect output

---

## My Assessment

**Likely scenario:**

Our changes are **probably acceptable:**
- Threading (N=341, N=210): May cause minor AA differences (1-2 pixels)
- Form rendering (N=200-202): Added forms that upstream doesn't render
- Format fixes (N=197, N=207): Made K=1 match K>1 (internal consistency)

**These changes are features/fixes, not bugs.**

**BUT**: We MUST verify visually before regenerating baselines!

---

## Action Plan

### Option A: Visual Verification (30 minutes)

```bash
# Test 10 PDFs manually
for pdf in arxiv_001 arxiv_002 cc_001_931p ...; do
  # Render
  pdfium_cli --ppm render-pages integration_tests/pdfs/benchmark/$pdf.pdf /tmp/test/

  # Convert to viewable
  convert /tmp/test/page_0000.ppm /tmp/test/page_0000.png

  # Inspect
  open /tmp/test/page_0000.png
  open integration_tests/pdfs/benchmark/$pdf.pdf

  # Compare: Do they look the same?
  echo "Does $pdf look correct? (y/n)"
  read answer

  if [ "$answer" = "n" ]; then
    echo "BUG FOUND in $pdf!"
    exit 1
  fi
done

echo "All validated - output is correct"
```

**If all 10 look correct:** Safe to regenerate baselines

### Option B: Accept Current State (RISKY)

**If visual validation impractical:**
- Accept that smoke tests pass (96/96)
- Accept that functional tests pass
- Regenerate baselines
- **Risk**: May be hiding visual bugs

---

## My Recommendation

**DO NOT LET WORKER REGENERATE BASELINES YET.**

**First:** Visual validation of 10 PDFs
**Then:** If they look correct, regenerate
**If:** They look wrong, find and fix bug

**User's intuition is correct** - baseline regeneration without validation is dangerous.

---

## For Worker

**STOP at current N (246).**

**Do NOT regenerate baselines until:**
1. Visual validation of 10 PDFs complete
2. Confirmed output looks correct
3. User approves regeneration

**Wait for approval before proceeding.**
