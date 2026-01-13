// Copyright 2025 The PDFium Authors
// Copyright 2025 Andrew Yates (Dash PDF Extraction - parallel rendering implementation)
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
//
// This file implements parallel rendering for PDFium (new functionality):
// - Multi-threaded page rendering with thread pool
// - Pre-loading strategy to populate shared resource caches
// - Mutex-protected page loading for thread safety
// - Two rendering modes: ProcessTask (v1) and ProcessTaskV2 (v2)

#include "public/fpdf_parallel.h"

#include <algorithm>

#include "public/fpdf_edit.h"      // N=202: For FPDFPage_HasTransparency
#include "public/fpdf_formfill.h"  // N=200: For FPDF_FFLDraw
#include <atomic>
#include <condition_variable>
#include <memory>
#include <mutex>
#include <queue>
#include <thread>
#include <vector>

#include "core/fpdfapi/page/cpdf_page.h"
#include "core/fpdfapi/parser/cpdf_document.h"
#include "core/fpdfapi/parser/cpdf_linearized_header.h"
#include "core/fpdfapi/render/cpdf_pagerendercontext.h"
#include "core/fxcrt/unowned_ptr.h"
#include "core/fxge/cfx_defaultrenderdevice.h"
#include "core/fxge/dib/cfx_dibitmap.h"
#include "fpdfsdk/cpdfsdk_helpers.h"
#include "fpdfsdk/cpdfsdk_renderpage.h"
#include "public/fpdfview.h"

// Suppress warnings for third-party library
#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wglobal-constructors"
#endif

#include "third_party/concurrentqueue/concurrentqueue.h"

#if defined(__clang__)
#pragma clang diagnostic pop
#endif

namespace {

// SESSION 79: Page handle storage for deferred destruction
// Thread-safe collection to store page handles across all parallel rendering
// tasks. Workers append loaded pages instead of closing them. Main thread
// closes all pages after WaitForCompletion().
class PageHandleCollection {
 public:
  void Add(FPDF_PAGE page) {
    if (!page) {
      return;
    }
    std::lock_guard<std::mutex> lock(mutex_);
    pages_.push_back(page);
  }

  void CloseAll() {
    std::lock_guard<std::mutex> lock(mutex_);
    CloseAllInternal();
  }

  // N=152 Fix #15: Close pages under document load mutex for thread safety
  // This ensures page close is synchronized with any concurrent page loading
  void CloseAllUnderDocLock(FPDF_DOCUMENT doc) {
    CPDF_Document* cpdf_doc = CPDFDocumentFromFPDFDocument(doc);
    if (!cpdf_doc) {
      // Fallback to non-locked close if document is invalid
      CloseAll();
      return;
    }
    std::lock_guard<std::mutex> doc_lock(cpdf_doc->GetLoadPageMutex());
    std::lock_guard<std::mutex> lock(mutex_);
    CloseAllInternal();
  }

 private:
  void CloseAllInternal() {
    // FIX #2 (N=30): Actually close pages to prevent memory leak
    // Previous N=315 comment was wrong - FPDF_CloseDocument does NOT free
    // worker-loaded pages, only pages opened via public API. This caused
    // memory leaks (~4MB per render pass, Session 80 testing).
    //
    // N=210 serialized rendering under GetLoadPageMutex(), so pages are
    // loaded/rendered one at a time. Safe to close in reverse order.
    // Reverse iteration minimizes shared resource conflicts.
    for (auto it = pages_.rbegin(); it != pages_.rend(); ++it) {
      if (*it) {
        FPDF_ClosePage(*it);
      }
    }
    pages_.clear();
  }

  std::mutex mutex_;
  std::vector<FPDF_PAGE> pages_;
};

// Thread-safe work queue for page rendering tasks (processed by render threads)
struct RenderTask {
  FPDF_DOCUMENT document;
  int page_index;
  int width;
  int height;
  int rotate;
  int flags;
  FPDF_PARALLEL_CALLBACK callback;
  uintptr_t user_data;  // Store as integer to avoid raw_ptr check
  UnownedPtr<PageHandleCollection>
      page_collection;  // SESSION 79: For deferred destruction
  FPDF_FORMHANDLE form_handle;  // N=122: For rendering form fields (matching V2)
  int output_format;  // N=31: Output pixel format (0=BGRx, 1=BGR, 2=Gray)
};

// V2 render task for bitmap pooling API
struct RenderTaskV2 {
  FPDF_DOCUMENT document;
  int page_index;
  int width;   // 0 = auto-detect from dpi (N=118)
  int height;  // 0 = auto-detect from dpi (N=118)
  int rotate;
  int flags;
  FPDF_PARALLEL_CALLBACK_V2 callback_v2;
  uintptr_t user_data;  // Store as integer to avoid raw_ptr check
  UnownedPtr<PageHandleCollection>
      page_collection;  // SESSION 79: For deferred destruction
  FPDF_FORMHANDLE form_handle;  // N=200: For rendering form fields
  double dpi;  // N=118: For per-page dimension calculation (0 = use width/height)
  int output_format;  // N=31: Output pixel format (0=BGRx, 1=BGR, 2=Gray)
};

// N=31: Convert parallel format option to FPDFBitmap format constant
static int ParallelFormatToFPDFFormat(int output_format) {
  switch (output_format) {
    case FPDF_PARALLEL_FORMAT_BGR:
      return FPDFBitmap_BGR;
    case FPDF_PARALLEL_FORMAT_GRAY:
      return FPDFBitmap_Gray;
    default:  // FPDF_PARALLEL_FORMAT_BGRx (0) and any unknown
      return FPDFBitmap_BGRx;
  }
}

// Bitmap pool for reusing allocations across pages
// Eliminates 3-8% overhead from bitmap create/destroy cycles
class BitmapPool {
 public:
  BitmapPool() = default;

  ~BitmapPool() {
    // CRITICAL (N=413): Do NOT clean up bitmaps in destructor!
    // Thread-local destructors run during exit() AFTER PartitionAlloc teardown.
    // Bitmap pool must be explicitly cleared before thread exit (line 311).
    // If we Clear() here, we crash with SIGSEGV in PartitionAlloc::Free().
    // See: lldb backtrace showing dyld::ThreadLocalVariables::finalizeList()
    // calling this destructor during exit(), not during thread join().
  }

