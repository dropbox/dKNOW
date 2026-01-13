// Copyright 2025 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <metal_stdlib>
using namespace metal;

// Vertex shader output structure
struct VertexOut {
  float4 position [[position]];
  float2 texCoord;
};

// Vertex shader - generates full-screen quad
vertex VertexOut vertex_main(uint vid [[vertex_id]]) {
  // Full-screen quad vertices (2 triangles)
  const float2 positions[6] = {
    float2(-1.0, -1.0),  // Bottom-left
    float2( 1.0, -1.0),  // Bottom-right
    float2(-1.0,  1.0),  // Top-left
    float2( 1.0, -1.0),  // Bottom-right
    float2( 1.0,  1.0),  // Top-right
    float2(-1.0,  1.0)   // Top-left
  };

  // Texture coordinates (flipped Y for PDF coordinate system)
  const float2 texCoords[6] = {
    float2(0.0, 1.0),  // Bottom-left
    float2(1.0, 1.0),  // Bottom-right
    float2(0.0, 0.0),  // Top-left
    float2(1.0, 1.0),  // Bottom-right
    float2(1.0, 0.0),  // Top-right
    float2(0.0, 0.0)   // Top-left
  };

  VertexOut out;
  out.position = float4(positions[vid], 0.0, 1.0);
  out.texCoord = texCoords[vid];
  return out;
}

// Fragment shader - texture sampling with linear filtering
fragment float4 fragment_main(VertexOut in [[stage_in]],
                               texture2d<float> tex [[texture(0)]]) {
  constexpr sampler s(coord::normalized,
                      address::clamp_to_edge,
                      filter::linear);
  return tex.sample(s, in.texCoord);
}

// Fragment shader with CMYK to RGB conversion
fragment float4 fragment_cmyk_to_rgb(VertexOut in [[stage_in]],
                                      texture2d<float> tex [[texture(0)]]) {
  constexpr sampler s(coord::normalized,
                      address::clamp_to_edge,
                      filter::linear);

  float4 cmyk = tex.sample(s, in.texCoord);

  // CMYK to RGB conversion
  // R = 1 - min(1, C * (1 - K) + K)
  // G = 1 - min(1, M * (1 - K) + K)
  // B = 1 - min(1, Y * (1 - K) + K)
  float c = cmyk.x;
  float m = cmyk.y;
  float y = cmyk.z;
  float k = cmyk.w;

  float one_minus_k = 1.0 - k;
  float r = 1.0 - min(1.0, c * one_minus_k + k);
  float g = 1.0 - min(1.0, m * one_minus_k + k);
  float b = 1.0 - min(1.0, y * one_minus_k + k);

  return float4(r, g, b, 1.0);
}

// Fragment shader with alpha blending
fragment float4 fragment_alpha_blend(VertexOut in [[stage_in]],
                                      texture2d<float> src [[texture(0)]],
                                      texture2d<float> dst [[texture(1)]],
                                      constant float& alpha [[buffer(0)]]) {
  constexpr sampler s(coord::normalized,
                      address::clamp_to_edge,
                      filter::linear);

  float4 src_color = src.sample(s, in.texCoord);
  float4 dst_color = dst.sample(s, in.texCoord);

  // Alpha blending: result = src * alpha + dst * (1 - alpha)
  return src_color * alpha + dst_color * (1.0 - alpha);
}

// Fragment shader for image composition (multiple layers)
fragment float4 fragment_composite(VertexOut in [[stage_in]],
                                    texture2d<float> base [[texture(0)]],
                                    texture2d<float> layer [[texture(1)]]) {
  constexpr sampler s(coord::normalized,
                      address::clamp_to_edge,
                      filter::linear);

  float4 base_color = base.sample(s, in.texCoord);
  float4 layer_color = layer.sample(s, in.texCoord);

  // Standard alpha compositing
  // C_out = C_src * A_src + C_dst * (1 - A_src)
  float src_alpha = layer_color.a;
  float3 rgb = layer_color.rgb * src_alpha + base_color.rgb * (1.0 - src_alpha);
  float a = src_alpha + base_color.a * (1.0 - src_alpha);

  return float4(rgb, a);
}
