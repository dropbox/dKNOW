// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfapi/parser/cpdf_indirect_object_holder.h"

#include <algorithm>
#include <memory>
#include <utility>

#include "core/fpdfapi/parser/cpdf_object.h"
#include "core/fpdfapi/parser/cpdf_parser.h"
#include "core/fxcrt/check.h"

namespace {

const CPDF_Object* FilterInvalidObjNum(const CPDF_Object* obj) {
  return obj && obj->GetObjNum() != CPDF_Object::kInvalidObjNum ? obj : nullptr;
}

}  // namespace

CPDF_IndirectObjectHolder::CPDF_IndirectObjectHolder()
    : byte_string_pool_(std::make_unique<ByteStringPool>()) {}

CPDF_IndirectObjectHolder::~CPDF_IndirectObjectHolder() {
  byte_string_pool_.DeleteObject();  // Make weak.
}

RetainPtr<const CPDF_Object> CPDF_IndirectObjectHolder::GetIndirectObject(
    uint32_t objnum) const {
  return pdfium::WrapRetain(GetIndirectObjectInternal(objnum));
}

RetainPtr<CPDF_Object> CPDF_IndirectObjectHolder::GetMutableIndirectObject(
    uint32_t objnum) {
  return pdfium::WrapRetain(
      const_cast<CPDF_Object*>(GetIndirectObjectInternal(objnum)));
}

const CPDF_Object* CPDF_IndirectObjectHolder::GetIndirectObjectInternal(
    uint32_t objnum) const {
  std::shared_lock<std::shared_mutex> lock(indirect_objs_mutex_);
  auto it = indirect_objs_.find(objnum);
  if (it == indirect_objs_.end()) {
    return nullptr;
  }

  return FilterInvalidObjNum(it->second.Get());
}

RetainPtr<CPDF_Object> CPDF_IndirectObjectHolder::GetOrParseIndirectObject(
    uint32_t objnum) {
  return pdfium::WrapRetain(GetOrParseIndirectObjectInternal(objnum));
}

CPDF_Object* CPDF_IndirectObjectHolder::GetOrParseIndirectObjectInternal(
    uint32_t objnum) {
  if (objnum == 0 || objnum == CPDF_Object::kInvalidObjNum) {
    return nullptr;
  }

  // FAST PATH: Try shared_lock first for cache hits (allows concurrent reads)
  // This is critical for parallel image rendering performance where all threads
  // read objects concurrently.
  {
    std::shared_lock<std::shared_mutex> shared_lock(indirect_objs_mutex_);
    auto it = indirect_objs_.find(objnum);
    if (it != indirect_objs_.end()) {
      // Object found in cache - return it without taking exclusive lock
      return const_cast<CPDF_Object*>(
          FilterInvalidObjNum(it->second.Get()));
    }
    // Object not found - fall through to slow path
  }  // Release shared_lock before taking unique_lock

  // SLOW PATH: Object not cached, need exclusive lock to parse and insert
  std::unique_lock<std::shared_mutex> unique_lock(indirect_objs_mutex_);

  // Double-check after upgrading lock (another thread may have loaded it)
  auto it = indirect_objs_.find(objnum);
  if (it != indirect_objs_.end()) {
    return const_cast<CPDF_Object*>(
        FilterInvalidObjNum(it->second.Get()));
  }

  // Add placeholder to prevent recursive parsing
  auto insert_result = indirect_objs_.insert(std::make_pair(objnum, nullptr));

  // Unlock during slow parsing operation (parsing doesn't need lock)
  unique_lock.unlock();
  RetainPtr<CPDF_Object> pNewObj = ParseIndirectObject(objnum);
  unique_lock.lock();

  if (!pNewObj) {
    indirect_objs_.erase(insert_result.first);
    return nullptr;
  }

  pNewObj->SetObjNum(objnum);
  last_obj_num_ = std::max(last_obj_num_, objnum);

  CPDF_Object* result = pNewObj.Get();
  insert_result.first->second = std::move(pNewObj);
  return result;
}

RetainPtr<CPDF_Object> CPDF_IndirectObjectHolder::ParseIndirectObject(
    uint32_t objnum) {
  return nullptr;
}

uint32_t CPDF_IndirectObjectHolder::AddIndirectObject(
    RetainPtr<CPDF_Object> pObj) {
  CHECK(!pObj->GetObjNum());
  std::unique_lock<std::shared_mutex> lock(indirect_objs_mutex_);
  pObj->SetObjNum(++last_obj_num_);
  indirect_objs_[last_obj_num_] = std::move(pObj);
  return last_obj_num_;
}

bool CPDF_IndirectObjectHolder::ReplaceIndirectObjectIfHigherGeneration(
    uint32_t objnum,
    RetainPtr<CPDF_Object> pObj) {
  DCHECK(objnum);
  if (!pObj || objnum == CPDF_Object::kInvalidObjNum) {
    return false;
  }

  std::unique_lock<std::shared_mutex> lock(indirect_objs_mutex_);
  auto& obj_holder = indirect_objs_[objnum];
  const CPDF_Object* old_object = FilterInvalidObjNum(obj_holder.Get());
  if (old_object && pObj->GetGenNum() <= old_object->GetGenNum()) {
    return false;
  }

  pObj->SetObjNum(objnum);
  obj_holder = std::move(pObj);
  last_obj_num_ = std::max(last_obj_num_, objnum);
  return true;
}

void CPDF_IndirectObjectHolder::DeleteIndirectObject(uint32_t objnum) {
  std::unique_lock<std::shared_mutex> lock(indirect_objs_mutex_);
  auto it = indirect_objs_.find(objnum);
  if (it == indirect_objs_.end() || !FilterInvalidObjNum(it->second.Get())) {
    return;
  }

  indirect_objs_.erase(it);
}
