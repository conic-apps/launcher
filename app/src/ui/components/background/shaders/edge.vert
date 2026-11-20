// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// The camera (`uCamX` … `uFarZ`) is not declared here: GLSL ES 3.0 has no
// `#include`, so the pass prelude in `gl.rs` is stitched on above this file.

layout(location = 0) in vec3 aPos;
layout(location = 1) in float aMix;
layout(location = 2) in vec3 aOther;
layout(location = 3) in float aSide;
uniform float uEdgeWidth;
out float vMix;
void main() {
  // The segments are expanded into fixed-width quads in screen space (in
  // pixels), so that a sub-pixel line is never dropped by the rasteriser and
  // cannot flicker as the camera moves.
  float dz0 = aPos.z - uCamZ;
  float dz1 = aOther.z - uCamZ;
  float s0 = uFocal / dz0;
  float s1 = uFocal / dz1;
  vec2 p0 = vec2((aPos.x - uCamX) * s0, (aPos.y - uCamY) * s0);
  vec2 p1 = vec2((aOther.x - uCamX) * s1, (aOther.y - uCamY) * s1);
  vec2 d = p1 - p0;
  vec2 n = vec2(-d.y, d.x);
  float len = length(n);
  vec2 off = len > 1e-5 ? (n / len) * (uEdgeWidth * 0.5) : vec2(0.0);
  vec2 p = p0 + off * aSide;
  float ndcZ = clamp((dz0 - uNearZ) / (uFarZ - uNearZ) * 2.0 - 1.0, -1.0, 1.0);
  gl_Position = vec4(p.x / uHalfW, p.y / uHalfH, ndcZ, 1.0);
  vMix = aMix;
}
