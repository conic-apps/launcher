// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// Premultiplied. Slint hands the borrowed texture to femtovg without
// `ImageFlags::PREMULTIPLIED`, so femtovg multiplies by alpha as it samples;
// the resolve pass divides it back out (see `resolve.frag`).

in float vAlpha;
uniform vec4 uBgColor;
uniform float uLayerAlpha;
out vec4 outColor;
void main() {
  float a = vAlpha * uLayerAlpha;
  outColor = vec4(uBgColor.rgb * a, a);
}
