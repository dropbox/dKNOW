// Copyright 2025 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "public/fpdf_text_batch.h"

#include <algorithm>
#include <vector>

#include "core/fpdfapi/font/cpdf_font.h"
#include "core/fpdfapi/page/cpdf_textobject.h"
#include "core/fpdftext/cpdf_textpage.h"
#include "core/fxcrt/fx_coordinates.h"
#include "fpdfsdk/cpdfsdk_helpers.h"

namespace {

// Threshold for merging characters into the same cell
// Characters within this distance (in font units) are considered same line
constexpr float kLineHeightTolerance = 0.5f;
// Characters within this horizontal distance are considered same cell
constexpr float kCharSpaceTolerance = 3.0f;

// Convert PDF font flags to our simplified font flags
int ConvertFontFlags(const CPDF_Font* font) {
  if (!font) {
    return 0;
  }

  int pdf_flags = font->GetFontFlags();
  int result = 0;

  // Map PDF font descriptor flags to our constants
  if (pdf_flags & 0x0001) result |= FPDF_TEXTCELL_FONT_FIXED_PITCH;
  if (pdf_flags & 0x0002) result |= FPDF_TEXTCELL_FONT_SERIF;
  if (pdf_flags & 0x0004) result |= FPDF_TEXTCELL_FONT_SYMBOLIC;
  if (pdf_flags & 0x0008) result |= FPDF_TEXTCELL_FONT_SCRIPT;
  if (pdf_flags & 0x0020) result |= FPDF_TEXTCELL_FONT_NONSYMBOLIC;
  if (pdf_flags & 0x0040) result |= FPDF_TEXTCELL_FONT_ITALIC;
  if (pdf_flags & 0x10000) result |= FPDF_TEXTCELL_FONT_ALLCAP;
  if (pdf_flags & 0x20000) result |= FPDF_TEXTCELL_FONT_SMALLCAP;
  if (pdf_flags & 0x40000) result |= FPDF_TEXTCELL_FONT_BOLD;

  return result;
}

// Check if two bounding boxes are on the same line
bool IsSameLine(const CFX_FloatRect& a, const CFX_FloatRect& b, float tolerance) {
  float a_center_y = (a.top + a.bottom) / 2.0f;
  float b_center_y = (b.top + b.bottom) / 2.0f;
  float height = std::max(a.top - a.bottom, b.top - b.bottom);
  return std::abs(a_center_y - b_center_y) < height * tolerance;
}

// Check if two characters should be in the same cell
bool ShouldMerge(const CFX_FloatRect& a, const CFX_FloatRect& b,
                 float font_size) {
  if (!IsSameLine(a, b, kLineHeightTolerance)) {
    return false;
  }
  // Check horizontal proximity
  float gap = b.left - a.right;
  return gap < font_size * kCharSpaceTolerance && gap > -font_size * 0.5f;
}

struct TextCell {
  CFX_FloatRect bbox;
  std::vector<wchar_t> text;
  float font_size;
  int font_flags;
  int char_start;
  int char_count;
};

}  // namespace

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFText_GetAllCellsBufferSizes(FPDF_TEXTPAGE text_page,
                                int* out_cell_count,
                                int* out_text_chars) {
  if (!out_cell_count || !out_text_chars) {
    return false;
  }

  CPDF_TextPage* textpage = CPDFTextPageFromFPDFTextPage(text_page);
  if (!textpage) {
    *out_cell_count = 0;
    *out_text_chars = 0;
    return false;
  }

  int char_count = textpage->CountChars();
  if (char_count <= 0) {
    *out_cell_count = 0;
    *out_text_chars = 0;
    return true;
  }

  // Group characters into cells
  std::vector<TextCell> cells;
  TextCell current_cell;
  current_cell.char_start = 0;
  current_cell.char_count = 0;
  current_cell.font_size = 0;
  current_cell.font_flags = 0;

  for (int i = 0; i < char_count; ++i) {
    const CPDF_TextPage::CharInfo& charinfo = textpage->GetCharInfo(i);
    wchar_t unicode = charinfo.unicode();

    // Skip control characters but count them for char indexing
    if (unicode == 0 || unicode == '\r') {
      continue;
    }

    CFX_FloatRect char_box = charinfo.char_box();
    float font_size = textpage->GetCharFontSize(i);

    // Get font flags from text object
    int font_flags = 0;
    if (const CPDF_TextObject* text_obj = charinfo.text_object()) {
      if (RetainPtr<CPDF_Font> font = text_obj->GetFont()) {
        font_flags = ConvertFontFlags(font.Get());
      }
    }

    // Start new cell or merge with current
    if (current_cell.char_count == 0) {
      // First character of a new cell
      current_cell.bbox = char_box;
      current_cell.text.clear();
      current_cell.text.push_back(unicode);
      current_cell.font_size = font_size;
      current_cell.font_flags = font_flags;
      current_cell.char_start = i;
      current_cell.char_count = 1;
    } else if (unicode == '\n' || !ShouldMerge(current_cell.bbox, char_box, font_size)) {
      // Finish current cell and start new one
      cells.push_back(current_cell);

      current_cell.bbox = char_box;
      current_cell.text.clear();
      if (unicode != '\n') {
        current_cell.text.push_back(unicode);
        current_cell.char_count = 1;
      } else {
        current_cell.char_count = 0;
      }
      current_cell.font_size = font_size;
      current_cell.font_flags = font_flags;
      current_cell.char_start = i;
    } else {
      // Merge with current cell
      current_cell.bbox.Union(char_box);
      current_cell.text.push_back(unicode);
      current_cell.char_count++;
      // Keep first font info (could average or use majority)
    }
  }

  // Don't forget last cell
  if (current_cell.char_count > 0) {
    cells.push_back(current_cell);
  }

  // Calculate total text size
  int total_chars = 0;
  for (const auto& cell : cells) {
    total_chars += static_cast<int>(cell.text.size());
  }

  *out_cell_count = static_cast<int>(cells.size());
  *out_text_chars = total_chars;
  return true;
}

