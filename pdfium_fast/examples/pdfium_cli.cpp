// Dash PDF Extraction - High-Performance PDFium CLI
// Copyright (C) 2025 Andrew Yates. All rights reserved.
//
// PDFium CLI - C++ Command-Line Interface
// Purpose: Fulfill CLAUDE.md requirement for "extremely efficient" C++ CLI
// Architecture: Three modes (bulk/fast/debug) for text extraction and image rendering
//
// Modes:
//   --workers N:  Multi-process with N workers (default 1, max 16)
//   --debug:      Tracing and diagnostic output
//
// Operations:
//   extract-text:  Text extraction to UTF-8 (v2.0.0 default)
//   render-pages:  Render pages to JPEG images (v2.0.0 default)
//
// Examples:
//   pdfium_cli extract-text input.pdf output.txt              # single-threaded (1 worker)
//   pdfium_cli --workers 4 extract-text large.pdf output.txt  # 4 workers
//   pdfium_cli --workers 8 extract-text large.pdf output.txt  # 8 workers
//   pdfium_cli --debug extract-text input.pdf output.txt      # debug mode
//   pdfium_cli --pages 1-10 extract-text input.pdf output.txt # pages 1-10 only

// Disable unsafe buffer warnings for official builds (N=526)
// argv strings are guaranteed null-terminated by POSIX, warnings are overly strict
#pragma clang diagnostic ignored "-Wunsafe-buffer-usage-in-libc-call"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <sys/stat.h>
#include <sys/resource.h>
#include <unistd.h>
#include <sys/wait.h>
#include <sys/mman.h>
#include <fcntl.h>

// macOS executable path support
#ifdef __APPLE__
#include <mach-o/dyld.h>
#endif

#include "public/fpdfview.h"
#include "public/fpdf_text.h"
#include "public/fpdf_formfill.h"
#include "public/fpdf_progressive.h"
#include "public/fpdf_edit.h"
#include "public/fpdf_parallel.h"
#include "testing/image_diff/image_diff_png.h"
#include "core/fxcrt/span.h"

// Image encoding support (JPEG)
#include <jpeglib.h>

// GPU acceleration support (macOS Metal)
#ifdef __APPLE__
#include "core/fxge/apple/fx_apple_metal.h"
#endif

#include <vector>
#include <chrono>
#include <filesystem>
#include <algorithm>
#include <map>
#include <thread>
#include <queue>
#include <mutex>
#include <condition_variable>
#include <memory>
#include <functional>
#include <atomic>

namespace fs = std::filesystem;

// ========================================
// Async I/O Thread Pool (v1.8.0, N=31)
// ========================================
// Purpose: Overlap disk writes with rendering
// Expected gain: 5-15% (hide I/O latency)
// Strategy: Submit write tasks to background threads, allow rendering to continue

class AsyncWriterPool {
public:
    AsyncWriterPool(int num_threads = 4, int max_queue_size = 8)
        : stop_(false), pending_writes_(0), max_queue_size_(max_queue_size) {
        for (int i = 0; i < num_threads; ++i) {
            workers_.emplace_back([this] { WorkerThread(); });
        }
    }

    ~AsyncWriterPool() {
        Shutdown();
    }

    // Submit write task (blocks if queue is full)
    void SubmitWrite(std::function<void()> write_task) {
        {
            std::unique_lock<std::mutex> lock(queue_mutex_);
            // Wait if queue is full (backpressure to limit memory usage)
            if (max_queue_size_ > 0) {
                queue_full_condition_.wait(lock, [this] {
                    return stop_ || task_queue_.size() < static_cast<size_t>(max_queue_size_);
                });
            }
            pending_writes_++;
            task_queue_.push(std::move(write_task));
        }
        condition_.notify_one();
    }

    // Wait for all pending writes to complete
    void WaitAll() {
        std::unique_lock<std::mutex> lock(queue_mutex_);
        done_condition_.wait(lock, [this] {
            return pending_writes_ == 0 && task_queue_.empty();
        });
    }

    // Shutdown thread pool (called by destructor)
    void Shutdown() {
        {
            std::unique_lock<std::mutex> lock(queue_mutex_);
            stop_ = true;
        }
        condition_.notify_all();
        for (auto& worker : workers_) {
            if (worker.joinable()) {
                worker.join();
            }
        }
    }

private:
    void WorkerThread() {
        while (true) {
            std::function<void()> task;
            {
                std::unique_lock<std::mutex> lock(queue_mutex_);
                condition_.wait(lock, [this] {
                    return stop_ || !task_queue_.empty();
                });

                if (stop_ && task_queue_.empty()) {
                    return;
                }

                task = std::move(task_queue_.front());
                task_queue_.pop();
            }

            // Execute write task (outside lock)
            task();

            // Decrement pending count and notify waiters
            {
                std::unique_lock<std::mutex> lock(queue_mutex_);
                pending_writes_--;
                // Notify producers that queue has space
                queue_full_condition_.notify_one();
                if (pending_writes_ == 0 && task_queue_.empty()) {
                    done_condition_.notify_all();
                }
            }
        }
    }

    std::vector<std::thread> workers_;
    std::queue<std::function<void()>> task_queue_;
    std::mutex queue_mutex_;
    std::condition_variable condition_;
    std::condition_variable done_condition_;
    std::condition_variable queue_full_condition_;
    bool stop_;
    int pending_writes_;
    int max_queue_size_;
};

// Mode enumeration
enum Mode {
    MODE_NORMAL,  // Normal execution (1+ workers)
    MODE_DEBUG,   // Debug/tracing
    MODE_WORKER   // Internal worker process
};

// Operation enumeration
enum Operation {
    OP_EXTRACT_TEXT,
    OP_EXTRACT_JSONL,
    OP_RENDER_PAGES
};

// Configuration
const int DEFAULT_WORKERS = 1;
const int MAX_WORKERS = 16;
const double DEFAULT_DPI = 300.0;

// v1.9.0: Smart Presets (N=43)
// Simplified UX for common rendering scenarios
enum class RenderPreset {
    NONE,        // No preset (use explicit flags)
    WEB,         // 150 DPI JPEG q85 (web display, 1.8x faster)
    THUMBNAIL,   // 72 DPI JPEG q80 max 1024px (2.3x faster)
    PRINT        // 300 DPI PNG (high quality)
};

struct PresetConfig {
    double dpi;
    bool use_jpeg;
    int jpeg_quality;
    int max_dimension;  // 0 = no limit
};

const PresetConfig PRESET_CONFIGS[] = {
    {300.0, false, 90, 0},      // NONE (defaults)
    {150.0, true, 85, 2048},    // WEB
    {72.0, true, 80, 1024},     // THUMBNAIL
    {300.0, false, 90, 0}       // PRINT
};

// Form callbacks structure (matching upstream pdfium_test.cc)
struct FormFillInfo : public FPDF_FORMFILLINFO {
    FPDF_FORMHANDLE form_handle;
    FPDF_DOCUMENT current_doc;
    FPDF_PAGE current_page;
    int current_page_index;
};

// v1.6.0: Progress Reporting (N=616)
// Real-time progress bars with ETA estimation
// Auto-disabled on non-TTY (pipes, redirects) for clean output
class ProgressReporter {
public:
    ProgressReporter(int total_pages, bool enabled)
        : total_(total_pages),
          current_(0),
          smart_mode_pages_(0),
          start_(std::chrono::steady_clock::now()),
          last_update_(start_),
          enabled_(enabled && isatty(STDERR_FILENO)) {
    }

    void Update(int current_page) {
        if (!enabled_ || total_ <= 0) return;  // Guard against div by zero

        current_ = current_page;
        auto now = std::chrono::steady_clock::now();

        // Update every 10 pages or 100ms (whichever longer)
        auto elapsed_since_update = std::chrono::duration_cast<std::chrono::milliseconds>(now - last_update_).count();
        if (current_ % 10 != 0 && elapsed_since_update < 100) {
            return;  // Too frequent
        }

        last_update_ = now;

        // Calculate progress
        int percent = (current_ * 100) / total_;
        auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(now - start_).count();
        double seconds = elapsed / 1000.0;
        double pps = (seconds > 0) ? (current_ / seconds) : 0.0;
        double eta_seconds = (pps > 0 && current_ < total_) ? ((total_ - current_) / pps) : 0.0;

        // Build progress bar (20 characters)
        const int bar_width = 20;
        int filled = (current_ * bar_width) / total_;
        char bar[bar_width + 1];
        for (int i = 0; i < bar_width; ++i) {
            if (i < filled) {
                bar[i] = '=';
            } else if (i == filled) {
                bar[i] = '>';
            } else {
                bar[i] = ' ';
            }
        }
        bar[bar_width] = '\0';

        // Print progress (carriage return for in-place update)
        fprintf(stderr, "\rProcessing: [%s] %d/%d (%d%%) - %.0f pps - ETA: %.1fs",
                bar, current_, total_, percent, pps, eta_seconds);
        fflush(stderr);
    }

    void RecordSmartModePage() {
        smart_mode_pages_++;
    }

    void Finish() {
        if (!enabled_) return;
        fprintf(stderr, "\n");  // Move to new line after progress bar
    }

    int GetSmartModePages() const { return smart_mode_pages_; }

private:
    int total_;
    int current_;
    int smart_mode_pages_;
    std::chrono::steady_clock::time_point start_;
    std::chrono::steady_clock::time_point last_update_;
    bool enabled_;
};

// v1.6.0: Memory Reporting (N=619)
// Peak memory usage tracking with per-page breakdown
class MemoryReporter {
public:
    static void PrintPeakMemory(int pages_processed) {
        if (pages_processed <= 0) return;

#ifdef __APPLE__
        struct rusage usage;
        if (getrusage(RUSAGE_SELF, &usage) != 0) {
            return;  // Silent failure (memory reporting is optional)
        }

        // macOS reports in bytes
        long peak_mb = usage.ru_maxrss / (1024 * 1024);
        long per_page_kb = (usage.ru_maxrss / 1024) / pages_processed;

        fprintf(stderr, "  Peak memory: %ld MB (%ld KB/page)\n", peak_mb, per_page_kb);
#elif defined(__linux__)
        struct rusage usage;
        if (getrusage(RUSAGE_SELF, &usage) != 0) {
            return;  // Silent failure (memory reporting is optional)
        }

        // Linux reports in KB
        long peak_mb = usage.ru_maxrss / 1024;
        long per_page_kb = usage.ru_maxrss / pages_processed;

        fprintf(stderr, "  Peak memory: %ld MB (%ld KB/page)\n", peak_mb, per_page_kb);
#endif
        // Other platforms: no implementation (silent skip)
    }
};

// v1.6.0: Performance Metrics (N=616)
// Detailed performance summary with threading efficiency and smart mode stats
class MetricsReporter {
public:
    MetricsReporter() : pages_processed_(0), smart_mode_pages_(0) {}

    void RecordStart() {
        start_ = std::chrono::steady_clock::now();
    }

    void RecordPage() {
        pages_processed_++;
    }

    void RecordSmartMode() {
        smart_mode_pages_++;
    }

    void PrintSummary(int thread_count, bool enable_smart_mode) {
        auto end = std::chrono::steady_clock::now();
        auto elapsed_ms = std::chrono::duration_cast<std::chrono::milliseconds>(end - start_).count();
        double seconds = elapsed_ms / 1000.0;
        double pps = (seconds > 0) ? (pages_processed_ / seconds) : 0.0;

        fprintf(stderr, "\nPerformance Summary:\n");
        fprintf(stderr, "  Total pages: %d\n", pages_processed_);
        fprintf(stderr, "  Processing time: %.2fs\n", seconds);
        fprintf(stderr, "  Throughput: %.0f pages/second\n", pps);

        if (thread_count > 1) {
            // Estimate threading efficiency (assuming K=1 baseline)
            // Based on N=341 measurements: K=4 = 3.65x, K=8 = 6.55x
            double expected_speedup = (thread_count == 4) ? 3.65 : (thread_count == 8) ? 6.55 : thread_count * 0.9;
            fprintf(stderr, "  Threading: %d threads (expected ~%.1fx speedup)\n", thread_count, expected_speedup);
        }

        if (enable_smart_mode && smart_mode_pages_ > 0) {
            double smart_percent = (pages_processed_ > 0) ? (100.0 * smart_mode_pages_ / pages_processed_) : 0.0;
            fprintf(stderr, "  Smart mode: %d pages (%.1f%% via JPEG fast path, 545x speedup)\n",
                    smart_mode_pages_, smart_percent);
        }

        // N=619: Memory reporting (optional, platform-specific)
        MemoryReporter::PrintPeakMemory(pages_processed_);
    }

private:
    std::chrono::steady_clock::time_point start_;
    int pages_processed_;
    int smart_mode_pages_;
};

// v1.6.0: Error Reporting (N=618)
// Actionable error messages with reason and solution
enum class ErrorCode {
    FileNotFound,
    DirectoryNotFound,
    CannotOpen,
    PasswordProtected,
    InvalidPDF,
    OutOfMemory,
    PermissionDenied,
    UnsupportedFeature,
    PageRangeInvalid,
    WorkerCountInvalid,
    ThreadCountInvalid,
    InvalidArgument,
    OutputDirCreationFailed
};

class ErrorReporter {
public:
    struct ErrorInfo {
        const char* reason;
        const char* solution;
    };

    static void ReportError(ErrorCode code, const std::string& context) {
        const ErrorInfo* info = GetErrorInfo(code);
        if (!info) {
            fprintf(stderr, "Error: Unknown error (code %d) - %s\n",
                    static_cast<int>(code), context.c_str());
            return;
        }

        fprintf(stderr, "\n");
        fprintf(stderr, "Error: %s\n", context.c_str());
        fprintf(stderr, "  Reason: %s\n", info->reason);
        fprintf(stderr, "  Solution: %s\n", info->solution);
        fprintf(stderr, "  Help: Run with --help for usage information\n");
        fprintf(stderr, "\n");
    }

private:
    static const ErrorInfo* GetErrorInfo(ErrorCode code);
};

// Error message map implementation (no global constructors)
const ErrorReporter::ErrorInfo* ErrorReporter::GetErrorInfo(ErrorCode code) {
    switch (code) {
        case ErrorCode::FileNotFound: {
            static const ErrorInfo info = {
                "File not found",
                "Check the file path is correct and the file exists"
            };
            return &info;
        }
        case ErrorCode::DirectoryNotFound: {
            static const ErrorInfo info = {
                "Directory not found",
                "Check the directory path is correct and the directory exists"
            };
            return &info;
        }
        case ErrorCode::CannotOpen: {
            static const ErrorInfo info = {
                "Cannot open file",
                "Check file permissions and ensure the file is not in use by another process"
            };
            return &info;
        }
        case ErrorCode::PasswordProtected: {
            static const ErrorInfo info = {
                "File is password-protected",
                "Decrypt the PDF first (password support not yet implemented)"
            };
            return &info;
        }
        case ErrorCode::InvalidPDF: {
            static const ErrorInfo info = {
                "Invalid or corrupted PDF structure",
                "Try opening in Adobe Reader to verify file integrity"
            };
            return &info;
        }
        case ErrorCode::OutOfMemory: {
            static const ErrorInfo info = {
                "Insufficient memory to process PDF",
                "Try processing fewer pages at once using --pages flag or reduce --workers count"
            };
            return &info;
        }
        case ErrorCode::PermissionDenied: {
            static const ErrorInfo info = {
                "Permission denied",
                "Check file/directory permissions or run with appropriate privileges"
            };
            return &info;
        }
        case ErrorCode::UnsupportedFeature: {
            static const ErrorInfo info = {
                "PDF uses unsupported features",
                "This is a PDFium upstream limitation. Consider reporting to PDFium team"
            };
            return &info;
        }
        case ErrorCode::PageRangeInvalid: {
            static const ErrorInfo info = {
                "Invalid page range specified",
                "Page range must be within document bounds (0-indexed)"
            };
            return &info;
        }
        case ErrorCode::WorkerCountInvalid: {
            static const ErrorInfo info = {
                "Invalid worker count",
                "Worker count must be between 1 and 16"
            };
            return &info;
        }
        case ErrorCode::ThreadCountInvalid: {
            static const ErrorInfo info = {
                "Invalid thread count",
                "Thread count must be between 1 and 32"
            };
            return &info;
        }
        case ErrorCode::InvalidArgument: {
            static const ErrorInfo info = {
                "Invalid command-line argument",
                "Check argument format and values"
            };
            return &info;
        }
        case ErrorCode::OutputDirCreationFailed: {
            static const ErrorInfo info = {
                "Cannot create output directory",
                "Check parent directory exists and you have write permissions"
            };
            return &info;
        }
    }
    // Unreachable: all enum values covered
    return nullptr;
}

// v1.6.0: Batch Processing Helper Functions (N=617)
// Pattern matching for glob patterns (*.pdf, report_*.pdf, etc.)
bool MatchesPattern(const fs::path& path, const std::string& pattern) {
    std::string filename = path.filename().string();

    // Simple glob matching (supports * and ?)
    // Convert pattern to regex-like matching
    size_t pat_idx = 0;
    size_t name_idx = 0;

    while (pat_idx < pattern.size() && name_idx < filename.size()) {
        if (pattern[pat_idx] == '*') {
            // Wildcard: match zero or more characters
            // Look ahead for next non-wildcard character
            pat_idx++;
            if (pat_idx >= pattern.size()) {
                // Pattern ends with *, matches rest of filename
                return true;
            }

            // Find next match of pattern[pat_idx] in filename
            while (name_idx < filename.size() && filename[name_idx] != pattern[pat_idx]) {
                name_idx++;
            }
            if (name_idx >= filename.size()) {
                return false;
            }
        } else if (pattern[pat_idx] == '?') {
            // Single character wildcard
            pat_idx++;
            name_idx++;
        } else {
            // Literal character match
            if (filename[name_idx] != pattern[pat_idx]) {
                return false;
            }
            pat_idx++;
            name_idx++;
        }
    }

    // Handle trailing wildcards
    while (pat_idx < pattern.size() && pattern[pat_idx] == '*') {
        pat_idx++;
    }

    // Both pattern and filename must be fully consumed
    return pat_idx >= pattern.size() && name_idx >= filename.size();
}

// Find all PDFs matching pattern in directory
std::vector<std::string> FindPDFs(const std::string& dir,
                                   const std::string& pattern,
                                   bool recursive) {
    std::vector<std::string> results;

    std::error_code ec;

    if (recursive) {
        // Recursive directory traversal
        for (const auto& entry : fs::recursive_directory_iterator(dir, ec)) {
            if (ec) {
                fprintf(stderr, "Filesystem error: %s\n", ec.message().c_str());
                break;
            }
            if (entry.is_regular_file() && MatchesPattern(entry.path(), pattern)) {
                results.push_back(entry.path().string());
            }
        }
    } else {
        // Non-recursive directory traversal
        for (const auto& entry : fs::directory_iterator(dir, ec)) {
            if (ec) {
                fprintf(stderr, "Filesystem error: %s\n", ec.message().c_str());
                break;
            }
            if (entry.is_regular_file() && MatchesPattern(entry.path(), pattern)) {
                results.push_back(entry.path().string());
            }
        }
    }

    // Sort for deterministic order
    std::sort(results.begin(), results.end());

    return results;
}

// Forward declarations
int ProcessBatch(const std::string& input_dir, const std::string& output_dir,
                 Operation operation, int worker_count, int thread_count, double dpi, bool use_ppm,
                 bool use_jpeg, int jpeg_quality, bool use_raw, int render_quality, bool benchmark_mode, bool force_alpha,
                 const std::string& pattern, bool recursive, int pixel_format = 0);
int extract_text_bulk(const char* pdf_path, const char* output_path, int start_page, int end_page, bool use_utf8);
int extract_text_fast(const char* pdf_path, const char* output_path, int worker_count, int start_page, int end_page, bool use_utf8 = true);
int extract_text_debug(const char* pdf_path, const char* output_path, bool use_utf8);
int extract_text_worker(const char* pdf_path, const char* output_path,
                        int start_page, int end_page, int worker_id, bool use_utf8 = true);
