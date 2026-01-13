// Copyright 2025 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TESTING_IMAGE_DIFF_IMAGE_DIFF_WEBP_H_
#define TESTING_IMAGE_DIFF_IMAGE_DIFF_WEBP_H_

#include <stdlib.h>  // for size_t.

#include <vector>

#include "core/fxcrt/span.h"

namespace image_diff_webp {

// Encode an RGBA pixel array into a WebP (lossless mode).
// Returns empty vector on failure.
std::vector<uint8_t> EncodeRGBAWebP(pdfium::span<const uint8_t> input,
                                    int width,
                                    int height,
                                    int row_byte_width);

// Encode a BGRA pixel array into a WebP (lossless mode).
// Returns empty vector on failure.
std::vector<uint8_t> EncodeBGRAWebP(pdfium::span<const uint8_t> input,
                                    int width,
                                    int height,
                                    int row_byte_width);

// Encode an RGB pixel array into a WebP (lossless mode).
// Returns empty vector on failure.
std::vector<uint8_t> EncodeRGBWebP(pdfium::span<const uint8_t> input,
                                   int width,
                                   int height,
                                   int row_byte_width);

}  // namespace image_diff_webp

#endif  // TESTING_IMAGE_DIFF_IMAGE_DIFF_WEBP_H_
