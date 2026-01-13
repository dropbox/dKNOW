// Reference C++ tool for JSONL extraction with character metadata
// Purpose: Validate that Rust extract_text_jsonl produces identical output
// Calls exact same 13 FPDFText_* APIs as rust/pdfium-sys/examples/extract_text_jsonl.rs

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#include "public/fpdfview.h"
#include "public/fpdf_text.h"

// JSON string escaping
void write_json_string(FILE* out, unsigned int codepoint) {
    switch (codepoint) {
        case '"':
            fprintf(out, "\\\"");
            break;
        case '\\':
            fprintf(out, "\\\\");
            break;
        case '\n':
            fprintf(out, "\\n");
            break;
        case '\r':
            fprintf(out, "\\r");
            break;
        case '\t':
            fprintf(out, "\\t");
            break;
        default:
            if (codepoint < 0x20) {
                fprintf(out, "\\u%04x", codepoint);
            } else if (codepoint <= 0x7F) {
                fprintf(out, "%c", (char)codepoint);
            } else if (codepoint <= 0xFFFF) {
                fprintf(out, "\\u%04x", codepoint);
            } else {
                // Encode as UTF-8 for JSON
                fprintf(out, "\\u%04x", codepoint);
            }
            break;
    }
}

int main(int argc, char* argv[]) {
    if (argc < 3 || argc > 4) {
        fprintf(stderr, "Usage: %s <input.pdf> <output.jsonl> [page_number]\n", argv[0]);
        fprintf(stderr, "  page_number: Extract single page (0-indexed, default: page 0)\n");
        return 1;
    }

    const char* pdf_path = argv[1];
    const char* output_path = argv[2];
    int page_num = (argc == 4) ? atoi(argv[3]) : 0;

    // Initialize PDFium
    FPDF_InitLibrary();

    // Load document
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Error: Failed to load PDF: %s\n", pdf_path);
        FPDF_DestroyLibrary();
        return 1;
    }

    int page_count = FPDF_GetPageCount(doc);
    if (page_num < 0 || page_num >= page_count) {
        fprintf(stderr, "Error: Invalid page number %d (document has %d pages)\n", page_num, page_count);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    // Open output file
    FILE* out = fopen(output_path, "w");
    if (!out) {
        fprintf(stderr, "Error: Failed to create output file: %s\n", output_path);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    // Load page
    FPDF_PAGE page = FPDF_LoadPage(doc, page_num);
    if (!page) {
        fprintf(stderr, "Error: Failed to load page %d\n", page_num);
        fclose(out);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    // Load text page
    FPDF_TEXTPAGE text_page = FPDFText_LoadPage(page);
    if (!text_page) {
        fprintf(stderr, "Error: Failed to load text for page %d\n", page_num);
        FPDF_ClosePage(page);
        fclose(out);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    int char_count = FPDFText_CountChars(text_page);
    fprintf(stderr, "Extracting %d characters from page %d\n", char_count, page_num);

    // Extract each character with metadata
    int i = 0;
    while (i < char_count) {
        // 1. Get Unicode character
        unsigned int unicode = FPDFText_GetUnicode(text_page, i);

        // Handle UTF-16 surrogate pairs (same logic as Rust)
        unsigned int codepoint;
        int chars_consumed;

        if (unicode >= 0xD800 && unicode <= 0xDBFF) {
            // High surrogate
            if (i + 1 < char_count) {
                unsigned int low = FPDFText_GetUnicode(text_page, i + 1);
                if (low >= 0xDC00 && low <= 0xDFFF) {
                    unsigned int high = unicode;
                    codepoint = ((high - 0xD800) << 10) + (low - 0xDC00) + 0x10000;
                    chars_consumed = 2;
                } else {
                    codepoint = 0xFFFD;
                    chars_consumed = 1;
                }
            } else {
                codepoint = 0xFFFD;
                chars_consumed = 1;
            }
        } else {
            codepoint = unicode;
            chars_consumed = 1;
        }

        // 2. Get bounding box
        double left, right, bottom, top;
        FPDFText_GetCharBox(text_page, i, &left, &right, &bottom, &top);

        // 3. Get origin
        double origin_x, origin_y;
        FPDFText_GetCharOrigin(text_page, i, &origin_x, &origin_y);

        // 4. Get font size
        double font_size = FPDFText_GetFontSize(text_page, i);

        // 5. Get font info
        char font_name[256] = {0};
        int font_flags = 0;
        unsigned long font_name_len = FPDFText_GetFontInfo(text_page, i, font_name, sizeof(font_name), &font_flags);
        if (font_name_len == 0) {
            strcpy(font_name, "unknown");
        }

        // 6. Get font weight
        int font_weight = FPDFText_GetFontWeight(text_page, i);

        // 7. Get fill color
        unsigned int fill_r, fill_g, fill_b, fill_a;
        FPDFText_GetFillColor(text_page, i, &fill_r, &fill_g, &fill_b, &fill_a);

        // 8. Get stroke color
        unsigned int stroke_r, stroke_g, stroke_b, stroke_a;
        FPDFText_GetStrokeColor(text_page, i, &stroke_r, &stroke_g, &stroke_b, &stroke_a);

        // 9. Get rotation angle
        float angle = FPDFText_GetCharAngle(text_page, i);

        // 10. Get transformation matrix
        FS_MATRIX matrix;
        FPDFText_GetMatrix(text_page, i, &matrix);

        // 11. Check if generated
        int is_generated = FPDFText_IsGenerated(text_page, i);

        // 12. Check if hyphen
        int is_hyphen = FPDFText_IsHyphen(text_page, i);

        // 13. Check for unicode mapping error
        int has_unicode_error = FPDFText_HasUnicodeMapError(text_page, i);

        // Escape font name for JSON
        char font_name_escaped[512] = {0};
        int out_idx = 0;
        size_t font_len = strlen(font_name);
        for (size_t j = 0; j < font_len && out_idx < 510; j++) {
            if (font_name[j] == '\\') {
                font_name_escaped[out_idx++] = '\\';
                font_name_escaped[out_idx++] = '\\';
            } else if (font_name[j] == '"') {
                font_name_escaped[out_idx++] = '\\';
                font_name_escaped[out_idx++] = '"';
            } else {
                font_name_escaped[out_idx++] = font_name[j];
            }
        }

        // Write JSON line
        fprintf(out, "{\"char\":\"");
        write_json_string(out, codepoint);
        fprintf(out, "\",\"unicode\":%u,\"bbox\":[%.17g,%.17g,%.17g,%.17g],\"origin\":[%.17g,%.17g],\"font_size\":%.17g,\"font_name\":\"%s\",\"font_flags\":%d,\"font_weight\":%d,\"fill_color\":[%u,%u,%u,%u],\"stroke_color\":[%u,%u,%u,%u],\"angle\":%.17g,\"matrix\":[%.17g,%.17g,%.17g,%.17g,%.17g,%.17g],\"is_generated\":%s,\"is_hyphen\":%s,\"has_unicode_error\":%s}\n",
            codepoint,
            left, bottom, right, top,
            origin_x, origin_y,
            font_size,
            font_name_escaped,
            font_flags,
            font_weight,
            fill_r, fill_g, fill_b, fill_a,
            stroke_r, stroke_g, stroke_b, stroke_a,
            angle,
            matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f,
            (is_generated == 1) ? "true" : "false",
            (is_hyphen == 1) ? "true" : "false",
            (has_unicode_error == 1) ? "true" : "false"
        );

        i += chars_consumed;
    }

    FPDFText_ClosePage(text_page);
    FPDF_ClosePage(page);

    fclose(out);
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    fprintf(stderr, "JSONL extraction complete: %s\n", output_path);
    return 0;
}
