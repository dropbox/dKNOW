# What is "Baseline Regeneration"?

**User asks:** "What do you mean 'Baseline regeneration'?"

---

## Simple Explanation

**Baselines = Expected Correct Output**

### How Tests Work

**Step 1:** Render a PDF
```bash
pdfium_cli render-pages test.pdf /tmp/output/
# Creates: page_0000.png
```

**Step 2:** Calculate MD5 hash
```bash
md5 page_0000.png
# Result: a1b2c3d4e5f6...
```

**Step 3:** Compare to baseline
```json
{
  "pdf_name": "test.pdf",
  "pages": {
    "0": "a1b2c3d4e5f6..."  ← This is the baseline (expected MD5)
  }
}
```

**Step 4:** Test passes if MD5 matches
```python
actual_md5 = "a1b2c3d4e5f6..."
expected_md5 = baseline["pages"]["0"]
assert actual_md5 == expected_md5  # PASS
```

---

## Why Regeneration is Needed

**When rendering code changes, output MD5 changes:**

### Example: BGR Mode Change (N=41)

**Before BGR:**
```bash
# Render with BGRA (4 bytes)
md5 page_0000.png
# Result: a1b2c3d4...
```

**After BGR:**
```bash
# Render with BGR (3 bytes) - different anti-aliasing!
md5 page_0000.png
# Result: f9e8d7c6...  ← DIFFERENT!
```

**Test fails:**
```python
actual = "f9e8d7c6..."
expected = "a1b2c3d4..."  # Old baseline
assert actual == expected  # FAIL! ✗
```

**Solution: Regenerate baselines**
```bash
# Create new expected outputs
pdfium_cli render-pages all_452_pdfs/ /tmp/new_baselines/

# Calculate new MD5s
for file in /tmp/new_baselines/*.png; do
  md5 $file >> new_baselines.json
done

# Replace old baselines with new
cp new_baselines.json baselines/
```

**Now tests pass again:**
```python
actual = "f9e8d7c6..."
expected = "f9e8d7c6..."  # New baseline
assert actual == expected  # PASS! ✓
```

---

## The Problem with Baseline Regeneration

### It Can Hide Bugs!

**If you introduce a bug and regenerate baselines:**

1. Bug makes output wrong
2. You regenerate baselines with wrong output
3. Tests now expect the wrong output
4. Tests pass, but output is actually broken!

**Example:**
```bash
# Bug: Colors inverted (red becomes blue)
pdfium_cli render-pages sunset.pdf /tmp/output/
# Creates: Blue sunset (WRONG!)

# Regenerate baselines with bug present
md5 page_0000.png
# Baseline now expects: blue sunset (WRONG!)

# Test passes because output matches wrong baseline
assert actual_md5 == wrong_baseline_md5  # PASS but OUTPUT IS WRONG!
```

### How to Regenerate Safely

**Must compare to upstream PDFium:**

```bash
# 1. Render with YOUR code
./out/Release/pdfium_cli render-pages test.pdf /tmp/yours/

# 2. Render with UPSTREAM PDFium (unmodified)
~/upstream_pdfium/pdfium_test test.pdf /tmp/upstream/

# 3. Compare visually or pixel-by-pixel
diff /tmp/yours/page_0000.png /tmp/upstream/page_0000.png

# 4. Only regenerate if outputs are identical or intentionally different
```

---

## Worker's Baseline Regeneration (N=213)

**What worker did:** Regenerated 424/452 PDF baselines

**Why:** BGR changes (N=41, N=197, N=207) changed rendering output

**Safe or not?**
- ✅ If worker compared to upstream: SAFE
- ❌ If worker just regenerated blindly: UNSAFE (may hide bugs)

**Need to verify:** Worker validated against upstream PDFium

---

## The Real Issue

**Baseline regeneration is a symptom, not the problem.**

**Root cause:** Code changes that alter output

**N=41-213 timeline:**
- N=41: BGR mode introduced (changed output)
- N=197: Fixed BGR bugs (changed output again)
- N=200-207: More fixes (output changed more)
- N=210: Threading fix (output stable)
- N=213: Regenerated baselines (to match fixed output)

**Each code change requires baseline regeneration IF output format changed.**

---

## Summary for User

**"Baseline regeneration" means:**
- Tests compare output to expected MD5 hashes (baselines)
- When code changes rendering, MD5s change
- Must update expected values (baselines) to match new output
- **Risky:** Can hide bugs if not validated against upstream

**Why needed:**
- BGR mode (N=41) changed rendering format
- Bug fixes (N=197-210) changed output again
- Baselines out of date, tests fail
- Worker regenerated baselines to match fixed output

**Proper process:**
1. Fix bug
2. Verify output matches upstream PDFium
3. Regenerate baselines
4. Tests pass with correct output
