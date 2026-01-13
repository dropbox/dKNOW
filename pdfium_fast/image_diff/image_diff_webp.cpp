// Copyright 2025 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "testing/image_diff/image_diff_webp.h"

#include <stdint.h>
#include <string.h>

#include <vector>

#include "core/fxcrt/check_op.h"
#include "core/fxcrt/fx_memcpy_wrappers.h"

#ifdef USE_SYSTEM_LIBWEBP
#include <webp/encode.h>
#else
#include "third_party/libwebp/src/webp/encode.h"
#endif

namespace image_diff_webp {

namespace {

// Converts BGRA->RGBA for WebP encoding
void ConvertBGRAtoRGBA(const uint8_t* bgra, int pixel_width, uint8_t* rgba) {
  for (int x = 0; x < pixel_width; x++) {
    const uint8_t* pixel_in = &bgra[x * 4];
    uint8_t* pixel_out = &rgba[x * 4];
    pixel_out[0] = pixel_in[2];  // B -> R
    pixel_out[1] = pixel_in[1];  // G -> G
    pixel_out[2] = pixel_in[0];  // R -> B
    pixel_out[3] = pixel_in[3];  // A -> A
  }
}

// Converts RGB to RGBA by adding opaque alpha channel
void ConvertRGBtoRGBA(const uint8_t* rgb, int pixel_width, uint8_t* rgba) {
  for (int x = 0; x < pixel_width; x++) {
    const uint8_t* pixel_in = &rgb[x * 3];
    uint8_t* pixel_out = &rgba[x * 4];
    FXSYS_memcpy(pixel_out, pixel_in, 3);
    pixel_out[3] = 0xff;  // Opaque alpha
  }
}

// Core WebP encoding function
std::vector<uint8_t> EncodeWebPInternal(const uint8_t* rgba_data,
                                         int width,
                                         int height,
                                         int stride) {
  std::vector<uint8_t> output;

  if (!rgba_data || width <= 0 || height <= 0 || stride < width * 4) {
    return output;  // Return empty vector on invalid input
  }

  // Configure WebP encoder for lossless compression
  WebPConfig config;
  if (!WebPConfigInit(&config)) {
    return output;
  }

  config.lossless = 1;   // Lossless mode for pixel-perfect correctness
  config.quality = 100;  // Maximum quality
  config.method = 0;     // Fastest encoding (0=fast, 6=slow but smaller files)

  if (!WebPValidateConfig(&config)) {
    return output;
  }

  // Initialize WebP picture
  WebPPicture picture;
  if (!WebPPictureInit(&picture)) {
    return output;
  }

  picture.width = width;
  picture.height = height;
  picture.use_argb = 1;  // Use ARGB format for lossless

  // Import RGBA data into WebP picture
  if (!WebPPictureImportRGBA(&picture, rgba_data, stride)) {
    WebPPictureFree(&picture);
    return output;
  }

  // Setup memory writer for output
  WebPMemoryWriter writer;
  WebPMemoryWriterInit(&writer);
  picture.writer = WebPMemoryWrite;
  picture.custom_ptr = &writer;

  // Encode to WebP
  bool encode_success = WebPEncode(&config, &picture);

  if (encode_success && writer.size > 0) {
    output.assign(writer.mem, writer.mem + writer.size);
  }

  // Cleanup
  WebPMemoryWriterClear(&writer);
  WebPPictureFree(&picture);

  return output;
}

}  // namespace

std::vector<uint8_t> EncodeRGBAWebP(pdfium::span<const uint8_t> input,
                                    int width,
                                    int height,
                                    int row_byte_width) {
  if (input.empty() || width <= 0 || height <= 0) {
    return std::vector<uint8_t>();
  }

  CHECK_GE(row_byte_width, width * 4);

  // RGBA data can be encoded directly
  return EncodeWebPInternal(input.data(), width, height, row_byte_width);
}

std::vector<uint8_t> EncodeBGRAWebP(pdfium::span<const uint8_t> input,
                                    int width,
                                    int height,
                                    int row_byte_width) {
  if (input.empty() || width <= 0 || height <= 0) {
    return std::vector<uint8_t>();
  }

  CHECK_GE(row_byte_width, width * 4);

  // Convert BGRA to RGBA
  std::vector<uint8_t> rgba_data(width * height * 4);
  const uint8_t* src = input.data();
  uint8_t* dst = rgba_data.data();

  for (int y = 0; y < height; y++) {
    ConvertBGRAtoRGBA(src, width, dst);
    src += row_byte_width;
    dst += width * 4;
  }

  return EncodeWebPInternal(rgba_data.data(), width, height, width * 4);
}

std::vector<uint8_t> EncodeRGBWebP(pdfium::span<const uint8_t> input,
                                   int width,
                                   int height,
                                   int row_byte_width) {
  if (input.empty() || width <= 0 || height <= 0) {
    return std::vector<uint8_t>();
  }

  CHECK_GE(row_byte_width, width * 3);

  // Convert RGB to RGBA (add opaque alpha)
  std::vector<uint8_t> rgba_data(width * height * 4);
  const uint8_t* src = input.data();
  uint8_t* dst = rgba_data.data();

  for (int y = 0; y < height; y++) {
    ConvertRGBtoRGBA(src, width, dst);
    src += row_byte_width;
    dst += width * 4;
  }

  return EncodeWebPInternal(rgba_data.data(), width, height, width * 4);
}

}  // namespace image_diff_webp
