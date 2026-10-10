// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The 3D world, drawn on the GPU.
//!
//! Slint's declarative API has no custom-shader hook, but it has an escape
//! hatch meant for exactly this: `Window::set_rendering_notifier` calls back
//! with the OpenGL context *current*, and hands over `get_proc_address`, so the
//! same code loads the GL entry points on every platform — and
//! `BorrowedOpenGLTextureBuilder` lets Slint composite a texture we drew into:
//! the shaders run on the GPU, and only the final compositing belongs to
//! Slint.
//!
//! The two passes, shader for shader: the fills are blended with
//! `ONE, ONE_MINUS_SRC_ALPHA` into a depth buffer and are *premultiplied*, the
//! outlines are screen-space quads drawn over them with blending off. Both
//! write into an offscreen framebuffer that Slint then samples, so there is no
//! per-frame copy back to the CPU.
//!
//! The geometry comes from [`Scene`], the verified terrain; only the projection
//! and the outline expansion happen on the GPU here.
//!
//! The callback runs inside Slint's own render pass, so whatever GL state it
//! touches has to be put back before returning. Everything here is saved and
//! restored for that reason. (femtovg re-establishes most of what it needs at
//! the top of every flush, but the framebuffer, the program, the vertex array
//! and the buffer bindings are not among them.)

use std::cell::{Cell, RefCell};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

use euclid::default::Size2D;
use glow::HasContext;
use slint::{
    BorrowedOpenGLTextureBuilder, BorrowedOpenGLTextureOrigin, ComponentHandle, GraphicsAPI, Image,
    RenderingState,
};

use super::scene::{self, CAMERA_SPEED, Scene};
use crate::slint_backend::{App, Background};

/// How far the parallax wrapper scales the background past the window — the
/// images are rendered this much larger so the scaling lands them 1:1 on device
/// pixels.
const WRAPPER_SCALE: f32 = 1.08;

/// Whether the GPU renderer is drawing the world.
///
/// The CPU rasteriser starts the app off and keeps drawing until the first GPU
/// frame has actually reached the window. Only then does it stand down: a
/// renderer that turns out not to have OpenGL — the software renderer on a
/// Windows machine without a GL driver, say — simply leaves this at `Pending`
/// (or sets `Failed`) and the fallback is already on screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Gpu {
    /// Registered, but no frame drawn yet.
    Pending,
    /// Drawing; the CPU rasteriser is out of a job.
    Running,
    /// Setup failed. The CPU rasteriser is the renderer.
    Failed,
}

/// The handover flag. The render callback and the controller both run on the UI
/// thread, but the rasteriser's worker has to read it too when a frame it
/// rendered arrives late, so it is an atomic behind an `Arc` rather than a cell
/// in the controller.
pub struct Flag(AtomicU8);

impl Flag {
    fn new() -> Arc<Self> {
        Arc::new(Self(AtomicU8::new(Gpu::Pending as u8)))
    }

    pub fn get(&self) -> Gpu {
        match self.0.load(Ordering::Relaxed) {
            1 => Gpu::Running,
            2 => Gpu::Failed,
            _ => Gpu::Pending,
        }
    }

    fn set(&self, state: Gpu) {
        self.0.store(state as u8, Ordering::Relaxed);
    }
}

/// What the render callback and the controller share. They both run on the UI
/// thread, but never inside each other's borrows, so these are cells rather
/// than part of the controller's `RefCell`.
pub struct Shared {
    pub gpu: Arc<Flag>,
    /// The camera. The GPU renderer advances it as it draws, so its speed is
    /// the rate frames actually arrive at, so a frame that never happens also
    /// never moves the camera.
    pub cam_z: Cell<f32>,
    /// Whether the camera is meant to be running at all (the setting, and the
    /// world being what the window shows).
    pub moving: Cell<bool>,
    /// Frames drawn since the renderer started, for the debug log.
    pub frames: Cell<u64>,
}

impl Shared {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            gpu: Flag::new(),
            cam_z: Cell::new(0.0),
            moving: Cell::new(false),
            frames: Cell::new(0),
        })
    }
}

pub type Handle = Rc<Shared>;

thread_local! {
    /// The GL objects, alive between the notifier's calls. It runs on the UI
    /// thread throughout, so a thread-local is where they belong.
    static RENDERER: RefCell<Option<Renderer>> = const { RefCell::new(None) };
}

