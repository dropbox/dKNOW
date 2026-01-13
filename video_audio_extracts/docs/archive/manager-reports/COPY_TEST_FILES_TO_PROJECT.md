# [MANAGER] Copy Test Files Into Project Directory

USER ISSUE: "File moved or deleted" - Worker tried to use ~/Downloads/Screen Recording...mov but it was moved.

USER DIRECTIVE: "move files that you use for tests into your own directory so that this doesn't happen again"

---

## PROBLEM

Worker is linking to files outside project:
- ~/Desktop/stuff/
- ~/Downloads/
- ~/Library/CloudStorage/Dropbox*/

**Risk**: Files get moved, deleted, or renamed → tests break

---

## SOLUTION

**Create stable test file directory IN PROJECT**:

```bash
mkdir -p test_files_stable/mp4
mkdir -p test_files_stable/mp3
mkdir -p test_files_stable/mov
mkdir -p test_files_stable/mkv
mkdir -p test_files_stable/m4a
mkdir -p test_files_stable/heic
```

**Copy files INTO project** (not symlink, actual copy):

```bash
# MP4 files
cp ~/Desktop/stuff/stuff/"May 5 - live labeling mocks.mp4" test_files_stable/mp4/01_may5_labeling_mocks.mp4
find ~/Desktop/stuff -name "*Screen Recording*2025-07-01*.mov" -exec cp {} test_files_stable/mov/01_screen_recording_july.mov \;
cp ~/Desktop/stuff/stuff/review*/april*/video1509128771.mp4 test_files_stable/mp4/02_april_meeting.mp4

# MP3 files
cp ~/Library/CloudStorage/Dropbox*/a.test/public/librivox/fabula_01_022_esopo_64kb.mp3 test_files_stable/mp3/01_fabula_esopo.mp3
cp ~/Library/CloudStorage/Dropbox*/a.test/public/librivox/fabula_01_023_esopo_64kb.mp3 test_files_stable/mp3/02_fabula_esopo.mp3
cp ~/Library/CloudStorage/Dropbox*/a.test/public/librivox/fabula_01_024_esopo_64kb.mp3 test_files_stable/mp3/03_fabula_esopo.mp3

# Kinetics MP4 (copy 10-20 files)
find ~/Library/CloudStorage/Dropbox*/Kinetics*/carving\ ice/*.mp4 -exec cp {} test_files_stable/mp4/ \; | head -20

# Document sources
cat > test_files_stable/README.md << 'DOC'
# Stable Test Files

These files are COPIED into project (not linked).
Will not break if external files moved/deleted.

Sources:
- Desktop/stuff: Local recordings
- Kinetics dataset: Real YouTube videos
- Dropbox: Audiobooks, recordings

All files <100MB for GitHub compatibility.
DOC
```

**Then reference stable files**:
```bash
# In test suite, use:
test_files_stable/mp4/01_may5_labeling_mocks.mp4  # Stable path
# NOT:
~/Desktop/stuff/stuff/"May 5 - live labeling mocks.mp4"  # Can move/delete
```

---

## BENEFITS

✅ **Stable paths** - won't break if external files moved
✅ **In git** - can commit small files (<100MB)
✅ **Self-contained** - project has own test files
✅ **Documented** - README explains sources

---

## USER FOUND THE FILE

User said: "that specific file was probably moved to stuff"

Worker should search:
```bash
find ~/Desktop/stuff -name "*Screen Recording*2025-07-01*.mov" 2>/dev/null
```

Copy it to test_files_stable/mov/ for permanent use.

---

## EXECUTE AT N=323

1. Create test_files_stable/ directory structure
2. Copy all external files INTO project
3. Use stable paths in test matrix
4. Document sources in README

This prevents "file moved or deleted" errors.
