// Copyright 2025 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/apple/fx_apple_metal.h"

#import <Foundation/Foundation.h>
#import <Metal/Metal.h>

#include <memory>
#include <vector>

#include "core/fxge/dib/cfx_dibitmap.h"

namespace pdfium {
namespace metal {

// Singleton instance
MetalRenderer* MetalRenderer::instance_ = nullptr;

// Private implementation (Pimpl pattern to hide Objective-C++ from C++ headers)
class MetalRenderer::Impl {
 public:
  Impl() : device_(nil), command_queue_(nil), pipeline_state_(nil) {}
  ~Impl() { Shutdown(); }

  bool Initialize() {
    @autoreleasepool {
      // Create Metal device
      device_ = MTLCreateSystemDefaultDevice();
      if (!device_) {
        return false;
      }

      // Create command queue
      command_queue_ = [device_ newCommandQueue];
      if (!command_queue_) {
        return false;
      }

      // Load shader library
      NSError* error = nil;
      NSString* shader_source = GetShaderSource();
      id<MTLLibrary> library = [device_ newLibraryWithSource:shader_source
                                                     options:nil
                                                       error:&error];
      if (!library) {
        NSLog(@"Failed to create shader library: %@", error);
        return false;
      }

      // Get shader functions
      id<MTLFunction> vertex_func = [library newFunctionWithName:@"vertex_main"];
      id<MTLFunction> fragment_func = [library newFunctionWithName:@"fragment_main"];

      if (!vertex_func || !fragment_func) {
        NSLog(@"Failed to load shader functions");
        return false;
      }

      // Create render pipeline
      MTLRenderPipelineDescriptor* pipeline_desc = [[MTLRenderPipelineDescriptor alloc] init];
      pipeline_desc.vertexFunction = vertex_func;
      pipeline_desc.fragmentFunction = fragment_func;
      pipeline_desc.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;

      // Enable blending for alpha compositing
      pipeline_desc.colorAttachments[0].blendingEnabled = YES;
      pipeline_desc.colorAttachments[0].rgbBlendOperation = MTLBlendOperationAdd;
      pipeline_desc.colorAttachments[0].alphaBlendOperation = MTLBlendOperationAdd;
      pipeline_desc.colorAttachments[0].sourceRGBBlendFactor = MTLBlendFactorSourceAlpha;
      pipeline_desc.colorAttachments[0].sourceAlphaBlendFactor = MTLBlendFactorSourceAlpha;
      pipeline_desc.colorAttachments[0].destinationRGBBlendFactor = MTLBlendFactorOneMinusSourceAlpha;
      pipeline_desc.colorAttachments[0].destinationAlphaBlendFactor = MTLBlendFactorOneMinusSourceAlpha;

      pipeline_state_ = [device_ newRenderPipelineStateWithDescriptor:pipeline_desc
                                                                error:&error];
      if (!pipeline_state_) {
        NSLog(@"Failed to create pipeline state: %@", error);
        return false;
      }

      return true;
    }
  }

  bool RenderBitmap(RetainPtr<CFX_DIBitmap> bitmap,
                    int width,
                    int height,
                    bool apply_antialiasing) {
    if (!device_ || !command_queue_ || !pipeline_state_) {
      return false;
    }

    @autoreleasepool {
      // Create texture from bitmap
      id<MTLTexture> texture = CreateTextureFromBitmap(bitmap, width, height);
      if (!texture) {
        return false;
      }

      // Create render target
      id<MTLTexture> render_target = CreateRenderTarget(width, height);
      if (!render_target) {
        return false;
      }

      // Execute rendering
      id<MTLCommandBuffer> command_buffer = [command_queue_ commandBuffer];
      if (!command_buffer) {
        return false;
      }

      MTLRenderPassDescriptor* render_pass = [MTLRenderPassDescriptor renderPassDescriptor];
      render_pass.colorAttachments[0].texture = render_target;
      render_pass.colorAttachments[0].loadAction = MTLLoadActionClear;
      render_pass.colorAttachments[0].clearColor = MTLClearColorMake(1.0, 1.0, 1.0, 1.0);
      render_pass.colorAttachments[0].storeAction = MTLStoreActionStore;

      // Enable MSAA if antialiasing requested
      id<MTLTexture> msaa_texture = nil;
      if (apply_antialiasing) {
        msaa_texture = CreateMSAATexture(width, height, 4);  // 4x MSAA
        if (msaa_texture) {
          render_pass.colorAttachments[0].texture = msaa_texture;
          render_pass.colorAttachments[0].resolveTexture = render_target;
          render_pass.colorAttachments[0].storeAction = MTLStoreActionMultisampleResolve;
        }
      }

      id<MTLRenderCommandEncoder> encoder = [command_buffer renderCommandEncoderWithDescriptor:render_pass];
      if (!encoder) {
        return false;
      }

      [encoder setRenderPipelineState:pipeline_state_];
      [encoder setFragmentTexture:texture atIndex:0];

      // Draw full-screen quad
      DrawFullScreenQuad(encoder);

      [encoder endEncoding];
      [command_buffer commit];
      [command_buffer waitUntilCompleted];

      // Copy result back to bitmap
      return CopyTextureToBitmap(render_target, bitmap);
    }
  }

