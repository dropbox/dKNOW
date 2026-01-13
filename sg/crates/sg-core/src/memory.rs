//! Memory profiling utilities for tracking allocations in hot paths.
//!
//! Provides cross-platform memory measurement for benchmarking and profiling.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counters for allocation tracking
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static ALLOCATION_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Tracking allocator that wraps the system allocator
/// and counts allocations for profiling.
pub struct TrackingAllocator;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let size = layout.size();
            let current = ALLOCATED.fetch_add(size, Ordering::Relaxed) + size;
            ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
            // Update peak if we've exceeded it
            let mut peak = PEAK_ALLOCATED.load(Ordering::Relaxed);
            while current > peak {
                match PEAK_ALLOCATED.compare_exchange_weak(
                    peak,
                    current,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size();
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            if new_size > old_size {
                let diff = new_size - old_size;
                let current = ALLOCATED.fetch_add(diff, Ordering::Relaxed) + diff;
                // Update peak
                let mut peak = PEAK_ALLOCATED.load(Ordering::Relaxed);
                while current > peak {
                    match PEAK_ALLOCATED.compare_exchange_weak(
                        peak,
                        current,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(p) => peak = p,
                    }
                }
            } else {
                let diff = old_size - new_size;
                ALLOCATED.fetch_sub(diff, Ordering::Relaxed);
            }
        }
        new_ptr
    }
}

/// Memory statistics snapshot
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStats {
    /// Current allocated bytes
    pub allocated: usize,
    /// Peak allocated bytes since last reset
    pub peak: usize,
    /// Number of allocations since last reset
    pub allocation_count: usize,
}

impl MemoryStats {
    /// Format as human-readable string
    pub fn display(&self) -> String {
        format!(
            "allocated: {}, peak: {}, allocations: {}",
            format_bytes(self.allocated),
            format_bytes(self.peak),
            self.allocation_count
        )
    }
}

/// Get current memory statistics from the tracking allocator
pub fn get_memory_stats() -> MemoryStats {
    MemoryStats {
        allocated: ALLOCATED.load(Ordering::Relaxed),
        peak: PEAK_ALLOCATED.load(Ordering::Relaxed),
        allocation_count: ALLOCATION_COUNT.load(Ordering::Relaxed),
    }
}

/// Reset peak and allocation count (keeps current allocated as-is)
pub fn reset_memory_stats() {
    let current = ALLOCATED.load(Ordering::Relaxed);
    PEAK_ALLOCATED.store(current, Ordering::Relaxed);
    ALLOCATION_COUNT.store(0, Ordering::Relaxed);
}

/// Get resident set size (RSS) from the operating system.
/// This is the actual physical memory used by the process.
#[cfg(target_os = "macos")]
pub fn get_rss_bytes() -> Option<usize> {
    use std::mem::MaybeUninit;

    #[repr(C)]
    struct TaskBasicInfo {
        virtual_size: u64,
        resident_size: u64,
        resident_size_max: u64,
        user_time: u64,
        system_time: u64,
        policy: i32,
        suspend_count: i32,
    }

    extern "C" {
        fn mach_task_self() -> u32;
        fn task_info(
            target_task: u32,
            flavor: i32,
            task_info_out: *mut TaskBasicInfo,
            task_info_out_count: *mut u32,
        ) -> i32;
    }

    const MACH_TASK_BASIC_INFO: i32 = 20;
    const MACH_TASK_BASIC_INFO_COUNT: u32 =
        (std::mem::size_of::<TaskBasicInfo>() / std::mem::size_of::<u32>()) as u32;

    unsafe {
        let mut info = MaybeUninit::<TaskBasicInfo>::uninit();
        let mut count = MACH_TASK_BASIC_INFO_COUNT;
        let result = task_info(
            mach_task_self(),
            MACH_TASK_BASIC_INFO,
            info.as_mut_ptr(),
            &mut count,
        );
        if result == 0 {
            Some(info.assume_init().resident_size as usize)
        } else {
            None
        }
    }
}