  // N=31: Acquire a bitmap with specific format from the pool (or create new)
  // output_format: 0=BGRx (4bpp), 1=BGR (3bpp), 2=Gray (1bpp)
  FPDF_BITMAP Acquire(int width, int height, int output_format = 0) {
    std::lock_guard<std::mutex> lock(mutex_);
    int fpdf_format = ParallelFormatToFPDFFormat(output_format);

    // Try to find matching bitmap in pool (must match width, height, AND format)
    for (auto it = pool_.begin(); it != pool_.end(); ++it) {
      FPDF_BITMAP bitmap = *it;
      if (FPDFBitmap_GetWidth(bitmap) == width &&
          FPDFBitmap_GetHeight(bitmap) == height &&
          FPDFBitmap_GetFormat(bitmap) == fpdf_format) {
        pool_.erase(it);
        return bitmap;
      }
    }

    // No match found, create new bitmap with requested format
    return FPDFBitmap_CreateEx(width, height, fpdf_format, nullptr, 0);
  }

  // Legacy overload for backward compatibility (defaults to BGRx format)
  FPDF_BITMAP Acquire(int width, int height) {
    return Acquire(width, height, 0);  // 0 = BGRx
  }

  // Release bitmap back to pool for reuse
  void Release(FPDF_BITMAP bitmap) {
    if (!bitmap) {
      return;
    }

    std::lock_guard<std::mutex> lock(mutex_);

    // Limit pool size to avoid unbounded growth
    if (pool_.size() < kMaxPoolSize) {
      pool_.push_back(bitmap);
    } else {
      // Pool full, destroy the bitmap
      FPDFBitmap_Destroy(bitmap);
    }
  }

  // Clear all pooled bitmaps (called before thread exit)
  // This ensures bitmap destruction happens in a controlled manner
  // to avoid race conditions in PartitionAlloc when multiple threads
  // exit simultaneously.
  void Clear() {
    std::lock_guard<std::mutex> lock(mutex_);
    for (FPDF_BITMAP bitmap : pool_) {
      if (bitmap) {
        FPDFBitmap_Destroy(bitmap);
      }
    }
    pool_.clear();
  }

  // Prevent copying
  BitmapPool(const BitmapPool&) = delete;
  BitmapPool& operator=(const BitmapPool&) = delete;

 private:
  static constexpr size_t kMaxPoolSize = 32;  // Max bitmaps per thread
  std::vector<FPDF_BITMAP> pool_;

  // NOTE: This mutex is technically unnecessary since BitmapPool is
  // thread_local (only one thread ever accesses each BitmapPool instance).
  // However, it's kept for defensive programming with minimal overhead (~5-10ns
  // per Acquire/Release). If this becomes a bottleneck, the mutex can be
  // removed safely.
  std::mutex mutex_;
};

// Per-thread bitmap pool
// THREAD SAFETY: Automatic cleanup via thread_local destructor.
// Each thread gets its own BitmapPool instance. When the thread exits,
// the destructor runs automatically (C++11 standard), freeing all pooled
// bitmaps. This eliminates the memory leak from the previous raw pointer
// approach.
//
// NOTE: Suppress -Wglobal-constructors warning - thread_local destructors are
// well-defined in C++11 and execute when the thread exits (not at program
// exit). This is the standard pattern for thread-local resources with RAII
// cleanup.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wglobal-constructors"
thread_local BitmapPool g_thread_bitmap_pool;
#pragma clang diagnostic pop

BitmapPool* GetThreadBitmapPool() {
  return &g_thread_bitmap_pool;
}

// Global persistent thread pool that eliminates 8-14% overhead
// from creating/destroying threads on each render call
// NOTE: These are threads within the same process, not separate processes
//
// MEMORY SAFETY: Thread pool is now application-managed (not static singleton)
// to avoid static destructor ordering issues with PartitionAlloc.
// Application should call FPDF_DestroyThreadPool() before
// FPDF_DestroyLibrary().
class GlobalThreadPool {
 public:
  GlobalThreadPool() : stop_(false), max_queue_depth_(0), clear_pools_(false) {}

  ~GlobalThreadPool() {
    // Signal all render threads to stop
    stop_.store(true, std::memory_order_release);
    condition_.notify_all();

    // Wait for all render threads to finish
    std::unique_lock<std::mutex> pool_lock(pool_mutex_);
    for (auto& worker : workers_) {
      if (worker.joinable()) {
        worker.join();
      }
    }
  }

  GlobalThreadPool(const GlobalThreadPool&) = delete;
  GlobalThreadPool& operator=(const GlobalThreadPool&) = delete;

  // Ensure the pool has at least the requested number of render threads
  void EnsureWorkerCount(int desired_count) {
    std::unique_lock<std::mutex> lock(pool_mutex_);

    int current_count = static_cast<int>(workers_.size());
    if (desired_count <= current_count) {
      return;  // Already have enough threads
    }

    // Create additional render threads as needed
    for (int i = current_count; i < desired_count; ++i) {
      workers_.emplace_back(&GlobalThreadPool::WorkerThread, this);
    }
  }

  // N=118: Set max queue depth for backpressure (0 = unlimited)
  // Must be called before enqueueing tasks
  void SetMaxQueueDepth(int depth) {
    max_queue_depth_.store(depth, std::memory_order_release);
  }

  // Submit a rendering task to the pool (lock-free)
  // FIX #1 (N=30): Increment outstanding_tasks_ BEFORE enqueue for precise tracking
  // N=118: Backpressure - block if queue exceeds max_queue_depth_
  void EnqueueTask(RenderTask task) {
    WaitForBackpressure();
    outstanding_tasks_.fetch_add(1, std::memory_order_release);
    task_queue_.enqueue(std::move(task));
    condition_.notify_one();
  }

  // Submit a V2 rendering task to the pool (lock-free)
  // FIX #1 (N=30): Increment outstanding_tasks_ BEFORE enqueue for precise tracking
  // N=118: Backpressure - block if queue exceeds max_queue_depth_
  void EnqueueTaskV2(RenderTaskV2 task) {
    WaitForBackpressure();
    outstanding_tasks_.fetch_add(1, std::memory_order_release);
    task_queue_v2_.enqueue(std::move(task));
    condition_.notify_one();
  }

