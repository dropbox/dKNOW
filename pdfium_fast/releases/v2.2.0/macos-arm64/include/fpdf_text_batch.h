// Copyright 2025 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Batch text extraction API for efficient per-page text cell retrieval.
// This API reduces FFI overhead by extracting all text cells in 2-3 calls
// instead of the 100-400 calls typically needed with standard APIs.

#ifndef PUBLIC_FPDF_TEXT_BATCH_H_
#define PUBLIC_FPDF_TEXT_BATCH_H_

#include <stdint.h>

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif

// Text cell information structure
// Each cell represents a contiguous run of text with consistent properties
typedef struct {
  // Bounding box in PDF coordinates (bottom-left origin)
  // Use page_height - top for top-left origin conversion
  float left;
  float bottom;
  float right;
  float top;

  // Text content location (offsets into UTF-16 buffer)
  int text_offset;   // Starting offset in text buffer (in UTF-16 code units)
  int text_length;   // Length of text (in UTF-16 code units)

  // Font information
  float font_size;   // Font size in points
  int font_flags;    // Font flags (bold=1, italic=2, fixed_pitch=4, etc.)

  // Character range in the text page (for advanced use)
  int char_start;    // Starting character index in FPDF_TEXTPAGE
  int char_count;    // Number of characters in this cell
} FPDF_TEXT_CELL_INFO;

// Font flag constants (matches PDF font descriptor flags)
#define FPDF_TEXTCELL_FONT_FIXED_PITCH  0x0001
#define FPDF_TEXTCELL_FONT_SERIF        0x0002
#define FPDF_TEXTCELL_FONT_SYMBOLIC     0x0004
#define FPDF_TEXTCELL_FONT_SCRIPT       0x0008
#define FPDF_TEXTCELL_FONT_NONSYMBOLIC  0x0020
#define FPDF_TEXTCELL_FONT_ITALIC       0x0040
#define FPDF_TEXTCELL_FONT_ALLCAP       0x10000
#define FPDF_TEXTCELL_FONT_SMALLCAP     0x20000
#define FPDF_TEXTCELL_FONT_BOLD         0x40000

// Word information structure
// Each word represents a sequence of non-whitespace characters
typedef struct {
  // Bounding box in PDF coordinates (bottom-left origin)
  float left;
  float bottom;
  float right;
  float top;

  // Character range in the text page
  int start_char;  // Starting character index in FPDF_TEXTPAGE
  int end_char;    // Ending character index (exclusive)

  // Text content location (offsets into UTF-16 buffer)
  int text_offset;  // Starting offset in text buffer
  int text_length;  // Length of text (in UTF-16 code units)
} FPDF_WORD_INFO;

// Function: FPDFText_GetAllCellsBufferSizes
//          Get the required buffer sizes for batch text extraction.
// Parameters:
//          text_page      - Handle to a text page. Returned by FPDFText_LoadPage.
//          out_cell_count - Receives the number of text cells on the page.
//          out_text_chars - Receives the total text length in UTF-16 code units.
// Return value:
//          Returns TRUE on success, FALSE if text_page is invalid or
//          output pointers are NULL.
// Comments:
//          Call this function first to determine buffer sizes, then allocate
//          buffers and call FPDFText_ExtractAllCells.
//
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFText_GetAllCellsBufferSizes(FPDF_TEXTPAGE text_page,
                                int* out_cell_count,
                                int* out_text_chars);