/// Registers the notifier. Returns false when the renderer cannot do OpenGL at
/// all — `set_rendering_notifier` itself rejects a renderer that has no such
/// hook, which is the software one's case.
pub fn install(ui: &App, shared: Handle) -> bool {
    // `CONIC_FORCE_CPU=1` (debug builds) keeps the GPU out of the way, which is
    // how the rasteriser's path gets exercised on a machine that does have
    // OpenGL.
    #[cfg(debug_assertions)]
    if std::env::var("CONIC_FORCE_CPU").is_ok() {
        return false;
    }
    let weak = ui.as_weak();
    let result = ui.window().set_rendering_notifier(move |state, api| {
        let Some(ui) = weak.upgrade() else { return };
        match state {
            RenderingState::RenderingSetup => {
                // A borrowed texture from a previous context is dead now, so
                // the layer goes back to the sky until the first draw below
                // hands over a fresh one. It is cleared here rather than in
                // `RenderingTeardown` on purpose: femtovg fires teardown from
                // inside the winit adapter's `suspend()`, which holds the
                // adapter's window `RefCell` mutably. Setting a Slint property
                // there marks the window dirty and calls `request_redraw`,
                // which re-borrows that same cell and panics with
                // "RefCell already mutably borrowed" — the crash on `⌘W`. Setup
                // runs from `draw()`, with no borrow held, and always precedes
                // the first frame of the new context, so nothing ever samples
                // the stale texture.
                ui.global::<Background>().set_world_image(Image::default());
                let size = target_size(&ui);
                match Renderer::new(api, size) {
                    Ok(renderer) => {
                        log::info!(
                            target: "background",
                            "rendering the world with OpenGL at {}x{}",
                            size.0,
                            size.1,
                        );
                        RENDERER.with(|slot| *slot.borrow_mut() = Some(renderer));
                    }
                    Err(error) => {
                        log::warn!(target: "background", "no GPU background: {error}");
                        shared.gpu.set(Gpu::Failed);
                    }
                }
            }
            RenderingState::BeforeRendering => {
                let drawn = RENDERER.with(|slot| {
                    slot.borrow_mut()
                        .as_mut()
                        .map(|renderer| renderer.draw(&ui, &shared))
                        .unwrap_or(false)
                });
                if drawn && shared.gpu.get() == Gpu::Pending {
                    shared.gpu.set(Gpu::Running);
                }
            }
            RenderingState::RenderingTeardown => {
                RENDERER.with(|slot| *slot.borrow_mut() = None);
                // The image property is not touched here: this state is
                // delivered from inside the adapter's `suspend()` and any
                // write would panic. `RenderingSetup` clears it instead.
            }
            _ => {}
        }
    });
    // A renderer with no OpenGL hook at all — the software one — makes this fail,
    // and the caller only saw a `false` it had to interpret. That is the ordinary
    // "no GPU here" case, so it is `debug` rather than a warning.
    let installed = result.is_ok();
    if !installed {
        log::debug!(target: "background", "the renderer has no OpenGL hook; using the CPU path");
    }
    installed
}

/// The pixels the world is drawn at: the window's physical size times the
/// parallax wrapper's scale, which is what the image is stretched over. No
/// downscaling: the GPU does not care how many pixels these are.
fn target_size(ui: &App) -> (i32, i32) {
    // `Window::size()` is already in physical pixels; the wrapper's 1.08 is on
    // top of that, so the texture lands 1:1 on device pixels.
    let size = ui.window().size();
    (
        (size.width as f32 * WRAPPER_SCALE).round().max(1.0) as i32,
        (size.height as f32 * WRAPPER_SCALE).round().max(1.0) as i32,
    )
}

// The four shaders, with two differences: the corner-radius clipping is gone
// (Slint rounds the window's corners when it composites the image), and the
// layer's 0.3 opacity is folded in, because this image is one layer of Slint's
// composite rather than an element of its own.

const FILL_VERTEX: &str = r#"
layout(location = 0) in vec3 aPos;
layout(location = 1) in float aAlpha;
uniform float uCamX;
uniform float uCamY;
uniform float uCamZ;
uniform float uFocal;
uniform float uHalfW;
uniform float uHalfH;
uniform float uNearZ;
uniform float uFarZ;
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
"#;

// Premultiplied. Slint hands a borrowed texture to
// femtovg *without* `ImageFlags::PREMULTIPLIED`, so femtovg multiplies by alpha
// as it samples — the framebuffer therefore holds premultiplied colour and a
// resolve pass divides it back out (see `RESOLVE_FRAGMENT`).
const FILL_FRAGMENT: &str = r#"
in float vAlpha;
uniform vec4 uBgColor;
uniform float uLayerAlpha;
out vec4 outColor;
void main() {
  float a = vAlpha * uLayerAlpha;
  outColor = vec4(uBgColor.rgb * a, a);
}
"#;