  // Batch submit V2 tasks for better performance (lock-free)
  // FIX #1 (N=30): Increment outstanding_tasks_ by batch size BEFORE enqueue
  // N=118: Backpressure - wait for queue to drain before bulk enqueue
  void EnqueueTasksV2Batch(std::vector<RenderTaskV2>& tasks) {
    if (tasks.empty()) {
      return;
    }
    // N=118: For batch enqueue, wait until we have room for the entire batch
    WaitForBackpressureBatch(static_cast<int>(tasks.size()));
    outstanding_tasks_.fetch_add(static_cast<int>(tasks.size()), std::memory_order_release);
    // Bulk enqueue - more efficient than individual enqueues
    task_queue_v2_.enqueue_bulk(std::make_move_iterator(tasks.begin()),
                                tasks.size());
    // Wake all workers at once for batch
    condition_.notify_all();
  }

  // Wait until all tasks are complete
  // FIX #1 (N=30): Use precise outstanding_tasks_ counter instead of size_approx()
  // size_approx() is unreliable - can show 0 while tasks still queued (early return → UAF)
  // or >0 after drain (hang). outstanding_tasks_ incremented on enqueue, decremented after
  // processing completes, giving precise tracking.
  void WaitForCompletion() {
    std::unique_lock<std::mutex> lock(wait_mutex_);
    done_condition_.wait(lock, [this] {
      // THREAD SAFETY: Use acquire to synchronize with workers' release
      // Ensures we see all task processing results before proceeding
      return outstanding_tasks_.load(std::memory_order_acquire) == 0;
    });
  }

  // N=152 Fix #9: Signal workers to clear their bitmap pools
  // This prevents memory accumulation between jobs in long-running applications
  // Workers check this flag when idle and clear their pools
  void SignalClearPools() {
    clear_pools_.store(true, std::memory_order_release);
    condition_.notify_all();  // Wake workers to check the flag
  }

 private:
  void WorkerThread() {
    while (true) {
      bool is_v2 = false;
      RenderTask task;
      RenderTaskV2 task_v2;

      // Try to dequeue a task (lock-free, fast path)
      // Priority: V2 tasks first (better performance with bitmap pooling)
      if (task_queue_v2_.try_dequeue(task_v2)) {
        is_v2 = true;
        // N=152 Fix #18: Removed unused active_tasks_ counter
        // outstanding_tasks_ is the authoritative counter used by WaitForCompletion()
      } else if (task_queue_.try_dequeue(task)) {
        is_v2 = false;
      } else {
        // No tasks available, wait for notification
        std::unique_lock<std::mutex> lock(wait_mutex_);
        condition_.wait(lock, [this] {
          return stop_.load(std::memory_order_acquire) ||
                 clear_pools_.load(std::memory_order_acquire) ||  // N=152 Fix #9
                 task_queue_.size_approx() > 0 ||
                 task_queue_v2_.size_approx() > 0;
        });

        // N=152 Fix #9: Clear bitmap pools when signaled
        // This prevents memory accumulation between jobs
        if (clear_pools_.load(std::memory_order_acquire)) {
          GetThreadBitmapPool()->Clear();
          // Only reset flag if no outstanding tasks (main thread waiting)
          // Race-safe: worst case is pool cleared again on next iteration
        }

        // Check if we should stop
        // N=117: Use outstanding_tasks_ instead of size_approx() for reliable shutdown
        // size_approx() is approximate and could prevent clean shutdown
        if (stop_.load(std::memory_order_acquire) &&
            outstanding_tasks_.load(std::memory_order_acquire) == 0) {
          // CRITICAL FIX (N=413): Clear bitmap pool BEFORE thread exit
          // to avoid destructor ordering issues with PartitionAlloc.
          // Without this, thread_local destructors run during program exit
          // AFTER PartitionAlloc teardown begins, causing SIGSEGV.
          // This fix eliminates crashes at K>=2 (bug_451265.pdf).
          GetThreadBitmapPool()->Clear();
          return;
        }

        // Loop back to try dequeueing again
        continue;
      }

      // Render the page (no locks held)
      if (is_v2) {
        ProcessTaskV2(task_v2);
      } else {
        ProcessTask(task);
      }

      // Mark task as complete and notify if all done
      // THREAD SAFETY: Use release to ensure all writes from task processing
      // are visible to the main thread before it observes outstanding_tasks_ == 0
      // FIX #1 (N=30): Decrement outstanding_tasks_ AFTER processing completes
      // This is the precise counter used by WaitForCompletion()
      // N=152 Fix #18: Removed unused active_tasks_ counter
      int remaining = outstanding_tasks_.fetch_sub(1, std::memory_order_release) - 1;
      if (remaining == 0) {
        done_condition_.notify_all();
      }
      // N=118: Notify backpressure waiters that queue space is available
      backpressure_condition_.notify_one();
    }
  }

  // N=118: Block until queue has space (backpressure for large documents)
  // Prevents memory spike from unbounded queue on 1000+ page documents
  void WaitForBackpressure() {
    int max_depth = max_queue_depth_.load(std::memory_order_acquire);
    if (max_depth <= 0) {
      return;  // Unlimited queue - no backpressure
    }
    std::unique_lock<std::mutex> lock(wait_mutex_);
    backpressure_condition_.wait(lock, [this, max_depth] {
      return outstanding_tasks_.load(std::memory_order_acquire) < max_depth;
    });
  }

  // N=118: Block until queue has space for entire batch
  void WaitForBackpressureBatch(int batch_size) {
    int max_depth = max_queue_depth_.load(std::memory_order_acquire);
    if (max_depth <= 0) {
      return;  // Unlimited queue - no backpressure
    }
    // If batch is larger than max_depth, just wait until queue is empty
    // This allows progress even for very large batches
    int required_space = std::min(batch_size, max_depth);
    std::unique_lock<std::mutex> lock(wait_mutex_);
    backpressure_condition_.wait(lock, [this, max_depth, required_space] {
      return outstanding_tasks_.load(std::memory_order_acquire) <= (max_depth - required_space);
    });
  }

