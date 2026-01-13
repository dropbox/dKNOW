# RAR Test File Creation Issue

## Problem

The `/opt/homebrew/bin/rar` binary (version 7.12) hangs indefinitely when attempting to create RAR archives. This affects the ability to generate test files for RAR format integration tests.

## Symptoms

- `rar a archive.rar file.txt` hangs without any output
- Process must be killed with SIGTERM/SIGKILL
- Occurs with all command line flags tried: `-y`, `-ep`, `-inul`, etc.
- Timeout after 5+ seconds with no activity
- No error messages or prompts displayed

## Root Cause

Likely causes:
1. RAR trial version (40-day limit) attempting to show GUI registration dialog
2. License acceptance prompt waiting for user input (not showing in headless mode)
3. Incompatibility with terminal/TTY configuration
4. Bug in Homebrew RAR cask version 7.12

## Attempted Solutions

1. ✗ Standard command: `rar a archive.rar file.txt`
2. ✗ With `-y` flag (assume yes): Still hangs
3. ✗ With `-inul` (no output): Still hangs
4. ✗ With stdin redirect: `< /dev/null` - Still hangs
5. ✗ With piped yes input: `yes "" | rar a ...` - Still hangs
6. ✗ Python subprocess with DEVNULL: Times out
7. ✗ Download public RAR test files: URLs not found (404 errors)

## Workaround Options

### Option 1: Use `unrar` for extraction tests only
- `unrar` works correctly for extraction
- Cannot create archives, only extract existing ones
- Limits testing to extraction functionality

### Option 2: Create RAR files externally
- Use WinRAR on Windows machine
- Use RAR command line on Linux (if available)
- Transfer to test corpus manually
- Document as one-time manual step

### Option 3: Use existing RAR files from other projects
- Search for `.rar` files in other repositories
- Download from test fixture repositories
- Verify license allows redistribution

### Option 4: Skip RAR creation tests temporarily
- Implement RAR extraction/parsing only
- Add stub tests for creation (marked as `#[ignore]`)
- Document limitation in code
- Re-visit when RAR tool is fixed

## Recommendation

**Use Option 2 (External Creation) + Option 4 (Skip Creation Tests)**

1. Ask user to create 5 test RAR files manually if needed
2. Implement RAR extraction and parsing
3. Add integration tests that use pre-created RAR files
4. Document in README that test RAR files must be created externally
5. Mark creation tests as `#[ignore]` or skip them

## Test Files Needed

Create these RAR archives externally:

1. **simple.rar** - Single text file (sample.txt)
2. **multi_files.rar** - Multiple files (sample.txt, data.json, readme.md)
3. **nested.rar** - Directory structure with subdirectories
4. **compressed_best.rar** - High compression (-m5)
5. **rar5_format.rar** - RAR5 format (-ma5) with recovery record (-rr)

## Commands (for external creation on working system)

```bash
# If you have working RAR on another system:
cd test-corpus/archives/rar/temp_content

# Simple RAR
rar a -ep ../simple.rar sample.txt

# Multi-file RAR
rar a -ep ../multi_files.rar sample.txt data.json readme.md

# Nested directory structure
mkdir -p subdir/nested
cp sample.txt subdir/
cp data.json subdir/nested/
rar a ../nested.rar subdir/

# High compression
rar a -m5 -ep ../compressed_best.rar *

# RAR5 format with recovery
rar a -ma5 -rr -ep ../rar5_format.rar *
```

## Status

- ✗ RAR creation via command line: **BLOCKED**
- ✓ RAR extraction (unrar): **WORKS**
- ⚠️  Test file creation: **REQUIRES MANUAL INTERVENTION**

## Date

2025-11-07 11:50 AM PT

## Next Steps

1. Document this issue for user
2. Ask user to create RAR files or provide existing ones
3. Proceed with RAR extraction implementation
4. Add integration tests using pre-existing RAR files
