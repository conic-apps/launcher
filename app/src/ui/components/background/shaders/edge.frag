// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

in float vMix;
uniform vec4 uBgColor;
uniform vec4 uEdgeColor;
uniform float uLayerAlpha;
out vec4 outColor;
void main() {
  vec3 c = mix(uBgColor.rgb, uEdgeColor.rgb, vMix);
  outColor = vec4(c * uLayerAlpha, uLayerAlpha);
}