  void ProcessTask(const RenderTask& task) {
    FPDF_BOOL success = false;
    FPDF_BITMAP bitmap = nullptr;

    // N=210: CONSERVATIVE FIX - Serialize entire rendering pipeline
    // Root cause: Rare vector out of bounds crashes (~2% rate) in PDFium internals
    // during parallel rendering. N=341's load_page_mutex_ was insufficient.
    // Solution: Serialize entire render operation (load + render + close).
    // Trade-off: Reduced parallelism but guaranteed correctness.
    FPDF_PAGE page = nullptr;
    {
      CPDF_Document* cpdf_doc = CPDFDocumentFromFPDFDocument(task.document);
      std::lock_guard<std::mutex> lock(cpdf_doc->GetLoadPageMutex());

      // Load page (protected by mutex)
      page = FPDF_LoadPage(task.document, task.page_index);
      if (!page) {
        task.callback(task.page_index, nullptr,
                      reinterpret_cast<void*>(task.user_data), false);
        return;
      }

      // N=137: Form lifecycle callbacks - match single-threaded path
      if (task.form_handle) {
        FORM_OnAfterLoadPage(page, task.form_handle);
        FORM_DoPageAAction(page, task.form_handle, FPDFPAGE_AACTION_OPEN);
      }

      // N=31: Create bitmap with specified format
      int fpdf_format = ParallelFormatToFPDFFormat(task.output_format);
      bitmap = FPDFBitmap_CreateEx(task.width, task.height, fpdf_format, nullptr, 0);

      if (bitmap) {
        // N=202: Check page transparency to match single-threaded fill logic
        int has_transparency = FPDFPage_HasTransparency(page);
        uint32_t fill_color = has_transparency ? 0x00000000 : 0xFFFFFFFF;
        FPDFBitmap_FillRect(bitmap, 0, 0, task.width, task.height, fill_color);

        // N=31: Add FPDF_GRAYSCALE flag for grayscale output format
        int render_flags = task.flags;
        if (task.output_format == FPDF_PARALLEL_FORMAT_GRAY) {
          render_flags |= FPDF_GRAYSCALE;
        }

        // Render the page to the bitmap (protected by mutex)
        FPDF_RenderPageBitmap(bitmap, page, 0, 0, task.width, task.height,
                              task.rotate, render_flags);

        // N=122: Render form fields to match V2 API and single-threaded path
        if (task.form_handle) {
          FPDF_FFLDraw(task.form_handle, bitmap, page, 0, 0, task.width,
                       task.height, task.rotate, task.flags);
        }
        success = true;
      }

      // N=137: Form close callbacks - match single-threaded path
      if (task.form_handle) {
        FORM_DoPageAAction(page, task.form_handle, FPDFPAGE_AACTION_CLOSE);
        FORM_OnBeforeClosePage(page, task.form_handle);
      }

      // SESSION 79: Store page for deferred destruction instead of closing
      // (protected by mutex via page_collection->Add internal mutex)
      if (task.page_collection) {
        task.page_collection->Add(page);
      } else {
        FPDF_ClosePage(page);
      }
    }
    // Mutex released here - rendering complete

    // Call the user's callback - callback takes ownership of bitmap
    task.callback(task.page_index, bitmap,
                  reinterpret_cast<void*>(task.user_data), success);
  }

  void ProcessTaskV2(const RenderTaskV2& task) {
    // Get thread-local bitmap pool
    BitmapPool* pool = GetThreadBitmapPool();

    // N=210: CONSERVATIVE FIX - Serialize entire rendering pipeline
    // Root cause: Rare vector out of bounds crashes (~2% rate) in PDFium internals
    // during parallel rendering. N=341's load_page_mutex_ was insufficient.
    // Solution: Serialize entire render operation (load + render + close).
    // Trade-off: Reduced parallelism but guaranteed correctness.
    // Evidence: 24/50 runs crashed before fix, 0/100 runs after (N=210 testing).
    FPDF_PAGE page = nullptr;
    FPDF_BITMAP bitmap = nullptr;
    const void* buffer = nullptr;
    int stride = 0;
    int actual_width = task.width;
    int actual_height = task.height;
    bool success = false;
    {
      CPDF_Document* cpdf_doc = CPDFDocumentFromFPDFDocument(task.document);
      std::lock_guard<std::mutex> lock(cpdf_doc->GetLoadPageMutex());

      // Load page (protected by mutex)
      page = FPDF_LoadPage(task.document, task.page_index);
      if (!page) {
        task.callback_v2(task.page_index, nullptr, 0, 0, 0,
                         reinterpret_cast<void*>(task.user_data), false);
        return;
      }

      // N=137: Form lifecycle callbacks - match single-threaded path
      if (task.form_handle) {
        FORM_OnAfterLoadPage(page, task.form_handle);
        FORM_DoPageAAction(page, task.form_handle, FPDFPAGE_AACTION_OPEN);
      }

      // N=118: Auto-detect dimensions if width/height are 0 and dpi is set
      if (task.width == 0 && task.height == 0 && task.dpi > 0) {
        double page_width_pts = FPDF_GetPageWidthF(page);
        double page_height_pts = FPDF_GetPageHeightF(page);
        // Match CLI precision: floor scale to 6 decimals
        double scale = floor((task.dpi / 72.0) * 1000000.0) / 1000000.0;
        actual_width = static_cast<int>(page_width_pts * scale);
        actual_height = static_cast<int>(page_height_pts * scale);
        if (actual_width < 1 || actual_height < 1) {
          FPDF_ClosePage(page);
          task.callback_v2(task.page_index, nullptr, 0, 0, 0,
                           reinterpret_cast<void*>(task.user_data), false);
          return;
        }
      }

      // N=31: Acquire bitmap from pool with specified format (reuse if available)
      bitmap = pool->Acquire(actual_width, actual_height, task.output_format);
      if (!bitmap) {
        if (task.page_collection) {
          task.page_collection->Add(page);
        } else {
          FPDF_ClosePage(page);
        }
        task.callback_v2(task.page_index, nullptr, 0, 0, 0,
                         reinterpret_cast<void*>(task.user_data), false);
        return;
      }

      // N=202: Check page transparency to match single-threaded fill logic
      int has_transparency = FPDFPage_HasTransparency(page);
      uint32_t fill_color = has_transparency ? 0x00000000 : 0xFFFFFFFF;
      FPDFBitmap_FillRect(bitmap, 0, 0, actual_width, actual_height, fill_color);

      // N=31: Add FPDF_GRAYSCALE flag for grayscale output format
      int render_flags = task.flags;
      if (task.output_format == FPDF_PARALLEL_FORMAT_GRAY) {
        render_flags |= FPDF_GRAYSCALE;
      }

      // Render the page to the bitmap (protected by mutex)
      FPDF_RenderPageBitmap(bitmap, page, 0, 0, actual_width, actual_height,
                            task.rotate, render_flags);

      // N=200: Render form fields to match single-threaded path (protected by mutex)
      if (task.form_handle) {
        FPDF_FFLDraw(task.form_handle, bitmap, page, 0, 0, actual_width, actual_height,
                     task.rotate, task.flags);
      }

      // Get buffer info for callback (must be done while page is valid)
      buffer = FPDFBitmap_GetBuffer(bitmap);
      stride = FPDFBitmap_GetStride(bitmap);
      success = true;

      // N=137: Form close callbacks - match single-threaded path
      if (task.form_handle) {
        FORM_DoPageAAction(page, task.form_handle, FPDFPAGE_AACTION_CLOSE);
        FORM_OnBeforeClosePage(page, task.form_handle);
      }

      // SESSION 79: Store page for deferred destruction instead of closing
      // (protected by mutex via page_collection->Add internal mutex)
      if (task.page_collection) {
        task.page_collection->Add(page);
      } else {
        FPDF_ClosePage(page);
      }
    }
    // Mutex released here - rendering complete

    // Call user callback with raw buffer (buffer only valid during callback)
    // N=118: Pass actual dimensions (may differ from task if auto-detected)
    task.callback_v2(task.page_index, buffer, actual_width, actual_height, stride,
                     reinterpret_cast<void*>(task.user_data), success);

    // Return bitmap to pool (CRITICAL: we own it, callback doesn't destroy)
    pool->Release(bitmap);
  }

