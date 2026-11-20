// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// The camera (`uCamX` … `uFarZ`) is not declared here: GLSL ES 3.0 has no
// `#include`, so the pass prelude in `gl.rs` is stitched on above this file.

layout(location = 0) in vec3 aPos;
layout(location = 1) in float aAlpha;
out float vAlpha;
void main() {
  float dz = aPos.z - uCamZ;
  float s = uFocal / dz;
  float ndcX = (aPos.x - uCamX) * s / uHalfW;
  float ndcY = (aPos.y - uCamY) * s / uHalfH;
  float ndcZ = clamp((dz - uNearZ) / (uFarZ - uNearZ) * 2.0 - 1.0, -1.0, 1.0);
  gl_Position = vec4(ndcX, ndcY, ndcZ, 1.0);
  vAlpha = aAlpha;
}
