// Copyright 2025 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXGE_APPLE_FX_APPLE_METAL_H_
#define CORE_FXGE_APPLE_FX_APPLE_METAL_H_

#include <memory>
#include <vector>

#include "core/fxcrt/retain_ptr.h"

class CFX_DIBitmap;

namespace pdfium {
namespace metal {

// Metal GPU acceleration for image rendering on macOS
class MetalRenderer {
 public:
  // Check if Metal is available on this system
  static bool IsAvailable();

  // Get singleton instance
  static MetalRenderer* GetInstance();

  MetalRenderer();
  ~MetalRenderer();

  // Initialize Metal device and command queue
  bool Initialize();

  // Render a bitmap using GPU acceleration
  // Returns true on success, false if GPU rendering failed (caller should fall back to CPU)
  bool RenderBitmap(RetainPtr<CFX_DIBitmap> bitmap,
                    int width,
                    int height,
                    bool apply_antialiasing);

  // Batch render multiple bitmaps (more efficient than individual calls)
  bool RenderBitmapBatch(const std::vector<RetainPtr<CFX_DIBitmap>>& bitmaps,
                         int width,
                         int height,
                         bool apply_antialiasing);

  // Get GPU device info
  const char* GetDeviceName() const;
  size_t GetMaxBufferLength() const;
  bool SupportsFamily(int family) const;

  // Shutdown and release resources
  void Shutdown();

 private:
  class Impl;
  std::unique_ptr<Impl> impl_;

  static MetalRenderer* instance_;
};

}  // namespace metal
}  // namespace pdfium

#endif  // CORE_FXGE_APPLE_FX_APPLE_METAL_H_