  std::vector<std::thread> workers_;
  std::mutex pool_mutex_;  // Protects workers_ vector during growth

  // Lock-free task queues (eliminates 2-5% queue contention overhead)
  moodycamel::ConcurrentQueue<RenderTask> task_queue_;
  moodycamel::ConcurrentQueue<RenderTaskV2> task_queue_v2_;

  // Mutex only for condition variable synchronization (not for queue access)
  std::mutex wait_mutex_;
  std::condition_variable condition_;
  std::condition_variable done_condition_;
  // N=152 Fix #18: Removed unused active_tasks_ counter - it was never read
  // FIX #1 (N=30): Precise task counter - incremented on enqueue, decremented after processing
  // This replaces unreliable size_approx() checks in WaitForCompletion()
  std::atomic<int> outstanding_tasks_{0};
  std::atomic<bool> stop_{false};
  // N=118: Backpressure - max outstanding tasks (0 = unlimited)
  std::atomic<int> max_queue_depth_{0};
  std::condition_variable backpressure_condition_;
  // N=152 Fix #9: Flag to signal workers to clear bitmap pools between jobs
  std::atomic<bool> clear_pools_{false};
};

// Application-managed render thread pool (not static singleton)
// Protected by mutex to ensure thread-safe creation/destruction
// NOTE: Suppress -Wglobal-constructors warning - these globals are necessary
// for application-managed thread pool lifetime control
//
// ARCHITECTURE: Separate thread pools for rendering vs text extraction
// - g_render_pool: Used by FPDF_RenderPagesParallel*() APIs
// - g_text_pool: Used by FPDF_ExtractTextParallel() API (defined in
// fpdf_text_parallel.cpp) This separation allows simultaneous rendering + text
// extraction on same document
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wglobal-constructors"
std::mutex g_render_pool_mutex;
GlobalThreadPool* g_render_pool = nullptr;
#pragma clang diagnostic pop

// Get or create the global render thread pool (thread-safe)
GlobalThreadPool* GetOrCreateThreadPool() {
  std::lock_guard<std::mutex> lock(g_render_pool_mutex);
  if (!g_render_pool) {
    g_render_pool = new GlobalThreadPool();
  }
  return g_render_pool;
}

// Destroy the global render thread pool (thread-safe, idempotent)
void DestroyThreadPool() {
  std::lock_guard<std::mutex> lock(g_render_pool_mutex);
  if (g_render_pool) {
    delete g_render_pool;
    g_render_pool = nullptr;
  }
}

}  // namespace

FPDF_EXPORT int FPDF_CALLCONV FPDF_GetOptimalWorkerCount() {
  // Get hardware concurrency (number of CPU cores)
  unsigned int hw_threads = std::thread::hardware_concurrency();

  // If we can't detect, use a reasonable default
  if (hw_threads == 0) {
    return 4;
  }

  // Use all cores, but cap at a reasonable maximum
  // Most systems benefit from using all cores for PDF rendering
  int optimal = static_cast<int>(hw_threads);
  return std::min(optimal, 16);  // Cap at 16 to avoid excessive overhead
}