int extract_jsonl_bulk(const char* pdf_path, const char* output_path, int page_num);
int extract_jsonl_debug(const char* pdf_path, const char* output_path, int page_num);
int render_pages_bulk(const char* pdf_path, const char* output_dir, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int start_page, int end_page, int thread_count, int render_quality, bool benchmark_mode, bool user_set_threads, bool enable_adaptive, bool force_alpha, int pixel_format = 0);
int render_pages_fast(const char* pdf_path, const char* output_dir, int worker_count, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int start_page, int end_page, int render_quality, bool benchmark_mode, bool force_alpha, int thread_count = 1);
int render_pages_debug(const char* pdf_path, const char* output_dir, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int render_quality, bool force_alpha);
int render_pages_worker(const char* pdf_path, const char* output_dir,
                        int start_page, int end_page, int worker_id, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int render_quality, bool force_alpha, int thread_count = 1, bool benchmark_mode = false, int pixel_format = 0);
int get_page_count(const char* pdf_path);
void write_bom(FILE* out);
void write_codepoint(FILE* out, unsigned int codepoint);
bool write_png(const char* filename, const std::vector<uint8_t>& png_data);
bool write_ppm(const char* filename, void* buffer, int stride, int width, int height, int bitmap_format);
bool write_jpeg(const char* filename, void* buffer, int stride, int width, int height, int quality, int pixel_format = 0);
bool is_scanned_page(FPDF_PAGE page);
bool render_scanned_page_fast(FPDF_PAGE page, const char* output_path);
int render_page_to_png(FPDF_DOCUMENT doc, FPDF_FORMHANDLE form, FormFillInfo* form_info, int page_index, const char* output_dir, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int render_quality, bool benchmark_mode, bool force_alpha);

// Form callback implementations (matching upstream pdfium_test.cc)
FPDF_PAGE GetPageForIndex(FPDF_FORMFILLINFO* param, FPDF_DOCUMENT doc, int index) {
    // Get form handle from our custom struct
    FormFillInfo* form_info = static_cast<FormFillInfo*>(param);

    // If requesting the current page being rendered, return it
    if (form_info->current_doc == doc &&
        form_info->current_page &&
        form_info->current_page_index == index) {
        return form_info->current_page;
    }

    // Otherwise load a new page
    // Note: For forms that reference other pages, we load them fresh
    // In a full application, you'd want to cache these
    FPDF_PAGE page = FPDF_LoadPage(doc, index);
    if (!page) {
        return nullptr;
    }

    FPDF_FORMHANDLE form_handle = form_info->form_handle;
    if (form_handle) {
        FORM_OnAfterLoadPage(page, form_handle);
        FORM_DoPageAAction(page, form_handle, FPDFPAGE_AACTION_OPEN);
    }

    return page;
}

void ExampleNamedAction(FPDF_FORMFILLINFO* pInfo, FPDF_BYTESTRING name) {
    // Named action callback (matching upstream)
    // For CLI, we just silently handle this
    (void)pInfo;
    (void)name;
}

void usage(const char* prog) {
    fprintf(stderr, "Usage: %s [flags] <operation> <input.pdf> <output>\n", prog);    fprintf(stderr, "\n");
    fprintf(stderr, "Flags:\n");
    fprintf(stderr, "  -h, --help        Show this help message\n");    fprintf(stderr, "  --workers N       Number of workers (default 1, max 16)\n");
    fprintf(stderr, "  --threads K       Number of render threads per worker (default 8, max 16)\n");
    fprintf(stderr, "  --no-adaptive     Disable adaptive threading (use fixed thread count)\n");
    fprintf(stderr, "  --pages START-END Process page range (e.g., --pages 1-10 or --pages 5)\n");
    fprintf(stderr, "  --preset MODE     Render preset: web|thumbnail|print\n");
    fprintf(stderr, "  --dpi N           Render DPI (default 300, range 72-600)\n");
    fprintf(stderr, "  --quality MODE    Render quality: none|fast|balanced|high (default balanced)\n");
    fprintf(stderr, "  --debug           Debug mode with tracing\n");
    fprintf(stderr, "  --format FMT      Output format: png|jpg|jpeg|ppm (default jpg for render-pages)\n");
    fprintf(stderr, "  --pixel-format F  Pixel format: bgrx (default), bgr (25%% less memory), gray (75%% less)\n");
    fprintf(stderr, "  --jpeg-quality N  JPEG quality: 0-100 (default 90, only for JPEG format)\n");
    fprintf(stderr, "  --ppm             Output PPM format (deprecated, use --format ppm)\n");
    fprintf(stderr, "  --benchmark       Skip file writes (benchmark mode, for performance testing)\n");
    fprintf(stderr, "  --batch           (Deprecated) Auto-detects directories\n");
    fprintf(stderr, "  --pattern GLOB    File pattern for batch (default: *.pdf)\n");
    fprintf(stderr, "  --recursive       (Deprecated) Recursive by default, use --no-recursive to disable\n");
    fprintf(stderr, "  --no-recursive    Disable recursive directory search (top-level only)\n");
    fprintf(stderr, "\n");
    fprintf(stderr, "Operations:\n");
    fprintf(stderr, "  extract-text      Extract text to UTF-8 format (default)\n");
    fprintf(stderr, "  extract-jsonl     Extract text with metadata in JSONL format (single page)\n");
    fprintf(stderr, "  render-pages      Render pages to JPEG images (default, 300 DPI)\n");
    fprintf(stderr, "\n");
    fprintf(stderr, "Presets (v1.9.0):\n");
    fprintf(stderr, "  web               150 DPI JPEG q85 (web display, 1.8x faster)\n");
    fprintf(stderr, "  thumbnail         72 DPI JPEG q80 (thumbnails, 2.3x faster)\n");
    fprintf(stderr, "  print             300 DPI PNG (high-quality printing)\n");
    fprintf(stderr, "\n");
    fprintf(stderr, "Examples:\n");
    fprintf(stderr, "  %s --preset web render-pages input.pdf output/\n", prog);
    fprintf(stderr, "  %s --preset thumbnail render-pages input.pdf thumbs/\n", prog);
    fprintf(stderr, "  %s extract-text input.pdf output.txt\n", prog);
    fprintf(stderr, "  %s --workers 4 extract-text large.pdf output.txt\n", prog);
    fprintf(stderr, "  %s --workers 8 --pages 1-50 extract-text input.pdf output.txt\n", prog);
    fprintf(stderr, "  %s --pages 5 render-pages input.pdf output_dir/\n", prog);
    fprintf(stderr, "  %s --debug extract-text input.pdf output.txt\n", prog);
    fprintf(stderr, "\n");
    fprintf(stderr, "Batch Processing (v2.0.0: Auto-detects directories):\n");
    fprintf(stderr, "  %s extract-text /pdfs/ /output/             # Auto-detects directory, recursive\n", prog);
    fprintf(stderr, "  %s --pattern \"report_*.pdf\" extract-text /docs/ /out/  # Pattern filter\n", prog);
    fprintf(stderr, "  %s render-pages /archive/ /images/          # Auto-detects directory\n", prog);
    fprintf(stderr, "\n");
    fprintf(stderr, "Optimization Strategies:\n");
    fprintf(stderr, "\n");
    fprintf(stderr, "  Smart mode (JPEG Fast Path)\n");
    fprintf(stderr, "    When: Scanned PDFs with embedded JPEG images\n");
    fprintf(stderr, "    How: Extract JPEG directly, skip rendering (545x speedup)\n");
    fprintf(stderr, "    Quality: Full quality, preserves original JPEG\n");
    fprintf(stderr, "    Detection: Automatic (single full-page image, >=95%% coverage)\n");
    fprintf(stderr, "    Note: Always enabled automatically\n");
    fprintf(stderr, "\n");
    fprintf(stderr, "  Multi-process parallelism\n");
    fprintf(stderr, "    When: Large PDFs (200+ pages recommended)\n");
    fprintf(stderr, "    How: Split work across N worker processes\n");
    fprintf(stderr, "    Speedup: 3-4x at 4 workers for large documents\n");
    fprintf(stderr, "    Example: --workers 4 (optimal for most systems)\n");
    fprintf(stderr, "\n");
    fprintf(stderr, "  Multi-threaded rendering (default: adaptive)\n");
    fprintf(stderr, "    When: Medium to large PDFs (50+ pages)\n");
    fprintf(stderr, "    How: Auto-selects thread count based on page count\n");
    fprintf(stderr, "    Speedup: Up to 6.5x (K=8) for image rendering\n");
    fprintf(stderr, "    Selection: <50 pages: K=1, 50+ pages: K=8\n");
    fprintf(stderr, "    Disable: --no-adaptive (uses fixed --threads value)\n");
}

// v1.6.0: Batch Processing Function (N=617)
// Process multiple PDFs in a directory with error handling and summary
int ProcessBatch(const std::string& input_dir, const std::string& output_dir,
                 Operation operation, int worker_count, int thread_count, double dpi, bool use_ppm,
                 bool use_jpeg, int jpeg_quality, bool use_raw, int render_quality, bool benchmark_mode, bool force_alpha,
                 const std::string& pattern, bool recursive, int pixel_format) {

    fprintf(stderr, "Batch mode: %s\n", recursive ? "recursive" : "non-recursive");
    fprintf(stderr, "Pattern: %s\n", pattern.c_str());

    // 1. Find all PDFs matching pattern
    std::vector<std::string> pdf_files = FindPDFs(input_dir, pattern, recursive);

    if (pdf_files.empty()) {
        fprintf(stderr, "Found 0 PDF file(s)\n\n");
        fprintf(stderr, "No PDFs to process - batch operation complete\n\n");
        return 0;  // Empty directory is not an error
    }

    fprintf(stderr, "Found %zu PDF file(s)\n\n", pdf_files.size());

    // 2. Create output directory if it doesn't exist
    std::error_code ec;
    fs::create_directories(output_dir, ec);
    if (ec) {
        fprintf(stderr, "Error: Failed to create output directory: %s (%s)\n",
                output_dir.c_str(), ec.message().c_str());
        return 1;
    }

    // 3. Process each PDF
    int succeeded = 0;
    int failed = 0;

    for (size_t i = 0; i < pdf_files.size(); i++) {
        const std::string& pdf = pdf_files[i];
        fprintf(stderr, "[%zu/%zu] Processing: %s\n", i + 1, pdf_files.size(), pdf.c_str());

        // Generate output path based on operation
        std::string output_subpath;
        fs::path pdf_path(pdf);
        fs::path input_base(input_dir);

        // Get relative path from input directory
        std::string relative_path = fs::relative(pdf_path.parent_path(), input_base).string();
        std::string pdf_basename = pdf_path.stem().string();

        int result = -1;

        // Execute operation based on type
        switch (operation) {
            case OP_EXTRACT_TEXT: {
                // Text output: directory/relative_path/basename.txt
                fs::path text_output_dir = fs::path(output_dir) / relative_path;
                std::error_code ec_text;
                fs::create_directories(text_output_dir, ec_text);
                if (ec_text) {
                    fprintf(stderr, "  Error: Failed to create directory: %s (%s)\n",
                            text_output_dir.string().c_str(), ec_text.message().c_str());
                    result = 1;
                    break;
                }
                std::string text_output_file = (text_output_dir / (pdf_basename + ".txt")).string();

                if (worker_count == 1) {
                    result = extract_text_bulk(pdf.c_str(), text_output_file.c_str(), -1, -1, true);
                } else {
                    result = extract_text_fast(pdf.c_str(), text_output_file.c_str(), worker_count, -1, -1);
                }
                break;
            }
            case OP_RENDER_PAGES: {
                // Image output: directory/relative_path/basename/
                fs::path image_output_dir = fs::path(output_dir) / relative_path / pdf_basename;
                std::error_code ec_img;
                fs::create_directories(image_output_dir, ec_img);
                if (ec_img) {
                    fprintf(stderr, "  Error: Failed to create directory: %s (%s)\n",
                            image_output_dir.string().c_str(), ec_img.message().c_str());
                    result = 1;
                    break;
                }

                if (worker_count == 1) {
                    result = render_pages_bulk(pdf.c_str(), image_output_dir.string().c_str(),
                                              dpi, use_ppm, use_jpeg, jpeg_quality, use_raw, -1, -1, thread_count,
                                              render_quality, benchmark_mode, false, false, force_alpha, pixel_format);
                } else {
                    result = render_pages_fast(pdf.c_str(), image_output_dir.string().c_str(),
                                              worker_count, dpi, use_ppm, use_jpeg, jpeg_quality, use_raw, -1, -1,
                                              render_quality, benchmark_mode, force_alpha, thread_count);
                }
                break;
            }
            case OP_EXTRACT_JSONL: {
                fprintf(stderr, "  WARNING: JSONL extraction not supported in batch mode (single-page only)\n");
                result = 1;
                break;
            }
        }

        if (result == 0) {
            succeeded++;
            fprintf(stderr, "  SUCCESS\n");
        } else {
            failed++;
            fprintf(stderr, "  ERROR: Failed to process %s (exit code %d)\n", pdf.c_str(), result);
            // Continue processing (don't abort batch)
        }
        fprintf(stderr, "\n");
    }

    // 4. Print summary
    fprintf(stderr, "==================================================\n");
    fprintf(stderr, "Batch Summary:\n");
    fprintf(stderr, "  Total: %zu PDF(s)\n", pdf_files.size());
    fprintf(stderr, "  Succeeded: %d\n", succeeded);
    fprintf(stderr, "  Failed: %d\n", failed);
    fprintf(stderr, "==================================================\n");

    return (failed > 0) ? 1 : 0;
}

