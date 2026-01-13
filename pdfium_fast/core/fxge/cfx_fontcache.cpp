// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fxge/cfx_fontcache.h"

#include "core/fxge/cfx_font.h"
#include "core/fxge/cfx_glyphcache.h"
#include "core/fxge/fx_font.h"

CFX_FontCache::CFX_FontCache() = default;

CFX_FontCache::~CFX_FontCache() = default;

RetainPtr<CFX_GlyphCache> CFX_FontCache::GetGlyphCache(const CFX_Font* font) {
  RetainPtr<CFX_Face> face = font->GetFace();
  const bool bExternal = !face;

  // LOCK-FREE FAST PATH: After pre-warming, all fonts are cached
  // C++ §26.2.5.1 guarantees map::find() is thread-safe for concurrent reads
  if (read_only_mode_.load(std::memory_order_acquire)) {
    auto& map = bExternal ? ext_glyph_cache_map_ : glyph_cache_map_;
    auto it = map.find(face.Get());
    if (it != map.end() && it->second) {
      return pdfium::WrapRetain(it->second.Get());
    }
    return nullptr;  // Cache miss in read-only mode
  }

  // PARALLEL RENDERING: Check cache with shared lock (concurrent reads)
  {
    std::shared_lock<std::shared_timed_mutex> read_lock(font_cache_mutex_);
    auto& map = bExternal ? ext_glyph_cache_map_ : glyph_cache_map_;
    auto it = map.find(face.Get());
    if (it != map.end() && it->second) {
      return pdfium::WrapRetain(it->second.Get());
    }
  }  // Release shared lock

  // Cache miss: Upgrade to unique lock for modification
  std::unique_lock<std::shared_timed_mutex> write_lock(font_cache_mutex_);

  // Double-check: Another thread may have added it while we were waiting
  auto& map = bExternal ? ext_glyph_cache_map_ : glyph_cache_map_;
  auto it = map.find(face.Get());
  if (it != map.end() && it->second) {
    return pdfium::WrapRetain(it->second.Get());
  }

  auto new_cache = pdfium::MakeRetain<CFX_GlyphCache>(face);
  map[face.Get()].Reset(new_cache.Get());
  return new_cache;
}

#if defined(PDF_USE_SKIA)
CFX_TypeFace* CFX_FontCache::GetDeviceCache(const CFX_Font* font) {
  return GetGlyphCache(font)->GetDeviceCache(font);
}
#endif

void CFX_FontCache::SetGlyphCachesReadOnlyMode(bool enabled) {
  // PARALLEL TEXT EXTRACTION: Enable read-only mode for all glyph caches
  // After pre-warming, all glyphs are cached → lock-free reads
  std::unique_lock<std::shared_timed_mutex> lock(font_cache_mutex_);

  for (const auto& entry : glyph_cache_map_) {
    if (entry.second) {
      entry.second->SetReadOnlyMode(enabled);
    }
  }

  for (const auto& entry : ext_glyph_cache_map_) {
    if (entry.second) {
      entry.second->SetReadOnlyMode(enabled);
    }
  }
}