/// Get resident set size (RSS) from the operating system.
#[cfg(target_os = "linux")]
pub fn get_rss_bytes() -> Option<usize> {
    use std::fs;
    // Read from /proc/self/statm - second field is RSS in pages
    let statm = fs::read_to_string("/proc/self/statm").ok()?;
    let fields: Vec<&str> = statm.split_whitespace().collect();
    let rss_pages: usize = fields.get(1)?.parse().ok()?;
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
    Some(rss_pages * page_size)
}

/// Get resident set size (RSS) from the operating system.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn get_rss_bytes() -> Option<usize> {
    None
}

/// Memory measurement guard for profiling a code section.
/// Records memory delta when dropped.
pub struct MemoryGuard {
    name: String,
    start_allocated: usize,
    start_peak: usize,
    start_count: usize,
    start_rss: Option<usize>,
}

impl MemoryGuard {
    /// Start measuring memory for a named section
    pub fn new(name: impl Into<String>) -> Self {
        let stats = get_memory_stats();
        Self {
            name: name.into(),
            start_allocated: stats.allocated,
            start_peak: stats.peak,
            start_count: stats.allocation_count,
            start_rss: get_rss_bytes(),
        }
    }

    /// Get the memory delta since guard creation
    pub fn delta(&self) -> MemoryDelta {
        let stats = get_memory_stats();
        let end_rss = get_rss_bytes();
        MemoryDelta {
            name: self.name.clone(),
            allocated_delta: stats.allocated as isize - self.start_allocated as isize,
            peak_delta: stats.peak.saturating_sub(self.start_peak),
            allocation_count: stats.allocation_count.saturating_sub(self.start_count),
            rss_delta: match (self.start_rss, end_rss) {
                (Some(start), Some(end)) => Some(end as isize - start as isize),
                _ => None,
            },
        }
    }
}

/// Memory delta from a profiled section
#[derive(Debug, Clone)]
pub struct MemoryDelta {
    /// Name of the profiled section
    pub name: String,
    /// Change in allocated bytes (can be negative if freed)
    pub allocated_delta: isize,
    /// Peak memory increase during the section
    pub peak_delta: usize,
    /// Number of allocations during the section
    pub allocation_count: usize,
    /// Change in RSS (if available)
    pub rss_delta: Option<isize>,
}

impl MemoryDelta {
    /// Format as human-readable string
    pub fn display(&self) -> String {
        let mut parts = vec![
            format!("allocated: {}", format_bytes_signed(self.allocated_delta)),
            format!("peak: +{}", format_bytes(self.peak_delta)),
            format!("allocations: {}", self.allocation_count),
        ];
        if let Some(rss) = self.rss_delta {
            parts.push(format!("rss: {}", format_bytes_signed(rss)));
        }
        format!("[{}] {}", self.name, parts.join(", "))
    }
}

/// Format bytes as human-readable string
pub fn format_bytes(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format signed bytes as human-readable string
pub fn format_bytes_signed(bytes: isize) -> String {
    let sign = if bytes >= 0 { "+" } else { "-" };
    let abs = bytes.unsigned_abs();
    format!("{}{}", sign, format_bytes(abs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_get_rss() {
        // Should return Some on macOS and Linux
        let rss = get_rss_bytes();
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            assert!(rss.is_some(), "RSS should be available on macOS/Linux");
            assert!(rss.unwrap() > 0, "RSS should be positive");
        }
    }

    #[test]
    fn test_memory_guard() {
        let guard = MemoryGuard::new("test_allocation");
        // Allocate some memory
        let _v: Vec<u8> = vec![0; 1024 * 1024]; // 1MB
        let delta = guard.delta();
        // Just verify the guard works - actual tracking depends on global allocator
        assert_eq!(delta.name, "test_allocation");
    }
}
