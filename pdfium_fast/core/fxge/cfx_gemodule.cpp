// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fxge/cfx_gemodule.h"

#include <atomic>
#include <mutex>

#include "core/fxcrt/check.h"
#include "core/fxge/cfx_folderfontinfo.h"
#include "core/fxge/cfx_fontcache.h"
#include "core/fxge/cfx_fontmgr.h"

namespace {

// Thread-safe singleton state
// Must be atomic to ensure memory visibility across threads.
std::atomic<CFX_GEModule*> g_pGEModule{nullptr};

// Returns the mutex for protecting CFX_GEModule singleton.
// Using function-local static to avoid global constructor/destructor warnings.
std::mutex& GetGEModuleMutex() {
  static std::mutex mutex;
  return mutex;
}

}  // namespace

CFX_GEModule::CFX_GEModule(const char** pUserFontPaths)
    : platform_(PlatformIface::Create()),
      font_mgr_(std::make_unique<CFX_FontMgr>()),
      font_cache_(std::make_unique<CFX_FontCache>()),
      user_font_paths_(pUserFontPaths) {}

CFX_GEModule::~CFX_GEModule() = default;

// static
void CFX_GEModule::Create(const char** pUserFontPaths) {
  std::lock_guard<std::mutex> lock(GetGEModuleMutex());

  // Double-check pattern: module might have been created by another thread
  // Use relaxed load inside mutex - mutex provides synchronization
  if (g_pGEModule.load(std::memory_order_relaxed)) {
    return;
  }

  CFX_GEModule* new_module = new CFX_GEModule(pUserFontPaths);
  new_module->platform_->Init();

  // Release-store: ensures all writes above are visible to threads that acquire-load
  g_pGEModule.store(new_module, std::memory_order_release);

  new_module->GetFontMgr()->GetBuiltinMapper()->SetSystemFontInfo(
      new_module->platform_->CreateDefaultSystemFontInfo());
}

// static
void CFX_GEModule::Destroy() {
  std::lock_guard<std::mutex> lock(GetGEModuleMutex());

  // Use relaxed load inside mutex - mutex provides synchronization
  CFX_GEModule* module = g_pGEModule.load(std::memory_order_relaxed);
  if (!module) {
    return;
  }

  delete module;
  // Relaxed store inside mutex is sufficient
  g_pGEModule.store(nullptr, std::memory_order_relaxed);
}

// static
CFX_GEModule* CFX_GEModule::Get() {
  // Fast path: no lock needed for read-only access after initialization.
  // Acquire-load ensures we see all writes made by Create() before the release-store.
  CFX_GEModule* module = g_pGEModule.load(std::memory_order_acquire);
  DCHECK(module);
  return module;
}
