// Micro-benchmark for BGRA→RGB conversion
// Measures scalar vs NEON performance

#include <chrono>
#include <iostream>
#include <vector>
#include <cstring>

#ifdef __ARM_NEON
#include <arm_neon.h>
#endif

// Scalar implementation (current production code)
void bgra_to_rgb_scalar(const unsigned char* src, unsigned char* dst,
                        int width, int height, int stride) {
    for (int y = 0; y < height; ++y) {
        const unsigned char* src_row = src + (y * stride);
        unsigned char* dst_row = dst + (y * width * 3);
        for (int x = 0; x < width; ++x) {
            dst_row[x * 3 + 0] = src_row[x * 4 + 2];  // R
            dst_row[x * 3 + 1] = src_row[x * 4 + 1];  // G
            dst_row[x * 3 + 2] = src_row[x * 4 + 0];  // B
        }
    }
}

#ifdef __ARM_NEON
// NEON implementation (16 pixels at a time)
void bgra_to_rgb_neon(const unsigned char* src, unsigned char* dst,
                      int width, int height, int stride) {
    for (int y = 0; y < height; ++y) {
        const unsigned char* src_row = src + (y * stride);
        unsigned char* dst_row = dst + (y * width * 3);

        int x = 0;
        // Process 16 pixels at a time (64 bytes BGRA → 48 bytes RGB)
        for (; x + 16 <= width; x += 16) {
            // Load 16 BGRA pixels (4 * 16 = 64 bytes)
            uint8x16x4_t bgra = vld4q_u8(src_row + x * 4);

            // Shuffle to RGB: bgra.val[2] = R, bgra.val[1] = G, bgra.val[0] = B
            uint8x16x3_t rgb;
            rgb.val[0] = bgra.val[2];  // R
            rgb.val[1] = bgra.val[1];  // G
            rgb.val[2] = bgra.val[0];  // B

            // Store 16 RGB pixels (3 * 16 = 48 bytes)
            vst3q_u8(dst_row + x * 3, rgb);
        }

        // Handle remaining pixels with scalar code
        for (; x < width; ++x) {
            dst_row[x * 3 + 0] = src_row[x * 4 + 2];  // R
            dst_row[x * 3 + 1] = src_row[x * 4 + 1];  // G
            dst_row[x * 3 + 2] = src_row[x * 4 + 0];  // B
        }
    }
}
#endif

int main(int argc, char** argv) {
    // Test with typical 300 DPI page dimensions
    int width = 2550;   // 8.5" * 300 DPI
    int height = 3300;  // 11" * 300 DPI
    int stride = width * 4;

    if (argc > 1) {
        width = std::atoi(argv[1]);
    }
    if (argc > 2) {
        height = std::atoi(argv[2]);
    }

    std::cout << "Benchmark: BGRA→RGB conversion\n";
    std::cout << "Resolution: " << width << "x" << height << " ("
              << (width * height / 1000000.0) << " MP)\n";
    std::cout << "Input: " << (stride * height / 1024 / 1024) << " MB BGRA\n";
    std::cout << "Output: " << (width * 3 * height / 1024 / 1024) << " MB RGB\n\n";

    // Allocate buffers
    std::vector<unsigned char> src_buf(stride * height);
    std::vector<unsigned char> dst_buf(width * 3 * height);

    // Fill with test data
    for (size_t i = 0; i < src_buf.size(); ++i) {
        src_buf[i] = i % 256;
    }

    const int iterations = 100;

    // Benchmark scalar
    auto start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < iterations; ++i) {
        bgra_to_rgb_scalar(src_buf.data(), dst_buf.data(), width, height, stride);
    }
    auto end = std::chrono::high_resolution_clock::now();
    auto scalar_us = std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();
    double scalar_ms = scalar_us / 1000.0 / iterations;

    std::cout << "Scalar:  " << scalar_ms << " ms/conversion ("
              << (width * height / scalar_ms / 1000.0) << " MP/s)\n";

#ifdef __ARM_NEON
    // Benchmark NEON
    start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < iterations; ++i) {
        bgra_to_rgb_neon(src_buf.data(), dst_buf.data(), width, height, stride);
    }
    end = std::chrono::high_resolution_clock::now();
    auto neon_us = std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();
    double neon_ms = neon_us / 1000.0 / iterations;

    std::cout << "NEON:    " << neon_ms << " ms/conversion ("
              << (width * height / neon_ms / 1000.0) << " MP/s)\n";
    std::cout << "Speedup: " << (scalar_ms / neon_ms) << "x\n";

    // Verify correctness
    std::vector<unsigned char> scalar_result(width * 3 * height);
    std::vector<unsigned char> neon_result(width * 3 * height);

    bgra_to_rgb_scalar(src_buf.data(), scalar_result.data(), width, height, stride);
    bgra_to_rgb_neon(src_buf.data(), neon_result.data(), width, height, stride);

    if (std::memcmp(scalar_result.data(), neon_result.data(), scalar_result.size()) == 0) {
        std::cout << "✓ Correctness verified (NEON matches scalar)\n";
    } else {
        std::cout << "✗ ERROR: NEON output differs from scalar!\n";
        return 1;
    }
#else
    std::cout << "NEON:    Not available (x86_64 or non-ARM platform)\n";
#endif

    return 0;
}