const EDGE_VERTEX: &str = r#"
layout(location = 0) in vec3 aPos;
layout(location = 1) in float aMix;
layout(location = 2) in vec3 aOther;
layout(location = 3) in float aSide;
uniform float uCamX;
uniform float uCamY;
uniform float uCamZ;
uniform float uFocal;
uniform float uHalfW;
uniform float uHalfH;
uniform float uNearZ;
uniform float uFarZ;
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
"#;

const EDGE_FRAGMENT: &str = r#"
in float vMix;
uniform vec4 uBgColor;
uniform vec4 uEdgeColor;
uniform float uLayerAlpha;
out vec4 outColor;
void main() {
  vec3 c = mix(uBgColor.rgb, uEdgeColor.rgb, vMix);
  outColor = vec4(c * uLayerAlpha, uLayerAlpha);
}
"#;

// The resolve: the world is drawn premultiplied, Slint samples it as straight,
// so this pass divides the alpha back out. It is what makes the two agree —
// without it everything drawn comes out `alpha` times too dark, which is a hard
// colour step at the horizon in a dark palette and an obvious one in a light
// one (a light crust has far more to lose).
const RESOLVE_VERTEX: &str = r#"
layout(location = 0) in vec2 aPos;
out vec2 vUv;
void main() {
  vUv = aPos * 0.5 + 0.5;
  gl_Position = vec4(aPos, 0.0, 1.0);
}
"#;

const RESOLVE_FRAGMENT: &str = r#"
in vec2 vUv;
uniform sampler2D uWorld;
out vec4 outColor;
void main() {
  vec4 c = texture(uWorld, vUv);
  outColor = vec4(c.a > 0.0 ? c.rgb / c.a : vec3(0.0), c.a);
}
"#;

/// `#version 330 core` on a desktop context, `#version 300 es` on an ES one.
/// The bodies are the same language above that line.
struct Dialect {
    version: &'static str,
    precision: &'static str,
}

impl Dialect {
    fn detect(gl: &glow::Context) -> Self {
        let version = unsafe { gl.get_parameter_string(glow::VERSION) };
        if version.contains("OpenGL ES") {
            Self {
                version: "#version 300 es\n",
                precision: "precision highp float;\n",
            }
        } else {
            Self {
                version: "#version 330 core\n",
                precision: "",
            }
        }
    }

    fn source(&self, fragment: bool, body: &str) -> String {
        let precision = if fragment { self.precision } else { "" };
        format!("{}{precision}{body}", self.version)
    }
}

/// The GL state a pass here changes, so it can be put back.
///
/// The notifier's callback runs inside Slint's own render pass, so anything
/// left changed leaks into the UI's drawing — and the UI is drawn with the same
/// context, right after this returns. femtovg re-establishes most of what it
/// needs at the top of every flush (its vertex array, attributes 0 and 1,
/// blending on, the depth test off); the framebuffer, the program, the vertex
/// array and the buffer bindings are what it does not.
struct State {
    framebuffer: i32,
    program: i32,
    vertex_array: i32,
    array_buffer: i32,
    element_buffer: i32,
    texture: i32,
    active_texture: i32,
    renderbuffer: i32,
    viewport: [i32; 4],
    blend: bool,
    depth_test: bool,
    scissor: bool,
    depth_mask: bool,
    depth_func: u32,
}

