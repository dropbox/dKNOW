// Copyright 2025 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// High-level API for parallel PDF page rendering
// This header provides utilities to efficiently render multiple pages in parallel

#ifndef PUBLIC_FPDF_PARALLEL_H_
#define PUBLIC_FPDF_PARALLEL_H_

#include "public/fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

// Output pixel format options for parallel rendering
// Use these values for FPDF_PARALLEL_OPTIONS.output_format
#define FPDF_PARALLEL_FORMAT_DEFAULT 0   // Default BGRx (4 bytes/pixel)
#define FPDF_PARALLEL_FORMAT_BGRx 0      // 4 bytes/pixel (B,G,R,unused)
#define FPDF_PARALLEL_FORMAT_BGR 1       // 3 bytes/pixel (B,G,R) - 33% less memory
#define FPDF_PARALLEL_FORMAT_GRAY 2      // 1 byte/pixel (grayscale) - 75% less memory

// Parallel rendering options
typedef struct {
  // Number of render threads to use (0 = auto-detect based on CPU cores)
  // NOTE: These are threads within a single process, NOT separate processes
  int worker_count;  // DEPRECATED NAME: Use "thread_count" terminology in new code

  // Maximum pages to queue per thread (0 = unlimited)
  int max_queue_size;

  // Form handle for rendering form fields (optional, NULL if no forms)
  // N=200: Must be provided to match single-threaded rendering behavior
  void* form_handle;  // FPDF_FORMHANDLE

  // N=118: DPI for per-page dimension calculation
  // When width=0 and height=0 in FPDF_RenderPagesParallelV2, use this DPI
  // to calculate dimensions per-page. Enables mixed-size PDF support.
  // If dpi=0 (default), width/height are required and used for all pages.
  double dpi;

  // N=31: Output pixel format for rendered bitmaps
  // 0 = FPDF_PARALLEL_FORMAT_BGRx (default, 4 bytes/pixel, backward compatible)
  // 1 = FPDF_PARALLEL_FORMAT_BGR (3 bytes/pixel, 33% memory bandwidth reduction)
  // 2 = FPDF_PARALLEL_FORMAT_GRAY (1 byte/pixel, 75% memory bandwidth reduction)
  // Note: BGR format is suitable for most use cases (RGB images, JPEG output)
  // Note: GRAY format is suitable for ML pipelines that don't need color
  int output_format;

  // Reserved for future use
  void* reserved[1];
} FPDF_PARALLEL_OPTIONS;

// Parallel rendering callback - called when a page render completes
// Parameters:
//   page_index: The page number that was rendered
//   bitmap: The rendered bitmap (owned by caller, must be freed)
//   user_data: User-provided context data
//   success: Whether rendering succeeded
typedef void (*FPDF_PARALLEL_CALLBACK)(int page_index,
                                      FPDF_BITMAP bitmap,
                                      void* user_data,
                                      FPDF_BOOL success);

// V2 callback for optimized bitmap pooling - called when a page render completes
// Parameters:
//   page_index: The page number that was rendered
//   buffer: Raw RGBA pixel data (NOT owned by caller, do NOT free)
//   width: Bitmap width in pixels
//   height: Bitmap height in pixels
//   stride: Number of bytes per row (may be larger than width*4 for alignment)
//   user_data: User-provided context data
//   success: Whether rendering succeeded
//
// IMPORTANT: buffer is only valid during the callback. Copy data if needed.
// Do NOT call FPDFBitmap_Destroy() - the library manages bitmap lifecycle.
typedef void (*FPDF_PARALLEL_CALLBACK_V2)(int page_index,
                                          const void* buffer,
                                          int width,
                                          int height,
                                          int stride,
                                          void* user_data,
                                          FPDF_BOOL success);

// Render multiple pages in parallel using multiple threads within one process
// This function creates a thread pool and renders the specified page range
// in parallel, calling the callback for each completed page.
//
// IMPORTANT: Uses threads within a single process, NOT multi-process parallelism
//
// Thread Safety:
//   - Each document should only be used by one parallel rendering operation at a time
//   - Multiple documents can be rendered in parallel safely
//   - The library must be initialized with FPDF_InitLibrary before calling this
//
// Parameters:
//   document: Handle to the document
//   start_page: First page to render (0-indexed)
//   page_count: Number of pages to render
//   width: Render width in pixels
//   height: Render height in pixels
//   rotate: Page rotation (0, 90, 180, 270 degrees)
//   flags: Rendering flags (see FPDF_RenderPageBitmap)
//   options: Parallel rendering options (NULL for defaults)
//   callback: Function to call when each page completes
//   user_data: User context passed to callback
//
// Returns:
//   FPDF_TRUE on success, FPDF_FALSE on failure
//
// Example:
//   void render_complete(int page, FPDF_BITMAP bitmap, void* data, FPDF_BOOL ok) {
//     if (ok) {
//       save_bitmap_to_file(bitmap, page);
//     }
//     FPDFBitmap_Destroy(bitmap);
//   }
//
//   FPDF_PARALLEL_OPTIONS opts = {0};
//   opts.worker_count = 4;  // Use 4 render threads (within this process)
//   FPDF_RenderPagesParallel(doc, 0, 100, 800, 600, 0, 0, &opts,
//                           render_complete, NULL);
//
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_RenderPagesParallel(FPDF_DOCUMENT document,
                        int start_page,
                        int page_count,
                        int width,
                        int height,
                        int rotate,
                        int flags,
                        FPDF_PARALLEL_OPTIONS* options,
                        FPDF_PARALLEL_CALLBACK callback,
                        void* user_data);