FPDF_EXPORT int FPDF_CALLCONV
FPDFText_ExtractAllCells(FPDF_TEXTPAGE text_page,
                         FPDF_TEXT_CELL_INFO* cells,
                         int max_cells,
                         unsigned short* text_buffer,
                         int text_buffer_chars) {
  if (!cells || max_cells <= 0 || !text_buffer || text_buffer_chars <= 0) {
    return -1;
  }

  CPDF_TextPage* textpage = CPDFTextPageFromFPDFTextPage(text_page);
  if (!textpage) {
    return -1;
  }

  int char_count = textpage->CountChars();
  if (char_count <= 0) {
    return 0;
  }

  // Build cells (same algorithm as GetAllCellsBufferSizes)
  std::vector<TextCell> text_cells;
  TextCell current_cell;
  current_cell.char_start = 0;
  current_cell.char_count = 0;
  current_cell.font_size = 0;
  current_cell.font_flags = 0;

  for (int i = 0; i < char_count; ++i) {
    const CPDF_TextPage::CharInfo& charinfo = textpage->GetCharInfo(i);
    wchar_t unicode = charinfo.unicode();

    if (unicode == 0 || unicode == '\r') {
      continue;
    }

    CFX_FloatRect char_box = charinfo.char_box();
    float font_size = textpage->GetCharFontSize(i);

    int font_flags = 0;
    if (const CPDF_TextObject* text_obj = charinfo.text_object()) {
      if (RetainPtr<CPDF_Font> font = text_obj->GetFont()) {
        font_flags = ConvertFontFlags(font.Get());
      }
    }

    if (current_cell.char_count == 0) {
      current_cell.bbox = char_box;
      current_cell.text.clear();
      current_cell.text.push_back(unicode);
      current_cell.font_size = font_size;
      current_cell.font_flags = font_flags;
      current_cell.char_start = i;
      current_cell.char_count = 1;
    } else if (unicode == '\n' || !ShouldMerge(current_cell.bbox, char_box, font_size)) {
      text_cells.push_back(current_cell);

      current_cell.bbox = char_box;
      current_cell.text.clear();
      if (unicode != '\n') {
        current_cell.text.push_back(unicode);
        current_cell.char_count = 1;
      } else {
        current_cell.char_count = 0;
      }
      current_cell.font_size = font_size;
      current_cell.font_flags = font_flags;
      current_cell.char_start = i;
    } else {
      current_cell.bbox.Union(char_box);
      current_cell.text.push_back(unicode);
      current_cell.char_count++;
    }
  }

  if (current_cell.char_count > 0) {
    text_cells.push_back(current_cell);
  }

  // Fill output arrays
  int cells_to_copy = std::min(static_cast<int>(text_cells.size()), max_cells);
  int text_offset = 0;

  for (int i = 0; i < cells_to_copy; ++i) {
    const TextCell& tc = text_cells[i];

    // Check we have room for text
    int text_len = static_cast<int>(tc.text.size());
    if (text_offset + text_len >= text_buffer_chars) {
      // Truncate
      cells_to_copy = i;
      break;
    }

    // Fill cell info
    cells[i].left = tc.bbox.left;
    cells[i].bottom = tc.bbox.bottom;
    cells[i].right = tc.bbox.right;
    cells[i].top = tc.bbox.top;
    cells[i].text_offset = text_offset;
    cells[i].text_length = text_len;
    cells[i].font_size = tc.font_size;
    cells[i].font_flags = tc.font_flags;
    cells[i].char_start = tc.char_start;
    cells[i].char_count = tc.char_count;

    // Copy text to buffer
    for (int j = 0; j < text_len; ++j) {
      text_buffer[text_offset + j] = static_cast<unsigned short>(tc.text[j]);
    }
    text_offset += text_len;
  }

  // Null terminate the text buffer
  if (text_offset < text_buffer_chars) {
    text_buffer[text_offset] = 0;
  }

  return cells_to_copy;
}

