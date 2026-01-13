// PDFium Bridge - Simple C API for Rust FFI
// Handles form callbacks internally, exposes simple interface

#include "public/fpdfview.h"
#include "public/fpdf_formfill.h"
#include "public/fpdf_text.h"
#include "public/fpdf_progressive.h"
#include "public/fpdf_edit.h"

#include <climits>  // FIX #6/#7 (N=30): For INT_MAX
#include <cstdint>  // FIX #6/#7 (N=30): For int64_t
#include <memory>
#include <new>      // FIX #8 (N=30): For std::nothrow
#include <string>
#include <vector>

// Simple C API (no callbacks!)
#ifdef __GNUC__
#define BRIDGE_EXPORT __attribute__((visibility("default")))
#else
#define BRIDGE_EXPORT
#endif

extern "C" {

// Structure to hold rendering result
struct RenderResult {
    unsigned char* pixels;  // RGB data
    int width;
    int height;
    int size;  // Total bytes
};

// Form callbacks (internal to C++)
namespace {

// Progressive rendering pause callback (matches upstream pdfium_test.cc)
// Always returns true to force rendering to break whenever possible
FPDF_BOOL NeedToPauseNow(IFSDK_PAUSE* p) {
    return true;
}

class FormCallbacks {
public:
    FPDF_DOCUMENT doc_;

    static FPDF_PAGE FFI_GetPage(FPDF_FORMFILLINFO* pThis,
                                  FPDF_DOCUMENT document,
                                  int page_index) {
        return FPDF_LoadPage(document, page_index);
    }

    static void FFI_ExecuteNamedAction(FPDF_FORMFILLINFO* pThis,
                                       FPDF_BYTESTRING named_action) {
        // No-op for non-interactive rendering
    }
};

}  // namespace

// Document context for batch rendering
struct DocumentContext {
    FPDF_DOCUMENT doc;
    FPDF_FORMHANDLE form;
    FPDF_FORMFILLINFO form_fill_info;  // Must stay alive while form is active
    int page_count;
};

// Initialize library (call once)
BRIDGE_EXPORT void pdfium_bridge_init() {
    FPDF_LIBRARY_CONFIG config = {};
    config.version = 4;  // Match upstream pdfium_test (testing/pdfium_test.cc:1933)
    config.m_pUserFontPaths = nullptr;
    config.m_pIsolate = nullptr;
    config.m_v8EmbedderSlot = 0;
    config.m_pPlatform = nullptr;  // Required for version 4
    config.m_RendererType = FPDF_RENDERERTYPE_AGG;
    FPDF_InitLibraryWithConfig(&config);
}

// Open document for batch rendering (more efficient than render_page per call)
BRIDGE_EXPORT DocumentContext* pdfium_bridge_open_document(const char* pdf_path) {
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, nullptr);
    if (!doc) {
        return nullptr;
    }

    // FIX #8 (N=30): Handle allocation failure with std::nothrow
    DocumentContext* ctx = new (std::nothrow) DocumentContext();
    if (!ctx) {
        FPDF_CloseDocument(doc);
        return nullptr;
    }
    ctx->doc = doc;
    ctx->page_count = FPDF_GetPageCount(doc);

    // Setup form callbacks (stored in ctx so they stay alive)
    // Version 1 because XFA is disabled (pdf_enable_xfa = false)
    ctx->form_fill_info = {};
    ctx->form_fill_info.version = 1;
    ctx->form_fill_info.FFI_GetPage = FormCallbacks::FFI_GetPage;
    ctx->form_fill_info.FFI_ExecuteNamedAction = FormCallbacks::FFI_ExecuteNamedAction;

    FPDF_FORMHANDLE form = FPDFDOC_InitFormFillEnvironment(doc, &ctx->form_fill_info);

    // Configure form
    if (form) {
        FPDF_SetFormFieldHighlightColor(form, 0, 0xFFE4DD);
        FPDF_SetFormFieldHighlightAlpha(form, 100);
        FORM_DoDocumentJSAction(form);
        FORM_DoDocumentOpenAction(form);
    }

    ctx->form = form;

    return ctx;
}

// Get page count from open document
BRIDGE_EXPORT int pdfium_bridge_get_page_count(DocumentContext* ctx) {
    if (!ctx) return 0;
    return ctx->page_count;
}