int main(int argc, char* argv[]) {
    // Check for worker mode (internal use)
    if (argc >= 2 && strcmp(argv[1], "--worker") == 0) {
        if (argc == 7 || argc == 8) {
            // Text extraction worker (argc 7 = old format UTF-32 LE, argc 8 = new format with encoding)
            bool use_utf8 = (argc == 8 && strcmp(argv[7], "utf8") == 0);  // default to UTF-32 LE if argc=7
            return extract_text_worker(argv[2], argv[3],
                                        atoi(argv[4]), atoi(argv[5]), atoi(argv[6]), use_utf8);
        } else if (argc == 10 || argc == 11 || argc == 12 || argc == 13 || argc == 14) {
            // Image rendering worker (has DPI, format, quality, and optional thread_count/jpeg_quality/benchmark parameters, smart mode always on)
            bool use_ppm = (strcmp(argv[8], "ppm") == 0);
            bool use_jpeg = (strcmp(argv[8], "jpg") == 0 || strcmp(argv[8], "jpeg") == 0);
            bool use_raw = (strcmp(argv[8], "bgra") == 0);
            int render_quality = atoi(argv[9]);
            bool force_alpha = (argc >= 11 && atoi(argv[10]) != 0);
            int thread_count = (argc >= 12) ? atoi(argv[11]) : 1;
            int jpeg_quality = (argc >= 13) ? atoi(argv[12]) : 90;
            bool benchmark_mode = (argc == 14 && atoi(argv[13]) != 0);
            return render_pages_worker(argv[2], argv[3],
                                        atoi(argv[4]), atoi(argv[5]), atoi(argv[6]),
                                        atof(argv[7]), use_ppm, use_jpeg, jpeg_quality, use_raw, render_quality, force_alpha, thread_count, benchmark_mode);
        } else {
            fprintf(stderr, "Worker usage:\n");
            fprintf(stderr, "  Text: --worker <pdf> <output> <start> <end> <id>\n");
            fprintf(stderr, "  Image: --worker <pdf> <output_dir> <start> <end> <id> <dpi> <format> <quality>\n");
            return 1;
        }
    }

    // Parse arguments
    // Check for help and version flags first (before other parsing)
    if (argc > 1 && (strcmp(argv[1], "--help") == 0 || strcmp(argv[1], "-h") == 0)) {
        usage(argv[0]);
        return 0;
    }
    if (argc > 1 && (strcmp(argv[1], "--version") == 0 || strcmp(argv[1], "-V") == 0)) {
        fprintf(stderr, "pdfium_fast v2.0.0\n");
        return 0;
    }

    Mode mode = MODE_NORMAL;
    int worker_count = DEFAULT_WORKERS;
    int thread_count = 8;  // Default: 8 threads (N=413 fixed K>=2 crashes)
    bool user_set_threads = false;  // N=349: Track if --threads was explicitly provided
    bool enable_adaptive = true;  // Enable adaptive threading by default (use --no-adaptive to disable)
    bool use_ppm = false;
    bool use_jpeg = false;  // N=15: JPEG output format
    bool user_set_format = false;  // v2.0.0: Track if user explicitly set format
    int jpeg_quality = 90;  // N=15: JPEG quality (0-100, default 90)
    bool use_raw = false;  // N=328: Raw BGRA output (no encoding)
    bool benchmark_mode = false;  // N=323: Skip file writes for benchmarking
    bool force_alpha = false;  // N=420: Force BGRA bitmap (alpha=1) for transparency optimization test
    int start_page = -1;  // -1 means "from beginning"
    int end_page = -1;    // -1 means "to end"
    int render_quality = 1;  // 0=balanced, 1=fast (default, matches v1.6.0 baselines), 2=high, 3=none
    std::string pattern = "*.pdf";  // N=617: File pattern for batch mode
    bool recursive = true;  // v2.0.0: Recursive by default (use --no-recursive to disable)
    double dpi = DEFAULT_DPI;  // v1.8.0: Configurable DPI (default 300, range 72-600)
    RenderPreset preset = RenderPreset::NONE;  // v1.9.0: Smart presets (N=43)
    bool use_utf8 = true;  // v2.0.0: UTF-8 by default (use --encoding utf32le for UTF-32 LE)
    int pixel_format = 0;  // N=50: Output pixel format (0=bgrx, 1=bgr, 2=gray)
    int arg_idx = 1;

    // Parse flags
    while (argc > arg_idx && argv[arg_idx][0] == '-' && argv[arg_idx][1] == '-') {
        if (strcmp(argv[arg_idx], "--workers") == 0) {
            arg_idx++;
            if (argc <= arg_idx) {
                ErrorReporter::ReportError(ErrorCode::InvalidArgument,
                                         "--workers flag requires a number (1-16)");
                usage(argv[0]);
                return 1;
            }
            char* end;
            long val = strtol(argv[arg_idx], &end, 10);
            if (*end == '\0' && val >= 1 && val <= MAX_WORKERS) {
                worker_count = (int)val;
                arg_idx++;
            } else {
                ErrorReporter::ReportError(ErrorCode::WorkerCountInvalid,
                                         std::string("Invalid worker count: ") + argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
        } else if (strcmp(argv[arg_idx], "--pages") == 0) {
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --pages requires a range (e.g., 1-10 or 5)\n");
                usage(argv[0]);
                return 1;
            }
            char* pages_arg = argv[arg_idx];
            if (strchr(pages_arg, '-')) {
                // Range: "1-10"
                if (sscanf(pages_arg, "%d-%d", &start_page, &end_page) != 2) {
                    fprintf(stderr, "Error: Invalid page range format: %s\n", pages_arg);
                    usage(argv[0]);
                    return 1;
                }
            } else {
                // Single page: "5"
                char* end;
                long val = strtol(pages_arg, &end, 10);
                if (*end == '\0' && val >= 0) {
                    start_page = end_page = (int)val;
                } else {
                    fprintf(stderr, "Error: Invalid page number: %s\n", pages_arg);
                    usage(argv[0]);
                    return 1;
                }
            }
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--debug") == 0) {
            mode = MODE_DEBUG;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--ppm") == 0) {
            use_ppm = true;
            user_set_format = true;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--format") == 0) {
            user_set_format = true;
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --format requires a format (png|jpg|jpeg|ppm)\n");
                usage(argv[0]);
                return 1;
            }
            if (strcmp(argv[arg_idx], "png") == 0) {
                use_ppm = false;
                use_jpeg = false;
            } else if (strcmp(argv[arg_idx], "jpg") == 0 || strcmp(argv[arg_idx], "jpeg") == 0) {
                use_ppm = false;
                use_jpeg = true;
            } else if (strcmp(argv[arg_idx], "ppm") == 0) {
                use_ppm = true;
                use_jpeg = false;
            } else {
                fprintf(stderr, "Error: Invalid format (must be png|jpg|jpeg|ppm): %s\n", argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--jpeg-quality") == 0) {
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --jpeg-quality requires a number (0-100)\n");
                usage(argv[0]);
                return 1;
            }
            char* end;
            long val = strtol(argv[arg_idx], &end, 10);
            if (*end == '\0' && val >= 0 && val <= 100) {
                jpeg_quality = (int)val;
                arg_idx++;
            } else {
                fprintf(stderr, "Error: Invalid JPEG quality (must be 0-100): %s\n", argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
        } else if (strcmp(argv[arg_idx], "--raw") == 0) {
            use_raw = true;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--benchmark") == 0) {
            benchmark_mode = true;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--force-alpha") == 0) {
            force_alpha = true;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--quality") == 0) {
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --quality requires a mode (none|fast|balanced|high)\n");
                usage(argv[0]);
                return 1;
            }
            if (strcmp(argv[arg_idx], "none") == 0) {
                render_quality = 3;
            } else if (strcmp(argv[arg_idx], "fast") == 0) {
                render_quality = 1;
            } else if (strcmp(argv[arg_idx], "balanced") == 0) {
                render_quality = 0;
            } else if (strcmp(argv[arg_idx], "high") == 0) {
                render_quality = 2;
            } else {
                fprintf(stderr, "Error: Invalid quality mode (must be none|fast|balanced|high): %s\n", argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--preset") == 0) {
            // v1.9.0: Smart presets (N=43)
            user_set_format = true;
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --preset requires a mode (web|thumbnail|print)\n");
                usage(argv[0]);
                return 1;
            }
            if (strcmp(argv[arg_idx], "web") == 0) {
                preset = RenderPreset::WEB;
            } else if (strcmp(argv[arg_idx], "thumbnail") == 0) {
                preset = RenderPreset::THUMBNAIL;
            } else if (strcmp(argv[arg_idx], "print") == 0) {
                preset = RenderPreset::PRINT;
            } else {
                fprintf(stderr, "Error: Invalid preset (must be web|thumbnail|print): %s\n", argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--dpi") == 0) {
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --dpi requires a number (72-600)\n");
                usage(argv[0]);
                return 1;
            }
            char* end;
            double val = strtod(argv[arg_idx], &end);
            if (*end == '\0' && val >= 72.0 && val <= 600.0) {
                dpi = val;
                arg_idx++;
            } else {
                fprintf(stderr, "Error: Invalid DPI (must be 72-600): %s\n", argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
        } else if (strcmp(argv[arg_idx], "--threads") == 0) {
            arg_idx++;
            if (argc <= arg_idx) {
                ErrorReporter::ReportError(ErrorCode::InvalidArgument,
                                         "--threads flag requires a number (1-32)");
                usage(argv[0]);
                return 1;
            }
            char* end;
            long val = strtol(argv[arg_idx], &end, 10);
            if (*end == '\0' && val >= 1 && val <= 32) {
                // FIX #4 (N=30): Clamp to min(16, hardware_concurrency)
                // Backend stability tested only to K=8, production cap at 16.
                unsigned int hw_threads = std::thread::hardware_concurrency();
                int max_threads = std::min(16, hw_threads > 0 ? static_cast<int>(hw_threads) : 16);
                thread_count = std::min(static_cast<int>(val), max_threads);
                if (thread_count != static_cast<int>(val)) {
                    fprintf(stderr, "Note: Thread count clamped from %ld to %d (hardware limit)\n", val, thread_count);
                }
                user_set_threads = true;  // N=349: Mark that user explicitly set --threads
                arg_idx++;
            } else {
                ErrorReporter::ReportError(ErrorCode::ThreadCountInvalid,
                                         std::string("Invalid thread count: ") + argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
        } else if (strcmp(argv[arg_idx], "--bulk") == 0) {
            // Backward compatibility: --bulk → --workers 1
            worker_count = 1;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--fast") == 0) {
            // Backward compatibility: --fast [N] → --workers N
            arg_idx++;
            if (argc > arg_idx && argv[arg_idx][0] != '-') {
                char* end;
                long val = strtol(argv[arg_idx], &end, 10);
                if (*end == '\0' && val >= 1 && val <= MAX_WORKERS) {
                    worker_count = (int)val;
                    arg_idx++;
                } else {
                    // --fast without valid number, use default 4 workers
                    worker_count = 4;
                }
            } else {
                // --fast without number, use default 4 workers
                worker_count = 4;
            }
        } else if (strcmp(argv[arg_idx], "--adaptive") == 0) {
            // Enable adaptive threading (auto-select K based on page count)
            enable_adaptive = true;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--no-adaptive") == 0) {
            // Disable adaptive threading (use fixed thread count)
            enable_adaptive = false;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--batch") == 0) {
            // v2.0.0: Deprecated flag (auto-detect is now default), silently ignore
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--pattern") == 0) {
            // N=617: File pattern for batch mode
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --pattern requires a glob pattern (e.g., '*.pdf')\n");
                usage(argv[0]);
                return 1;
            }
            pattern = argv[arg_idx];
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--recursive") == 0) {
            // N=617: Enable recursive directory search (deprecated, now default)
            recursive = true;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--no-recursive") == 0) {
            // v2.0.0: Disable recursive directory search (non-recursive, top-level only)
            recursive = false;
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--encoding") == 0) {
            // v2.0.0: Encoding selection (utf8 or utf32le)
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --encoding requires a value (utf8 or utf32le)\n");
                usage(argv[0]);
                return 1;
            }
            if (strcmp(argv[arg_idx], "utf8") == 0) {
                use_utf8 = true;
            } else if (strcmp(argv[arg_idx], "utf32le") == 0) {
                use_utf8 = false;
            } else {
                fprintf(stderr, "Error: Invalid encoding: %s (must be utf8 or utf32le)\n", argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
            arg_idx++;
        } else if (strcmp(argv[arg_idx], "--pixel-format") == 0) {
            // N=50: Pixel format selection (bgrx, bgr, or gray)
            arg_idx++;
            if (argc <= arg_idx) {
                fprintf(stderr, "Error: --pixel-format requires a value (bgrx, bgr, or gray)\n");
                usage(argv[0]);
                return 1;
            }
            if (strcmp(argv[arg_idx], "bgrx") == 0) {
                pixel_format = FPDF_PARALLEL_FORMAT_BGRx;
            } else if (strcmp(argv[arg_idx], "bgr") == 0) {
                pixel_format = FPDF_PARALLEL_FORMAT_BGR;
            } else if (strcmp(argv[arg_idx], "gray") == 0) {
                pixel_format = FPDF_PARALLEL_FORMAT_GRAY;
            } else {
                fprintf(stderr, "Error: Invalid pixel format: %s (must be bgrx, bgr, or gray)\n", argv[arg_idx]);
                usage(argv[0]);
                return 1;
            }
            arg_idx++;
        } else {
            fprintf(stderr, "Error: Unknown flag: %s\n", argv[arg_idx]);
            usage(argv[0]);
            return 1;
        }
    }

    // Parse operation
    if (argc <= arg_idx) {
        fprintf(stderr, "Error: Operation required\n");
        usage(argv[0]);
        return 1;
    }

    Operation operation;
    if (strcmp(argv[arg_idx], "extract-text") == 0) {
        operation = OP_EXTRACT_TEXT;
    } else if (strcmp(argv[arg_idx], "extract-jsonl") == 0) {
        operation = OP_EXTRACT_JSONL;
    } else if (strcmp(argv[arg_idx], "render-pages") == 0) {
        operation = OP_RENDER_PAGES;
        // v2.0.0: Default to JPEG for image rendering (unless user explicitly set format)
        if (!user_set_format) {
            use_jpeg = true;  // Smart default: JPEG (not PNG)
        }
    } else {
        fprintf(stderr, "Error: Unknown operation: %s\n", argv[arg_idx]);
        usage(argv[0]);
        return 1;
    }
    arg_idx++;

    // Issue #5 fix: Prevent CPU oversubscription in hybrid N×K mode
    // If worker_count > 1 (multi-process) AND thread_count > 1 (multi-threaded),
    // cap total threads to hardware_concurrency to prevent oversubscription
    if (worker_count > 1 && thread_count > 1) {
        unsigned int hw_concurrency = std::thread::hardware_concurrency();
        if (hw_concurrency == 0) hw_concurrency = 8;  // Fallback if unknown
        int total_threads = worker_count * thread_count;
        if (total_threads > (int)hw_concurrency) {
            // Auto-scale thread_count down to prevent oversubscription
            int new_thread_count = std::max(1, (int)hw_concurrency / worker_count);
            fprintf(stderr, "Note: Reducing threads from %d to %d (total %d×%d=%d exceeds %u cores)\n",
                    thread_count, new_thread_count, worker_count, thread_count, total_threads, hw_concurrency);
            thread_count = new_thread_count;
        }
    }

    // Parse paths
    if (argc < arg_idx + 2) {
        fprintf(stderr, "Error: Input and output paths required\n");
        usage(argv[0]);
        return 1;
    }

    const char* pdf_path = argv[arg_idx];
    const char* output_path = argv[arg_idx + 1];

    // Check input path exists
    struct stat st;
    if (stat(pdf_path, &st) != 0) {
        // Guess if user intended a file or directory based on extension
        std::string path_str(pdf_path);
        bool likely_file = (path_str.length() >= 4 && path_str.substr(path_str.length() - 4) == ".pdf");
        ErrorCode error_code = likely_file ? ErrorCode::FileNotFound : ErrorCode::DirectoryNotFound;
        std::string message = likely_file ?
            std::string("Cannot find PDF file: ") + pdf_path :
            std::string("Cannot find directory: ") + pdf_path;
        ErrorReporter::ReportError(error_code, message);
        return 1;
    }

    // v2.0.0: Auto-detect directory vs file (no --batch flag needed)
    if (S_ISDIR(st.st_mode)) {
        // Auto-detected directory: dispatch to batch processor
        return ProcessBatch(pdf_path, output_path, operation, worker_count, thread_count, dpi,
                          use_ppm, use_jpeg, jpeg_quality, use_raw, render_quality, benchmark_mode, force_alpha,
                          pattern, recursive, pixel_format);
    }

    // Single file mode: ensure input is a file
    if (!S_ISREG(st.st_mode)) {
        fprintf(stderr, "Error: Input must be a PDF file or directory: %s\n", pdf_path);
        return 1;
    }

    // Validate page range if specified
    if (start_page != -1 || end_page != -1) {
        int page_count = get_page_count(pdf_path);
        if (page_count < 0) {
            ErrorReporter::ReportError(ErrorCode::CannotOpen,
                                     std::string("Cannot read PDF: ") + pdf_path);
            return 2;
        }

        // Set defaults for unspecified bounds
        if (start_page == -1) start_page = 0;
        if (end_page == -1) end_page = page_count - 1;

        // Validate range
        if (start_page < 0 || end_page >= page_count || start_page > end_page) {
            char buf[256];
            snprintf(buf, sizeof(buf), "Page range %d-%d invalid (document has %d pages, 0-indexed)",
                    start_page, end_page, page_count);
            ErrorReporter::ReportError(ErrorCode::PageRangeInvalid, buf);
            return 1;
        }
    }

    // v1.9.0: Apply preset configuration (N=43)
    // Presets provide simple defaults for common use cases
    // Note: Flags parsed after --preset in command line will override preset values
    if (preset != RenderPreset::NONE) {
        const PresetConfig& config = PRESET_CONFIGS[static_cast<int>(preset)];

        // Apply preset values
        dpi = config.dpi;
        use_jpeg = config.use_jpeg;
        if (use_jpeg) {
            jpeg_quality = config.jpeg_quality;
        }

        // Note: max_dimension is NOT applied here - too complex for v1.9.0
        // Future work: implement downscaling logic in render_page_to_png
    }

    // Execute operation
    switch (operation) {
        case OP_EXTRACT_TEXT:
            if (mode == MODE_DEBUG) {
                fprintf(stderr, "Mode: debug (tracing enabled)\n");
                return extract_text_debug(pdf_path, output_path, use_utf8);
            } else {
                if (worker_count == 1) {
                    fprintf(stderr, "Mode: single-threaded (1 worker)\n");
                    return extract_text_bulk(pdf_path, output_path, start_page, end_page, use_utf8);
                } else {
                    fprintf(stderr, "Mode: multi-process (%d workers)\n", worker_count);
                    return extract_text_fast(pdf_path, output_path, worker_count, start_page, end_page, use_utf8);
                }
            }
        case OP_EXTRACT_JSONL:
            {
                // JSONL extraction: single page only
                // Priority: --pages flag > positional argument > default (page 0)
                int page_num = 0;

                // First check --pages flag
                if (start_page >= 0) {
                    // --pages was specified
                    if (start_page != end_page) {
                        fprintf(stderr, "Error: extract-jsonl only supports single page, use --pages N (not range)\n");
                        return 1;
                    }
                    page_num = start_page;
                } else if (argc > arg_idx + 2) {
                    // Check positional argument
                    char* end;
                    long val = strtol(argv[arg_idx + 2], &end, 10);
                    if (*end == '\0' && val >= 0) {
                        page_num = (int)val;
                    } else {
                        fprintf(stderr, "Error: Invalid page number: %s\n", argv[arg_idx + 2]);
                        return 1;
                    }
                }
                // JSONL doesn't support multi-worker mode (single page only)
                if (worker_count > 1) {
                    fprintf(stderr, "Warning: JSONL extraction is single-page only, ignoring worker count\n");
                }
                if (mode == MODE_DEBUG) {
                    fprintf(stderr, "Mode: debug (single page %d, tracing enabled)\n", page_num);
                    return extract_jsonl_debug(pdf_path, output_path, page_num);
                } else {
                    fprintf(stderr, "Mode: single page %d\n", page_num);
                    return extract_jsonl_bulk(pdf_path, output_path, page_num);
                }
            }
        case OP_RENDER_PAGES:
            // Create output directory if it doesn't exist
            {
                struct stat st_dir;
                if (stat(output_path, &st_dir) != 0) {
                    // Directory doesn't exist, try to create it
                    #ifdef _WIN32
                    if (mkdir(output_path) != 0) {
                    #else
                    if (mkdir(output_path, 0755) != 0) {
                    #endif
                        fprintf(stderr, "Error: Failed to create output directory: %s\n", output_path);
                        return 1;
                    }
                }
            }
            if (mode == MODE_DEBUG) {
                fprintf(stderr, "Mode: debug (tracing enabled, %.0f DPI, %s, smart)\n", dpi, use_ppm ? "PPM" : (use_raw ? "BGRA" : (use_jpeg ? "JPEG" : "PNG")));
                return render_pages_debug(pdf_path, output_path, dpi, use_ppm, use_jpeg, jpeg_quality, use_raw, render_quality, force_alpha);
            } else {
                if (worker_count == 1) {
                    // N=320: Adaptive threading moved to render_pages_bulk()
                    // (avoids double PDF open overhead - page count determined there)
                    if (thread_count == 1) {
                        fprintf(stderr, "Mode: single-threaded (1 worker, 1 thread, %.0f DPI, %s, smart)\n", dpi, use_ppm ? "PPM" : (use_raw ? "BGRA" : (use_jpeg ? "JPEG" : "PNG")));
                    } else {
                        fprintf(stderr, "Mode: multi-threaded (1 worker, %d threads, %.0f DPI, %s, smart)\n", thread_count, dpi, use_ppm ? "PPM" : (use_raw ? "BGRA" : (use_jpeg ? "JPEG" : "PNG")));
                    }
                    return render_pages_bulk(pdf_path, output_path, dpi, use_ppm, use_jpeg, jpeg_quality, use_raw, start_page, end_page, thread_count, render_quality, benchmark_mode, user_set_threads, enable_adaptive, force_alpha, pixel_format);
                } else {
                    if (thread_count == 1) {
                        fprintf(stderr, "Mode: multi-process (%d workers, %.0f DPI, %s, smart)\n", worker_count, dpi, use_ppm ? "PPM" : (use_raw ? "BGRA" : (use_jpeg ? "JPEG" : "PNG")));
                    } else {
                        fprintf(stderr, "Mode: hybrid N×K (%d workers, %d threads each, %.0f DPI, %s, smart)\n", worker_count, thread_count, dpi, use_ppm ? "PPM" : (use_raw ? "BGRA" : (use_jpeg ? "JPEG" : "PNG")));
                    }
                    // TODO: Pass pixel_format to render_pages_fast when worker subprocess support is added
                    return render_pages_fast(pdf_path, output_path, worker_count, dpi, use_ppm, use_jpeg, jpeg_quality, use_raw, start_page, end_page, render_quality, benchmark_mode, force_alpha, thread_count);
                }
            }
    }
}

// Get page count helper
int get_page_count(const char* pdf_path) {
    FPDF_InitLibrary();
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        FPDF_DestroyLibrary();
        return -1;
    }
    int count = FPDF_GetPageCount(doc);
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();
    return count;
}

// Write UTF-32 LE BOM
void write_bom(FILE* out) {
    unsigned char bom[4] = {0xFF, 0xFE, 0x00, 0x00};
    fwrite(bom, 1, 4, out);
}

// Write UTF-8 BOM
void write_utf8_bom(FILE* out) {
    unsigned char bom[3] = {0xEF, 0xBB, 0xBF};
    fwrite(bom, 1, 3, out);
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

// Write codepoint as UTF-8
void write_utf8_codepoint(FILE* out, unsigned int codepoint) {
    unsigned char bytes[4];
    size_t len = 0;
    if (codepoint < 0x80) {
        bytes[0] = (unsigned char)codepoint;
        len = 1;
    } else if (codepoint < 0x800) {
        bytes[0] = (unsigned char)(0xC0 | (codepoint >> 6));
        bytes[1] = (unsigned char)(0x80 | (codepoint & 0x3F));
        len = 2;
    } else if (codepoint < 0x10000) {
        bytes[0] = (unsigned char)(0xE0 | (codepoint >> 12));
        bytes[1] = (unsigned char)(0x80 | ((codepoint >> 6) & 0x3F));
        bytes[2] = (unsigned char)(0x80 | (codepoint & 0x3F));
        len = 3;
    } else if (codepoint < 0x110000) {
        bytes[0] = (unsigned char)(0xF0 | (codepoint >> 18));
        bytes[1] = (unsigned char)(0x80 | ((codepoint >> 12) & 0x3F));
        bytes[2] = (unsigned char)(0x80 | ((codepoint >> 6) & 0x3F));
        bytes[3] = (unsigned char)(0x80 | (codepoint & 0x3F));
        len = 4;
    } else {
        // Invalid codepoint - write replacement character
        bytes[0] = 0xEF; bytes[1] = 0xBF; bytes[2] = 0xBD;
        len = 3;
    }
    fwrite(bytes, 1, len, out);
}

// Append codepoint as UTF-8 to buffer
void append_utf8_codepoint(std::vector<unsigned char>& buffer, unsigned int codepoint) {
    if (codepoint < 0x80) {
        // 1-byte UTF-8 (ASCII)
        buffer.push_back((unsigned char)codepoint);
    } else if (codepoint < 0x800) {
        // 2-byte UTF-8
        buffer.push_back((unsigned char)(0xC0 | (codepoint >> 6)));
        buffer.push_back((unsigned char)(0x80 | (codepoint & 0x3F)));
    } else if (codepoint < 0x10000) {
        // 3-byte UTF-8
        buffer.push_back((unsigned char)(0xE0 | (codepoint >> 12)));
        buffer.push_back((unsigned char)(0x80 | ((codepoint >> 6) & 0x3F)));
        buffer.push_back((unsigned char)(0x80 | (codepoint & 0x3F)));
    } else if (codepoint < 0x110000) {
        // 4-byte UTF-8
        buffer.push_back((unsigned char)(0xF0 | (codepoint >> 18)));
        buffer.push_back((unsigned char)(0x80 | ((codepoint >> 12) & 0x3F)));
        buffer.push_back((unsigned char)(0x80 | ((codepoint >> 6) & 0x3F)));
        buffer.push_back((unsigned char)(0x80 | (codepoint & 0x3F)));
    } else {
        // Invalid codepoint - use replacement character U+FFFD
        buffer.push_back(0xEF);
        buffer.push_back(0xBF);
        buffer.push_back(0xBD);
    }
}

// ========================================
// Bulk Mode: Single-threaded extraction
// ========================================

int extract_text_bulk(const char* pdf_path, const char* output_path, int start_page, int end_page, bool use_utf8 = true) {
    FPDF_InitLibrary();

    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Error: Failed to load PDF: %s\n", pdf_path);
        FPDF_DestroyLibrary();
        return 2;
    }

    FILE* out = fopen(output_path, "wb");
    if (!out) {
        fprintf(stderr, "Error: Failed to create output file: %s\n", output_path);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    // Write file-level BOM (UTF-8 or UTF-32 LE)
    if (use_utf8) {
        write_utf8_bom(out);
    } else {
        write_bom(out);
    }

    int page_count = FPDF_GetPageCount(doc);
    if (page_count <= 0) {
        // Gracefully handle 0-page PDFs: output BOM only, return success
        if (fclose(out) != 0) {
            fprintf(stderr, "Error: Failed to close output file '%s': %s\n", output_path, strerror(errno));
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return 1;
        }
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 0;
    }

    // Set page range defaults if not specified
    if (start_page == -1) start_page = 0;
    if (end_page == -1) end_page = page_count - 1;

    // Task 2.3: Buffer Pooling - Reusable page buffer to reduce malloc/free overhead
    // Initial capacity: 256KB (typical page has ~10K-100K chars = 40KB-400KB UTF-32)
    std::vector<unsigned char> page_buffer;
    page_buffer.reserve(256 * 1024);

    // Extract specified page range
    for (int page_idx = start_page; page_idx <= end_page; page_idx++) {
        // Clear buffer for reuse (retains capacity)
        page_buffer.clear();

        // Write page separator BOM (not for first page in range, UTF-32 LE only)
        if (page_idx > start_page && !use_utf8) {
            // Add BOM to buffer instead of writing directly
            page_buffer.push_back(0xFF);
            page_buffer.push_back(0xFE);
            page_buffer.push_back(0x00);
            page_buffer.push_back(0x00);
        }

        FPDF_PAGE page = FPDF_LoadPage(doc, page_idx);
        if (!page) {
            fprintf(stderr, "Warning: Failed to load page %d\n", page_idx);
            continue;
        }

        FPDF_TEXTPAGE text_page = FPDFText_LoadPage(page);
        if (!text_page) {
            fprintf(stderr, "Warning: Failed to load text for page %d\n", page_idx);
            FPDF_ClosePage(page);
            continue;
        }

        int char_count = FPDFText_CountChars(text_page);

        // Pre-allocate for this page if needed (char_count * 4 bytes per char)
        size_t required_capacity = page_buffer.size() + (char_count * 4);
        if (page_buffer.capacity() < required_capacity) {
            page_buffer.reserve(required_capacity);
        }

        // Task 2.5: ASCII Fast Path - Detect ASCII-only pages for 15-25% speedup
        // First pass: Check if all characters are ASCII (0-127)
        bool is_ascii = true;
        for (int i = 0; i < char_count; i++) {
            unsigned int unicode = FPDFText_GetUnicode(text_page, i);
            if (unicode > 127) {
                is_ascii = false;
                break;
            }
        }

        if (is_ascii && char_count > 0) {
            // Fast path: Direct byte output (ASCII is same in UTF-8 and UTF-32)
            // No surrogate pair handling needed
            // No bit shifting or masking operations
            for (int i = 0; i < char_count; i++) {
                unsigned int unicode = FPDFText_GetUnicode(text_page, i);
                if (use_utf8) {
                    // UTF-8: 1 byte per ASCII char
                    page_buffer.push_back((unsigned char)unicode);
                } else {
                    // UTF-32 LE: 4 bytes per char (but only lower byte is non-zero for ASCII)
                    page_buffer.push_back((unsigned char)unicode);
                    page_buffer.push_back(0);
                    page_buffer.push_back(0);
                    page_buffer.push_back(0);
                }
            }
        } else {
            // Slow path: Full Unicode handling with UTF-16 surrogate pairs
            int i = 0;
            while (i < char_count) {
                unsigned int unicode = FPDFText_GetUnicode(text_page, i);
                unsigned int codepoint;

                // Handle UTF-16 surrogate pairs
                int chars_consumed = 1;
                if (unicode >= 0xD800 && unicode <= 0xDBFF) {
                    // High surrogate - need to read low surrogate
                    if (i + 1 < char_count) {
                        unsigned int low = FPDFText_GetUnicode(text_page, i + 1);
                        if (low >= 0xDC00 && low <= 0xDFFF) {
                            // Valid surrogate pair
                            codepoint = ((unicode - 0xD800) << 10) + (low - 0xDC00) + 0x10000;
                            chars_consumed = 2;
                        } else {
                            // Invalid surrogate pair - use replacement character
                            codepoint = 0xFFFD;
                        }
                    } else {
                        // High surrogate at end of text - invalid
                        codepoint = 0xFFFD;
                    }
                } else if (unicode >= 0xDC00 && unicode <= 0xDFFF) {
                    // Lone low surrogate (invalid) - use replacement character
                    codepoint = 0xFFFD;
                } else {
                    codepoint = unicode;
                }

                // Append codepoint to buffer (UTF-8 or UTF-32 LE)
                if (use_utf8) {
                    append_utf8_codepoint(page_buffer, codepoint);
                } else {
                    page_buffer.push_back((codepoint) & 0xFF);
                    page_buffer.push_back((codepoint >> 8) & 0xFF);
                    page_buffer.push_back((codepoint >> 16) & 0xFF);
                    page_buffer.push_back((codepoint >> 24) & 0xFF);
                }
                i += chars_consumed;
            }
        }

        // Single write for entire page (reduces system call overhead)
        if (!page_buffer.empty()) {
            size_t written = fwrite(page_buffer.data(), 1, page_buffer.size(), out);
            if (written != page_buffer.size()) {
                fprintf(stderr, "Error: Failed to write page %d to '%s': %s\n", page_idx + 1, output_path, strerror(errno));
                FPDFText_ClosePage(text_page);
                FPDF_ClosePage(page);
                fclose(out);
                FPDF_CloseDocument(doc);
                FPDF_DestroyLibrary();
                return 1;
            }
        }

        FPDFText_ClosePage(text_page);
        FPDF_ClosePage(page);
    }

    if (fclose(out) != 0) {
        fprintf(stderr, "Error: Failed to close output file '%s': %s\n", output_path, strerror(errno));
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    fprintf(stderr, "Text extraction complete: %s\n", output_path);
    return 0;
}

// ========================================
// Fast Mode: Multi-process extraction
// ========================================

int extract_text_fast(const char* pdf_path, const char* output_path, int worker_count, int start_page, int end_page, bool use_utf8) {
    // Get page count
    int total_page_count = get_page_count(pdf_path);
    if (total_page_count < 0) {
        fprintf(stderr, "Error: Failed to get page count\n");
        return 2;
    }

    // Set page range defaults if not specified
    if (start_page == -1) start_page = 0;
    if (end_page == -1) end_page = total_page_count - 1;

    // Validate page range
    if (start_page < 0 || end_page >= total_page_count || start_page > end_page) {
        fprintf(stderr, "Error: Invalid page range %d-%d (document has %d pages)\n",
                start_page, end_page, total_page_count);
        return 2;
    }

    int page_count = end_page - start_page + 1;
    fprintf(stderr, "Processing %d pages with %d workers\n", page_count, worker_count);

    // Calculate pages per worker
    int pages_per_worker = (page_count + worker_count - 1) / worker_count;

    // Spawn worker processes
    pid_t* pids = (pid_t*)malloc(sizeof(pid_t) * worker_count);
    char** temp_files = (char**)malloc(sizeof(char*) * worker_count);
    if (!pids || !temp_files) {
        fprintf(stderr, "Error: Memory allocation failed\n");
        free(pids);
        free(temp_files);
        return -1;
    }
    int actual_workers = 0;

    for (int worker_id = 0; worker_id < worker_count; worker_id++) {
        int worker_start = start_page + (worker_id * pages_per_worker);
        int worker_end = worker_start + pages_per_worker;  // EXCLUSIVE end
        if (worker_end > end_page + 1) worker_end = end_page + 1;  // Adjust to exclusive bound

        if (worker_start > end_page) {
            break;
        }

        // Create temp file path
        temp_files[worker_id] = (char*)malloc(256);
        if (!temp_files[worker_id]) {
            fprintf(stderr, "Error: Memory allocation failed for temp file path\n");
            // Clean up previously allocated temp files
            for (int j = 0; j < worker_id; j++) {
                free(temp_files[j]);
            }
            free(pids);
            free(temp_files);
            return -1;
        }
        // Use mkstemp for secure temp file creation (prevent symlink attacks)
        char temp_template[] = "/tmp/pdfium_worker_XXXXXX";
        int temp_fd = mkstemp(temp_template);
        if (temp_fd < 0) {
            fprintf(stderr, "Error: Failed to create secure temp file: %s\n", strerror(errno));
            free(temp_files[worker_id]);
            for (int j = 0; j < worker_id; j++) {
                unlink(temp_files[j]);
                free(temp_files[j]);
            }
            free(pids);
            free(temp_files);
            return -1;
        }
        close(temp_fd);  // Will be reopened by worker
        strncpy(temp_files[worker_id], temp_template, 256);
        temp_files[worker_id][255] = '\0';  // Ensure null termination

        // Fork worker
        pid_t pid = fork();
        if (pid == 0) {
            // Child process - exec worker
            char start_str[16], end_str[16], id_str[16];
            snprintf(start_str, sizeof(start_str), "%d", worker_start);
            snprintf(end_str, sizeof(end_str), "%d", worker_end);
            snprintf(id_str, sizeof(id_str), "%d", worker_id);

            // Get executable path
            char exe_path[1024];
#ifdef __APPLE__
            uint32_t size = sizeof(exe_path);
            if (_NSGetExecutablePath(exe_path, &size) != 0) {
                fprintf(stderr, "Error: Failed to get executable path\n");
                exit(1);
            }
#else
            ssize_t len = readlink("/proc/self/exe", exe_path, sizeof(exe_path) - 1);
            if (len == -1) {
                fprintf(stderr, "Error: Failed to get executable path\n");
                exit(1);
            }
            exe_path[len] = '\0';
#endif

            const char* encoding = use_utf8 ? "utf8" : "utf32le";
            execl(exe_path, exe_path, "--worker", pdf_path, temp_files[worker_id],
                  start_str, end_str, id_str, encoding, NULL);

            // If exec fails
            fprintf(stderr, "Error: Failed to exec worker\n");
            exit(1);
        } else if (pid > 0) {
            // Parent process
            pids[actual_workers] = pid;
            actual_workers++;
        } else {
            fprintf(stderr, "Error: Failed to fork worker %d\n", worker_id);
            // Clean up all allocated resources before returning
            for (int j = 0; j <= worker_id; j++) {
                unlink(temp_files[j]);  // Delete temp file first
                free(temp_files[j]);
            }
            free(pids);
            free(temp_files);
            return 3;
        }
    }

    // Wait for all workers
    int all_success = 1;
    for (int i = 0; i < actual_workers; i++) {
        int status;
        waitpid(pids[i], &status, 0);
        if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
            fprintf(stderr, "Error: Worker %d failed\n", i);
            all_success = 0;
        }
    }

    if (!all_success) {
        free(pids);
        for (int i = 0; i < actual_workers; i++) {
            unlink(temp_files[i]);
            free(temp_files[i]);
        }
        free(temp_files);
        return 3;
    }

    // Merge worker outputs
    FILE* out = fopen(output_path, "wb");
    if (!out) {
        fprintf(stderr, "Error: Failed to create output file: %s\n", output_path);
        free(pids);
        for (int i = 0; i < actual_workers; i++) {
            unlink(temp_files[i]);
            free(temp_files[i]);
        }
        free(temp_files);
        return 1;
    }

    // Write file-level BOM
    if (use_utf8) {
        write_utf8_bom(out);
    } else {
        write_bom(out);
    }

    // Concatenate worker outputs
    for (int i = 0; i < actual_workers; i++) {
        FILE* temp = fopen(temp_files[i], "rb");
        if (!temp) {
            fprintf(stderr, "Error: Failed to open temp file: %s\n", temp_files[i]);
            continue;
        }

        char buffer[8192];
        size_t bytes;
        bool write_failed = false;
        while ((bytes = fread(buffer, 1, sizeof(buffer), temp)) > 0) {
            size_t written = fwrite(buffer, 1, bytes, out);
            if (written != bytes) {
                fprintf(stderr, "Error: Failed to write to output file '%s': %s\n", output_path, strerror(errno));
                write_failed = true;
                break;
            }
        }

        fclose(temp);
        unlink(temp_files[i]);
        free(temp_files[i]);

        if (write_failed) {
            // Clean up remaining temp files
            for (int j = i + 1; j < actual_workers; j++) {
                unlink(temp_files[j]);
                free(temp_files[j]);
            }
            fclose(out);
            free(pids);
            free(temp_files);
            return 1;
        }
    }

    if (fclose(out) != 0) {
        fprintf(stderr, "Error: Failed to close output file '%s': %s\n", output_path, strerror(errno));
        free(pids);
        free(temp_files);
        return 1;
    }
    free(pids);
    free(temp_files);

    fprintf(stderr, "Text extraction complete: %s\n", output_path);
    return 0;
}

// ========================================
// Debug Mode: Extraction with tracing
// ========================================

int extract_text_debug(const char* pdf_path, const char* output_path, bool use_utf8) {
    fprintf(stderr, "[TRACE] FPDF_InitLibrary()\n");
    FPDF_InitLibrary();

    fprintf(stderr, "[TRACE] FPDF_LoadDocument(%s)\n", pdf_path);
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "[ERROR] Failed to load PDF\n");
        FPDF_DestroyLibrary();
        return 2;
    }
    fprintf(stderr, "[TRACE] Document loaded: %p\n", doc);

    int page_count = FPDF_GetPageCount(doc);
    fprintf(stderr, "[TRACE] FPDF_GetPageCount() -> %d\n", page_count);

    FILE* out = fopen(output_path, "wb");
    if (!out) {
        fprintf(stderr, "[ERROR] Failed to create output file\n");
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    // Write BOM based on encoding (UTF-8 or UTF-32 LE)
    if (use_utf8) {
        write_utf8_bom(out);
    } else {
        write_bom(out);
    }
    fprintf(stderr, "[TRACE] Wrote file BOM (%s)\n", use_utf8 ? "UTF-8" : "UTF-32 LE");

    int total_chars = 0;

    for (int page_idx = 0; page_idx < page_count; page_idx++) {
        // Page separator BOM (UTF-32 LE only, not for UTF-8)
        if (page_idx > 0 && !use_utf8) {
            write_bom(out);
        }

        fprintf(stderr, "[TRACE] Processing page %d/%d\n", page_idx + 1, page_count);

        FPDF_PAGE page = FPDF_LoadPage(doc, page_idx);
        if (!page) {
            fprintf(stderr, "[WARN] Failed to load page %d\n", page_idx);
            continue;
        }

        FPDF_TEXTPAGE text_page = FPDFText_LoadPage(page);
        if (!text_page) {
            fprintf(stderr, "[WARN] Failed to load text for page %d\n", page_idx);
            FPDF_ClosePage(page);
            continue;
        }

        int char_count = FPDFText_CountChars(text_page);
        fprintf(stderr, "[DEBUG] Page %d: %d characters\n", page_idx, char_count);
        total_chars += char_count;

        int surrogate_pairs = 0;
        int i = 0;
        while (i < char_count) {
            unsigned int unicode = FPDFText_GetUnicode(text_page, i);
            unsigned int codepoint;
            int chars_consumed = 1;

            if (unicode >= 0xD800 && unicode <= 0xDBFF) {
                // High surrogate - need to read low surrogate
                surrogate_pairs++;
                if (i + 1 < char_count) {
                    unsigned int low = FPDFText_GetUnicode(text_page, i + 1);
                    if (low >= 0xDC00 && low <= 0xDFFF) {
                        // Valid surrogate pair
                        codepoint = ((unicode - 0xD800) << 10) + (low - 0xDC00) + 0x10000;
                        chars_consumed = 2;
                    } else {
                        // Invalid surrogate pair - use replacement character
                        codepoint = 0xFFFD;
                    }
                } else {
                    // High surrogate at end of text - invalid
                    codepoint = 0xFFFD;
                }
            } else if (unicode >= 0xDC00 && unicode <= 0xDFFF) {
                // Lone low surrogate (invalid) - use replacement character
                codepoint = 0xFFFD;
            } else {
                codepoint = unicode;
            }

            // Write codepoint in appropriate encoding
            if (use_utf8) {
                write_utf8_codepoint(out, codepoint);
            } else {
                write_codepoint(out, codepoint);
            }
            i += chars_consumed;
        }

        if (surrogate_pairs > 0) {
            fprintf(stderr, "[DEBUG]   - Surrogate pairs: %d\n", surrogate_pairs);
        }

        FPDFText_ClosePage(text_page);
        FPDF_ClosePage(page);
    }

    if (fclose(out) != 0) {
        fprintf(stderr, "Error: Failed to close output file '%s': %s\n", output_path, strerror(errno));
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    fprintf(stderr, "[SUMMARY] Total: %d pages, %d characters\n", page_count, total_chars);
    fprintf(stderr, "[TRACE] Text extraction complete: %s\n", output_path);
    return 0;
}

// ========================================
// Worker Process (internal)
// ========================================

int extract_text_worker(const char* pdf_path, const char* output_path,
                        int start_page, int end_page, int worker_id, bool use_utf8) {
    FPDF_InitLibrary();

    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Worker %d: Failed to load PDF\n", worker_id);
        FPDF_DestroyLibrary();
        return 2;
    }

    FILE* out = fopen(output_path, "wb");
    if (!out) {
        fprintf(stderr, "Worker %d: Failed to create output file\n", worker_id);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    // Task 2.3: Buffer Pooling - Reusable page buffer
    std::vector<unsigned char> page_buffer;
    page_buffer.reserve(256 * 1024);

    // Process assigned pages
    for (int page_idx = start_page; page_idx < end_page; page_idx++) {
        // Clear buffer for reuse
        page_buffer.clear();

        FPDF_PAGE page = FPDF_LoadPage(doc, page_idx);
        if (!page) {
            fprintf(stderr, "Worker %d: Failed to load page %d\n", worker_id, page_idx);
            continue;
        }

        FPDF_TEXTPAGE text_page = FPDFText_LoadPage(page);
        if (!text_page) {
            fprintf(stderr, "Worker %d: Failed to load text for page %d\n", worker_id, page_idx);
            FPDF_ClosePage(page);
            continue;
        }

        // Write page BOM (skip first page of worker 0, controller adds file BOM)
        // UTF-32 LE only (UTF-8 doesn't use page separators)
        if (!use_utf8 && !(worker_id == 0 && page_idx == start_page)) {
            page_buffer.push_back(0xFF);
            page_buffer.push_back(0xFE);
            page_buffer.push_back(0x00);
            page_buffer.push_back(0x00);
        }

        int char_count = FPDFText_CountChars(text_page);

        // Pre-allocate for this page
        size_t required_capacity = page_buffer.size() + (char_count * 4);
        if (page_buffer.capacity() < required_capacity) {
            page_buffer.reserve(required_capacity);
        }

        // Task 2.5: ASCII Fast Path
        bool is_ascii = true;
        for (int i = 0; i < char_count; i++) {
            unsigned int unicode = FPDFText_GetUnicode(text_page, i);
            if (unicode > 127) {
                is_ascii = false;
                break;
            }
        }

        if (is_ascii && char_count > 0) {
            // Fast path: Direct byte output (ASCII is same in UTF-8 and UTF-32 LE)
            for (int i = 0; i < char_count; i++) {
                unsigned int unicode = FPDFText_GetUnicode(text_page, i);
                if (use_utf8) {
                    // UTF-8: 1 byte per ASCII char
                    page_buffer.push_back((unsigned char)unicode);
                } else {
                    // UTF-32 LE: 4 bytes per char
                    page_buffer.push_back((unsigned char)unicode);
                    page_buffer.push_back(0);
                    page_buffer.push_back(0);
                    page_buffer.push_back(0);
                }
            }
        } else {
            // Slow path: Full Unicode handling
            int i = 0;
            while (i < char_count) {
                unsigned int unicode = FPDFText_GetUnicode(text_page, i);
                unsigned int codepoint;
                int chars_consumed = 1;

                if (unicode >= 0xD800 && unicode <= 0xDBFF) {
                    // High surrogate - need to read low surrogate
                    if (i + 1 < char_count) {
                        unsigned int low = FPDFText_GetUnicode(text_page, i + 1);
                        if (low >= 0xDC00 && low <= 0xDFFF) {
                            // Valid surrogate pair
                            codepoint = ((unicode - 0xD800) << 10) + (low - 0xDC00) + 0x10000;
                            chars_consumed = 2;
                        } else {
                            // Invalid surrogate pair - use replacement character
                            codepoint = 0xFFFD;
                        }
                    } else {
                        // High surrogate at end of text - invalid
                        codepoint = 0xFFFD;
                    }
                } else if (unicode >= 0xDC00 && unicode <= 0xDFFF) {
                    // Lone low surrogate (invalid) - use replacement character
                    codepoint = 0xFFFD;
                } else {
                    codepoint = unicode;
                }

                // Append codepoint to buffer (UTF-8 or UTF-32 LE)
                if (use_utf8) {
                    append_utf8_codepoint(page_buffer, codepoint);
                } else {
                    // UTF-32 LE: 4 bytes per codepoint
                    page_buffer.push_back((codepoint) & 0xFF);
                    page_buffer.push_back((codepoint >> 8) & 0xFF);
                    page_buffer.push_back((codepoint >> 16) & 0xFF);
                    page_buffer.push_back((codepoint >> 24) & 0xFF);
                }
                i += chars_consumed;
            }
        }

        // Single write for entire page
        if (!page_buffer.empty()) {
            size_t written = fwrite(page_buffer.data(), 1, page_buffer.size(), out);
            if (written != page_buffer.size()) {
                fprintf(stderr, "Worker %d: Failed to write page %d to '%s': %s\n", worker_id, page_idx + 1, output_path, strerror(errno));
                FPDFText_ClosePage(text_page);
                FPDF_ClosePage(page);
                fclose(out);
                FPDF_CloseDocument(doc);
                FPDF_DestroyLibrary();
                return 1;
            }
        }

        FPDFText_ClosePage(text_page);
        FPDF_ClosePage(page);
    }

    if (fclose(out) != 0) {
        fprintf(stderr, "Error: Failed to close output file '%s': %s\n", output_path, strerror(errno));
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    return 0;
}

// ========================================
// Image Rendering Functions
// ========================================

bool write_png(const char* filename, const std::vector<uint8_t>& png_data) {
    // Use memory-mapped I/O for zero-copy writes (v1.8.0 optimization)
    int fd = open(filename, O_RDWR | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        fprintf(stderr, "Error: Failed to create PNG file: %s\n", filename);
        return false;
    }

    size_t size = png_data.size();

    // Handle empty data - mmap(size=0) is undefined behavior per POSIX
    if (size == 0) {
        close(fd);
        return true;  // Empty file created successfully
    }

    // Set file size
    if (ftruncate(fd, size) != 0) {
        fprintf(stderr, "Error: Failed to set file size for: %s\n", filename);
        close(fd);
        return false;
    }

    // Memory-map the file
    void* mapped = mmap(NULL, size, PROT_WRITE, MAP_SHARED, fd, 0);
    if (mapped == MAP_FAILED) {
        fprintf(stderr, "Error: Failed to mmap file: %s\n", filename);
        close(fd);
        return false;
    }

    // Write directly to memory-mapped region (zero-copy)
    memcpy(mapped, png_data.data(), size);

    // Unmap and close
    munmap(mapped, size);
    close(fd);

    return true;
}

// Write PPM P6 format (matching upstream testing/helpers/write.cc:266-315)
// N=41: Updated to support both BGR (3 bytes) and BGRA (4 bytes) formats
bool write_ppm(const char* filename, void* buffer, int stride, int width, int height, int bitmap_format) {
    // Check dimensions
    if (stride < 0 || width < 0 || height < 0) {
        return false;
    }

    // Check for integer overflow BEFORE multiplication
    // width * height * 3 must fit in int
    if (width > 0 && height > INT_MAX / width) {
        return false;  // width * height would overflow
    }
    int out_len = width * height;
    if (out_len > INT_MAX / 3) {
        return false;  // out_len * 3 would overflow
    }
    out_len *= 3;

    FILE* fp = fopen(filename, "wb");
    if (!fp) {
        fprintf(stderr, "Error: Failed to create PPM file: %s\n", filename);
        return false;
    }

    // Write P6 header (binary RGB) - N=197: no comment to match parallel path
    fprintf(fp, "P6\n%d %d\n255\n", width, height);

    const uint8_t* src = static_cast<const uint8_t*>(buffer);
    std::vector<uint8_t> result(out_len);

    if (bitmap_format == FPDFBitmap_BGR) {
        // N=41: BGR format (3 bytes per pixel) - direct conversion
        for (int h = 0; h < height; ++h) {
            const uint8_t* src_line = src + (stride * h);
            uint8_t* dest_line = result.data() + (width * h * 3);
            for (int w = 0; w < width; ++w) {
                // Source: B, G, R (3 bytes per pixel)
                // Dest: R, G, B (3 bytes per pixel)
                dest_line[w * 3] = src_line[(w * 3) + 2];      // R
                dest_line[(w * 3) + 1] = src_line[(w * 3) + 1]; // G
                dest_line[(w * 3) + 2] = src_line[w * 3];      // B
            }
        }
    } else {
        // BGRA/BGRx format (4 bytes per pixel) - discard alpha
        for (int h = 0; h < height; ++h) {
            const uint8_t* src_line = src + (stride * h);
            uint8_t* dest_line = result.data() + (width * h * 3);
            for (int w = 0; w < width; ++w) {
                // Source: B, G, R, A (4 bytes per pixel)
                // Dest: R, G, B (3 bytes per pixel)
                dest_line[w * 3] = src_line[(w * 4) + 2];      // R
                dest_line[(w * 3) + 1] = src_line[(w * 4) + 1]; // G
                dest_line[(w * 3) + 2] = src_line[w * 4];      // B
            }
        }
    }

    size_t written = fwrite(result.data(), 1, out_len, fp);
    fclose(fp);

    if (written != (size_t)out_len) {
        fprintf(stderr, "Error: Failed to write PPM data to: %s\n", filename);
        return false;
    }

    return true;
}

bool write_bgra(const char* filename, const void* buffer, int stride, int width, int height) {
    // Check dimensions
    if (stride < 0 || width < 0 || height < 0) {
        return false;
    }

    FILE* fp = fopen(filename, "wb");
    if (!fp) {
        fprintf(stderr, "Error: Failed to create BGRA file: %s\n", filename);
        return false;
    }

    // Write simple header: format, width, height
    fprintf(fp, "BGRA %d %d\n", width, height);

    // Check for integer overflow before multiplication
    if (width > 0 && static_cast<size_t>(width) > SIZE_MAX / 4) {
        fclose(fp);
        return false;  // width * 4 would overflow
    }

    // Write raw bitmap data (no conversion, no encoding)
    const uint8_t* src = static_cast<const uint8_t*>(buffer);
    size_t bytes_per_row = static_cast<size_t>(width) * 4;  // BGRA = 4 bytes per pixel

    for (int y = 0; y < height; y++) {
        const uint8_t* row = src + (y * stride);
        size_t written = fwrite(row, 1, bytes_per_row, fp);
        if (written != bytes_per_row) {
            fprintf(stderr, "Error: Failed to write BGRA data at row %d: %s\n", y, filename);
            fclose(fp);
            return false;
        }
    }

    fclose(fp);
    return true;
}

// Write JPEG format using libjpeg-turbo
bool write_jpeg(const char* filename, void* buffer, int stride, int width, int height, int quality, int pixel_format) {
    // Check dimensions and quality
    if (stride < 0 || width < 0 || height < 0) {
        return false;
    }
    if (quality < 0 || quality > 100) {
        quality = 90;  // Default quality
    }

    // Open output file
    FILE* fp = fopen(filename, "wb");
    if (!fp) {
        fprintf(stderr, "Error: Failed to create JPEG file: %s\n", filename);
        return false;
    }

    // Initialize JPEG compression
    struct jpeg_compress_struct cinfo;
    struct jpeg_error_mgr jerr;
    cinfo.err = jpeg_std_error(&jerr);
    jpeg_create_compress(&cinfo);
    jpeg_stdio_dest(&cinfo, fp);

    // N=50: Set compression parameters based on pixel format
    cinfo.image_width = width;
    cinfo.image_height = height;
    if (pixel_format == FPDF_PARALLEL_FORMAT_GRAY) {
        cinfo.input_components = 1;  // Grayscale
        cinfo.in_color_space = JCS_GRAYSCALE;
    } else {
        cinfo.input_components = 3;  // RGB
        cinfo.in_color_space = JCS_RGB;
    }
    jpeg_set_defaults(&cinfo);
    jpeg_set_quality(&cinfo, quality, TRUE);

    // Start compression
    jpeg_start_compress(&cinfo, TRUE);

    // Check for integer overflow before multiplication
    if (width > 0 && width > INT_MAX / 3) {
        jpeg_destroy_compress(&cinfo);
        fclose(fp);
        return false;  // width * 3 would overflow
    }

    uint8_t* src_ptr = static_cast<uint8_t*>(buffer);

    // N=50: Handle different pixel formats
    if (pixel_format == FPDF_PARALLEL_FORMAT_GRAY) {
        // Grayscale: direct write (1 byte per pixel)
        for (int y = 0; y < height; y++) {
            JSAMPROW row_pointer = src_ptr + (y * stride);
            jpeg_write_scanlines(&cinfo, &row_pointer, 1);
        }
    } else if (pixel_format == FPDF_PARALLEL_FORMAT_BGR) {
        // BGR: convert to RGB (3 bytes per pixel)
        std::vector<uint8_t> rgb_row(width * 3);
        for (int y = 0; y < height; y++) {
            uint8_t* src = src_ptr + (y * stride);
            uint8_t* dst = rgb_row.data();

            for (int x = 0; x < width; x++) {
                dst[0] = src[2];  // R
                dst[1] = src[1];  // G
                dst[2] = src[0];  // B
                src += 3;
                dst += 3;
            }

            JSAMPROW row_pointer = rgb_row.data();
            jpeg_write_scanlines(&cinfo, &row_pointer, 1);
        }
    } else {
        // BGRx (default): convert BGRA to RGB (4 bytes per pixel)
        std::vector<uint8_t> rgb_row(width * 3);
        for (int y = 0; y < height; y++) {
            uint8_t* src = src_ptr + (y * stride);
            uint8_t* dst = rgb_row.data();

            for (int x = 0; x < width; x++) {
                dst[0] = src[2];  // R = B in BGRA
                dst[1] = src[1];  // G = G
                dst[2] = src[0];  // B = R in BGRA
                src += 4;  // Skip alpha
                dst += 3;
            }

            JSAMPROW row_pointer = rgb_row.data();
            jpeg_write_scanlines(&cinfo, &row_pointer, 1);
        }
    }

    // Finish compression and cleanup
    jpeg_finish_compress(&cinfo);
    jpeg_destroy_compress(&cinfo);
    fclose(fp);

    return true;
}

// ========================================
// Smart Scanned PDF Detection and Fast Path
// ========================================

bool is_scanned_page(FPDF_PAGE page) {
    // Check if page has exactly one object
    int obj_count = FPDFPage_CountObjects(page);
    if (obj_count != 1) {
        return false;
    }

    // Check if the single object is an image
    FPDF_PAGEOBJECT obj = FPDFPage_GetObject(page, 0);
    if (FPDFPageObj_GetType(obj) != FPDF_PAGEOBJ_IMAGE) {
        return false;
    }

    // Check if image covers >= 95% of page area
    FS_RECTF obj_bounds;
    if (!FPDFPageObj_GetBounds(obj, &obj_bounds.left, &obj_bounds.bottom,
                                &obj_bounds.right, &obj_bounds.top)) {
        return false;
    }

    double page_width = FPDF_GetPageWidthF(page);
    double page_height = FPDF_GetPageHeightF(page);

    double obj_area = (obj_bounds.right - obj_bounds.left) * (obj_bounds.top - obj_bounds.bottom);
    double page_area = page_width * page_height;

    if (page_area <= 0.0) {
        return false;  // Invalid page dimensions
    }
    double coverage = obj_area / page_area;

    return coverage >= 0.95;
}

bool render_scanned_page_fast(FPDF_PAGE page, const char* output_path) {
    // Get the single image object
    FPDF_PAGEOBJECT img_obj = FPDFPage_GetObject(page, 0);

    // Check if image uses DCTDecode (JPEG) filter
    int filter_count = FPDFImageObj_GetImageFilterCount(img_obj);
    bool is_jpeg = false;

    for (int i = 0; i < filter_count; i++) {
        // Get filter name length
        unsigned long filter_len = FPDFImageObj_GetImageFilter(img_obj, i, nullptr, 0);
        if (filter_len == 0) continue;

        // Get filter name
        std::vector<char> filter_name(filter_len);
        FPDFImageObj_GetImageFilter(img_obj, i, filter_name.data(), filter_len);

        // Check if it's DCTDecode (JPEG)
        if (strncmp(filter_name.data(), "DCTDecode", 9) == 0) {
            is_jpeg = true;
            break;
        }
    }

    if (!is_jpeg) {
        // Not a JPEG image, fallback to normal rendering
        return false;
    }

    // Try to get raw JPEG data size
    unsigned long raw_size = FPDFImageObj_GetImageDataRaw(img_obj, nullptr, 0);

    if (raw_size == 0) {
        // No raw data available, fallback to normal rendering
        return false;
    }

    // Extract raw JPEG bytes
    std::vector<unsigned char> jpeg_data(raw_size);
    unsigned long actual_size = FPDFImageObj_GetImageDataRaw(img_obj, jpeg_data.data(), raw_size);

    if (actual_size == 0 || actual_size > raw_size) {
        return false;
    }

    // Verify JPEG header (FF D8 FF)
    if (actual_size < 3 || jpeg_data[0] != 0xFF || jpeg_data[1] != 0xD8 || jpeg_data[2] != 0xFF) {
        // Not a valid JPEG, fallback
        return false;
    }

    // Save the raw JPEG at native resolution (no decode/re-encode)
    // This achieves 545x speedup by avoiding bitmap rendering pipeline
    FILE* f = fopen(output_path, "wb");
    if (!f) {
        fprintf(stderr, "Error: Failed to open file for writing: %s\n", output_path);
        return false;
    }

    size_t written = fwrite(jpeg_data.data(), 1, actual_size, f);
    fclose(f);

    if (written != actual_size) {
        fprintf(stderr, "Error: Failed to write JPEG data\n");
        return false;
    }

    return true;
}

int render_page_to_png(FPDF_DOCUMENT doc, FPDF_FORMHANDLE form, FormFillInfo* form_info, int page_index, const char* output_dir, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int render_quality, bool benchmark_mode, bool force_alpha) {
    FPDF_PAGE page = FPDF_LoadPage(doc, page_index);
    if (!page) {
        fprintf(stderr, "Error: Failed to load page %d\n", page_index);
        return 1;
    }

    // Set current page info for form callbacks
    if (form_info) {
        form_info->current_doc = doc;
        form_info->current_page = page;
        form_info->current_page_index = page_index;
    }

    // Call form callbacks after page load (matching upstream pdfium_test.cc:829-830)
    if (form) {
        FORM_OnAfterLoadPage(page, form);
        FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_OPEN);
    }

    // Smart mode: Try JPEG→JPEG fast path for scanned pages (always enabled)
    // Skip smart mode if PPM/raw output requested (correctness validation requires exact format)
    // Skip smart mode if benchmark mode (need to measure rendering performance)
    if (!use_ppm && !use_raw && !benchmark_mode && is_scanned_page(page)) {
        char output_path[512];
        int path_len = snprintf(output_path, sizeof(output_path), "%s/page_%05d.jpg", output_dir, page_index);
        if (path_len >= (int)sizeof(output_path)) {
            fprintf(stderr, "Error: Output path too long (max 511 chars)\n");
            FPDF_ClosePage(page);
            return 1;
        }

        if (render_scanned_page_fast(page, output_path)) {
            // Fast path succeeded - clean up and return
            if (form) {
                FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
                FORM_OnBeforeClosePage(page, form);
            }
            if (form_info) {
                form_info->current_page = nullptr;
                form_info->current_page_index = -1;
            }
            FPDF_ClosePage(page);
            return 0;
        }
        // Fast path failed - fallback to normal rendering below
    }

    // Get page dimensions
    double width_pts = FPDF_GetPageWidthF(page);
    double height_pts = FPDF_GetPageHeightF(page);

    // Convert to pixels at specified DPI
    // Floor scale to 6 decimals to match upstream pdfium_test --scale=4.166666 behavior
    double scale_raw = dpi / 72.0;
    double scale = floor(scale_raw * 1000000.0) / 1000000.0;
    double width_raw = width_pts * scale;
    double height_raw = height_pts * scale;

    // Check for overflow - pixel dimensions must fit in int
    if (width_raw > INT_MAX || width_raw < 1 || height_raw > INT_MAX || height_raw < 1) {
        fprintf(stderr, "Error: Page %d dimensions too large for rendering (%.0fx%.0f pixels)\n",
                page_index, width_raw, height_raw);
        FPDF_ClosePage(page);
        return 1;
    }
    int width_px = (int)width_raw;
    int height_px = (int)height_raw;

    // N=207: Always use BGRx (4 bytes, alpha=0) to match parallel path's BitmapPool
    // Parallel path (fpdf_parallel.cpp:139) always uses FPDFBitmap_Create(w, h, 0)
    // Must use same format in single-threaded path for K=1 == K>1 correctness
    // N=197 incorrectly used needs_alpha ? 1 : 0, causing K=1 vs K>1 differences on transparent pages
    int has_transparency = FPDFPage_HasTransparency(page);

    // N=207: Always use alpha=0 (BGRx) to match parallel path's BitmapPool (fpdf_parallel.cpp:139)
    // BitmapPool.Acquire() always calls FPDFBitmap_Create(width, height, 0)
    // Both K=1 and K>1 must use same format for byte-for-byte identical output
    // Note: Upstream pdfium_test uses transparency-aware format, but we prioritize K=1==K>1
    FPDF_BITMAP bitmap = FPDFBitmap_Create(width_px, height_px, 0);
    if (!bitmap) {
        fprintf(stderr, "Error: Failed to create bitmap for page %d\n", page_index);
        FPDF_ClosePage(page);
        return 1;
    }

    // Fill with appropriate background (matches upstream and Rust)
    uint32_t fill_color = has_transparency ? 0x00000000 : 0xFFFFFFFF;
    FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);

    // Compute render flags based on quality setting
    // quality: 0=balanced (default AA), 1=fast (no AA), 2=high (same as balanced), 3=none (no AA + limited cache)
    int flags = FPDF_ANNOT;  // Always render annotations
    if (render_quality == 1) {
        // Fast mode: disable all anti-aliasing
        flags |= FPDF_RENDER_NO_SMOOTHTEXT | FPDF_RENDER_NO_SMOOTHIMAGE | FPDF_RENDER_NO_SMOOTHPATH;
    } else if (render_quality == 3) {
        // None mode: disable all anti-aliasing + limit image cache
        flags |= FPDF_RENDER_NO_SMOOTHTEXT | FPDF_RENDER_NO_SMOOTHIMAGE | FPDF_RENDER_NO_SMOOTHPATH;
        flags |= FPDF_RENDER_LIMITEDIMAGECACHE;  // Reduce memory usage for image-heavy PDFs
    }
    // balanced (0) and high (2) use default AA (no additional flags)
    // Render page (matching upstream pdfium_test.cc:1073-1075)
    auto render_start = std::chrono::high_resolution_clock::now();
    FPDF_RenderPageBitmap(bitmap, page, 0, 0, width_px, height_px, 0, flags);

    // Draw form fields on top (matching upstream pdfium_test.cc:1001-1003)
    if (form) {
        FPDF_FFLDraw(form, bitmap, page, 0, 0, width_px, height_px, 0, flags);
    }
    auto render_end = std::chrono::high_resolution_clock::now();
    double render_time_ms = std::chrono::duration<double, std::milli>(render_end - render_start).count();

    // Get bitmap data and format
    void* buffer = FPDFBitmap_GetBuffer(bitmap);
    int stride = FPDFBitmap_GetStride(bitmap);
    int bitmap_format = FPDFBitmap_GetFormat(bitmap);

    bool success = false;
    char filename[512];
    double encode_time_ms = 0.0;
    double write_time_ms = 0.0;

    if (use_raw) {
        // N=328: Write raw BGRA file (no encoding, direct bitmap dump)
        snprintf(filename, sizeof(filename), "%s/page_%05d.bgra", output_dir, page_index);
        auto write_start = std::chrono::high_resolution_clock::now();
        if (!benchmark_mode) {
            success = write_bgra(filename, buffer, stride, width_px, height_px);
        } else {
            success = true;  // Skip write in benchmark mode
        }
        auto write_end = std::chrono::high_resolution_clock::now();
        write_time_ms = std::chrono::duration<double, std::milli>(write_end - write_start).count();
    } else if (use_ppm) {
        // Write PPM file (for MD5 validation against upstream)
        // N=41: Pass bitmap_format to handle both BGR and BGRA
        snprintf(filename, sizeof(filename), "%s/page_%05d.ppm", output_dir, page_index);
        auto write_start = std::chrono::high_resolution_clock::now();
        if (!benchmark_mode) {
            success = write_ppm(filename, buffer, stride, width_px, height_px, bitmap_format);
        } else {
            success = true;  // Skip write in benchmark mode
        }
        auto write_end = std::chrono::high_resolution_clock::now();
        write_time_ms = std::chrono::duration<double, std::milli>(write_end - write_start).count();
    } else if (use_jpeg) {
        // Write JPEG file using libjpeg-turbo
        snprintf(filename, sizeof(filename), "%s/page_%05d.jpg", output_dir, page_index);
        auto write_start = std::chrono::high_resolution_clock::now();
        if (!benchmark_mode) {
            success = write_jpeg(filename, buffer, stride, width_px, height_px, jpeg_quality);
        } else {
            success = true;  // Skip write in benchmark mode
        }
        auto write_end = std::chrono::high_resolution_clock::now();
        write_time_ms = std::chrono::duration<double, std::milli>(write_end - write_start).count();
    } else {
        // Convert to PNG using image_diff_png
        // N=41: Use EncodeBGRPNG for BGR format (3 bytes), EncodeBGRAPNG for BGRA format (4 bytes)
        auto input = pdfium::span(static_cast<uint8_t*>(buffer),
                                  static_cast<size_t>(stride) * height_px);
        auto encode_start = std::chrono::high_resolution_clock::now();
        std::vector<uint8_t> png_data;
        if (bitmap_format == FPDFBitmap_BGR) {
            // 3-byte BGR format (no alpha channel)
            png_data = image_diff_png::EncodeBGRPNG(input, width_px, height_px, stride);
        } else {
            // 4-byte BGRA format (with alpha channel)
            png_data = image_diff_png::EncodeBGRAPNG(input, width_px, height_px, stride, false);
        }
        auto encode_end = std::chrono::high_resolution_clock::now();
        encode_time_ms = std::chrono::duration<double, std::milli>(encode_end - encode_start).count();

        if (png_data.empty()) {
            fprintf(stderr, "Error: Failed to encode PNG for page %d\n", page_index);
            FPDFBitmap_Destroy(bitmap);
            FPDF_ClosePage(page);
            return 1;
        }

        // Write PNG file
        snprintf(filename, sizeof(filename), "%s/page_%05d.png", output_dir, page_index);
        auto write_start = std::chrono::high_resolution_clock::now();
        if (!benchmark_mode) {
            success = write_png(filename, png_data);
        } else {
            success = true;  // Skip write in benchmark mode
        }
        auto write_end = std::chrono::high_resolution_clock::now();
        write_time_ms = std::chrono::duration<double, std::milli>(write_end - write_start).count();
    }

    // Print timing breakdown for profiling
    double total_time_ms = render_time_ms + encode_time_ms + write_time_ms;
    if (total_time_ms > 0.0) {
        fprintf(stderr, "Page %d timing: render=%.2fms (%.1f%%), encode=%.2fms (%.1f%%), write=%.2fms (%.1f%%), total=%.2fms\n",
                page_index,
                render_time_ms, (render_time_ms / total_time_ms) * 100.0,
                encode_time_ms, (encode_time_ms / total_time_ms) * 100.0,
                write_time_ms, (write_time_ms / total_time_ms) * 100.0,
                total_time_ms);
    } else {
        fprintf(stderr, "Page %d timing: total=0.00ms (instant)\n", page_index);
    }

    FPDFBitmap_Destroy(bitmap);

    // Call form callbacks before page close (matching upstream pdfium_test.cc:1575-1578)
    if (form) {
        FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
        FORM_OnBeforeClosePage(page, form);
    }

    // Clear current page info
    if (form_info) {
        form_info->current_page = nullptr;
        form_info->current_page_index = -1;
    }

    FPDF_ClosePage(page);

    return success ? 0 : 1;
}

// ========================================
// Multi-threaded rendering support
// ========================================

struct RenderContext {
    const char* output_dir;
    double dpi;
    bool use_ppm;
    bool use_jpeg;  // N=16: JPEG output format
    int jpeg_quality;  // N=16: JPEG quality (0-100)
    bool use_raw;  // N=328: Raw BGRA output (no encoding)
    bool benchmark_mode;  // N=323: Skip file writes for benchmarking
    bool force_alpha;  // N=420: Force BGRA bitmap for transparency optimization test
    int pixel_format;  // N=50: Output pixel format (0=bgrx, 1=bgr, 2=gray)
    std::atomic<int> pages_completed;
    std::atomic<int> pages_failed;
    // v1.6.0: Progress and metrics tracking (N=616)
    ProgressReporter* progress;
    MetricsReporter* metrics;
    int total_pages;  // For progress calculation
    // v1.8.0: Async I/O (N=31)
    AsyncWriterPool* writer_pool;
};

void parallel_render_callback(int page_index, const void* buffer, int width, int height, int stride, void* user_data, FPDF_BOOL success) {
    RenderContext* ctx = (RenderContext*)user_data;

    if (!success) {
        fprintf(stderr, "Warning: Failed to render page %d\n", page_index);
        ctx->pages_failed++;
        return;
    }

    // N=323: Skip file writes in benchmark mode (keep encoding to measure performance)
    if (!ctx->benchmark_mode) {
        // Generate output path
        // N=223: Fixed page naming collision for pages >=10000
        // Old trick: std::to_string(10000 + page_index).substr(1) fails at page 10000+
        // New: Use snprintf with %05d (backward compatible with existing tests/baselines)
        char page_num[16];
        snprintf(page_num, sizeof(page_num), "%05d", page_index);
        std::string output_path = std::string(ctx->output_dir) + "/page_" + page_num + ".";

        // N=49: Zero-copy sync I/O (was: async with buffer copy)
        // Benefit: 2x less memory traffic per page (~8.7 MB saved at 300 DPI)
        // Trade-off: No overlap of disk writes with next page rendering, but
        // memory-bound systems benefit more from reduced bandwidth than I/O overlap

        if (ctx->use_raw) {
            // N=49: Zero-copy sync I/O (was: async with buffer copy)
            // Benefit: 2x less memory traffic per page (~8.7 MB saved at 300 DPI)
            output_path += "bgra";

            // Write directly - no buffer copy needed for sync I/O
            if (!write_bgra(output_path.c_str(), buffer, stride, width, height)) {
                fprintf(stderr, "Error: Failed to write BGRA: %s\n", output_path.c_str());
                ctx->pages_failed++;
            }
        } else if (ctx->use_ppm) {
            // N=49: Zero-copy sync I/O for PPM
            // Use write_ppm which handles BGRA→RGB conversion internally
            output_path += "ppm";

            // Write directly using existing write_ppm function
            if (!write_ppm(output_path.c_str(), const_cast<void*>(buffer), stride, width, height, FPDFBitmap_BGRx)) {
                fprintf(stderr, "Error: Failed to write PPM: %s\n", output_path.c_str());
                ctx->pages_failed++;
            }
        } else if (ctx->use_jpeg) {
            // N=49: Zero-copy sync I/O for JPEG (was: async with buffer copy)
            // Benefit: No ~8.7 MB buffer copy per page at 300 DPI
            output_path += "jpg";

            // N=50: Pass pixel_format to write_jpeg for format-aware encoding
            if (!write_jpeg(output_path.c_str(), const_cast<void*>(buffer), stride, width, height, ctx->jpeg_quality, ctx->pixel_format)) {
                fprintf(stderr, "Error: Failed to write JPEG: %s\n", output_path.c_str());
                ctx->pages_failed++;
            }
        } else {
            // N=49: Sync I/O for PNG (encoding is sync, now write is sync too)
            output_path += "png";
            auto input = pdfium::span(static_cast<const uint8_t*>(buffer),
                                      static_cast<size_t>(stride) * height);
            std::vector<uint8_t> png_data =
                image_diff_png::EncodeBGRAPNG(input, width, height, stride, false);

            if (png_data.empty()) {
                fprintf(stderr, "Error: Failed to encode PNG for page %d\n", page_index);
                ctx->pages_failed++;
                return;
            }

            // Write directly - sync I/O
            if (!write_png(output_path.c_str(), png_data)) {
                fprintf(stderr, "Error: Failed to write PNG: %s\n", output_path.c_str());
                ctx->pages_failed++;
            }
        }
    }

    ctx->pages_completed++;

    // v1.6.0: Update progress and metrics (N=616)
    if (ctx->metrics) {
        ctx->metrics->RecordPage();
    }
    if (ctx->progress) {
        // Calculate total pages processed so far (including smart mode)
        ctx->progress->Update(ctx->pages_completed + ctx->progress->GetSmartModePages());
    }
}

// ========================================
// Bulk Mode: Single/Multi-threaded rendering
// ========================================

int render_pages_bulk(const char* pdf_path, const char* output_dir, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int start_page, int end_page, int thread_count, int render_quality, bool benchmark_mode, bool user_set_threads, bool enable_adaptive, bool force_alpha, int pixel_format) {
    // Initialize with AGG renderer (matching upstream pdfium_test.cc main())
    FPDF_LIBRARY_CONFIG config;
    config.version = 4;
    config.m_pUserFontPaths = nullptr;
    config.m_pIsolate = nullptr;
    config.m_v8EmbedderSlot = 0;
    config.m_pPlatform = nullptr;
    config.m_RendererType = FPDF_RENDERERTYPE_AGG;
    FPDF_InitLibraryWithConfig(&config);

    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Error: Failed to load PDF: %s\n", pdf_path);
        FPDF_DestroyLibrary();
        return 2;
    }

    int page_count = FPDF_GetPageCount(doc);
    if (page_count < 0) {
        fprintf(stderr, "Error: Failed to get page count\n");
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 2;
    }

    // Handle 0-page PDFs gracefully (no pages to render)
    if (page_count == 0) {
        fprintf(stderr, "PDF has 0 pages, no rendering needed\n");
        fprintf(stderr, "Rendering complete: %s\n", output_dir);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 0;
    }

    // Set page range defaults if not specified
    if (start_page == -1) start_page = 0;
    if (end_page == -1) end_page = page_count - 1;

    // N=349/N=525: Adaptive threading (opt-in with --adaptive flag)
    // Threading bug fixed at N=341 (load_page_mutex_): 100% stable at K=4/8
    // K=8 regression fixed at N=524 (indirect result of N=341/N=522 stability work)
    //
    // Adaptive threading auto-selects optimal K based on page count:
    // - Small (<50 pages): K=1 (overhead dominates, Amdahl's Law)
    // - Medium/Large (50+ pages): K=8 (2.82x for 1931p, 6.5x for 201p)
    //
    // N=524 findings: K=8 now optimal for ALL PDF sizes including >1000 pages
    // - 1931-page PDF: K=8 at 2.82x vs K=4 at 2.74x (K=8 is 3% faster)
    // - 821-page PDF: K=8 is 1.34x faster than K=4
    // - 569-page PDF: K=8 is 1.23x faster than K=4
    //
    // ONLY activate if ALL conditions met:
    // 1. User enabled --adaptive flag
    // 2. User did NOT explicitly set --threads
    // 3. Workload large enough (>= 50 pages)

    int pages_to_render = end_page - start_page + 1;
    if (enable_adaptive && !user_set_threads && pages_to_render >= 50) {
        // N=117: Improved adaptive threading - scale to min(8, hw_threads, pages)
        // No point running 8 threads for 5 pages (more threads than work units)
        unsigned int hw_threads = std::thread::hardware_concurrency();
        if (hw_threads == 0) hw_threads = 4;  // Conservative fallback if unknown
        int optimal_k = std::min({8, static_cast<int>(hw_threads), pages_to_render});
        thread_count = optimal_k;
        fprintf(stderr, "Auto-selected %d threads for %d pages (hw_concurrency=%u)\n",
                thread_count, pages_to_render, hw_threads);
    }

    // Initialize form fill environment with callbacks (matching upstream pdfium_test.cc:1672-1708)
    FormFillInfo form_callbacks = {};
    form_callbacks.version = 1;
    form_callbacks.FFI_GetPage = GetPageForIndex;
    form_callbacks.FFI_ExecuteNamedAction = ExampleNamedAction;
    form_callbacks.current_doc = nullptr;
    form_callbacks.current_page = nullptr;
    form_callbacks.current_page_index = -1;

    FPDF_FORMHANDLE form = FPDFDOC_InitFormFillEnvironment(doc, &form_callbacks);
    form_callbacks.form_handle = form;
    form_callbacks.current_doc = doc;

    if (form) {
        // Set form field appearance (matching upstream pdfium_test.cc:1705-1706)
        FPDF_SetFormFieldHighlightColor(form, FPDF_FORMFIELD_UNKNOWN, 0xFFE4DD);
        FPDF_SetFormFieldHighlightAlpha(form, 100);
        FORM_DoDocumentJSAction(form);
        FORM_DoDocumentOpenAction(form);
    }

    int num_pages_to_render = end_page - start_page + 1;
    fprintf(stderr, "Rendering %d pages at %.0f DPI (%s)\n", num_pages_to_render, dpi, use_ppm ? "PPM" : (use_jpeg ? "JPEG" : "PNG"));

    // v1.6.0: Initialize progress and metrics reporting (N=616)
    ProgressReporter progress(num_pages_to_render, !benchmark_mode);  // Disable progress in benchmark mode
    MetricsReporter metrics;
    metrics.RecordStart();

    // v1.8.0 N=31: Create async writer pool (4 threads for I/O)
    // Purpose: Overlap disk writes with rendering (5-15% speedup)
    AsyncWriterPool writer_pool(4);

    // Choose rendering path based on thread count
    if (thread_count > 1) {
        // N=522: Smart mode + threading integration
        // Pre-scan for scanned pages (JPEG fast path eligible) and handle them before parallel rendering
        // This allows 545x speedup for scanned pages even with K>1

        // Skip smart mode if PPM/raw output requested (correctness validation requires exact format)
        // Skip smart mode if benchmark mode (need to measure rendering performance)
        bool enable_smart_mode = !use_ppm && !use_raw && !benchmark_mode;

        std::vector<bool> is_scanned_map(end_page - start_page + 1, false);
        int scanned_count = 0;

        if (enable_smart_mode) {
            // N=136: Single-pass smart mode - detect AND extract in one pass
            // Previously: Pass 1 loaded pages to check is_scanned_page(), Pass 2 loaded AGAIN to extract
            // Now: Load each page once, check if scanned, extract immediately if yes
            // Result: 50% reduction in page load overhead for scanned PDFs
            fprintf(stderr, "JPEG fast path: scanning and extracting in single pass...\n");
            auto smart_start = std::chrono::high_resolution_clock::now();
            int pages_processed = 0;
            for (int i = start_page; i <= end_page; ++i) {
                FPDF_PAGE page = FPDF_LoadPage(doc, i);
                if (page) {
                    if (is_scanned_page(page)) {
                        // Page is scanned - extract immediately while page is already loaded
                        char output_path[512];
                        int path_len = snprintf(output_path, sizeof(output_path), "%s/page_%05d.jpg", output_dir, i);
                        if (path_len >= (int)sizeof(output_path)) {
                            fprintf(stderr, "Error: Output path too long (max 511 chars)\n");
                            FPDF_ClosePage(page);
                            continue;
                        }

                        if (!render_scanned_page_fast(page, output_path)) {
                            fprintf(stderr, "Warning: Fast path failed for page %d, will use normal rendering\n", i);
                            // Leave is_scanned_map[i - start_page] = false (default)
                        } else {
                            is_scanned_map[i - start_page] = true;
                            scanned_count++;
                            metrics.RecordSmartMode();
                            progress.RecordSmartModePage();
                            pages_processed++;
                            metrics.RecordPage();
                            progress.Update(pages_processed);
                        }
                    }
                    FPDF_ClosePage(page);
                }
            }
            auto smart_end = std::chrono::high_resolution_clock::now();
            double smart_time_ms = std::chrono::duration<double, std::milli>(smart_end - smart_start).count();
            if (scanned_count > 0) {
                // N=136: Smart mode timing metrics for optimization tuning
                fprintf(stderr, "JPEG fast path: %d pages in %.1fms (%.0f pages/sec)\n",
                        scanned_count, smart_time_ms, scanned_count * 1000.0 / smart_time_ms);
            }
        }

        int remaining_pages = num_pages_to_render - scanned_count;
        if (remaining_pages == 0) {
            // All pages extracted via fast path, no rendering needed
            fprintf(stderr, "All pages extracted via JPEG fast path, rendering complete\n");
            progress.Finish();
            metrics.PrintSummary(thread_count, enable_smart_mode);
            if (form) FPDFDOC_ExitFormFillEnvironment(form);
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return 0;
        }

        // Multi-threaded rendering using FPDF_RenderPagesParallelV2 for non-scanned pages
        fprintf(stderr, "Using parallel rendering with %d threads for %d remaining pages\n", thread_count, remaining_pages);

        // N=194: PRE-LOAD all non-scanned pages to populate resource caches (avoids deadlock)
        // Strategy from ~/pdfium-old-threaded/fpdfsdk/fpdf_parallel.cpp:701-709
        // Loading pages sequentially populates CPDF_DocPageData caches (images, patterns, colorspaces)
        // before parallel rendering starts, preventing AB-BA deadlock from concurrent cache population.
        // See: reports/feature-image-threading/PRELOADING_SOLUTION_N194_2025-11-15.md
        // N=313: Pre-loading is REQUIRED - disabling causes hangs/deadlocks
        // N=257: Call form callbacks during pre-loading to match single-threaded behavior
        fprintf(stderr, "Pre-loading %d non-scanned pages to populate resource caches...\n", remaining_pages);
        for (int i = start_page; i <= end_page; ++i) {
            if (enable_smart_mode && is_scanned_map[i - start_page]) {
                continue;  // Skip scanned pages (already extracted)
            }
            FPDF_PAGE page = FPDF_LoadPage(doc, i);
            if (page) {
                // N=257: Call form callbacks to match single-threaded path
                if (form) {
                    FORM_OnAfterLoadPage(page, form);
                    FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_OPEN);
                    FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
                }
                FPDF_ClosePage(page);  // Close page but caches remain populated
            }
        }
        fprintf(stderr, "Pre-loading complete, starting parallel rendering\n");

        // N=522: Smart mode + threading - render non-scanned pages in parallel
        // Strategy: Find contiguous ranges of non-scanned pages and render them in parallel batches
        // This maintains threading benefit while allowing JPEG fast path for scanned pages

        int total_pages_completed = scanned_count;  // Already extracted scanned pages
        int total_pages_failed = 0;

        // Find contiguous ranges of non-scanned pages
        int range_start = -1;
        for (int i = start_page; i <= end_page + 1; ++i) {
            bool is_scanned = (i <= end_page) && enable_smart_mode && is_scanned_map[i - start_page];
            bool at_end = (i > end_page);

            if (!is_scanned && !at_end && range_start == -1) {
                // Start of a new non-scanned range
                range_start = i;
            } else if ((is_scanned || at_end) && range_start != -1) {
                // End of non-scanned range - render this range in parallel
                int range_end = i - 1;
                int range_length = range_end - range_start + 1;

                fprintf(stderr, "Rendering non-scanned pages %d-%d (%d pages) in parallel...\n",
                        range_start, range_end, range_length);

                // Calculate bitmap dimensions for this range (assume uniform page size)
                FPDF_PAGE first_page = FPDF_LoadPage(doc, range_start);
                if (!first_page) {
                    fprintf(stderr, "Error: Failed to load page %d for dimension calculation\n", range_start);
                    total_pages_failed += range_length;
                    range_start = -1;
                    continue;
                }

                double width_pts = FPDF_GetPageWidthF(first_page);
                double height_pts = FPDF_GetPageHeightF(first_page);
                double scale_raw = dpi / 72.0;
                double scale = floor(scale_raw * 1000000.0) / 1000000.0;
                double width_raw = width_pts * scale;
                double height_raw = height_pts * scale;

                // Check for overflow - pixel dimensions must fit in int
                if (width_raw > INT_MAX || width_raw < 1 || height_raw > INT_MAX || height_raw < 1) {
                    fprintf(stderr, "Error: Page %d dimensions too large for rendering (%.0fx%.0f pixels)\n",
                            range_start, width_raw, height_raw);
                    FPDF_ClosePage(first_page);
                    total_pages_failed += range_length;
                    range_start = -1;
                    continue;
                }
                int width_px = (int)width_raw;
                int height_px = (int)height_raw;
                FPDF_ClosePage(first_page);

                // Set up parallel rendering options
                FPDF_PARALLEL_OPTIONS opts = {};
                opts.worker_count = thread_count;
                opts.max_queue_size = 0;  // Unlimited queue
                opts.form_handle = form;  // N=200: Pass form handle for correct rendering
                opts.output_format = pixel_format;  // N=50: Pixel format (bgrx/bgr/gray)

                // Set up rendering context
                RenderContext render_ctx = {};
                render_ctx.output_dir = output_dir;
                render_ctx.dpi = dpi;
                render_ctx.use_ppm = use_ppm;
                render_ctx.use_jpeg = use_jpeg;
                render_ctx.jpeg_quality = jpeg_quality;
                render_ctx.use_raw = use_raw;
                render_ctx.benchmark_mode = benchmark_mode;
                render_ctx.force_alpha = force_alpha;
                render_ctx.pixel_format = pixel_format;  // N=50: Pixel format
                render_ctx.pages_completed = 0;
                render_ctx.pages_failed = 0;
                // v1.6.0: Progress and metrics tracking (N=616)
                render_ctx.progress = &progress;
                render_ctx.metrics = &metrics;
                render_ctx.total_pages = num_pages_to_render;
                // v1.8.0: Async I/O (N=31)
                render_ctx.writer_pool = &writer_pool;

                // Compute render flags
                int flags = FPDF_ANNOT;
                if (render_quality == 1) {
                    flags |= FPDF_RENDER_NO_SMOOTHTEXT | FPDF_RENDER_NO_SMOOTHIMAGE | FPDF_RENDER_NO_SMOOTHPATH;
                } else if (render_quality == 3) {
                    flags |= FPDF_RENDER_NO_SMOOTHTEXT | FPDF_RENDER_NO_SMOOTHIMAGE | FPDF_RENDER_NO_SMOOTHPATH;
                    flags |= FPDF_RENDER_LIMITEDIMAGECACHE;
                }                // Render this range in parallel
                FPDF_BOOL result = FPDF_RenderPagesParallelV2(
                    doc,
                    range_start,
                    range_length,
                    width_px,
                    height_px,
                    0,  // rotation
                    flags,
                    &opts,
                    parallel_render_callback,
                    &render_ctx
                );

                if (!result) {
                    fprintf(stderr, "Error: Parallel rendering failed for range %d-%d\n", range_start, range_end);
                    total_pages_failed += range_length;
                } else {
                    total_pages_completed += render_ctx.pages_completed;
                    total_pages_failed += render_ctx.pages_failed;
                }

                range_start = -1;  // Reset for next range
            }
        }

        fprintf(stderr, "Rendering complete: %d pages succeeded, %d failed\n",
                total_pages_completed, total_pages_failed);
        // FIX #12 (N=30): Propagate failures to exit code
        if (total_pages_failed > 0) {
            // Clean up before returning non-zero
            writer_pool.WaitAll();
            if (form) FPDFDOC_ExitFormFillEnvironment(form);
            FPDF_CloseDocument(doc);
            if (thread_count > 1) FPDF_DestroyThreadPool();
            FPDF_DestroyLibrary();
            return 1;
        }
    } else {
        // Single-threaded rendering (existing path)
        int pages_processed = 0;
        int pages_failed = 0;
        for (int page_idx = start_page; page_idx <= end_page; page_idx++) {
            if (render_page_to_png(doc, form, &form_callbacks, page_idx, output_dir, dpi, use_ppm, use_jpeg, jpeg_quality, use_raw, render_quality, benchmark_mode, force_alpha) != 0) {
                fprintf(stderr, "Warning: Failed to render page %d\n", page_idx);
                pages_failed++;
            }
            // v1.6.0: Track progress and metrics (N=616)
            pages_processed++;
            metrics.RecordPage();
            progress.Update(pages_processed);
        }
        // FIX #12 (N=30): Propagate failures to exit code
        if (pages_failed > 0) {
            fprintf(stderr, "Rendering had %d failures\n", pages_failed);
            writer_pool.WaitAll();
            if (form) FPDFDOC_ExitFormFillEnvironment(form);
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return 1;
        }
    }

    // v1.8.0 N=31: Wait for all async writes to complete before cleanup
    writer_pool.WaitAll();

    // Clean up form handle
    if (form) {
        FPDFDOC_ExitFormFillEnvironment(form);
    }

    FPDF_CloseDocument(doc);

    // Clean up thread pool if we used parallel rendering
    if (thread_count > 1) {
        FPDF_DestroyThreadPool();
    }

    FPDF_DestroyLibrary();

    // v1.6.0: Finish progress reporting and print performance summary (N=616)
    progress.Finish();
    bool enable_smart_mode = !use_ppm && !use_raw && !benchmark_mode;
    metrics.PrintSummary(thread_count, enable_smart_mode);

    fprintf(stderr, "Rendering complete: %s\n", output_dir);
    return 0;
}

// ========================================
// Fast Mode: Multi-process rendering
// ========================================

int render_pages_fast(const char* pdf_path, const char* output_dir, int worker_count, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int start_page, int end_page, int render_quality, bool benchmark_mode, bool force_alpha, int thread_count) {
    // Get page count
    int total_page_count = get_page_count(pdf_path);
    if (total_page_count < 0) {
        fprintf(stderr, "Error: Failed to get page count\n");
        return 2;
    }

    // Handle 0-page PDFs gracefully
    if (total_page_count == 0) {
        fprintf(stderr, "Rendering complete: %s\n", output_dir);
        return 0;
    }

    // Set page range defaults if not specified
    if (start_page == -1) start_page = 0;
    if (end_page == -1) end_page = total_page_count - 1;

    // Validate page range
    if (start_page < 0 || end_page >= total_page_count || start_page > end_page) {
        fprintf(stderr, "Error: Invalid page range %d-%d (document has %d pages)\n",
                start_page, end_page, total_page_count);
        return 2;
    }

    int page_count = end_page - start_page + 1;

    const char* format_name = use_raw ? "BGRA" : (use_ppm ? "PPM" : (use_jpeg ? "JPEG" : "PNG"));
    fprintf(stderr, "Rendering %d pages with %d workers at %.0f DPI (%s)\n",
            page_count, worker_count, dpi, format_name);

    // Calculate pages per worker
    int pages_per_worker = (page_count + worker_count - 1) / worker_count;

    // Spawn worker processes
    pid_t* pids = (pid_t*)malloc(sizeof(pid_t) * worker_count);
    if (!pids) {
        fprintf(stderr, "Error: Memory allocation failed\n");
        return -1;
    }
    int actual_workers = 0;

    for (int worker_id = 0; worker_id < worker_count; worker_id++) {
        int worker_start = start_page + (worker_id * pages_per_worker);
        int worker_end = worker_start + pages_per_worker;  // EXCLUSIVE end
        if (worker_end > end_page + 1) worker_end = end_page + 1;  // Adjust to exclusive bound

        if (worker_start > end_page) {
            break;
        }

        // Fork worker
        pid_t pid = fork();
        if (pid == 0) {
            // Child process - exec worker
            char start_str[16], end_str[16], id_str[16], dpi_str[16], quality_str[16], alpha_str[16], thread_str[16];
            snprintf(start_str, sizeof(start_str), "%d", worker_start);
            snprintf(end_str, sizeof(end_str), "%d", worker_end);
            snprintf(id_str, sizeof(id_str), "%d", worker_id);
            snprintf(dpi_str, sizeof(dpi_str), "%.1f", dpi);
            snprintf(quality_str, sizeof(quality_str), "%d", render_quality);
            snprintf(alpha_str, sizeof(alpha_str), "%d", force_alpha ? 1 : 0);
            snprintf(thread_str, sizeof(thread_str), "%d", thread_count);

            // Get executable path
            char exe_path[1024];
#ifdef __APPLE__
            uint32_t size = sizeof(exe_path);
            if (_NSGetExecutablePath(exe_path, &size) != 0) {
                fprintf(stderr, "Error: Failed to get executable path\n");
                exit(1);
            }
#else
            ssize_t len = readlink("/proc/self/exe", exe_path, sizeof(exe_path) - 1);
            if (len == -1) {
                fprintf(stderr, "Error: Failed to get executable path\n");
                exit(1);
            }
            exe_path[len] = '\0';
#endif

            const char* format_str = use_raw ? "bgra" : (use_ppm ? "ppm" : (use_jpeg ? "jpg" : "png"));
            char jpeg_q_str[16];
            snprintf(jpeg_q_str, sizeof(jpeg_q_str), "%d", jpeg_quality);
            const char* benchmark_str = benchmark_mode ? "1" : "0";
            execl(exe_path, exe_path, "--worker", pdf_path, output_dir,
                  start_str, end_str, id_str, dpi_str, format_str, quality_str, alpha_str, thread_str, jpeg_q_str, benchmark_str, NULL);

            // If exec fails
            fprintf(stderr, "Error: Failed to exec worker\n");
            exit(1);
        } else if (pid > 0) {
            // Parent process
            pids[actual_workers] = pid;
            actual_workers++;
        } else {
            fprintf(stderr, "Error: Failed to fork worker %d\n", worker_id);
            free(pids);
            return 3;
        }
    }

    // Wait for all workers
    int all_success = 1;
    for (int i = 0; i < actual_workers; i++) {
        int status;
        waitpid(pids[i], &status, 0);
        if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
            fprintf(stderr, "Error: Worker %d failed\n", i);
            all_success = 0;
        }
    }

    free(pids);

    if (!all_success) {
        return 3;
    }

    fprintf(stderr, "Rendering complete: %s\n", output_dir);
    return 0;
}

// ========================================
// Debug Mode: Rendering with tracing
// ========================================

int render_pages_debug(const char* pdf_path, const char* output_dir, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int render_quality, bool force_alpha) {
    fprintf(stderr, "[TRACE] FPDF_InitLibraryWithConfig() - AGG renderer\n");
    FPDF_LIBRARY_CONFIG config;
    config.version = 4;
    config.m_pUserFontPaths = nullptr;
    config.m_pIsolate = nullptr;
    config.m_v8EmbedderSlot = 0;
    config.m_pPlatform = nullptr;
    config.m_RendererType = FPDF_RENDERERTYPE_AGG;
    FPDF_InitLibraryWithConfig(&config);

    fprintf(stderr, "[TRACE] FPDF_LoadDocument(%s)\n", pdf_path);
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "[ERROR] Failed to load PDF\n");
        FPDF_DestroyLibrary();
        return 2;
    }
    fprintf(stderr, "[TRACE] Document loaded: %p\n", doc);

    int page_count = FPDF_GetPageCount(doc);
    fprintf(stderr, "[TRACE] FPDF_GetPageCount() -> %d\n", page_count);

    if (page_count < 0) {
        fprintf(stderr, "[ERROR] Failed to get page count\n");
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 2;
    }

    // Handle 0-page PDFs gracefully
    if (page_count == 0) {
        fprintf(stderr, "[TRACE] PDF has 0 pages, no rendering needed\n");
        fprintf(stderr, "[TRACE] Rendering complete: %s\n", output_dir);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 0;
    }

    fprintf(stderr, "[TRACE] Rendering at %.0f DPI (%s)\n", dpi, use_ppm ? "PPM" : "PNG");

    // Initialize form fill environment with callbacks
    fprintf(stderr, "[TRACE] Initializing form fill environment with callbacks\n");
    FormFillInfo form_callbacks = {};
    form_callbacks.version = 1;
    form_callbacks.FFI_GetPage = GetPageForIndex;
    form_callbacks.FFI_ExecuteNamedAction = ExampleNamedAction;
    form_callbacks.current_doc = nullptr;
    form_callbacks.current_page = nullptr;
    form_callbacks.current_page_index = -1;

    FPDF_FORMHANDLE form = FPDFDOC_InitFormFillEnvironment(doc, &form_callbacks);
    form_callbacks.form_handle = form;
    form_callbacks.current_doc = doc;

    if (form) {
        FORM_DoDocumentJSAction(form);
        FORM_DoDocumentOpenAction(form);
        fprintf(stderr, "[TRACE] Form handle initialized: %p\n", form);
    }

    for (int page_idx = 0; page_idx < page_count; page_idx++) {
        fprintf(stderr, "[TRACE] Processing page %d/%d\n", page_idx + 1, page_count);

        if (render_page_to_png(doc, form, &form_callbacks, page_idx, output_dir, dpi, use_ppm, use_jpeg, jpeg_quality, use_raw, render_quality, false, force_alpha) != 0) {
            fprintf(stderr, "[WARN] Failed to render page %d\n", page_idx);
        }
    }

    // Clean up form handle
    if (form) {
        fprintf(stderr, "[TRACE] Cleaning up form handle\n");
        FPDFDOC_ExitFormFillEnvironment(form);
    }

    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    fprintf(stderr, "[SUMMARY] Rendered %d pages\n", page_count);
    fprintf(stderr, "[TRACE] Rendering complete: %s\n", output_dir);
    return 0;
}

// ========================================
// Worker Process (internal) - Rendering
// ========================================

int render_pages_worker(const char* pdf_path, const char* output_dir,
                        int start_page, int end_page, int worker_id, double dpi, bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, int render_quality, bool force_alpha, int thread_count, bool benchmark_mode, int pixel_format) {
    // N=152 Fix #20: Cap per-worker threads to prevent oversubscription
    // Each worker should use at most hardware_concurrency threads (conservative)
    // or 16 threads (hard cap to avoid excessive context switching)
    int hw_threads = static_cast<int>(std::thread::hardware_concurrency());
    if (hw_threads < 1) hw_threads = 4;  // Fallback if hw_concurrency unknown
    int max_per_worker = std::min(hw_threads, 16);
    if (thread_count > max_per_worker) {
        thread_count = max_per_worker;
    }

    // Initialize with AGG renderer (matching upstream pdfium_test.cc main())
    FPDF_LIBRARY_CONFIG config;
    config.version = 4;
    config.m_pUserFontPaths = nullptr;
    config.m_pIsolate = nullptr;
    config.m_v8EmbedderSlot = 0;
    config.m_pPlatform = nullptr;
    config.m_RendererType = FPDF_RENDERERTYPE_AGG;
    FPDF_InitLibraryWithConfig(&config);

    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Worker %d: Failed to load PDF\n", worker_id);
        FPDF_DestroyLibrary();
        return 2;
    }

    // Initialize form fill environment with callbacks
    FormFillInfo form_callbacks = {};
    form_callbacks.version = 1;
    form_callbacks.FFI_GetPage = GetPageForIndex;
    form_callbacks.FFI_ExecuteNamedAction = ExampleNamedAction;
    form_callbacks.current_doc = nullptr;
    form_callbacks.current_page = nullptr;
    form_callbacks.current_page_index = -1;

    FPDF_FORMHANDLE form = FPDFDOC_InitFormFillEnvironment(doc, &form_callbacks);
    form_callbacks.form_handle = form;
    form_callbacks.current_doc = doc;

    if (form) {
        // Set form field appearance (matching upstream pdfium_test.cc:1705-1706)
        FPDF_SetFormFieldHighlightColor(form, FPDF_FORMFIELD_UNKNOWN, 0xFFE4DD);
        FPDF_SetFormFieldHighlightAlpha(form, 100);
        FORM_DoDocumentJSAction(form);
        FORM_DoDocumentOpenAction(form);
    }

    // v1.8.0 N=31: Create async writer pool for worker process
    AsyncWriterPool writer_pool(4);

    // Render assigned pages
    if (thread_count == 1) {
        // Single-threaded rendering
        for (int page_idx = start_page; page_idx < end_page; page_idx++) {
            if (render_page_to_png(doc, form, &form_callbacks, page_idx, output_dir, dpi, use_ppm, use_jpeg, jpeg_quality, use_raw, render_quality, benchmark_mode, force_alpha) != 0) {
                fprintf(stderr, "Worker %d: Failed to render page %d\n", worker_id, page_idx);
            }
        }
    } else {
        // Multi-threaded rendering (N=426: N×K combined, N=522: Smart mode support)
        int num_pages_to_render = end_page - start_page;

        // N=522: Smart mode + threading integration (same approach as render_pages_bulk)
        // Pre-scan for scanned pages and handle via JPEG fast path before parallel rendering
        // N=34: benchmark_mode now propagated from parent to workers
        bool enable_smart_mode = !use_ppm && !use_raw && !benchmark_mode;

        std::vector<bool> is_scanned_map(num_pages_to_render, false);
        int scanned_count = 0;

        if (enable_smart_mode) {
            // Pre-scan phase: Detect scanned pages
            for (int page_idx = start_page; page_idx < end_page; page_idx++) {
                FPDF_PAGE page = FPDF_LoadPage(doc, page_idx);
                if (page) {
                    if (is_scanned_page(page)) {
                        is_scanned_map[page_idx - start_page] = true;
                        scanned_count++;
                    }
                    FPDF_ClosePage(page);
                }
            }

            if (scanned_count > 0) {
                // JPEG extraction phase
                for (int page_idx = start_page; page_idx < end_page; page_idx++) {
                    if (is_scanned_map[page_idx - start_page]) {
                        FPDF_PAGE page = FPDF_LoadPage(doc, page_idx);
                        if (page) {
                            char output_path[512];
                            int path_len = snprintf(output_path, sizeof(output_path), "%s/page_%05d.jpg", output_dir, page_idx);
                            if (path_len >= (int)sizeof(output_path)) {
                                fprintf(stderr, "Error: Output path too long (max 511 chars)\n");
                                FPDF_ClosePage(page);
                                continue;
                            }

                            if (!render_scanned_page_fast(page, output_path)) {
                                // Fall back to normal rendering
                                is_scanned_map[page_idx - start_page] = false;
                                scanned_count--;
                            }
                            FPDF_ClosePage(page);
                        }
                    }
                }
            }
        }

        int remaining_pages = num_pages_to_render - scanned_count;
        if (remaining_pages == 0) {
            // All pages extracted via fast path
            if (form) FPDFDOC_ExitFormFillEnvironment(form);
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return 0;
        }

        // Pre-load non-scanned pages to populate caches (critical for thread safety, N=196)
        // N=257: Call form callbacks during pre-loading to match single-threaded behavior
        // Root cause of K=8 vs K=1 rendering mismatch: Form callbacks (FORM_OnAfterLoadPage,
        // FORM_DoPageAAction) set up page state that affects rendering. Pre-loading without
        // callbacks causes K=8 to produce different output than K=1.
        for (int page_idx = start_page; page_idx < end_page; page_idx++) {
            if (enable_smart_mode && is_scanned_map[page_idx - start_page]) {
                continue;  // Skip scanned pages (already extracted)
            }
            FPDF_PAGE page = FPDF_LoadPage(doc, page_idx);
            if (page) {
                // Call form callbacks to match single-threaded path (N=257 fix)
                if (form) {
                    FORM_OnAfterLoadPage(page, form);
                    FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_OPEN);
                    FORM_DoPageAAction(page, form, FPDFPAGE_AACTION_CLOSE);
                }
                FPDF_ClosePage(page);
            }
        }

        // N=522: Smart mode + threading - render non-scanned pages in parallel
        // Same contiguous range batching approach as render_pages_bulk
        int total_pages_completed = scanned_count;
        int total_pages_failed = 0;

        // Find contiguous ranges of non-scanned pages and render each range
        int range_start = -1;
        for (int i = start_page; i < end_page + 1; ++i) {
            bool is_scanned = (i < end_page) && enable_smart_mode && is_scanned_map[i - start_page];
            bool at_end = (i >= end_page);

            if (!is_scanned && !at_end && range_start == -1) {
                range_start = i;
            } else if ((is_scanned || at_end) && range_start != -1) {
                int range_end = i - 1;
                int range_length = range_end - range_start + 1;

                // N=152 Fix #5/#14: Use per-page dimension calculation (same as bulk path)
                // Set opts.dpi and pass width=0,height=0 to enable per-page dimension
                // calculation in ProcessTaskV2. This fixes rendering of mixed-size PDFs.

                // Set up parallel rendering options
                FPDF_PARALLEL_OPTIONS opts = {};
                opts.worker_count = thread_count;
                opts.max_queue_size = 0;
                opts.form_handle = form;  // N=200: Pass form handle for correct rendering
                opts.dpi = dpi;  // N=152 Fix #5/#14: Enable per-page dimension calculation
                opts.output_format = pixel_format;  // N=50: Pixel format (bgrx/bgr/gray)

                // Set up rendering context
                RenderContext render_ctx = {};
                render_ctx.output_dir = output_dir;
                render_ctx.dpi = dpi;
                render_ctx.use_ppm = use_ppm;
                render_ctx.use_jpeg = use_jpeg;
                render_ctx.jpeg_quality = jpeg_quality;
                render_ctx.use_raw = use_raw;
                render_ctx.benchmark_mode = benchmark_mode;  // N=34: Fixed - was hardcoded false
                render_ctx.force_alpha = force_alpha;
                render_ctx.pixel_format = pixel_format;  // N=50: Pixel format
                render_ctx.pages_completed = 0;
                render_ctx.pages_failed = 0;
                // N=221: Explicit initialization of optional fields (code review N33 #2)
                render_ctx.progress = nullptr;
                render_ctx.metrics = nullptr;
                render_ctx.total_pages = range_length;
                // v1.8.0: Async I/O (N=31)
                render_ctx.writer_pool = &writer_pool;

                // Compute render flags
                int flags = FPDF_ANNOT;
                if (render_quality == 1) {
                    flags |= FPDF_RENDER_NO_SMOOTHTEXT | FPDF_RENDER_NO_SMOOTHIMAGE | FPDF_RENDER_NO_SMOOTHPATH;
                } else if (render_quality == 3) {
                    flags |= FPDF_RENDER_NO_SMOOTHTEXT | FPDF_RENDER_NO_SMOOTHIMAGE | FPDF_RENDER_NO_SMOOTHPATH;
                    flags |= FPDF_RENDER_LIMITEDIMAGECACHE;
                }
                // N=152 Fix #5/#14: Render with per-page dimensions (pass width=0, height=0)
                FPDF_BOOL result = FPDF_RenderPagesParallelV2(
                    doc,
                    range_start,
                    range_length,
                    0,  // width=0: use per-page dimension via opts.dpi
                    0,  // height=0: use per-page dimension via opts.dpi
                    0,  // rotation
                    flags,
                    &opts,
                    parallel_render_callback,
                    &render_ctx
                );

                if (!result) {
                    fprintf(stderr, "Worker %d: Parallel rendering failed for range %d-%d\n", worker_id, range_start, range_end);
                    total_pages_failed += range_length;
                } else {
                    total_pages_completed += render_ctx.pages_completed;
                    total_pages_failed += render_ctx.pages_failed;
                }

                range_start = -1;
            }
        }

        // Clean up thread pool (used for parallel rendering)
        FPDF_DestroyThreadPool();

        // FIX #12 (N=30): Propagate failures to exit code
        if (total_pages_failed > 0) {
            fprintf(stderr, "Worker %d: Rendering had %d failures out of %d completed\n",
                    worker_id, total_pages_failed, total_pages_completed);
            writer_pool.WaitAll();
            if (form) FPDFDOC_ExitFormFillEnvironment(form);
            FPDF_CloseDocument(doc);
            FPDF_DestroyLibrary();
            return 1;
        }
    }

    // v1.8.0 N=31: Wait for all async writes to complete before cleanup
    writer_pool.WaitAll();

    // Clean up form handle
    if (form) {
        FPDFDOC_ExitFormFillEnvironment(form);
    }

    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    return 0;
}

// ========================================
// JSONL Extraction - Helper Functions
// ========================================

// Escape string for JSON output
static void write_json_escaped_string(FILE* out, const char* str) {
    if (!str) {
        fprintf(out, "null");
        return;
    }

    fprintf(out, "\"");
    for (const char* p = str; *p; p++) {
        switch (*p) {
            case '"':  fprintf(out, "\\\""); break;
            case '\\': fprintf(out, "\\\\"); break;
            case '\n': fprintf(out, "\\n"); break;
            case '\r': fprintf(out, "\\r"); break;
            case '\t': fprintf(out, "\\t"); break;
            case '\b': fprintf(out, "\\b"); break;
            case '\f': fprintf(out, "\\f"); break;
            default:
                if ((unsigned char)*p < 0x20) {
                    fprintf(out, "\\u%04x", (unsigned char)*p);
                } else {
                    fputc(*p, out);
                }
                break;
        }
    }
    fprintf(out, "\"");
}

// Escape Unicode character for JSON output
static void write_json_escaped_char(FILE* out, unsigned int codepoint) {
    char buf[5] = {0};

    if (codepoint < 0x80) {
        // ASCII
        buf[0] = (char)codepoint;
        buf[1] = 0;
    } else if (codepoint < 0x800) {
        // 2-byte UTF-8
        buf[0] = (char)(0xC0 | (codepoint >> 6));
        buf[1] = (char)(0x80 | (codepoint & 0x3F));
        buf[2] = 0;
    } else if (codepoint < 0x10000) {
        // 3-byte UTF-8
        buf[0] = (char)(0xE0 | (codepoint >> 12));
        buf[1] = (char)(0x80 | ((codepoint >> 6) & 0x3F));
        buf[2] = (char)(0x80 | (codepoint & 0x3F));
        buf[3] = 0;
    } else if (codepoint < 0x110000) {
        // 4-byte UTF-8
        buf[0] = (char)(0xF0 | (codepoint >> 18));
        buf[1] = (char)(0x80 | ((codepoint >> 12) & 0x3F));
        buf[2] = (char)(0x80 | ((codepoint >> 6) & 0x3F));
        buf[3] = (char)(0x80 | (codepoint & 0x3F));
        buf[4] = 0;
    } else {
        // Invalid codepoint - use replacement character
        buf[0] = (char)0xEF;
        buf[1] = (char)0xBF;
        buf[2] = (char)0xBD;
        buf[3] = 0;
    }

    write_json_escaped_string(out, buf);
}

// ========================================
// JSONL Extraction - Bulk Mode
// ========================================

int extract_jsonl_bulk(const char* pdf_path, const char* output_path, int page_num) {
    FPDF_InitLibrary();

    // Load document
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "Error: Failed to load PDF: %s\n", pdf_path);
        FPDF_DestroyLibrary();
        return 2;
    }

    // Get page count
    int page_count = FPDF_GetPageCount(doc);
    if (page_num < 0 || page_num >= page_count) {
        fprintf(stderr, "Error: Invalid page number %d (document has %d pages)\n", page_num, page_count);
        FPDF_CloseDocument(doc);
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

    // Load page
    FPDF_PAGE page = FPDF_LoadPage(doc, page_num);
    if (!page) {
        fprintf(stderr, "Error: Failed to load page %d\n", page_num);
        fclose(out);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 2;
    }

    // Load text page
    FPDF_TEXTPAGE text_page = FPDFText_LoadPage(page);
    if (!text_page) {
        fprintf(stderr, "Error: Failed to load text for page %d\n", page_num);
        FPDF_ClosePage(page);
        fclose(out);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 2;
    }

    int char_count = FPDFText_CountChars(text_page);
    fprintf(stderr, "Extracting %d characters from page %d\n", char_count, page_num);

    // Extract each character with metadata
    int i = 0;
    while (i < char_count) {
        // 1. Get Unicode character (handle surrogate pairs)
        unsigned int unicode = FPDFText_GetUnicode(text_page, i);
        unsigned int codepoint;
        int chars_consumed;

        if (unicode >= 0xD800 && unicode <= 0xDBFF) {
            // High surrogate - need to read low surrogate
            if (i + 1 < char_count) {
                unsigned int low = FPDFText_GetUnicode(text_page, i + 1);
                if (low >= 0xDC00 && low <= 0xDFFF) {
                    // Valid surrogate pair
                    codepoint = ((unicode - 0xD800) << 10) + (low - 0xDC00) + 0x10000;
                    chars_consumed = 2;
                } else {
                    // Invalid surrogate pair - use replacement character
                    codepoint = 0xFFFD;
                    chars_consumed = 1;
                }
            } else {
                // High surrogate at end of text - invalid
                codepoint = 0xFFFD;
                chars_consumed = 1;
            }
        } else if (unicode >= 0xDC00 && unicode <= 0xDFFF) {
            // Lone low surrogate (invalid) - use replacement character
            codepoint = 0xFFFD;
            chars_consumed = 1;
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

        // 5. Get font info (name and flags)
        char font_name[256] = "unknown";
        int font_flags = 0;
        int font_name_len = FPDFText_GetFontInfo(text_page, i, NULL, 0, &font_flags);
        if (font_name_len > 0 && font_name_len < (int)sizeof(font_name)) {
            unsigned char buffer[256];
            FPDFText_GetFontInfo(text_page, i, buffer, font_name_len, &font_flags);
            // Remove trailing nulls
            int len = font_name_len - 1;
            while (len > 0 && buffer[len] == 0) len--;
            if (len > 0) {
                memcpy(font_name, buffer, len + 1);
                font_name[len + 1] = 0;
            }
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
        double angle = FPDFText_GetCharAngle(text_page, i);

        // 10. Get transformation matrix
        FS_MATRIX matrix;
        FPDFText_GetMatrix(text_page, i, &matrix);

        // 11. Check if generated
        int is_generated = FPDFText_IsGenerated(text_page, i);

        // 12. Check if hyphen
        int is_hyphen = FPDFText_IsHyphen(text_page, i);

        // 13. Check for unicode mapping error
        int has_unicode_error = FPDFText_HasUnicodeMapError(text_page, i);

        // Write JSON line
        fprintf(out, "{\"char\":");
        write_json_escaped_char(out, codepoint);
        fprintf(out, ",\"unicode\":%u", codepoint);
        fprintf(out, ",\"bbox\":[%f,%f,%f,%f]", left, bottom, right, top);
        fprintf(out, ",\"origin\":[%f,%f]", origin_x, origin_y);
        fprintf(out, ",\"font_size\":%f", font_size);
        fprintf(out, ",\"font_name\":");
        write_json_escaped_string(out, font_name);
        fprintf(out, ",\"font_flags\":%d", font_flags);
        fprintf(out, ",\"font_weight\":%d", font_weight);
        fprintf(out, ",\"fill_color\":[%u,%u,%u,%u]", fill_r, fill_g, fill_b, fill_a);
        fprintf(out, ",\"stroke_color\":[%u,%u,%u,%u]", stroke_r, stroke_g, stroke_b, stroke_a);
        fprintf(out, ",\"angle\":%f", angle);
        fprintf(out, ",\"matrix\":[%f,%f,%f,%f,%f,%f]",
                matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f);
        fprintf(out, ",\"is_generated\":%s", is_generated ? "true" : "false");
        fprintf(out, ",\"is_hyphen\":%s", is_hyphen ? "true" : "false");
        fprintf(out, ",\"has_unicode_error\":%s", has_unicode_error ? "true" : "false");
        fprintf(out, "}\n");

        i += chars_consumed;
    }

    // Cleanup
    FPDFText_ClosePage(text_page);
    FPDF_ClosePage(page);
    if (fclose(out) != 0) {
        fprintf(stderr, "Error: Failed to close output file '%s': %s\n", output_path, strerror(errno));
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    fprintf(stderr, "JSONL extraction complete: %s\n", output_path);
    return 0;
}

// ========================================
// JSONL Extraction - Debug Mode
// ========================================

int extract_jsonl_debug(const char* pdf_path, const char* output_path, int page_num) {
    fprintf(stderr, "[TRACE] FPDF_InitLibrary()\n");
    FPDF_InitLibrary();

    fprintf(stderr, "[TRACE] FPDF_LoadDocument(%s)\n", pdf_path);
    FPDF_DOCUMENT doc = FPDF_LoadDocument(pdf_path, NULL);
    if (!doc) {
        fprintf(stderr, "[ERROR] Failed to load PDF\n");
        FPDF_DestroyLibrary();
        return 2;
    }
    fprintf(stderr, "[TRACE] Document loaded: %p\n", doc);

    int page_count = FPDF_GetPageCount(doc);
    fprintf(stderr, "[TRACE] FPDF_GetPageCount() -> %d\n", page_count);

    if (page_num < 0 || page_num >= page_count) {
        fprintf(stderr, "[ERROR] Invalid page number %d (document has %d pages)\n", page_num, page_count);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }

    FILE* out = fopen(output_path, "wb");
    if (!out) {
        fprintf(stderr, "[ERROR] Failed to create output file: %s\n", output_path);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }
    fprintf(stderr, "[TRACE] Output file opened: %s\n", output_path);

    fprintf(stderr, "[TRACE] FPDF_LoadPage(%d)\n", page_num);
    FPDF_PAGE page = FPDF_LoadPage(doc, page_num);
    if (!page) {
        fprintf(stderr, "[ERROR] Failed to load page %d\n", page_num);
        fclose(out);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 2;
    }
    fprintf(stderr, "[TRACE] Page loaded: %p\n", page);

    fprintf(stderr, "[TRACE] FPDFText_LoadPage()\n");
    FPDF_TEXTPAGE text_page = FPDFText_LoadPage(page);
    if (!text_page) {
        fprintf(stderr, "[ERROR] Failed to load text for page %d\n", page_num);
        FPDF_ClosePage(page);
        fclose(out);
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 2;
    }
    fprintf(stderr, "[TRACE] Text page loaded: %p\n", text_page);

    int char_count = FPDFText_CountChars(text_page);
    fprintf(stderr, "[TRACE] FPDFText_CountChars() -> %d\n", char_count);
    fprintf(stderr, "[INFO] Extracting %d characters with metadata\n", char_count);

    int surrogate_pairs = 0;
    int i = 0;
    while (i < char_count) {
        unsigned int unicode = FPDFText_GetUnicode(text_page, i);
        unsigned int codepoint;
        int chars_consumed;

        if (unicode >= 0xD800 && unicode <= 0xDBFF) {
            if (i + 1 < char_count) {
                unsigned int low = FPDFText_GetUnicode(text_page, i + 1);
                if (low >= 0xDC00 && low <= 0xDFFF) {
                    codepoint = ((unicode - 0xD800) << 10) + (low - 0xDC00) + 0x10000;
                    chars_consumed = 2;
                    surrogate_pairs++;
                    fprintf(stderr, "[DEBUG] Surrogate pair at char %d: U+%04X U+%04X -> U+%06X\n",
                            i, unicode, low, codepoint);
                } else {
                    codepoint = 0xFFFD;
                    chars_consumed = 1;
                    fprintf(stderr, "[WARN] Invalid surrogate pair at char %d\n", i);
                }
            } else {
                codepoint = 0xFFFD;
                chars_consumed = 1;
                fprintf(stderr, "[WARN] Lone high surrogate at end of text\n");
            }
        } else {
            codepoint = unicode;
            chars_consumed = 1;
        }

        double left, right, bottom, top;
        FPDFText_GetCharBox(text_page, i, &left, &right, &bottom, &top);

        double origin_x, origin_y;
        FPDFText_GetCharOrigin(text_page, i, &origin_x, &origin_y);

        double font_size = FPDFText_GetFontSize(text_page, i);

        char font_name[256] = "unknown";
        int font_flags = 0;
        int font_name_len = FPDFText_GetFontInfo(text_page, i, NULL, 0, &font_flags);
        if (font_name_len > 0 && font_name_len < (int)sizeof(font_name)) {
            unsigned char buffer[256];
            FPDFText_GetFontInfo(text_page, i, buffer, font_name_len, &font_flags);
            int len = font_name_len - 1;
            while (len > 0 && buffer[len] == 0) len--;
            if (len > 0) {
                memcpy(font_name, buffer, len + 1);
                font_name[len + 1] = 0;
            }
        }

        int font_weight = FPDFText_GetFontWeight(text_page, i);

        unsigned int fill_r, fill_g, fill_b, fill_a;
        FPDFText_GetFillColor(text_page, i, &fill_r, &fill_g, &fill_b, &fill_a);

        unsigned int stroke_r, stroke_g, stroke_b, stroke_a;
        FPDFText_GetStrokeColor(text_page, i, &stroke_r, &stroke_g, &stroke_b, &stroke_a);

        double angle = FPDFText_GetCharAngle(text_page, i);

        FS_MATRIX matrix;
        FPDFText_GetMatrix(text_page, i, &matrix);

        int is_generated = FPDFText_IsGenerated(text_page, i);
        int is_hyphen = FPDFText_IsHyphen(text_page, i);
        int has_unicode_error = FPDFText_HasUnicodeMapError(text_page, i);

        fprintf(out, "{\"char\":");
        write_json_escaped_char(out, codepoint);
        fprintf(out, ",\"unicode\":%u", codepoint);
        fprintf(out, ",\"bbox\":[%f,%f,%f,%f]", left, bottom, right, top);
        fprintf(out, ",\"origin\":[%f,%f]", origin_x, origin_y);
        fprintf(out, ",\"font_size\":%f", font_size);
        fprintf(out, ",\"font_name\":");
        write_json_escaped_string(out, font_name);
        fprintf(out, ",\"font_flags\":%d", font_flags);
        fprintf(out, ",\"font_weight\":%d", font_weight);
        fprintf(out, ",\"fill_color\":[%u,%u,%u,%u]", fill_r, fill_g, fill_b, fill_a);
        fprintf(out, ",\"stroke_color\":[%u,%u,%u,%u]", stroke_r, stroke_g, stroke_b, stroke_a);
        fprintf(out, ",\"angle\":%f", angle);
        fprintf(out, ",\"matrix\":[%f,%f,%f,%f,%f,%f]",
                matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f);
        fprintf(out, ",\"is_generated\":%s", is_generated ? "true" : "false");
        fprintf(out, ",\"is_hyphen\":%s", is_hyphen ? "true" : "false");
        fprintf(out, ",\"has_unicode_error\":%s", has_unicode_error ? "true" : "false");
        fprintf(out, "}\n");

        i += chars_consumed;
    }

    FPDFText_ClosePage(text_page);
    FPDF_ClosePage(page);
    if (fclose(out) != 0) {
        fprintf(stderr, "Error: Failed to close output file '%s': %s\n", output_path, strerror(errno));
        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
        return 1;
    }
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();

    fprintf(stderr, "[SUMMARY] Extracted %d characters (%d surrogate pairs)\n",
            char_count, surrogate_pairs);
    fprintf(stderr, "[TRACE] JSONL extraction complete: %s\n", output_path);
    return 0;
}
