# MANAGER: Answering User Questions

**User asks:**
1. "Why do we want BGR mode? It seems slower."
2. "What is the 'recursive' option?"

---

## Question 1: BGR Mode - You're RIGHT, Remove It

### The Facts

**Claimed:** 3.68% faster
**Measured:** 0.976x (2.4% SLOWER)
**Verdict:** BGR mode provides NO benefit

### Why It Exists

Worker implemented it thinking "25% less memory bandwidth" would help.

**Reality:** Memory-bound system doesn't benefit from bandwidth reduction (profiling showed 90% time in memory stalls, not bandwidth).

### What to Do: REMOVE BGR MODE

**It provides:**
- ❌ No speed benefit (2.4% slower)
- ❌ No memory savings (same RAM usage)
- ❌ Only "theoretical" bandwidth reduction (doesn't matter)
- ✅ Added complexity (detecting transparency)

**Recommendation:** Remove BGR mode entirely (revert to always BGRA)

**Files to revert:**
- examples/pdfium_cli.cpp (BGR mode logic)
- Any documentation mentioning BGR

**Worker should do (N=134):**
```bash
# Revert BGR mode changes
git log --grep="BGR Mode" --oneline | head -1
# Find commit: 0b5a3f85c3 [WORKER0] # 41: BGR Mode Implementation

# Create revert
git revert 0b5a3f85c3 -m "[WORKER0] # 134: Remove BGR Mode - No Performance Benefit

Per MANAGER measurement: BGR is 2.4% SLOWER, not faster.

Measured (N=MANAGER verification):
- BGR (3 bytes):  0.432s
- BGRA (4 bytes): 0.421s
- Result: BGR slower

Theory (25% less bandwidth) doesn't apply to memory-bound system.

Reverting to always use BGRA (4 bytes) for simplicity.

Tests should still pass (no functional change, just removes optimization)."
```

---

## Question 2: Recursive Option - Simple Explanation

### What It Means

**Without recursive (--no-recursive):**
```
/my_pdfs/
├── doc1.pdf          ← Processed
├── doc2.pdf          ← Processed
└── 2024/
    ├── jan.pdf       ← SKIPPED
    └── reports/
        └── q1.pdf    ← SKIPPED
```
Only processes PDFs in the top-level directory.

**With recursive (default in v2.0.0):**
```
/my_pdfs/
├── doc1.pdf          ← Processed
├── doc2.pdf          ← Processed
└── 2024/
    ├── jan.pdf       ← Processed
    └── reports/
        └── q1.pdf    ← Processed
```
Processes PDFs in ALL subdirectories (searches entire tree).

### Why It's Default

**Most users want recursive:**
- PDFs are often organized in subdirectories (by year, department, etc.)
- Who wants to skip subdirectories? Very rare use case.

**Example:** Your 100K PDFs are probably spread across many folders:
```
/dataset_a/
├── 2020/
│   ├── january/
│   │   └── ... (1000 PDFs)
│   └── february/
│       └── ... (1000 PDFs)
├── 2021/
│   └── ... (10,000 PDFs)
└── 2022/
    └── ... (10,000 PDFs)
```

With recursive: Processes all 100K PDFs automatically
Without recursive: Only processes top-level (0 PDFs in this case!)

### How to Use

**Default (recursive):**
```bash
pdfium_cli extract-text /dataset_a/ /output/
# Searches ALL subdirectories
```

**Top-level only (rare):**
```bash
pdfium_cli --no-recursive extract-text /dataset_a/ /output/
# Only top-level files (skips subdirectories)
```

---

## Summary for User

**BGR mode:** You're right - it's useless. Remove it (N=134).

**Recursive:** Means "search all subdirectories, not just top level"
- Default: ON (most users want this)
- Disable: --no-recursive (rare)

**Your 100K extraction:**
```bash
# This processes ALL PDFs in ALL subdirectories
pdfium_cli extract-text /pdfs/ /output/
```

**If your PDFs are in one flat directory:** Recursive doesn't matter (no subdirs to search).

**If your PDFs are in subdirectories:** You WANT recursive (otherwise you get 0 files!).