  bool RenderBitmapBatch(const std::vector<RetainPtr<CFX_DIBitmap>>& bitmaps,
                         int width,
                         int height,
                         bool apply_antialiasing) {
    if (bitmaps.empty()) {
      return true;
    }

    if (!device_ || !command_queue_ || !pipeline_state_) {
      return false;
    }

    // For batch rendering, we submit multiple pages in a single command buffer
    @autoreleasepool {
      id<MTLCommandBuffer> command_buffer = [command_queue_ commandBuffer];
      if (!command_buffer) {
        return false;
      }

      // Store render targets for later readback
      std::vector<id<MTLTexture>> render_targets;
      render_targets.reserve(bitmaps.size());

      // Encode all render passes
      for (size_t i = 0; i < bitmaps.size(); i++) {
        const auto& bitmap = bitmaps[i];
        if (!bitmap) {
          return false;
        }

        // Create texture from bitmap
        id<MTLTexture> texture = CreateTextureFromBitmap(bitmap, width, height);
        if (!texture) {
          return false;
        }

        // Create render target
        id<MTLTexture> render_target = CreateRenderTarget(width, height);
        if (!render_target) {
          return false;
        }
        render_targets.push_back(render_target);

        // Setup render pass
        MTLRenderPassDescriptor* render_pass = [MTLRenderPassDescriptor renderPassDescriptor];
        render_pass.colorAttachments[0].texture = render_target;
        render_pass.colorAttachments[0].loadAction = MTLLoadActionClear;
        render_pass.colorAttachments[0].clearColor = MTLClearColorMake(1.0, 1.0, 1.0, 1.0);
        render_pass.colorAttachments[0].storeAction = MTLStoreActionStore;

        // Enable MSAA if antialiasing requested
        id<MTLTexture> msaa_texture = nil;
        if (apply_antialiasing) {
          msaa_texture = CreateMSAATexture(width, height, 4);
          if (msaa_texture) {
            render_pass.colorAttachments[0].texture = msaa_texture;
            render_pass.colorAttachments[0].resolveTexture = render_target;
            render_pass.colorAttachments[0].storeAction = MTLStoreActionMultisampleResolve;
          }
        }

        // Encode render commands
        id<MTLRenderCommandEncoder> encoder = [command_buffer renderCommandEncoderWithDescriptor:render_pass];
        if (!encoder) {
          return false;
        }

        [encoder setRenderPipelineState:pipeline_state_];
        [encoder setFragmentTexture:texture atIndex:0];
        DrawFullScreenQuad(encoder);
        [encoder endEncoding];
      }

      // Submit all work to GPU at once
      [command_buffer commit];
      [command_buffer waitUntilCompleted];

      // After GPU completes, copy all results back to bitmaps
      for (size_t i = 0; i < bitmaps.size(); i++) {
        if (!CopyTextureToBitmap(render_targets[i], bitmaps[i])) {
          return false;
        }
      }

      return true;
    }
  }

  const char* GetDeviceName() const {
    if (!device_) {
      return "No Metal device";
    }
    return [[device_ name] UTF8String];
  }

  size_t GetMaxBufferLength() const {
    if (!device_) {
      return 0;
    }
    return [device_ maxBufferLength];
  }

