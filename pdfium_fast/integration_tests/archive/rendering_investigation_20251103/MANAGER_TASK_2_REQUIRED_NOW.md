# 🚨 MANAGER: TASK 2 REQUIRED NOW - Disable Multi-Process

**Date:** 2025-11-03 15:05 PST
**For:** WORKER0 (Iteration 7+)
**Priority:** CRITICAL - Required before continuing Task 1

---

## STOP: Complete Task 2 Before Progressive Rendering

**Worker status:**
- Task 1: 95% complete (was 68%, now 95%)
- Task 2: 0% complete (IGNORED)
- Task 3-5: Not started

**Worker wants to:** Implement progressive rendering

**MANAGER orders:** STOP. Complete Task 2 FIRST.

---

## Why Task 2 Is Critical

### User Reminder (Just Now)

> "remember that the underlying pdfium library is not threadsafe"

### PDFium Constraint

**From fpdfview.h:11:**
```c
// NOTE: None of the PDFium APIs are thread-safe
```

**Your code (line 133):**
```rust
// NOTE: Always use multiprocess mode (even for worker_count=1)
```

**This violates the constraint!** Even separate processes can have issues.

### Race Conditions Already Found

**Evidence from iterations #110-111:**
- Different pages fail with different worker counts
- Non-deterministic output observed
- MANAGER documented this (commit 2307a29f58)

---

## TASK 2: Disable Multi-Process (FROM ROADMAP)

**File:** `rust/pdfium-sys/examples/render_pages.rs`
**Lines:** 75-155

### Required Changes

**1. Force single-threaded (line 97):**
```rust
// OLD:
let worker_count = if !numeric_args.is_empty() { ... } else { ... }

// NEW:
let worker_count = 1;  // Force single-threaded (PDFium not thread-safe)

if !numeric_args.is_empty() {
    let requested = numeric_args[0].parse::<usize>().unwrap_or(1);
    if requested > 1 {
        eprintln!("Warning: Multi-process requested but not supported.");
        eprintln!("Reason: Vanilla PDFium is not thread-safe.");
        eprintln!("Using single-threaded mode (worker_count=1).");
    }
}
```

**2. Remove multi-process code path (line 136):**
```rust
// OLD:
let result = if worker_count == 1 {
    render_single_threaded(...)
} else {
    render_multiprocess(...)  // REMOVE THIS
};

// NEW:
let result = render_single_threaded(pdf_path, output_dir, page_count, dpi, md5_mode, ppm_mode);
// Always single-threaded
```

**3. Mark multi-process functions as deprecated:**
```rust
#[deprecated(note = "Multi-process disabled - PDFium not thread-safe")]
fn render_multiprocess(...) -> Result<(), String> {
    Err("Multi-process rendering is disabled. Use single-threaded mode.".to_string())
}

#[deprecated]
fn worker_main() {
    eprintln!("Worker mode disabled");
    process::exit(1);
}
```

---

## Testing After Task 2

```bash
# Should work:
render_pages test.pdf out/ 1 300 --ppm

# Should show warning but still work:
render_pages test.pdf out/ 4 300 --ppm
# Output: "Warning: Multi-process requested but not supported. Using single-threaded."
```

---

## Why NOW, Not Later

**Reasons:**
1. User explicitly reminded about thread-safety
2. Task 2 is in the roadmap (uncompleted)
3. Multi-process has race conditions
4. Baseline must be simple and correct
5. Can add optimizations LATER as separate feature

**95% pass rate is good enough for baseline!**
- Form APIs fixed main issues
- Remaining 5% (10 PDFs) can be:
  - Documented as edge cases
  - Fixed later as enhancement
  - Acceptable for baseline certification

---

## Directive: Complete Task 2 Now

**DO THIS (Iteration 7):**
1. ✅ Disable multi-process (force worker_count=1)
2. ✅ Add warning message if >1 requested
3. ✅ Remove/deprecate multi-process functions
4. ✅ Test: Smoke tests still pass
5. ✅ Commit: "Task 2 Complete - Multi-Process Disabled"

**THEN (Iteration 8+):**
- Continue with progressive rendering if needed
- Or document 95% pass rate as acceptable
- Or investigate remaining 10 PDFs

---

## Time Limit

Worker has spent 6 iterations on Task 1.
95% pass rate achieved.

**Decision:**
- Complete Task 2 NOW (1 iteration)
- Declare baseline system complete at 95%
- Document remaining 5% as future work

**OR:**
- Spend 2 more iterations on Task 1 → 100%
- Then do Task 2

**Recommendation:** Complete Task 2 now. 95% is acceptable for baseline.

---

**Reference:** MANAGER_TASK_2_REQUIRED_NOW.md (this file)
