// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

// The world's three passes in one module. WGSL has no `#include`, so a file per
// pass would carry three hand-kept copies of the camera block; the entry points
// below say which pass is which, and `gpu.rs` builds one pipeline for each pair.

struct Globals {
  // The camera's x, y and z, then its focal length in pixels.
  cam: vec4<f32>,
  // Half the target's width and height, then the near and far planes.
  plane: vec4<f32>,
  // The crust, which every face is painted, and the outline colour.
  crust: vec4<f32>,
  outline: vec4<f32>,
  // The layer's opacity, then the outline's width in device pixels.
  opacity: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;

// The depth of a point `dz` in front of the camera: 0 at the near plane and 1 at
// the far one, which is WebGPU's range — a value outside it is clipped, so the
// projection has to clamp.
fn depth(dz: f32) -> f32 {
  return clamp((dz - globals.plane.z) / (globals.plane.w - globals.plane.z), 0.0, 1.0);
}

// The fill: one quad per visible face, each blended over the ones behind it.

struct Fill {
  @builtin(position) clip: vec4<f32>,
  @location(0) alpha: f32,
};

@vertex
fn fill_vertex(@location(0) position: vec3<f32>, @location(1) alpha: f32) -> Fill {
  let dz = position.z - globals.cam.z;
  let scale = globals.cam.w / dz;
  let ndc = vec2<f32>(
    (position.x - globals.cam.x) * scale / globals.plane.x,
    (position.y - globals.cam.y) * scale / globals.plane.y,
  );
  return Fill(vec4<f32>(ndc, depth(dz), 1.0), alpha);
}

@fragment
fn fill_fragment(v: Fill) -> @location(0) vec4<f32> {
  // Premultiplied, so the faces compose without a seam where they overlap.
  let alpha = v.alpha * globals.opacity.x;
  return vec4<f32>(globals.crust.rgb * alpha, alpha);
}

// The outlines: one quad per segment, expanded in screen space.

struct Edge {
  @builtin(position) clip: vec4<f32>,
  @location(0) blend: f32,
};

@vertex
fn edge_vertex(
  @location(0) position: vec3<f32>,
  @location(1) blend: f32,
  @location(2) other: vec3<f32>,
  @location(3) side: f32,
) -> Edge {
  // A segment is expanded into a fixed-width quad in screen space, in pixels,
  // so that a sub-pixel line is never dropped by the rasteriser and cannot
  // flicker as the camera moves.
  let dz0 = position.z - globals.cam.z;
  let dz1 = other.z - globals.cam.z;
  let s0 = globals.cam.w / dz0;
  let s1 = globals.cam.w / dz1;
  let p0 = vec2<f32>((position.x - globals.cam.x) * s0, (position.y - globals.cam.y) * s0);
  let p1 = vec2<f32>((other.x - globals.cam.x) * s1, (other.y - globals.cam.y) * s1);
  let across = p1 - p0;
  let normal = vec2<f32>(-across.y, across.x);
  let span = length(normal);
  let offset = select(vec2<f32>(0.0), normal / span * (globals.opacity.y * 0.5), span > 1e-5);
  let p = p0 + offset * side;
  let ndc = vec2<f32>(p.x / globals.plane.x, p.y / globals.plane.y);
  return Edge(vec4<f32>(ndc, depth(dz0), 1.0), blend);
}

@fragment
fn edge_fragment(v: Edge) -> @location(0) vec4<f32> {
  let alpha = globals.opacity.x;
  return vec4<f32>(mix(globals.crust.rgb, globals.outline.rgb, v.blend) * alpha, alpha);
}

// The resolve: the world is drawn premultiplied, Slint samples it as straight,
// so this pass divides the alpha back out. It is what makes the two agree —
// without it everything drawn comes out `alpha` times too dark, which is a hard
// colour step at the horizon in a dark palette and an obvious one in a light one
// (a light crust has far more to lose).

// Its own group 0: the resolve reads none of the camera block, and a pipeline
// layout has to have every group it lists bound when it draws.
@group(0) @binding(0) var world: texture_2d<f32>;
@group(0) @binding(1) var world_sampler: sampler;

struct Resolve {
  @builtin(position) clip: vec4<f32>,
  @location(0) uv: vec2<f32>,
};

@vertex
fn resolve_vertex(@builtin(vertex_index) index: u32) -> Resolve {
  // A triangle that covers the target, so there is no vertex buffer to bind.
  let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
  let clip = corner * 2.0 - 1.0;
  // The target's rows start at the top and the clip space's y points up, so the
  // texture coordinate is the clip position with its y flipped.
  let uv = vec2<f32>(corner.x, 1.0 - corner.y);
  return Resolve(vec4<f32>(clip, 0.0, 1.0), uv);
}

@fragment
fn resolve_fragment(v: Resolve) -> @location(0) vec4<f32> {
  let premultiplied = textureSample(world, world_sampler, v.uv);
  let rgb = select(vec3<f32>(0.0), premultiplied.rgb / premultiplied.a, premultiplied.a > 0.0);
  return vec4<f32>(rgb, premultiplied.a);
}
