# Audio/Video Strategy - Document Parser Perspective

**Date:** 2025-11-11
**Context:** User clarified "Audio and video are handled by another system"

---

## Strategic Options

### Option A: Out of Scope (Recommended)

**Approach:** Don't handle audio/video in docling at all

**Rationale:**
- Docling is a **document** parser
- Audio/video are **media** files, not documents
- Another system already handles them
- No overlap in functionality

**Implementation:**
```rust
InputFormat::Wav | InputFormat::Mp3 |
InputFormat::Mp4 | InputFormat::Mkv |
InputFormat::Mov | InputFormat::Avi => {
    return Err(DoclingError::UnsupportedFormat(
        format!("{:?} is a media file. Use dedicated audio/video processing system.", format)
    ));
}
```

**Benefit:** Clear separation of concerns, no wasted effort

---

### Option B: Metadata Only (Minimal)

**Approach:** Extract basic metadata, no transcription

**What to extract:**
- File size, duration
- Codec information
- Resolution (for video)
- Sample rate (for audio)
- Title, artist, album (from tags)
- Creation date

**Implementation:**
```rust
use ffmpeg::format::context::Input;

pub struct AudioVideoBackend;

impl DocumentBackend for AudioVideoBackend {
    fn parse_file(&self, path: &Path) -> Result<Document> {
        // Use ffmpeg-next crate to read metadata
        let input = ffmpeg::format::input(&path)?;

        let metadata = input.metadata();
        let duration = input.duration();

        // Create simple DocItem with metadata
        let doc_items = vec![
            DocItem::Text {
                text: format!("File: {}\nDuration: {}s\nFormat: {:?}",
                    path.display(), duration, format),
                // ... metadata fields
            }
        ];

        let markdown = serialize_metadata(&doc_items);

        Ok(Document {
            markdown,
            content_blocks: Some(doc_items),
            ...
        })
    }
}
```

**Output example:**
```markdown
# Audio File: song.mp3

- **Duration:** 3:45
- **Format:** MP3
- **Bitrate:** 320kbps
- **Sample Rate:** 44.1kHz
- **Title:** Example Song
- **Artist:** Example Artist
- **Album:** Example Album
```

**Benefit:** Provides basic document representation, useful for indexing/search

---

### Option C: Integration Point (Referral)

**Approach:** Detect media files, return reference to other system

**Implementation:**
```rust
pub struct MediaReferralResult {
    pub media_type: MediaType,
    pub file_path: PathBuf,
    pub refer_to_system: String,  // "audio-transcription-service"
    pub metadata: HashMap<String, String>,
}

impl DocumentBackend for AudioVideoBackend {
    fn parse_file(&self, path: &Path) -> Result<Document> {
        // Detect it's media
        let media_type = detect_media_type(path)?;

        // Create referral DocItem
        let doc_items = vec![
            DocItem::Text {
                text: format!(
                    "[Media File: {}]\n\n\
                    This {} file should be processed by the audio/video system.\n\
                    File path: {}",
                    path.file_name().unwrap().to_str().unwrap(),
                    media_type,
                    path.display()
                ),
                ...
            }
        ];

        Ok(Document {
            markdown: format!("[Media File - Refer to audio/video system]"),
            content_blocks: Some(doc_items),
            ...
        })
    }
}
```

**Benefit:** Clear integration point, documents how to handle media files

---

### Option D: Remove from InputFormat Enum

**Approach:** Remove audio/video formats entirely from docling

**Changes:**
```rust
// Remove from InputFormat enum:
- Wav, Mp3, Mp4, Mkv, Mov, Avi

// Remove from format detection:
- Audio/video extensions
```

**Benefit:** Clean scope, no confusion about what docling handles

**Trade-off:** Less flexible for future use cases

---

## Recommendation

**I recommend Option A (Out of Scope) with clear error messages.**

**Rationale:**
1. **Clear separation:** Docling = documents, Other system = media
2. **No wasted effort:** Don't implement what another system does
3. **Clear errors:** Users get helpful message pointing to correct system
4. **Keep enum:** InputFormat can still recognize them (for routing/errors)
5. **No maintenance:** Don't have to keep audio/video parsers updated

**Implementation:**
```rust
// In converter.rs
InputFormat::Wav | InputFormat::Mp3 | InputFormat::Mp4 |
InputFormat::Mkv | InputFormat::Mov | InputFormat::Avi => {
    Err(DoclingError::UnsupportedFormat(format!(
        "{:?} is an audio/video file. \
        Docling handles document formats only. \
        For audio/video processing, use [audio-video-system-name].",
        format
    )))
}
```

**Add to CLAUDE.md:**
```markdown
## Audio/Video Formats

**OUT OF SCOPE:** Audio and video files are handled by a separate system.

Docling focuses on document formats only. For media files:
- WAV, MP3, MP4, MKV, MOV, AVI → Use dedicated audio/video system
- Docling will return clear error pointing to correct system
```

---

## Alternative: If You Want Minimal Support

**If useful for indexing/search, Option B (metadata only) makes sense:**

- **Effort:** 6-10 commits (one per format)
- **Dependencies:** `ffmpeg-next` or `symphonia` crates
- **Output:** Basic metadata (duration, format, tags)
- **No transcription:** That's the other system's job
- **DocItems:** Simple text with metadata fields

**Use case:** Document management system that needs to index media files

---

## Questions for You

**Q1: Should docling handle audio/video at all?**
- A) No - Out of scope entirely (recommended)
- B) Yes - Metadata extraction only
- C) Yes - Full integration with transcription

**Q2: Should we keep them in InputFormat enum?**
- A) Yes - Keep for recognition/error messages
- B) No - Remove entirely

**Q3: Priority?**
- A) High - Implement soon
- B) Low - Defer indefinitely
- C) Never - Mark as out of scope

**My recommendation: Option A (out of scope) + Keep in enum + Clear error messages**

What's your preference?