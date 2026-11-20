// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// The resolve: the world is drawn premultiplied, Slint samples it as straight,
// so this pass divides the alpha back out. It is what makes the two agree —
// without it everything drawn comes out `alpha` times too dark, which is a hard
// colour step at the horizon in a dark palette and an obvious one in a light
// one (a light crust has far more to lose).

in vec2 vUv;
uniform sampler2D uWorld;
out vec4 outColor;
void main() {
  vec4 c = texture(uWorld, vUv);
  outColor = vec4(c.a > 0.0 ? c.rgb / c.a : vec3(0.0), c.a);
}