  bool SupportsFamily(int family) const {
    if (!device_) {
      return false;
    }
    // Check Metal feature set support
    return [device_ supportsFamily:(MTLGPUFamily)family];
  }

  void Shutdown() {
    pipeline_state_ = nil;
    command_queue_ = nil;
    device_ = nil;
  }

 private:
  id<MTLDevice> device_;
  id<MTLCommandQueue> command_queue_;
  id<MTLRenderPipelineState> pipeline_state_;

  NSString* GetShaderSource() {
    // Inline Metal shader source
    // (In production, this would be loaded from metal_shaders.metal)
    return @R"(
      #include <metal_stdlib>
      using namespace metal;

      struct VertexOut {
        float4 position [[position]];
        float2 texCoord;
      };

      vertex VertexOut vertex_main(uint vid [[vertex_id]]) {
        // Full-screen quad vertices
        const float2 positions[6] = {
          float2(-1.0, -1.0), float2(1.0, -1.0), float2(-1.0, 1.0),
          float2(1.0, -1.0), float2(1.0, 1.0), float2(-1.0, 1.0)
        };
        const float2 texCoords[6] = {
          float2(0.0, 1.0), float2(1.0, 1.0), float2(0.0, 0.0),
          float2(1.0, 1.0), float2(1.0, 0.0), float2(0.0, 0.0)
        };

        VertexOut out;
        out.position = float4(positions[vid], 0.0, 1.0);
        out.texCoord = texCoords[vid];
        return out;
      }

      fragment float4 fragment_main(VertexOut in [[stage_in]],
                                     texture2d<float> tex [[texture(0)]]) {
        constexpr sampler s(coord::normalized, address::clamp_to_edge, filter::linear);
        return tex.sample(s, in.texCoord);
      }
    )";
  }

  id<MTLTexture> CreateTextureFromBitmap(RetainPtr<CFX_DIBitmap> bitmap,
                                          int width,
                                          int height) {
    if (!bitmap || !device_) {
      return nil;
    }

    MTLTextureDescriptor* desc = [MTLTextureDescriptor
        texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                     width:width
                                    height:height
                                 mipmapped:NO];
    desc.usage = MTLTextureUsageShaderRead;

    id<MTLTexture> texture = [device_ newTextureWithDescriptor:desc];
    if (!texture) {
      return nil;
    }

    // Upload bitmap data to texture
    const uint8_t* buffer = bitmap->GetBuffer().data();
    NSUInteger bytes_per_row = bitmap->GetPitch();

    [texture replaceRegion:MTLRegionMake2D(0, 0, width, height)
               mipmapLevel:0
                 withBytes:buffer
               bytesPerRow:bytes_per_row];

    return texture;
  }

  id<MTLTexture> CreateRenderTarget(int width, int height) {
    if (!device_) {
      return nil;
    }

    MTLTextureDescriptor* desc = [MTLTextureDescriptor
        texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                     width:width
                                    height:height
                                 mipmapped:NO];
    desc.usage = MTLTextureUsageRenderTarget | MTLTextureUsageShaderRead;

    return [device_ newTextureWithDescriptor:desc];
  }

  id<MTLTexture> CreateMSAATexture(int width, int height, int sample_count) {
    if (!device_) {
      return nil;
    }

    MTLTextureDescriptor* desc = [MTLTextureDescriptor
        texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                     width:width
                                    height:height
                                 mipmapped:NO];
    desc.textureType = MTLTextureType2DMultisample;
    desc.sampleCount = sample_count;
    desc.usage = MTLTextureUsageRenderTarget;

    return [device_ newTextureWithDescriptor:desc];
  }

  void DrawFullScreenQuad(id<MTLRenderCommandEncoder> encoder) {
    // Draw 6 vertices (2 triangles) for full-screen quad
    [encoder drawPrimitives:MTLPrimitiveTypeTriangle
                vertexStart:0
                vertexCount:6];
  }

