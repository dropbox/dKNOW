// Reference C++ tool for text extraction
// Purpose: Validate that Rust tools produce identical output
// Calls exact same FPDFText APIs as rust/pdfium-sys/examples/extract_text.rs

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "public/fpdfview.h"
#include "public/fpdf_text.h"

// Write UTF-32 LE BOM
void write_bom(FILE* out) {
    unsigned char bom[4] = {0xFF, 0xFE, 0x00, 0x00};
    fwrite(bom, 1, 4, out);
}

// Write codepoint as UTF-32 LE
void write_codepoint(FILE* out, unsigned int codepoint) {
    unsigned char bytes[4];
    bytes[0] = (codepoint) & 0xFF;
    bytes[1] = (codepoint >> 8) & 0xFF;
    bytes[2] = (codepoint >> 16) & 0xFF;
    bytes[3] = (codepoint >> 24) & 0xFF;
    fwrite(bytes, 1, 4, out);
}

int main(int argc, char* argv[]) {
    if (argc != 3) {
        fprintf(stderr, "Usage: %s <input.pdf> <output.txt>\n", argv[0]);
        return 1;
    }

    const char* pdf_path = argv[1];
    const char* output_path = argv[2];

    // Initialize PDFium
    FPDF_InitLibrary();

    // Load document
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Error: Failed to load PDF: %s\n", pdf_path);
        FPDF_DestroyLibrary();
        return 1;
    }

    // Open output file
    FILE* out = fopen(output_path, "wb");
    if (!out) {
        fprintf(stderr, "Error: Failed to create output file: %s\n", output_path);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    // Write initial BOM
    write_bom(out);

    int page_count = FPDF_GetPageCount(doc);

    // Extract each page
    for (int page_idx = 0; page_idx < page_count; page_idx++) {
        FPDF_PAGE page = FPDF_LoadPage(doc, page_idx);
        if (!page) {
            fprintf(stderr, "Error: Failed to load page %d\n", page_idx);
            continue;
        }

        FPDF_TEXTPAGE text_page = FPDFText_LoadPage(page);
        if (!text_page) {
            fprintf(stderr, "Error: Failed to load text for page %d\n", page_idx);
            FPDF_ClosePage(page);
            continue;
        }

        int char_count = FPDFText_CountChars(text_page);

        // Extract each character
        int i = 0;
        while (i < char_count) {
            unsigned int unicode = FPDFText_GetUnicode(text_page, i);

            // Handle UTF-16 surrogate pairs (same logic as Rust)
            unsigned int codepoint;
            int chars_consumed;

            if (unicode >= 0xD800 && unicode <= 0xDBFF) {
                // High surrogate - need low surrogate
                if (i + 1 < char_count) {
                    unsigned int low = FPDFText_GetUnicode(text_page, i + 1);
                    if (low >= 0xDC00 && low <= 0xDFFF) {
                        // Valid surrogate pair
                        unsigned int high = unicode;
                        codepoint = ((high - 0xD800) << 10) + (low - 0xDC00) + 0x10000;
                        chars_consumed = 2;
                    } else {
                        // Invalid surrogate pair
                        codepoint = 0xFFFD;  // Replacement character
                        chars_consumed = 1;
                    }
                } else {
                    // High surrogate at end
                    codepoint = 0xFFFD;
                    chars_consumed = 1;
                }
            } else {
                // Regular character
                codepoint = unicode;
                chars_consumed = 1;
            }

            write_codepoint(out, codepoint);
            i += chars_consumed;
        }

        FPDFText_ClosePage(text_page);
        FPDF_ClosePage(page);

        // Write page separator BOM
        if (page_idx < page_count - 1) {
            write_bom(out);
        }
    }

    fclose(out);
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    fprintf(stderr, "Text extraction complete: %s\n", output_path);
    return 0;
}