FPDF_EXPORT int FPDF_CALLCONV
FPDFText_ExtractAllChars(FPDF_TEXTPAGE text_page,
                         void* buffer,
                         int buflen) {
  CPDF_TextPage* textpage = CPDFTextPageFromFPDFTextPage(text_page);
  if (!textpage) {
    return -1;
  }

  int char_count = textpage->CountChars();
  if (char_count <= 0) {
    return 0;
  }

  // Each character uses 24 bytes
  constexpr int kBytesPerChar = 24;
  int required_size = char_count * kBytesPerChar;

  if (!buffer) {
    return char_count;  // Return count if buffer is null (query mode)
  }

  if (buflen < required_size) {
    return -1;  // Buffer too small
  }

  uint8_t* out = static_cast<uint8_t*>(buffer);
  int output_count = 0;

  for (int i = 0; i < char_count; ++i) {
    const CPDF_TextPage::CharInfo& charinfo = textpage->GetCharInfo(i);
    wchar_t unicode = charinfo.unicode();

    // Skip control characters
    if (unicode == 0 || unicode == '\r') {
      continue;
    }

    const CFX_FloatRect& box = charinfo.char_box();
    float font_size = textpage->GetCharFontSize(i);

    // Write character data
    // bytes 0-3: Unicode codepoint (uint32_t)
    uint32_t codepoint = static_cast<uint32_t>(unicode);
    memcpy(out, &codepoint, sizeof(codepoint));
    out += sizeof(codepoint);

    // bytes 4-7: left (float)
    float left = box.left;
    memcpy(out, &left, sizeof(left));
    out += sizeof(left);

    // bytes 8-11: bottom (float)
    float bottom = box.bottom;
    memcpy(out, &bottom, sizeof(bottom));
    out += sizeof(bottom);

    // bytes 12-15: right (float)
    float right = box.right;
    memcpy(out, &right, sizeof(right));
    out += sizeof(right);

    // bytes 16-19: top (float)
    float top = box.top;
    memcpy(out, &top, sizeof(top));
    out += sizeof(top);

    // bytes 20-23: font_size (float)
    memcpy(out, &font_size, sizeof(font_size));
    out += sizeof(font_size);

    output_count++;
  }

  return output_count;
}