FPDF_EXPORT int FPDF_CALLCONV
FPDF_GetOptimalWorkerCountForDocument(FPDF_DOCUMENT document) {
  // Validate document handle
  if (!document) {
    return 1;  // Single-threaded fallback
  }

  // Get document characteristics
  int page_count = FPDF_GetPageCount(document);
  unsigned int hw_threads = std::thread::hardware_concurrency();

  // Fallback if hardware detection fails
  if (hw_threads == 0) {
    hw_threads = 4;
  }

  int hardware_threads = static_cast<int>(hw_threads);

  // CRITICAL INSIGHT: Content type matters MORE than page count!
  //
  // 15% SAMPLE BENCHMARK (18 PDFs, 324 tests, 2025-10-15) REVISED STRATEGY:
  // Previous assumption "TEXT_HEAVY needs 1 worker" was WRONG!
  //
  // New empirical data shows TEXT_HEAVY PDFs scale VERY WELL with parallelism:
  // - 165-page TEXT_HEAVY: 16w = 1923 pps, 1w = 176 pps (10.9x speedup!)
  // - 85-page TEXT_HEAVY: 16w = 1910 pps, 4w = 201 pps (9.5x speedup!)
  // - 821-page TEXT_HEAVY: 4w = 1135 pps, 1w = 379 pps (3.0x speedup!)
  // - 25-page TEXT_HEAVY: 16w = 734 pps, 4w = 239 pps (3.1x speedup!)
  //
  // IMAGE_HEAVY PDFs also scale well with parallelism:
  // - 159-page IMAGE_HEAVY: optimal = 1554 pps, 8w = 675 pps
  //
  // Content-type detection: Use file size per page as proxy
  // - TEXT_HEAVY: < 15KB/page (fonts, vectors, minimal images)
  // - MIXED_CONTENT: 15-100KB/page
  // - IMAGE_HEAVY: > 100KB/page (scanned documents, photo-heavy)
  // - JPG_WRAPPER: 500KB-5MB/page (scanned PDFs with one JPG per page)

  // Get file size for content-type detection
  CPDF_Document* cpdf_doc = CPDFDocumentFromFPDFDocument(document);
  int64_t file_size = 0;
  if (cpdf_doc && cpdf_doc->GetParser()) {
    file_size = cpdf_doc->GetParser()->GetDocumentSize();
  }

  // Calculate size per page (bytes per page)
  int64_t size_per_page =
      (page_count > 0 && file_size > 0) ? file_size / page_count : 0;

  // Very small documents: thread pool overhead exceeds benefit
  if (page_count < 4) {
    return 1;
  }

  // Content-type specific optimization strategy:

  // TEXT_HEAVY PDFs: < 15KB/page
  // NEW STRATEGY (2025-10-15): TEXT_HEAVY PDFs scale VERY WELL with
  // parallelism! Previous "1 worker optimal" assumption was based on incorrect
  // data.
  //
  // Empirical data shows aggressive parallelism works great:
  // - Small (25 pages): 16w = 734 pps, 1w = 202 pps (3.6x speedup)
  // - Medium (85 pages): 16w = 1910 pps, 4w = 201 pps (9.5x speedup)
  // - Large (165 pages): 16w = 1923 pps, 1w = 176 pps (10.9x speedup)
  // - Very Large (821 pages): 4w = 1135 pps, 1w = 379 pps (3.0x speedup)
  //
  // Strategy: Use max parallelism for small/medium, moderate for very large
  // CRITICAL: Never use more workers than pages (user feedback 2025-10-15)
  if (size_per_page > 0 && size_per_page < 15000) {
    if (page_count < 400) {
      // Small-to-large text documents: aggressive parallelism works great
      return std::min({page_count, 16, hardware_threads});
    } else {
      // Very large text documents: moderate parallelism to reduce memory
      // pressure 821-page benchmark: 4w = 1135 pps, 16w = 1126 pps (similar, 4w
      // saves memory)
      return std::min({page_count, 4, hardware_threads});
    }
  }

  // IMAGE_HEAVY PDFs: > 100KB/page
  // Image decompression has minimal shared state, scales well with parallelism
  if (size_per_page >= 100000) {
    if (page_count < 150) {
      // Small-medium image documents
      return std::min({page_count, 4, hardware_threads});
    } else if (page_count < 300) {
      // Medium-large image documents: aggressive parallelism works well
      // Benchmark: 291-page IMAGE_HEAVY → 16 workers = 635.6 pps (4.79x)
      return std::min({page_count, 16, hardware_threads});
    } else {
      // Very large image documents: reduce slightly to avoid memory/IO
      // contention
      return std::min({page_count, 8, hardware_threads});
    }
  }

  // MIXED_CONTENT PDFs: 15-100KB/page OR unknown content type (fallback)
  // Moderate parallelism balances contention vs throughput
  if (page_count < 150) {
    // Small-medium mixed documents
    // Benchmark: 124-page IMAGE_HEAVY → 4 workers = 268.0 pps (2.23x)
    // Benchmark: 134-page MIXED → 4 workers = 430.9 pps (2.03x)
    return std::min({page_count, 4, hardware_threads});
  } else if (page_count < 300) {
    // Medium-large mixed documents
    return std::min({page_count, 8, hardware_threads});
  } else if (page_count < 600) {
    // Large mixed documents
    // Benchmark: 569-page MIXED → 4 workers = 317.4 pps (1.74x)
    return std::min({page_count, 4, hardware_threads});
  } else {
    // Very large mixed documents: conservative approach
    return std::min({page_count, 4, hardware_threads});
  }
}

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
                         void* user_data) {
  // Validate parameters
  if (!document || !callback || page_count <= 0 || width <= 0 || height <= 0) {
    return false;
  }

  // Get total page count
  int total_pages = FPDF_GetPageCount(document);
  if (start_page < 0 || start_page >= total_pages) {
    return false;
  }

  // Clamp page_count to available pages
  page_count = std::min(page_count, total_pages - start_page);
  if (page_count <= 0) {
    return false;
  }

  // Determine thread count for parallel rendering
  // If not specified, use document-aware adaptive selection
  int thread_count = options && options->worker_count > 0
                         ? options->worker_count
                         : FPDF_GetOptimalWorkerCountForDocument(document);

  // SESSION 80: 2-worker cap RE-ENABLED - upstream PDFium bug workaround
  //
  // Root cause: PDFium's resource management assumes single-threaded sequential
  // page access. Parallel page loading breaks shared resource lifetime:
  //   - Pages share fonts/images/color spaces from document dictionaries
  //   - Closing page X frees shared resources that page Y still needs
  //   - Results in UAF during page destruction (even on main thread)
  //
  // Session 79 finding: Deferred destruction prevents CONCURRENT destruction
  // but does NOT fix SHARED RESOURCE management. Closing pages in any order
  // (forward/reverse/deferred) causes UAF with >2 workers.
  //
  // Session 80 finding: Cannot leak pages - FPDF_CloseDocument() only frees
  // pages opened via public API, not internal worker-loaded pages. Leak grows
  // ~4MB per render pass (tested with 2-page PDF).
  //
  // User directive: No worker caps. Fix bugs properly instead of limiting
  // parallelism. User has 12 cores - use them.

  // For single-threaded case, just render sequentially
  // This avoids thread pool overhead for small documents
  if (thread_count == 1 || page_count == 1) {
    // N=31: Get output format from options (0=BGRx for backward compatibility)
    int output_format = options ? options->output_format : 0;
    int fpdf_format = ParallelFormatToFPDFFormat(output_format);

    for (int i = 0; i < page_count; ++i) {
      int page_index = start_page + i;

      FPDF_PAGE page = FPDF_LoadPage(document, page_index);
      if (!page) {
        callback(page_index, nullptr, user_data, false);
        continue;
      }

      // N=31: Create bitmap with specified format
      FPDF_BITMAP bitmap = FPDFBitmap_CreateEx(width, height, fpdf_format, nullptr, 0);
      if (bitmap) {
        // N=202: Check page transparency to match single-threaded fill logic
        int has_transparency = FPDFPage_HasTransparency(page);
        uint32_t fill_color = has_transparency ? 0x00000000 : 0xFFFFFFFF;
        FPDFBitmap_FillRect(bitmap, 0, 0, width, height, fill_color);

        // N=31: Add FPDF_GRAYSCALE flag for grayscale output format
        int render_flags = flags;
        if (output_format == FPDF_PARALLEL_FORMAT_GRAY) {
          render_flags |= FPDF_GRAYSCALE;
        }
        FPDF_RenderPageBitmap(bitmap, page, 0, 0, width, height, rotate, render_flags);

        // N=117: Render form fields to match V2 API behavior
        if (options && options->form_handle) {
          FPDF_FFLDraw((FPDF_FORMHANDLE)options->form_handle, bitmap, page,
                       0, 0, width, height, rotate, render_flags);
        }

        callback(page_index, bitmap, user_data, true);
      } else {
        callback(page_index, nullptr, user_data, false);
      }

      FPDF_ClosePage(page);
    }
    return true;
  }

  // N=119: REMOVED SEQUENTIAL PRE-FETCH (matching V2 API behavior)
  //
  // Previous behavior: Pre-fetched all page dictionaries sequentially before
  // parallel rendering. This was intended to reduce lock contention by
  // populating the page_list_ cache.
  //
  // Problem: V2 API analysis showed pre-fetch destroys parallelism:
  //   WITH pre-fetch: 1.04x scaling (basically no parallelism)
  //   WITHOUT pre-fetch: 4.28x scaling (excellent parallelism)
  //
  // Root cause: Sequential FPDF_LoadPage() does most work (page parsing,
  // resource loading) before parallel phase, leaving only bitmap rendering.
  //
  // Solution: Workers load pages on-demand in parallel. Lock contention in
  // page tree traversal is far cheaper than sequential loading for 100+ pages.

  // SESSION 79: Create page handle collection for deferred destruction
  // Workers will store loaded pages here instead of closing them immediately
  PageHandleCollection page_collection;

  // Use global persistent thread pool
  // This eliminates 8-14% overhead from creating/destroying threads
  GlobalThreadPool* pool = GetOrCreateThreadPool();
  pool->EnsureWorkerCount(thread_count);

  // N=118: Set backpressure limit if specified (prevents memory spike on large docs)
  int max_queue = options && options->max_queue_size > 0
                      ? options->max_queue_size
                      : (page_count > 256 ? 256 : 0);  // Auto-enable for large docs
  pool->SetMaxQueueDepth(max_queue);

  // Enqueue all rendering tasks
  for (int i = 0; i < page_count; ++i) {
    RenderTask task;
    task.document = document;
    task.page_index = start_page + i;
    task.width = width;
    task.height = height;
    task.rotate = rotate;
    task.flags = flags;
    task.callback = callback;
    task.user_data = reinterpret_cast<uintptr_t>(user_data);
    task.page_collection = &page_collection;  // SESSION 79: Pass collection
    // N=122: Pass form_handle to match V2 API and single-threaded path
    task.form_handle = options ? (FPDF_FORMHANDLE)options->form_handle : nullptr;
    // N=31: Pass output format (0=BGRx default for backward compatibility)
    task.output_format = options ? options->output_format : 0;

    pool->EnqueueTask(std::move(task));
  }

  // Wait for all tasks to complete
  pool->WaitForCompletion();

  // FIX #3 (N=30): Removed 100ms sleep - unnecessary with precise outstanding_tasks_ counter
  // The old sleep masked races from unreliable size_approx(). With FIX #1's precise
  // counter, WaitForCompletion() guarantees all tasks are fully processed before returning.

  // FIX #2 (N=30): Close all pages to prevent memory leaks
  // Pages must be closed - FPDF_CloseDocument doesn't free worker-loaded pages.
  // N=152 Fix #15: Close under document lock for thread safety
  page_collection.CloseAllUnderDocLock(document);

  // N=152 Fix #9: Clear bitmap pools after job completes
  // This prevents memory accumulation in long-running applications
  pool->SignalClearPools();

  return true;
}

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
                           void* user_data) {
  // Validate parameters
  // N=118: width=0 and height=0 allowed if options->dpi > 0 (auto-detect mode)
  bool auto_detect = (width == 0 && height == 0 && options && options->dpi > 0);
  if (!document || !callback || page_count <= 0) {
    return false;
  }
  if (!auto_detect && (width <= 0 || height <= 0)) {
    return false;
  }

  // Get total page count
  int total_pages = FPDF_GetPageCount(document);
  if (start_page < 0 || start_page >= total_pages) {
    return false;
  }

  // Clamp page_count to available pages
  page_count = std::min(page_count, total_pages - start_page);
  if (page_count <= 0) {
    return false;
  }

  // Determine thread count for parallel rendering
  int thread_count = options && options->worker_count > 0
                         ? options->worker_count
                         : FPDF_GetOptimalWorkerCountForDocument(document);

  // User directive: No worker caps. Fix bugs properly instead of limiting
  // parallelism. User has 12 cores - use them.

  // For single-threaded case, render sequentially with bitmap pooling
  if (thread_count == 1 || page_count == 1) {
    BitmapPool* pool = GetThreadBitmapPool();
    double dpi = options ? options->dpi : 0;
    // N=31: Get output format from options (0=BGRx for backward compatibility)
    int output_format = options ? options->output_format : 0;

    for (int i = 0; i < page_count; ++i) {
      int page_index = start_page + i;

      FPDF_PAGE page = FPDF_LoadPage(document, page_index);
      if (!page) {
        callback(page_index, nullptr, 0, 0, 0, user_data, false);
        continue;
      }

      // N=118: Calculate dimensions from DPI if auto-detect mode
      int actual_width = width;
      int actual_height = height;
      if (auto_detect) {
        double page_width_pts = FPDF_GetPageWidthF(page);
        double page_height_pts = FPDF_GetPageHeightF(page);
        double scale = floor((dpi / 72.0) * 1000000.0) / 1000000.0;
        actual_width = static_cast<int>(page_width_pts * scale);
        actual_height = static_cast<int>(page_height_pts * scale);
        if (actual_width < 1 || actual_height < 1) {
          FPDF_ClosePage(page);
          callback(page_index, nullptr, 0, 0, 0, user_data, false);
          continue;
        }
      }

      // N=31: Acquire bitmap from pool with specified format
      FPDF_BITMAP bitmap = pool->Acquire(actual_width, actual_height, output_format);
      if (!bitmap) {
        FPDF_ClosePage(page);
        callback(page_index, nullptr, 0, 0, 0, user_data, false);
        continue;
      }

      // N=202: Check page transparency to match single-threaded fill logic
      int has_transparency = FPDFPage_HasTransparency(page);
      uint32_t fill_color = has_transparency ? 0x00000000 : 0xFFFFFFFF;
      FPDFBitmap_FillRect(bitmap, 0, 0, actual_width, actual_height, fill_color);

      // N=31: Add FPDF_GRAYSCALE flag for grayscale output format
      int render_flags = flags;
      if (output_format == FPDF_PARALLEL_FORMAT_GRAY) {
        render_flags |= FPDF_GRAYSCALE;
      }
      FPDF_RenderPageBitmap(bitmap, page, 0, 0, actual_width, actual_height, rotate, render_flags);

      // N=200: Render form fields to match single-threaded path
      if (options && options->form_handle) {
        FPDF_FFLDraw((FPDF_FORMHANDLE)options->form_handle, bitmap, page,
                     0, 0, actual_width, actual_height, rotate, render_flags);
      }

      // Get buffer for callback
      const void* buffer = FPDFBitmap_GetBuffer(bitmap);
      int stride = FPDFBitmap_GetStride(bitmap);

      callback(page_index, buffer, actual_width, actual_height, stride, user_data, true);

      FPDF_ClosePage(page);
      pool->Release(bitmap);  // Return to pool
    }
    return true;
  }

  // SESSION 51: REMOVED SEQUENTIAL PRE-FETCH (was causing 0% parallelism)
  //
  // ANALYSIS: The pre-fetch logic was intended to reduce lock contention by
  // 4-5% by pre-populating page_list_ cache before workers start. However,
  // empirical testing showed it completely destroyed parallelism:
  //
  // WITH pre-fetch:
  //   - 1 worker: 562.4 pps
  //   - 8 workers: 585.0 pps
  //   - Scaling: 1.04x (NO parallelism)
  //   - Pre-fetch took 40% of total time (115ms of 293ms)
  //
  // WITHOUT pre-fetch:
  //   - 1 worker: 162.3 pps
  //   - 8 workers: 694.6 pps
  //   - Scaling: 4.28x (excellent parallelism)
  //
  // ROOT CAUSE: Sequential FPDF_LoadPage() calls for all pages BEFORE parallel
  // rendering starts. This does most of the work (page parsing, resource
  // loading) sequentially, leaving only bitmap rendering for parallelization.
  //
  // SOLUTION: Remove pre-fetch entirely. Workers load pages on-demand in
  // parallel. Any lock contention in page tree traversal is far cheaper than
  // sequential page loading for 100+ pages.

  // SESSION 79: Create page handle collection for deferred destruction
  // Workers will store loaded pages here instead of closing them immediately
  PageHandleCollection page_collection;

  // Use global persistent thread pool with bitmap pooling
  GlobalThreadPool* pool = GetOrCreateThreadPool();
  pool->EnsureWorkerCount(thread_count);

  // N=118: Set backpressure limit if specified (prevents memory spike on large docs)
  // Default 256 tasks max if not specified (0 in options means use default)
  int max_queue = options && options->max_queue_size > 0
                      ? options->max_queue_size
                      : (page_count > 256 ? 256 : 0);  // Auto-enable for large docs
  pool->SetMaxQueueDepth(max_queue);

  // Build all tasks first, then batch submit (more efficient)
  std::vector<RenderTaskV2> tasks;
  tasks.reserve(page_count);
  double dpi = options ? options->dpi : 0;
  int output_format = options ? options->output_format : 0;  // N=31
  for (int i = 0; i < page_count; ++i) {
    RenderTaskV2 task;
    task.document = document;
    task.page_index = start_page + i;
    task.width = width;
    task.height = height;
    task.rotate = rotate;
    task.flags = flags;
    task.callback_v2 = callback;
    task.user_data = reinterpret_cast<uintptr_t>(user_data);
    task.page_collection = &page_collection;  // SESSION 79: Pass collection
    task.form_handle = options ? (FPDF_FORMHANDLE)options->form_handle : nullptr;  // N=200
    task.dpi = dpi;  // N=118: For per-page dimension calculation
    task.output_format = output_format;  // N=31: Output pixel format
    tasks.push_back(std::move(task));
  }

  // Batch enqueue all tasks at once (lock-free, very efficient)
  pool->EnqueueTasksV2Batch(tasks);

  // Wait for all tasks to complete
  pool->WaitForCompletion();

  // FIX #3 (N=30): Removed 100ms sleep - unnecessary with precise outstanding_tasks_ counter
  // The old sleep masked races from unreliable size_approx(). With FIX #1's precise
  // counter, WaitForCompletion() guarantees all tasks are fully processed before returning.

  // FIX #2 (N=30): Close all pages to prevent memory leaks
  // Pages must be closed - FPDF_CloseDocument doesn't free worker-loaded pages.
  // N=152 Fix #15: Close under document lock for thread safety
  page_collection.CloseAllUnderDocLock(document);

  // N=152 Fix #9: Clear bitmap pools after job completes
  // This prevents memory accumulation in long-running applications
  pool->SignalClearPools();

  return true;
}

FPDF_EXPORT void FPDF_CALLCONV FPDF_DestroyThreadPool() {
  // Clean up global thread pool and all pooled bitmaps
  // This must be called before FPDF_DestroyLibrary() to ensure proper
  // cleanup order and avoid crashes from static destructor ordering issues
  DestroyThreadPool();
}