// Function: FPDFText_ExtractAllCells
//          Extract all text cells from a page in a single call.
// Parameters:
//          text_page         - Handle to a text page. Returned by FPDFText_LoadPage.
//          cells             - Pre-allocated array to receive cell information.
//                              Array size should be at least out_cell_count from
//                              FPDFText_GetAllCellsBufferSizes.
//          max_cells         - Size of the cells array.
//          text_buffer       - Pre-allocated buffer to receive UTF-16LE text.
//                              Buffer size should be at least (out_text_chars + 1)
//                              to include null terminator.
//          text_buffer_chars - Size of text_buffer in UTF-16 code units.
// Return value:
//          Number of cells extracted on success, -1 on error.
// Comments:
//          This function extracts ALL text cells from a page in one call,
//          eliminating the need for multiple FPDFText_CountRects/GetRect/
//          GetBoundedText calls.
//
//          Text cells are ordered by reading order (top-to-bottom, left-to-right
//          for LTR documents).
//
//          Each cell's text_offset and text_length point into the shared
//          text_buffer. The text is UTF-16LE encoded.
//
//          Example usage:
//            int cell_count, text_chars;
//            FPDFText_GetAllCellsBufferSizes(text_page, &cell_count, &text_chars);
//
//            FPDF_TEXT_CELL_INFO* cells = malloc(cell_count * sizeof(*cells));
//            unsigned short* text = malloc((text_chars + 1) * sizeof(*text));
//
//            int extracted = FPDFText_ExtractAllCells(text_page, cells,
//                                                     cell_count, text,
//                                                     text_chars + 1);
//
//            for (int i = 0; i < extracted; i++) {
//              // Process cells[i], text starts at text + cells[i].text_offset
//            }
//
//            free(cells);
//            free(text);
//
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_ExtractAllCells(FPDF_TEXTPAGE text_page,
                         FPDF_TEXT_CELL_INFO* cells,
                         int max_cells,
                         unsigned short* text_buffer,
                         int text_buffer_chars);

// Function: FPDFText_ExtractAllChars
//          Extract all character information from a page in a single call.
// Parameters:
//          text_page   - Handle to a text page. Returned by FPDFText_LoadPage.
//          buffer      - Pre-allocated buffer to receive character data.
//                        Each character uses 24 bytes:
//                        - bytes 0-3: Unicode codepoint (uint32_t)
//                        - bytes 4-7: left (float)
//                        - bytes 8-11: bottom (float)
//                        - bytes 12-15: right (float)
//                        - bytes 16-19: top (float)
//                        - bytes 20-23: font_size (float)
//          buflen      - Size of buffer in bytes.
// Return value:
//          Number of characters extracted, or if buffer is NULL, number of
//          characters available. Returns -1 on error.
// Comments:
//          This is a lower-level API that extracts per-character data without
//          grouping into cells. Useful for custom text layout analysis.
//
//          Use FPDFText_CountChars to determine the number of characters.
//
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_ExtractAllChars(FPDF_TEXTPAGE text_page,
                         void* buffer,
                         int buflen);

// Function: FPDFText_CountWords
//          Get the number of words on a page.
// Parameters:
//          text_page - Handle to a text page. Returned by FPDFText_LoadPage.
// Return value:
//          Number of words on the page, or -1 on error.
// Comments:
//          Words are sequences of non-whitespace characters separated by
//          whitespace or large gaps. This function counts words for buffer
//          allocation before calling FPDFText_ExtractWords.
//
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_CountWords(FPDF_TEXTPAGE text_page);

// Function: FPDFText_ExtractWords
//          Extract all words from a page in a single call.
// Parameters:
//          text_page         - Handle to a text page. Returned by FPDFText_LoadPage.
//          words             - Pre-allocated array to receive word information.
//                              Array size should be at least FPDFText_CountWords.
//          max_words         - Size of the words array.
//          text_buffer       - Pre-allocated buffer to receive UTF-16LE text.
//                              Use FPDFText_CountChars to determine size.
//          text_buffer_chars - Size of text_buffer in UTF-16 code units.
// Return value:
//          Number of words extracted on success, -1 on error.
// Comments:
//          This function extracts ALL words from a page in one call.
//          Words are separated by whitespace characters (space, tab, newline)
//          or by large gaps between characters.
//
//          Each word's text_offset and text_length point into the shared
//          text_buffer. The text is UTF-16LE encoded.
//
//          Example usage:
//            int word_count = FPDFText_CountWords(text_page);
//            int char_count = FPDFText_CountChars(text_page);
//
//            FPDF_WORD_INFO* words = malloc(word_count * sizeof(*words));
//            unsigned short* text = malloc((char_count + 1) * sizeof(*text));
//
//            int extracted = FPDFText_ExtractWords(text_page, words,
//                                                   word_count, text,
//                                                   char_count + 1);
//
//            for (int i = 0; i < extracted; i++) {
//              // Process words[i], text starts at text + words[i].text_offset
//            }
//
//            free(words);
//            free(text);
//
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_ExtractWords(FPDF_TEXTPAGE text_page,
                      FPDF_WORD_INFO* words,
                      int max_words,
                      unsigned short* text_buffer,
                      int text_buffer_chars);

#ifdef __cplusplus
}
#endif

#endif  // PUBLIC_FPDF_TEXT_BATCH_H_
