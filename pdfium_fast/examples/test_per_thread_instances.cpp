// Test: Per-Thread PDFium Instances
// Purpose: Verify that each thread can safely call FPDF_InitLibrary()
// Decision Gate: If this crashes, STOP threading work and keep multi-process
//
// Usage: test_per_thread_instances <input.pdf> <num_threads>
// Example: test_per_thread_instances document.pdf 4
//
// Expected behavior:
// ✅ SUCCESS: Multiple threads render pages simultaneously without crashes
// ❌ FAIL: Crashes or wrong output → Threading NOT viable

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <thread>
#include <vector>
#include <atomic>
#include <chrono>

#include "public/fpdfview.h"
#include "public/fpdf_text.h"

// Thread-safe counters for reporting
std::atomic<int> pages_processed(0);
std::atomic<int> pages_failed(0);

// Thread worker: Each thread gets its own PDFium instance
void worker_thread(const char* pdf_path, int thread_id, int num_threads, int total_pages) {
    // CRITICAL TEST: Each thread initializes its own PDFium instance
    // If this crashes or causes data races, threading is NOT viable
    FPDF_InitLibrary();

    // Load document (each thread loads independently)
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Thread %d: Failed to load PDF\n", thread_id);
        FPDF_DestroyLibrary();
        return;
    }

    int page_count = FPDF_GetPageCount(doc);

    // Process assigned pages (round-robin distribution)
    for (int page_idx = thread_id; page_idx < page_count; page_idx += num_threads) {
        // Load page
        FPDF_PAGE page = FPDF_LoadPage(doc, page_idx);
        if (!page) {
            fprintf(stderr, "Thread %d: Failed to load page %d\n", thread_id, page_idx);
            pages_failed++;
            continue;
        }

        // Get page dimensions
        double width = FPDF_GetPageWidthF(page);
        double height = FPDF_GetPageHeightF(page);

        // Render at 72 DPI (1:1 scale) for this test
        int render_width = (int)(width);
        int render_height = (int)(height);

        // Create bitmap
        FPDF_BITMAP bitmap = FPDFBitmap_Create(render_width, render_height, 0);
        if (!bitmap) {
            fprintf(stderr, "Thread %d: Failed to create bitmap for page %d\n", thread_id, page_idx);
            FPDF_ClosePage(page);
            pages_failed++;
            continue;
        }

        // Fill with white background
        FPDFBitmap_FillRect(bitmap, 0, 0, render_width, render_height, 0xFFFFFFFF);

        // Render page
        FPDF_RenderPageBitmap(bitmap, page, 0, 0, render_width, render_height, 0, FPDF_ANNOT);

        // Get bitmap buffer (validate rendering worked)
        void* buffer = FPDFBitmap_GetBuffer(bitmap);
        int stride = FPDFBitmap_GetStride(bitmap);

        // Simple validation: Check buffer is not null and not all white
        bool has_content = false;
        if (buffer) {
            unsigned char* data = (unsigned char*)buffer;
            // Sample a few pixels to verify rendering occurred
            for (int i = 0; i < stride * render_height && i < 1000; i += 4) {
                if (data[i] != 0xFF || data[i+1] != 0xFF || data[i+2] != 0xFF) {
                    has_content = true;
                    break;
                }
            }
        }

        if (!has_content) {
            fprintf(stderr, "Thread %d: Warning - Page %d appears blank\n", thread_id, page_idx);
        }

        // Cleanup
        FPDFBitmap_Destroy(bitmap);
        FPDF_ClosePage(page);

        pages_processed++;
    }

    // Cleanup
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    fprintf(stderr, "Thread %d: Completed successfully\n", thread_id);
}

int main(int argc, char* argv[]) {
    if (argc != 3) {
        fprintf(stderr, "Usage: %s <input.pdf> <num_threads>\n", argv[0]);
        fprintf(stderr, "Example: %s document.pdf 4\n", argv[0]);
        return 1;
    }

    const char* pdf_path = argv[1];
    int num_threads = atoi(argv[2]);

    if (num_threads < 1 || num_threads > 32) {
        fprintf(stderr, "Error: num_threads must be between 1 and 32\n");
        return 1;
    }

    fprintf(stderr, "\n========================================\n");
    fprintf(stderr, "Per-Thread PDFium Instance Test\n");
    fprintf(stderr, "========================================\n");
    fprintf(stderr, "PDF: %s\n", pdf_path);
    fprintf(stderr, "Threads: %d\n", num_threads);
    fprintf(stderr, "Testing: Each thread calls FPDF_InitLibrary()\n");
    fprintf(stderr, "========================================\n\n");

    // Get page count (single-threaded pre-check)
    FPDF_InitLibrary();
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Error: Failed to load PDF: %s\n", pdf_path);
        FPDF_DestroyLibrary();
        return 1;
    }
    int total_pages = FPDF_GetPageCount(doc);
    fprintf(stderr, "Total pages: %d\n", total_pages);
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    // Launch threads
    fprintf(stderr, "Launching %d threads...\n\n", num_threads);

    auto start_time = std::chrono::high_resolution_clock::now();

    std::vector<std::thread> threads;
    for (int i = 0; i < num_threads; i++) {
        threads.emplace_back(worker_thread, pdf_path, i, num_threads, total_pages);
    }

    // Wait for all threads
    for (auto& t : threads) {
        t.join();
    }

    auto end_time = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end_time - start_time);

    // Report results
    fprintf(stderr, "\n========================================\n");
    fprintf(stderr, "Test Results\n");
    fprintf(stderr, "========================================\n");
    fprintf(stderr, "Pages processed: %d / %d\n", pages_processed.load(), total_pages);
    fprintf(stderr, "Pages failed: %d\n", pages_failed.load());
    fprintf(stderr, "Duration: %lld ms\n", (long long)duration.count());
    fprintf(stderr, "========================================\n\n");

    if (pages_processed.load() == total_pages && pages_failed.load() == 0) {
        fprintf(stderr, "✅ SUCCESS: Per-thread PDFium instances work!\n");
        fprintf(stderr, "Next: Proceed to stress testing (N=3)\n\n");
        return 0;
    } else {
        fprintf(stderr, "❌ FAILURE: Some pages failed to render\n");
        fprintf(stderr, "Investigate: Race conditions or resource issues\n\n");
        return 1;
    }
}