// Render page from open document (for batch rendering)
BRIDGE_EXPORT RenderResult* pdfium_bridge_render_page_from_doc(
    DocumentContext* ctx,
    int page_index,
    double dpi) {
    if (!ctx || !ctx->doc) {
        return nullptr;
    }

    // FIX #10 (N=30): Early bounds check for page_index
    // Prevents resource leaks on invalid page index
    if (page_index < 0 || page_index >= ctx->page_count) {
        return nullptr;
    }

    FPDF_DOCUMENT doc = ctx->doc;
    FPDF_FORMHANDLE form = ctx->form;

    // Load page
    FPDF_PAGE page = FPDF_LoadPage(doc, page_index);
    if (!page) {
        return nullptr;
    }

    // Form page callbacks
    if (form) {
        FORM_OnAfterLoadPage(page, form);
        FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_OPEN);
    }

    // Get dimensions
    double width_pts = FPDF_GetPageWidthF(page);
    double height_pts = FPDF_GetPageHeightF(page);

    // CRITICAL: Truncate scale to 6 decimals to match upstream pdfium_test
    // Otherwise dimensions differ by 1px (2550 vs 2549)
    double scale = dpi / 72.0;
    scale = static_cast<int>(scale * 1000000.0) / 1000000.0;

    double width_d = width_pts * scale;
    double height_d = height_pts * scale;

    // FIX #6 (N=30): Bounds check pixel dimensions before int cast
    // Prevents overflow on malicious PDFs with extreme dimensions
    if (width_d <= 0 || width_d > INT_MAX / 4 ||
        height_d <= 0 || height_d > INT_MAX / 4) {
        if (form) {
            FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
            FORM_OnBeforeClosePage(page, form);
        }
        FPDF_ClosePage(page);
        return nullptr;
    }

    int width_px = static_cast<int>(width_d);
    int height_px = static_cast<int>(height_d);

    // FIX #6 (N=30): Verify stride won't overflow
    if (width_px > INT_MAX / 4) {
        if (form) {
            FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
            FORM_OnBeforeClosePage(page, form);
        }
        FPDF_ClosePage(page);
        return nullptr;
    }

    // CRITICAL: Use CreateEx with format matching upstream pdfium_test
    // FPDFBitmap_Create uses default format which differs from pdfium_test
    // Reference: testing/pdfium_test.cc InitializeBitmap()
    bool has_transparency = FPDFPage_HasTransparency(page);
    int format = has_transparency ? FPDFBitmap_BGRA : FPDFBitmap_BGRx;

    // CRITICAL: Use explicit stride to match upstream (width * sizeof(uint32_t))
    // Reference: testing/pdfium_test.cc:1027
    int stride = width_px * static_cast<int>(sizeof(uint32_t));

    FPDF_BITMAP bitmap = FPDFBitmap_CreateEx(
        width_px, height_px, format,
        nullptr,  // Let PDFium allocate buffer
        stride    // Match upstream stride calculation
    );
    if (!bitmap) {
        if (form) {
            FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
            FORM_OnBeforeClosePage(page, form);
        }
        FPDF_ClosePage(page);
        return nullptr;
    }

    // Fill with appropriate background (matches upstream)
    FPDF_DWORD fill_color = has_transparency ? 0x00000000 : 0xFFFFFFFF;
    FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);

    // Try one-shot rendering instead of progressive (test hypothesis)
    // Reference: testing/pdfium_test.cc OneShotBitmapPageRenderer::Start (line 1073)
    FPDF_RenderPageBitmap(
        bitmap, page,
        0, 0,  // start_x, start_y
        width_px, height_px,  // size_x, size_y
        0,  // rotate
        FPDF_ANNOT  // flags
    );

    // Form cleanup
    if (form) {
        FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
        FORM_OnBeforeClosePage(page, form);
    }

    // Get buffer
    void* buffer = FPDFBitmap_GetBuffer(bitmap);
    int actual_stride = FPDFBitmap_GetStride(bitmap);

    // FIX #7 (N=30): Compute rgb_size in int64_t to prevent overflow
    int64_t rgb_size_64 = static_cast<int64_t>(width_px) * height_px * 3;
    if (rgb_size_64 <= 0 || rgb_size_64 > INT_MAX) {
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        return nullptr;
    }
    int rgb_size = static_cast<int>(rgb_size_64);

    // FIX #8 (N=30): Handle allocation failure with std::nothrow
    unsigned char* rgb_data = new (std::nothrow) unsigned char[rgb_size];
    if (!rgb_data) {
        // FIX #9 (N=30): Proper cleanup on allocation failure
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        return nullptr;
    }

    const unsigned char* src = static_cast<const unsigned char*>(buffer);
    for (int y = 0; y < height_px; ++y) {
        const unsigned char* src_row = src + (y * actual_stride);
        unsigned char* dst_row = rgb_data + (y * width_px * 3);
        for (int x = 0; x < width_px; ++x) {
            dst_row[x * 3 + 0] = src_row[x * 4 + 2];  // R
            dst_row[x * 3 + 1] = src_row[x * 4 + 1];  // G
            dst_row[x * 3 + 2] = src_row[x * 4 + 0];  // B
        }
    }

    // FIX #8 (N=30): Handle allocation failure with std::nothrow
    RenderResult* result = new (std::nothrow) RenderResult();
    if (!result) {
        // FIX #9 (N=30): Cleanup rgb_data on allocation failure
        delete[] rgb_data;
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        return nullptr;
    }
    result->pixels = rgb_data;
    result->width = width_px;
    result->height = height_px;
    result->size = rgb_size;

    // Cleanup
    FPDFBitmap_Destroy(bitmap);
    FPDF_ClosePage(page);

    return result;
}