impl State {
    fn save(gl: &glow::Context) -> Self {
        unsafe {
            let mut viewport = [0i32; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
            Self {
                framebuffer: gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING),
                program: gl.get_parameter_i32(glow::CURRENT_PROGRAM),
                vertex_array: gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING),
                array_buffer: gl.get_parameter_i32(glow::ARRAY_BUFFER_BINDING),
                element_buffer: gl.get_parameter_i32(glow::ELEMENT_ARRAY_BUFFER_BINDING),
                texture: gl.get_parameter_i32(glow::TEXTURE_BINDING_2D),
                active_texture: gl.get_parameter_i32(glow::ACTIVE_TEXTURE),
                renderbuffer: gl.get_parameter_i32(glow::RENDERBUFFER_BINDING),
                viewport,
                blend: gl.is_enabled(glow::BLEND),
                depth_test: gl.is_enabled(glow::DEPTH_TEST),
                scissor: gl.is_enabled(glow::SCISSOR_TEST),
                depth_mask: gl.get_parameter_i32(glow::DEPTH_WRITEMASK) != 0,
                depth_func: gl.get_parameter_i32(glow::DEPTH_FUNC) as u32,
            }
        }
    }

    fn restore(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(
                glow::FRAMEBUFFER,
                NonZeroU32::new(self.framebuffer as u32).map(glow::NativeFramebuffer),
            );
            gl.use_program(NonZeroU32::new(self.program as u32).map(glow::NativeProgram));
            // The element buffer binding is part of the vertex array's state,
            // so it goes back with the array that was current — and only for an
            // array that existed: with none bound, the binding is the default
            // array's, which none of this touches.
            let vertex_array =
                NonZeroU32::new(self.vertex_array as u32).map(glow::NativeVertexArray);
            gl.bind_vertex_array(vertex_array);
            gl.bind_buffer(
                glow::ARRAY_BUFFER,
                NonZeroU32::new(self.array_buffer as u32).map(glow::NativeBuffer),
            );
            if vertex_array.is_some() {
                gl.bind_buffer(
                    glow::ELEMENT_ARRAY_BUFFER,
                    NonZeroU32::new(self.element_buffer as u32).map(glow::NativeBuffer),
                );
            }
            // The texture binding is per unit, so the unit comes back first.
            gl.active_texture(self.active_texture as u32);
            gl.bind_texture(
                glow::TEXTURE_2D,
                NonZeroU32::new(self.texture as u32).map(glow::NativeTexture),
            );
            gl.bind_renderbuffer(
                glow::RENDERBUFFER,
                NonZeroU32::new(self.renderbuffer as u32).map(glow::NativeRenderbuffer),
            );
            gl.viewport(
                self.viewport[0],
                self.viewport[1],
                self.viewport[2],
                self.viewport[3],
            );
            gl.depth_mask(self.depth_mask);
            gl.depth_func(self.depth_func);
            for (capability, enabled) in [
                (glow::BLEND, self.blend),
                (glow::DEPTH_TEST, self.depth_test),
                (glow::SCISSOR_TEST, self.scissor),
            ] {
                if enabled {
                    gl.enable(capability);
                } else {
                    gl.disable(capability);
                }
            }
        }
    }
}

/// A linked program and where its uniforms live.
struct Program {
    id: glow::Program,
    uniforms: std::collections::HashMap<&'static str, glow::UniformLocation>,
}

impl Program {
    fn build(
        gl: &glow::Context,
        dialect: &Dialect,
        vertex: &str,
        fragment: &str,
        names: &[&'static str],
    ) -> Result<Self, String> {
        let id = unsafe {
            let program = gl.create_program()?;
            let mut shaders = Vec::new();
            for (kind, body, is_fragment) in [
                (glow::VERTEX_SHADER, vertex, false),
                (glow::FRAGMENT_SHADER, fragment, true),
            ] {
                let shader = gl.create_shader(kind)?;
                let source = dialect.source(is_fragment, body);
                gl.shader_source(shader, &source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    let log = gl.get_shader_info_log(shader);
                    gl.delete_shader(shader);
                    for shader in shaders {
                        gl.delete_shader(shader);
                    }
                    gl.delete_program(program);
                    return Err(format!("shader did not compile: {log}"));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }
            gl.link_program(program);
            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }
            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(format!("program did not link: {log}"));
            }
            program
        };
        let mut uniforms = std::collections::HashMap::new();
        for name in names {
            let location = unsafe {
                gl.get_uniform_location(id, name)
                    .ok_or_else(|| format!("uniform {name} is missing"))?
            };
            uniforms.insert(*name, location);
        }
        Ok(Self { id, uniforms })
    }

    fn uniform(&self, name: &str) -> Option<&glow::UniformLocation> {
        self.uniforms.get(name)
    }
}

/// The fill pass's vertex layout: `x y z alpha`.
const FILL_STRIDE: i32 = 16;
/// The outline pass's: `x y z mix other_x other_y other_z side`.
const EDGE_STRIDE: i32 = 32;