// Get optimal thread count for parallel rendering
// Returns the recommended number of render threads based on CPU cores
// NOTE: Returns thread count (threads within one process), not process count
FPDF_EXPORT int FPDF_CALLCONV FPDF_GetOptimalWorkerCount();

// Get optimal thread count for a specific document
// Returns the recommended number of render threads based on both CPU cores
// and document characteristics (page count, complexity, etc.)
// This function provides better performance by avoiding over-parallelization
// for small documents and maximizing parallelism for large documents.
//
// NOTE: Returns thread count (threads within one process), not process count
//
// Parameters:
//   document: Handle to the document
//
// Returns:
//   Optimal thread count for this document (1 to hardware_threads)
//
// Example:
//   int threads = FPDF_GetOptimalWorkerCountForDocument(doc);
//   FPDF_PARALLEL_OPTIONS opts = {0};
//   opts.worker_count = threads;  // Use recommended thread count
//   FPDF_RenderPagesParallel(doc, 0, page_count, 800, 600, 0, 0,
//                           &opts, callback, NULL);
FPDF_EXPORT int FPDF_CALLCONV
FPDF_GetOptimalWorkerCountForDocument(FPDF_DOCUMENT document);

// Explicitly clean up the global thread pool and bitmap pools
// This function should be called before FPDF_DestroyLibrary() to ensure
// proper cleanup order and avoid crashes during exit.
//
// Thread Safety:
//   - Must be called when no parallel rendering is in progress
//   - Must be called before FPDF_DestroyLibrary()
//   - Can be called multiple times safely (idempotent)
//
// Memory Impact:
//   - Frees all pooled bitmaps (~1-2MB per thread)
//   - Joins and destroys all worker threads
//
// Performance Impact:
//   - Next parallel render will recreate threads (8-14% overhead on first call)
//   - Recommended to call only once at application shutdown
//
// Example:
//   // At application shutdown
//   FPDF_DestroyThreadPool();  // Clean up threads and bitmap pools
//   FPDF_DestroyLibrary();     // Then destroy PDFium library
//
FPDF_EXPORT void FPDF_CALLCONV FPDF_DestroyThreadPool();

// Render multiple pages in parallel with optimized bitmap pooling (V2 API)
// This version uses bitmap pooling for better performance by reusing bitmap
// allocations across pages. The callback receives a raw buffer pointer instead
// of an FPDF_BITMAP handle.
//
// Performance: ~10-20% faster than V1 API due to eliminated bitmap allocation overhead
//
// Thread Safety: Same as FPDF_RenderPagesParallel
//
// Parameters:
//   Same as FPDF_RenderPagesParallel except:
//   callback: V2 callback that receives raw buffer (do NOT destroy)
//
// Returns:
//   FPDF_TRUE on success, FPDF_FALSE on failure
//
// Example:
//   void render_complete_v2(int page, const void* buf, int w, int h, int stride,
//                           void* data, FPDF_BOOL ok) {
//     if (ok) {
//       // Copy buffer immediately (only valid during callback)
//       std::vector<uint8_t> pixels(h * stride);
//       memcpy(pixels.data(), buf, h * stride);
//       save_pixels_to_file(pixels, w, h, stride, page);
//     }
//     // Do NOT call FPDFBitmap_Destroy() - library owns the bitmap
//   }
//
//   FPDF_PARALLEL_OPTIONS opts = {0};
//   opts.worker_count = 16;  // Use 16 render threads (within this process)
//   FPDF_RenderPagesParallelV2(doc, 0, 100, 1920, 1080, 0, 0, &opts,
//                             render_complete_v2, NULL);
//
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_RenderPagesParallelV2(FPDF_DOCUMENT document,
                           int start_page,
                           int page_count,
                           int width,
                           int height,
                           int rotate,
                           int flags,
                           FPDF_PARALLEL_OPTIONS* options,
                           FPDF_PARALLEL_CALLBACK_V2 callback,
                           void* user_data);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_PARALLEL_H_