namespace {

// Check if a character is whitespace (word boundary)
bool IsWordBreak(wchar_t ch) {
  return ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' ||
         ch == 0x00A0 ||  // Non-breaking space
         ch == 0x2002 ||  // En space
         ch == 0x2003 ||  // Em space
         ch == 0x2009 ||  // Thin space
         ch == 0x200B;    // Zero-width space
}

// Word structure for internal use
struct Word {
  CFX_FloatRect bbox;
  std::vector<wchar_t> text;
  int start_char;
  int end_char;
};

// Build word list from text page
std::vector<Word> BuildWordList(CPDF_TextPage* textpage) {
  std::vector<Word> words;
  int char_count = textpage->CountChars();
  if (char_count <= 0) {
    return words;
  }

  Word current_word;
  current_word.start_char = -1;
  current_word.end_char = -1;
  bool in_word = false;
  CFX_FloatRect prev_box;
  float prev_font_size = 0;

  for (int i = 0; i < char_count; ++i) {
    const CPDF_TextPage::CharInfo& charinfo = textpage->GetCharInfo(i);
    wchar_t unicode = charinfo.unicode();

    // Skip null characters
    if (unicode == 0) {
      continue;
    }

    CFX_FloatRect char_box = charinfo.char_box();
    float font_size = textpage->GetCharFontSize(i);

    // Check for word break
    bool is_break = IsWordBreak(unicode);

    // Also check for large gap between characters (word boundary)
    bool has_gap = false;
    if (in_word && !is_break && prev_font_size > 0) {
      float gap = char_box.left - prev_box.right;
      // Gap > 0.3 * font_size indicates word boundary
      if (gap > prev_font_size * 0.3f) {
        has_gap = true;
      }
    }

    if (is_break || has_gap) {
      // End current word if any
      if (in_word && current_word.text.size() > 0) {
        current_word.end_char = i;
        words.push_back(current_word);
      }
      in_word = false;
      current_word.text.clear();
      current_word.start_char = -1;
    }

    if (!is_break) {
      if (!in_word) {
        // Start new word
        current_word.bbox = char_box;
        current_word.text.clear();
        current_word.text.push_back(unicode);
        current_word.start_char = i;
        in_word = true;
      } else {
        // Continue word
        current_word.bbox.Union(char_box);
        current_word.text.push_back(unicode);
      }
      prev_box = char_box;
      prev_font_size = font_size;
    }
  }

  // Don't forget last word
  if (in_word && current_word.text.size() > 0) {
    current_word.end_char = char_count;
    words.push_back(current_word);
  }

  return words;
}

}  // namespace

FPDF_EXPORT int FPDF_CALLCONV
FPDFText_CountWords(FPDF_TEXTPAGE text_page) {
  CPDF_TextPage* textpage = CPDFTextPageFromFPDFTextPage(text_page);
  if (!textpage) {
    return -1;
  }

  std::vector<Word> words = BuildWordList(textpage);
  return static_cast<int>(words.size());
}

FPDF_EXPORT int FPDF_CALLCONV
FPDFText_ExtractWords(FPDF_TEXTPAGE text_page,
                      FPDF_WORD_INFO* words,
                      int max_words,
                      unsigned short* text_buffer,
                      int text_buffer_chars) {
  if (!words || max_words <= 0 || !text_buffer || text_buffer_chars <= 0) {
    return -1;
  }

  CPDF_TextPage* textpage = CPDFTextPageFromFPDFTextPage(text_page);
  if (!textpage) {
    return -1;
  }

  std::vector<Word> word_list = BuildWordList(textpage);
  if (word_list.empty()) {
    return 0;
  }

  // Fill output arrays
  int words_to_copy = std::min(static_cast<int>(word_list.size()), max_words);
  int text_offset = 0;

  for (int i = 0; i < words_to_copy; ++i) {
    const Word& w = word_list[i];

    // Check we have room for text
    int text_len = static_cast<int>(w.text.size());
    if (text_offset + text_len >= text_buffer_chars) {
      // Truncate
      words_to_copy = i;
      break;
    }

    // Fill word info
    words[i].left = w.bbox.left;
    words[i].bottom = w.bbox.bottom;
    words[i].right = w.bbox.right;
    words[i].top = w.bbox.top;
    words[i].start_char = w.start_char;
    words[i].end_char = w.end_char;
    words[i].text_offset = text_offset;
    words[i].text_length = text_len;

    // Copy text to buffer
    for (int j = 0; j < text_len; ++j) {
      text_buffer[text_offset + j] = static_cast<unsigned short>(w.text[j]);
    }
    text_offset += text_len;
  }

  // Null terminate the text buffer
  if (text_offset < text_buffer_chars) {
    text_buffer[text_offset] = 0;
  }

  return words_to_copy;
}