struct Renderer {
    gl: glow::Context,
    size: (i32, i32),
    framebuffer: Option<glow::Framebuffer>,
    color: Option<glow::Texture>,
    depth: Option<glow::Renderbuffer>,
    /// The un-premultiplied copy Slint samples.
    resolved_framebuffer: Option<glow::Framebuffer>,
    resolved: Option<glow::Texture>,
    resolve: Program,
    resolve_vao: glow::VertexArray,
    /// Owned so the vertex array's buffer stays alive (the array keeps the
    /// binding, not the object).
    _resolve_vertices: glow::Buffer,
    /// The image Slint composites, rebuilt whenever the framebuffer is.
    image: Option<Image>,
    fill: Program,
    edge: Program,
    fill_vao: glow::VertexArray,
    fill_vertices: glow::Buffer,
    fill_indices: glow::Buffer,
    edge_vao: glow::VertexArray,
    edge_vertices: glow::Buffer,
    scene: Scene,
    started: Instant,
    /// When the last frame was drawn, for the camera's step.
    last_draw: Option<Instant>,
    /// Whether `CONIC_GL_DUMP` has been served.
    dumped: bool,
}

impl Renderer {
    fn new(api: &GraphicsAPI, size: (i32, i32)) -> Result<Self, String> {
        let GraphicsAPI::NativeOpenGL { get_proc_address } = api else {
            return Err("the renderer is not using OpenGL".into());
        };
        // Safe: the context is current, which is what the notifier promises.
        let gl = unsafe { glow::Context::from_loader_function_cstr(|name| get_proc_address(name)) };
        unsafe {
            log::info!(
                target: "background",
                "GL: {} — {} — GLSL {}",
                gl.get_parameter_string(glow::VERSION),
                gl.get_parameter_string(glow::RENDERER),
                gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION),
            );
        }
        let dialect = Dialect::detect(&gl);
        let fill = Program::build(
            &gl,
            &dialect,
            FILL_VERTEX,
            FILL_FRAGMENT,
            &[
                "uCamX",
                "uCamY",
                "uCamZ",
                "uFocal",
                "uHalfW",
                "uHalfH",
                "uNearZ",
                "uFarZ",
                "uBgColor",
                "uLayerAlpha",
            ],
        )?;
        let edge = Program::build(
            &gl,
            &dialect,
            EDGE_VERTEX,
            EDGE_FRAGMENT,
            &[
                "uCamX",
                "uCamY",
                "uCamZ",
                "uFocal",
                "uHalfW",
                "uHalfH",
                "uNearZ",
                "uFarZ",
                "uBgColor",
                "uEdgeColor",
                "uLayerAlpha",
                "uEdgeWidth",
            ],
        )?;
        let resolve = Program::build(&gl, &dialect, RESOLVE_VERTEX, RESOLVE_FRAGMENT, &["uWorld"])?;
        let (resolve_vao, resolve_vertices) = unsafe {
            let vao = gl.create_vertex_array()?;
            let vertices = gl.create_buffer()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&[-1.0f32, -1.0, 3.0, -1.0, -1.0, 3.0]),
                glow::STATIC_DRAW,
            );
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            (vao, vertices)
        };

        let (fill_vao, fill_vertices, fill_indices, edge_vao, edge_vertices) = unsafe {
            let fill_vao = gl.create_vertex_array()?;
            let fill_vertices = gl.create_buffer()?;
            let fill_indices = gl.create_buffer()?;
            let edge_vao = gl.create_vertex_array()?;
            let edge_vertices = gl.create_buffer()?;

            gl.bind_vertex_array(Some(fill_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(fill_vertices));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, FILL_STRIDE, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 1, glow::FLOAT, false, FILL_STRIDE, 12);
            // The element buffer is part of the vertex array's state.
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(fill_indices));

            gl.bind_vertex_array(Some(edge_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(edge_vertices));
            for (index, size, offset) in [(0, 3, 0), (1, 1, 12), (2, 3, 16), (3, 1, 28)] {
                gl.enable_vertex_attrib_array(index);
                gl.vertex_attrib_pointer_f32(index, size, glow::FLOAT, false, EDGE_STRIDE, offset);
            }

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            (
                fill_vao,
                fill_vertices,
                fill_indices,
                edge_vao,
                edge_vertices,
            )
        };

        let mut renderer = Self {
            gl,
            size: (0, 0),
            framebuffer: None,
            color: None,
            depth: None,
            resolved_framebuffer: None,
            resolved: None,
            resolve,
            resolve_vao,
            _resolve_vertices: resolve_vertices,
            image: None,
            fill,
            edge,
            fill_vao,
            fill_vertices,
            fill_indices,
            edge_vao,
            edge_vertices,
            scene: Scene::new(),
            started: Instant::now(),
            last_draw: None,
            dumped: false,
        };
        renderer.resize(size)?;
        Ok(renderer)
    }

    /// (Re)builds the offscreen target everything is drawn into.
    fn resize(&mut self, size: (i32, i32)) -> Result<(), String> {
        if self.size == size && self.framebuffer.is_some() {
            return Ok(());
        }
        self.size = size;
        self.image = None;
        let gl = &self.gl;
        let state = State::save(gl);
        unsafe {
            if let Some(framebuffer) = self.framebuffer.take() {
                gl.delete_framebuffer(framebuffer);
            }
            if let Some(texture) = self.color.take() {
                gl.delete_texture(texture);
            }
            if let Some(depth) = self.depth.take() {
                gl.delete_renderbuffer(depth);
            }
            if let Some(framebuffer) = self.resolved_framebuffer.take() {
                gl.delete_framebuffer(framebuffer);
            }
            if let Some(texture) = self.resolved.take() {
                gl.delete_texture(texture);
            }
            let make_texture = || -> Result<glow::Texture, String> {
                let texture = gl.create_texture()?;
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    size.0,
                    size.1,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                // Nearest: the image lands on the wrapper 1:1, and a filter
                // would soften the terrain's finest lines for nothing.
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::NEAREST as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::NEAREST as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_S,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_T,
                    glow::CLAMP_TO_EDGE as i32,
                );
                Ok(texture)
            };
            let texture = make_texture()?;
            let resolved = make_texture()?;
            // The world is drawn in a depth buffer and never sampled from, so a
            // renderbuffer is enough. (A 16-bit one was measured and changes
            // nothing.)
            let renderbuffer = gl.create_renderbuffer()?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(renderbuffer));
            gl.renderbuffer_storage(glow::RENDERBUFFER, glow::DEPTH_COMPONENT24, size.0, size.1);

            let framebuffer = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(renderbuffer),
            );
            let resolved_framebuffer = gl.create_framebuffer()?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resolved_framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(resolved),
                0,
            );
            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            state.restore(gl);
            if status != glow::FRAMEBUFFER_COMPLETE {
                return Err(format!("incomplete framebuffer: {status:#x}"));
            }
            self.framebuffer = Some(framebuffer);
            self.color = Some(texture);
            self.depth = Some(renderbuffer);
            self.resolved_framebuffer = Some(resolved_framebuffer);
            self.resolved = Some(resolved);
        }
        Ok(())
    }

    /// Draws one frame. Returns whether the window now has a fresh world image.
    fn draw(&mut self, ui: &App, shared: &Shared) -> bool {
        // `resize` carries readable reasons — "incomplete framebuffer: …", "the
        // renderer is not using OpenGL" — and the identical error class *is*
        // logged on the first `Renderer::new`. Dropping it here made a window
        // resize or a scale change fail into the same silence as the first build.
        if let Err(error) = self.resize(target_size(ui)) {
            log::warn!(target: "background", "no GPU background: {error}");
            shared.gpu.set(Gpu::Failed);
            return false;
        }
        let (Some(framebuffer), Some(_), Some(resolved)) =
            (self.framebuffer, self.color, self.resolved)
        else {
            return false;
        };
        let background = ui.global::<Background>();
        // While a custom background covers the world there is nothing to draw:
        // the texture keeps the frame it has, and the layer's `visible` binding
        // decides whether Slint looks at it at all. Reporting no frame keeps the
        // handover honest — this renderer has not put anything on screen yet —
        // and the world is drawn on the first frame after it comes back.
        if !background.get_world_shown() {
            return false;
        }
        let crust = background.get_crust();
        let bg = [
            f32::from(crust.red()) / 255.0,
            f32::from(crust.green()) / 255.0,
            f32::from(crust.blue()) / 255.0,
        ];
        // Line colour: every flavour but Latte outlines in white
        // (`const c = latte ? 0 : 255`).
        let edge = if background.get_dark() {
            [1.0, 1.0, 1.0]
        } else {
            [0.0, 0.0, 0.0]
        };

        let (width, height) = (self.size.0 as f32, self.size.1 as f32);
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        let focal = half_w / scene::FOV_HALF_TAN;
        let aspect = height / width;

        // The geometry only changes when the camera crosses a block boundary or
        // the window does; between rebuilds this is a no-op.
        let cam_z = self.advance(shared);
        let started = Instant::now();
        if self.scene.update(cam_z, aspect, true) {
            self.upload();
            log::debug!(
                target: "background",
                "world rebuilt in {:.1}ms — {} faces, {} outlines",
                started.elapsed().as_secs_f64() * 1000.0,
                self.scene.faces.len(),
                self.scene.edge_vertices.len() / 8 / 6,
            );
        }

        let state = State::save(&self.gl);
        unsafe {
            let gl = &self.gl;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.viewport(0, 0, self.size.0, self.size.1);
            gl.disable(glow::SCISSOR_TEST);
            // Transparent, so the sky layer under this one shows through —
            // `clear_color(0, 0, 0, 0)`. The depth mask is set first,
            // because the outline pass leaves it off and a frame that cannot
            // clear its depth buffer would let the ground quad fail `LEQUAL`
            // against the previous frame's depth.
            gl.depth_mask(true);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.color_mask(true, true, true, true);

            // The program has to be current before its uniforms are set —
            // `glUniform*` writes into whatever is bound.
            gl.use_program(Some(self.fill.id));
            self.set_common_uniforms(&self.fill, cam_z, focal, half_w, half_h);
            gl.uniform_4_f32(self.fill.uniform("uBgColor"), bg[0], bg[1], bg[2], 1.0);
            gl.uniform_1_f32(self.fill.uniform("uLayerAlpha"), scene::LAYER_ALPHA);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LEQUAL);
            gl.depth_mask(true);
            gl.bind_vertex_array(Some(self.fill_vao));
            gl.draw_elements(
                glow::TRIANGLES,
                self.scene.fill_indices.len() as i32,
                glow::UNSIGNED_INT,
                0,
            );

            gl.use_program(Some(self.edge.id));
            self.set_common_uniforms(&self.edge, cam_z, focal, half_w, half_h);
            gl.uniform_4_f32(self.edge.uniform("uBgColor"), bg[0], bg[1], bg[2], 1.0);
            gl.uniform_4_f32(
                self.edge.uniform("uEdgeColor"),
                edge[0],
                edge[1],
                edge[2],
                1.0,
            );
            gl.uniform_1_f32(self.edge.uniform("uLayerAlpha"), scene::LAYER_ALPHA);
            gl.uniform_1_f32(self.edge.uniform("uEdgeWidth"), scene::EDGE_WIDTH_DEVICE_PX);
            gl.disable(glow::BLEND);
            gl.depth_func(glow::LEQUAL);
            gl.depth_mask(false);
            gl.bind_vertex_array(Some(self.edge_vao));
            gl.draw_arrays(
                glow::TRIANGLES,
                0,
                (self.scene.edge_vertices.len() / 8) as i32,
            );

            // Un-premultiply into the texture Slint samples (see
            // `RESOLVE_FRAGMENT`). Blending off: every pixel is replaced.
            if let (Some(framebuffer), Some(color)) = (self.resolved_framebuffer, self.color) {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
                gl.disable(glow::BLEND);
                gl.disable(glow::DEPTH_TEST);
                gl.depth_mask(true);
                gl.use_program(Some(self.resolve.id));
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(color));
                gl.uniform_1_i32(self.resolve.uniform("uWorld"), 0);
                gl.bind_vertex_array(Some(self.resolve_vao));
                gl.draw_arrays(glow::TRIANGLES, 0, 3);
            }

            if !self.dumped {
                self.dumped = true;
                self.dump(cam_z);
            }
        }
        state.restore(&self.gl);

        // The image is only rebuilt when the texture is: Slint composites the
        // same borrowed texture every frame, and its contents are what change.
        if self.image.is_none() {
            let Some(id) = NonZeroU32::new(resolved.0.get()) else {
                // Permanent for this context: `draw` answers false from here on
                // and `Gpu` never reaches `Running`, so the controller logs "no
                // OpenGL renderer" — a wrong diagnosis. This is the real one.
                log::warn!(
                    target: "background",
                    "the borrowed OpenGL texture handle is not valid; the GPU world cannot \
                     be drawn"
                );
                return false;
            };
            // Safe: the texture is ours and belongs to the context that is
            // current in this callback.
            let image = unsafe {
                BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
                    id,
                    Size2D::new(self.size.0 as u32, self.size.1 as u32),
                )
            }
            // GL writes its first row at the bottom of the viewport, so that is
            // where the texture's data starts.
            .origin(BorrowedOpenGLTextureOrigin::BottomLeft)
            .build();
            self.image = Some(image.clone());
            background.set_world_image(image);
        }

        let frames = shared.frames.get() + 1;
        shared.frames.set(frames);
        if log::log_enabled!(target: "background", log::Level::Debug) && frames.is_multiple_of(120)
        {
            log::debug!(
                target: "background",
                "world drawn at {:.1} fps over {:.1}s ({} faces)",
                frames as f64 / self.started.elapsed().as_secs_f64(),
                self.started.elapsed().as_secs_f64(),
                self.scene.faces.len(),
            );
        }
        true
    }

    /// Writes the frame this renderer just drew to `CONIC_GL_DUMP`, so the
    /// GPU's world can be inspected as a PNG without a window.
    ///
    /// The frame is read back from the framebuffer rather than grabbed off the
    /// window, so what is compared is the world on its own — no layer opacity,
    /// no rounding, no UI over it. Only the first frame is written, which with
    /// `CONIC_GL_CAM_Z` pinned is the frame worth looking at.
    #[cfg(debug_assertions)]
    fn dump(&self, cam_z: f32) {
        let Ok(path) = std::env::var("CONIC_GL_DUMP") else {
            return;
        };
        let (width, height) = (self.size.0 as u32, self.size.1 as u32);
        let stride = (width * 4) as usize;
        let mut pixels = vec![0u8; stride * height as usize];
        unsafe {
            self.gl
                .bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer);
            self.gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        // What GL hands back starts at the bottom row; an image starts at the
        // top one.
        let mut flipped = vec![0u8; pixels.len()];
        for row in 0..height as usize {
            let source = &pixels[row * stride..(row + 1) * stride];
            let target = (height as usize - 1 - row) * stride;
            flipped[target..target + stride].copy_from_slice(source);
        }
        match image::save_buffer(&path, &flipped, width, height, image::ColorType::Rgba8) {
            Ok(()) => log::info!(
                target: "background",
                "wrote {width}x{height} at camera {cam_z} to {path}"
            ),
            Err(error) => log::warn!(target: "background", "could not write {path}: {error}"),
        }
    }

    #[cfg(not(debug_assertions))]
    fn dump(&self, _cam_z: f32) {}

    /// Steps the camera for this frame, by the time since the last one. Nothing
    /// else moves it: a window that is not being repainted — occluded, or a
    /// compositor that has stopped asking — does not move the world either, so
    /// it is where it was when the frames come back rather than a jump ahead.
    fn advance(&mut self, shared: &Shared) -> f32 {
        #[cfg(debug_assertions)]
        if let Ok(pinned) = std::env::var("CONIC_GL_CAM_Z")
            && let Ok(pinned) = pinned.trim().parse()
        {
            return pinned;
        }
        let now = Instant::now();
        let dt = self
            .last_draw
            .map(|last| (now - last).as_secs_f32().clamp(0.0, 0.05))
            .unwrap_or(0.0);
        self.last_draw = Some(now);
        let cam_z = shared.cam_z.get()
            + if shared.moving.get() {
                CAMERA_SPEED * dt
            } else {
                0.0
            };
        shared.cam_z.set(cam_z);
        cam_z
    }

    /// Uploads the buffers [`Scene`] has just rebuilt.
    fn upload(&mut self) {
        unsafe {
            let gl = &self.gl;
            let previous_array_buffer = gl.get_parameter_i32(glow::ARRAY_BUFFER_BINDING);
            let previous_element_buffer = gl.get_parameter_i32(glow::ELEMENT_ARRAY_BUFFER_BINDING);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.fill_vertices));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&self.scene.fill_vertices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.fill_indices));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&self.scene.fill_indices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.edge_vertices));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&self.scene.edge_vertices),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(
                glow::ARRAY_BUFFER,
                NonZeroU32::new(previous_array_buffer as u32).map(glow::NativeBuffer),
            );
            gl.bind_buffer(
                glow::ELEMENT_ARRAY_BUFFER,
                NonZeroU32::new(previous_element_buffer as u32).map(glow::NativeBuffer),
            );
        }
    }

    /// The camera's half of the uniforms. Both programs name these the same, so
    /// the two passes are set up by the same code — with the program already
    /// current, which is what makes the writes land.
    fn set_common_uniforms(
        &self,
        program: &Program,
        cam_z: f32,
        focal: f32,
        half_w: f32,
        half_h: f32,
    ) {
        unsafe {
            let gl = &self.gl;
            gl.uniform_1_f32(program.uniform("uCamX"), scene::CAM_X);
            gl.uniform_1_f32(program.uniform("uCamY"), scene::CAM_Y);
            gl.uniform_1_f32(program.uniform("uCamZ"), cam_z);
            gl.uniform_1_f32(program.uniform("uFocal"), focal);
            gl.uniform_1_f32(program.uniform("uHalfW"), half_w);
            gl.uniform_1_f32(program.uniform("uHalfH"), half_h);
            gl.uniform_1_f32(program.uniform("uNearZ"), scene::NEAR_PLANE);
            gl.uniform_1_f32(program.uniform("uFarZ"), scene::FADE_END);
        }
    }
}