// Close document context
BRIDGE_EXPORT void pdfium_bridge_close_document(DocumentContext* ctx) {
    if (!ctx) return;

    if (ctx->form) {
        FORM_DoDocumentAAction(ctx->form, FPDFDOC_AACTION_WC);
        FPDFDOC_ExitFormFillEnvironment(ctx->form);
    }
    if (ctx->doc) {
        FPDF_CloseDocument(ctx->doc);
    }
    delete ctx;
}

// Render single page to RGB buffer
BRIDGE_EXPORT RenderResult* pdfium_bridge_render_page(const char* pdf_path,
                                                        int page_index,
                                                        double dpi) {
    // Load document
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, nullptr);
    if (!doc) {
        return nullptr;
    }

    // FIX #10 (N=30): Early bounds check for page_index
    // Prevents resource leaks on invalid page index
    int page_count = FPDF_GetPageCount(doc);
    if (page_index < 0 || page_index >= page_count) {
        FPDF_CloseDocument(doc);
        return nullptr;
    }

    // Setup form callbacks
    // Version 1 because XFA is disabled (pdf_enable_xfa = false)
    FPDF_FORMFILLINFO form_fill_info = {};
    form_fill_info.version = 1;
    form_fill_info.FFI_GetPage = FormCallbacks::FFI_GetPage;
    form_fill_info.FFI_ExecuteNamedAction = FormCallbacks::FFI_ExecuteNamedAction;

    FPDF_FORMHANDLE form = FPDFDOC_InitFormFillEnvironment(doc, &form_fill_info);

    // Configure form
    if (form) {
        FPDF_SetFormFieldHighlightColor(form, 0, 0xFFE4DD);
        FPDF_SetFormFieldHighlightAlpha(form, 100);
        FORM_DoDocumentJSAction(form);
        FORM_DoDocumentOpenAction(form);
    }

    // Load page
    FPDF_PAGE page = FPDF_LoadPage(doc, page_index);
    if (!page) {
        if (form) FPDFDOC_ExitFormFillEnvironment(form);
        FPDF_CloseDocument(doc);
        return nullptr;
    }

    // Form page callbacks
    if (form) {
        FORM_OnAfterLoadPage(page, form);
        FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_OPEN);
    }

    // Get dimensions
    double width_pts = FPDF_GetPageWidthF(page);
    double height_pts = FPDF_GetPageHeightF(page);

    // CRITICAL: Truncate scale to 6 decimals to match upstream pdfium_test
    // Otherwise dimensions differ by 1px (2550 vs 2549)
    double scale = dpi / 72.0;
    scale = static_cast<int>(scale * 1000000.0) / 1000000.0;

    double width_d = width_pts * scale;
    double height_d = height_pts * scale;

    // FIX #6 (N=30): Bounds check pixel dimensions before int cast
    // Prevents overflow on malicious PDFs with extreme dimensions
    if (width_d <= 0 || width_d > INT_MAX / 4 ||
        height_d <= 0 || height_d > INT_MAX / 4) {
        if (form) {
            FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
            FORM_OnBeforeClosePage(page, form);
        }
        FPDF_ClosePage(page);
        if (form) FPDFDOC_ExitFormFillEnvironment(form);
        FPDF_CloseDocument(doc);
        return nullptr;
    }

    int width_px = static_cast<int>(width_d);
    int height_px = static_cast<int>(height_d);

    // FIX #6 (N=30): Verify stride won't overflow
    if (width_px > INT_MAX / 4) {
        if (form) {
            FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
            FORM_OnBeforeClosePage(page, form);
        }
        FPDF_ClosePage(page);
        if (form) FPDFDOC_ExitFormFillEnvironment(form);
        FPDF_CloseDocument(doc);
        return nullptr;
    }

    // CRITICAL: Use CreateEx with format matching upstream pdfium_test
    bool has_transparency = FPDFPage_HasTransparency(page);
    int format = has_transparency ? FPDFBitmap_BGRA : FPDFBitmap_BGRx;

    // CRITICAL: Use explicit stride to match upstream (width * sizeof(uint32_t))
    // Reference: testing/pdfium_test.cc:1027
    int stride = width_px * static_cast<int>(sizeof(uint32_t));

    FPDF_BITMAP bitmap = FPDFBitmap_CreateEx(
        width_px, height_px, format,
        nullptr,  // Let PDFium allocate buffer
        stride    // Match upstream stride calculation
    );
    if (!bitmap) {
        if (form) {
            FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
            FORM_OnBeforeClosePage(page, form);
        }
        FPDF_ClosePage(page);
        if (form) FPDFDOC_ExitFormFillEnvironment(form);
        FPDF_CloseDocument(doc);
        return nullptr;
    }

    // Fill with appropriate background (matches upstream)
    FPDF_DWORD fill_color = has_transparency ? 0x00000000 : 0xFFFFFFFF;
    FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);

    // Try one-shot rendering instead of progressive (test hypothesis)
    // Reference: testing/pdfium_test.cc OneShotBitmapPageRenderer::Start (line 1073)
    FPDF_RenderPageBitmap(
        bitmap, page,
        0, 0,  // start_x, start_y
        width_px, height_px,  // size_x, size_y
        0,  // rotate
        FPDF_ANNOT  // flags
    );

    // Form cleanup
    if (form) {
        FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
        FORM_OnBeforeClosePage(page, form);
    }

    // Get buffer
    void* buffer = FPDFBitmap_GetBuffer(bitmap);
    int actual_stride = FPDFBitmap_GetStride(bitmap);

    // FIX #7 (N=30): Compute rgb_size in int64_t to prevent overflow
    int64_t rgb_size_64 = static_cast<int64_t>(width_px) * height_px * 3;
    if (rgb_size_64 <= 0 || rgb_size_64 > INT_MAX) {
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        if (form) {
            FORM_DoDocumentAAction(form, FPDFDOC_AACTION_WC);
            FPDFDOC_ExitFormFillEnvironment(form);
        }
        FPDF_CloseDocument(doc);
        return nullptr;
    }
    int rgb_size = static_cast<int>(rgb_size_64);

    // FIX #8 (N=30): Handle allocation failure with std::nothrow
    unsigned char* rgb_data = new (std::nothrow) unsigned char[rgb_size];
    if (!rgb_data) {
        // FIX #9 (N=30): Proper cleanup on allocation failure
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        if (form) {
            FORM_DoDocumentAAction(form, FPDFDOC_AACTION_WC);
            FPDFDOC_ExitFormFillEnvironment(form);
        }
        FPDF_CloseDocument(doc);
        return nullptr;
    }

    const unsigned char* src = static_cast<const unsigned char*>(buffer);
    for (int y = 0; y < height_px; ++y) {
        const unsigned char* src_row = src + (y * actual_stride);
        unsigned char* dst_row = rgb_data + (y * width_px * 3);
        for (int x = 0; x < width_px; ++x) {
            dst_row[x * 3 + 0] = src_row[x * 4 + 2];  // R
            dst_row[x * 3 + 1] = src_row[x * 4 + 1];  // G
            dst_row[x * 3 + 2] = src_row[x * 4 + 0];  // B
        }
    }

    // FIX #8 (N=30): Handle allocation failure with std::nothrow
    RenderResult* result = new (std::nothrow) RenderResult();
    if (!result) {
        // FIX #9 (N=30): Cleanup rgb_data on allocation failure
        delete[] rgb_data;
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);
        if (form) {
            FORM_DoDocumentAAction(form, FPDFDOC_AACTION_WC);
            FPDFDOC_ExitFormFillEnvironment(form);
        }
        FPDF_CloseDocument(doc);
        return nullptr;
    }
    result->pixels = rgb_data;
    result->width = width_px;
    result->height = height_px;
    result->size = rgb_size;

    // Cleanup
    FPDFBitmap_Destroy(bitmap);
    FPDF_ClosePage(page);
    if (form) {
        FORM_DoDocumentAAction(form, FPDFDOC_AACTION_WC);
        FPDFDOC_ExitFormFillEnvironment(form);
    }
    FPDF_CloseDocument(doc);

    return result;
}

// Free result
BRIDGE_EXPORT void pdfium_bridge_free_result(RenderResult* result) {
    if (result) {
        delete[] result->pixels;
        delete result;
    }
}

// Cleanup library
BRIDGE_EXPORT void pdfium_bridge_destroy() {
    FPDF_DestroyLibrary();
}

}  // extern "C"
