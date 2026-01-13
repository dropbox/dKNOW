// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef CORE_FXGE_CFX_FONTCACHE_H_
#define CORE_FXGE_CFX_FONTCACHE_H_

#include <atomic>
#include <map>
#include <mutex>
#include <shared_mutex>

#include "core/fxcrt/fx_system.h"
#include "core/fxcrt/retain_ptr.h"
#include "core/fxge/cfx_glyphcache.h"

class CFX_Font;

class CFX_FontCache {
 public:
  CFX_FontCache();
  ~CFX_FontCache();

  RetainPtr<CFX_GlyphCache> GetGlyphCache(const CFX_Font* font);
#if defined(PDF_USE_SKIA)
  CFX_TypeFace* GetDeviceCache(const CFX_Font* font);
#endif

  // Enable/disable read-only mode for all cached glyph caches
  // Used for lock-free parallel text extraction after pre-warming
  void SetGlyphCachesReadOnlyMode(bool enabled);

  // Enable/disable read-only mode for font cache itself
  // After pre-warming, all fonts are cached → lock-free reads
  void SetReadOnlyMode(bool enabled) {
    read_only_mode_.store(enabled, std::memory_order_release);
  }

 private:
  // Thread-safety: Protects glyph cache map access for concurrent rendering.
  // Multiple threads may call GetGlyphCache() simultaneously, requiring
  // synchronized map operations to prevent data races.
  // PARALLEL TEXT: Use shared_timed_mutex (C++14) to allow concurrent reads (cache hits)
  // Only writers (cache misses) block each other
  mutable std::shared_timed_mutex font_cache_mutex_;

  // Read-only mode flag for lock-free parallel text extraction
  std::atomic<bool> read_only_mode_{false};

  std::map<CFX_Face*, ObservedPtr<CFX_GlyphCache>> glyph_cache_map_;
  std::map<CFX_Face*, ObservedPtr<CFX_GlyphCache>> ext_glyph_cache_map_;
};

#endif  // CORE_FXGE_CFX_FONTCACHE_H_