  bool RenderBitmapToCommandBuffer(id<MTLCommandBuffer> command_buffer,
                                    RetainPtr<CFX_DIBitmap> bitmap,
                                    int width,
                                    int height,
                                    bool apply_antialiasing) {
    if (!device_ || !pipeline_state_ || !command_buffer || !bitmap) {
      return false;
    }

    @autoreleasepool {
      // Create texture from bitmap
      id<MTLTexture> texture = CreateTextureFromBitmap(bitmap, width, height);
      if (!texture) {
        return false;
      }

      // Create render target
      id<MTLTexture> render_target = CreateRenderTarget(width, height);
      if (!render_target) {
        return false;
      }

      // Setup render pass
      MTLRenderPassDescriptor* render_pass = [MTLRenderPassDescriptor renderPassDescriptor];
      render_pass.colorAttachments[0].texture = render_target;
      render_pass.colorAttachments[0].loadAction = MTLLoadActionClear;
      render_pass.colorAttachments[0].clearColor = MTLClearColorMake(1.0, 1.0, 1.0, 1.0);
      render_pass.colorAttachments[0].storeAction = MTLStoreActionStore;

      // Enable MSAA if antialiasing requested
      id<MTLTexture> msaa_texture = nil;
      if (apply_antialiasing) {
        msaa_texture = CreateMSAATexture(width, height, 4);  // 4x MSAA
        if (msaa_texture) {
          render_pass.colorAttachments[0].texture = msaa_texture;
          render_pass.colorAttachments[0].resolveTexture = render_target;
          render_pass.colorAttachments[0].storeAction = MTLStoreActionMultisampleResolve;
        }
      }

      // Encode render commands
      id<MTLRenderCommandEncoder> encoder = [command_buffer renderCommandEncoderWithDescriptor:render_pass];
      if (!encoder) {
        return false;
      }

      [encoder setRenderPipelineState:pipeline_state_];
      [encoder setFragmentTexture:texture atIndex:0];

      // Draw full-screen quad
      DrawFullScreenQuad(encoder);

      [encoder endEncoding];

      // Copy result back to bitmap (must happen before command buffer commits)
      return CopyTextureToBitmap(render_target, bitmap);
    }
  }

  bool CopyTextureToBitmap(id<MTLTexture> texture, RetainPtr<CFX_DIBitmap> bitmap) {
    if (!texture || !bitmap) {
      return false;
    }

    // Read texture data back to CPU
    uint8_t* buffer = bitmap->GetWritableBuffer().data();
    NSUInteger bytes_per_row = bitmap->GetPitch();

    [texture getBytes:buffer
          bytesPerRow:bytes_per_row
           fromRegion:MTLRegionMake2D(0, 0, texture.width, texture.height)
          mipmapLevel:0];

    return true;
  }
};

// MetalRenderer implementation
MetalRenderer::MetalRenderer() : impl_(std::make_unique<Impl>()) {}
MetalRenderer::~MetalRenderer() = default;

bool MetalRenderer::IsAvailable() {
  @autoreleasepool {
    id<MTLDevice> device = MTLCreateSystemDefaultDevice();
    return device != nil;
  }
}

MetalRenderer* MetalRenderer::GetInstance() {
  if (!instance_) {
    instance_ = new MetalRenderer();
    if (!instance_->Initialize()) {
      delete instance_;
      instance_ = nullptr;
    }
  }
  return instance_;
}

bool MetalRenderer::Initialize() {
  return impl_->Initialize();
}

bool MetalRenderer::RenderBitmap(RetainPtr<CFX_DIBitmap> bitmap,
                                  int width,
                                  int height,
                                  bool apply_antialiasing) {
  return impl_->RenderBitmap(bitmap, width, height, apply_antialiasing);
}

bool MetalRenderer::RenderBitmapBatch(const std::vector<RetainPtr<CFX_DIBitmap>>& bitmaps,
                                       int width,
                                       int height,
                                       bool apply_antialiasing) {
  return impl_->RenderBitmapBatch(bitmaps, width, height, apply_antialiasing);
}

const char* MetalRenderer::GetDeviceName() const {
  return impl_->GetDeviceName();
}

size_t MetalRenderer::GetMaxBufferLength() const {
  return impl_->GetMaxBufferLength();
}

bool MetalRenderer::SupportsFamily(int family) const {
  return impl_->SupportsFamily(family);
}

void MetalRenderer::Shutdown() {
  impl_->Shutdown();
}

}  // namespace metal
}  // namespace pdfium
