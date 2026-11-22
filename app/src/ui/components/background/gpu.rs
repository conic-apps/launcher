// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The 3D world, drawn on the GPU through Slint's rendering notifier.
//!
//! The notifier hands over the wgpu device and queue Skia itself draws with, so
//! the world renders into a texture of our own on that device and Slint
//! composites the result as an image — the GPU work stays in one submission of
//! one queue, and nothing has to be read back. A surface that is not wgpu (Skia's
//! software fallback) has no device to give: the CPU rasteriser keeps the
//! background and this module reports [`Gpu::Failed`].

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

use slint::{ComponentHandle, GraphicsAPI, Image, RenderingState};

use super::scene::{self, CAMERA_SPEED, Scene};
use crate::slint_backend::{App, Background};

/// The parallax wrapper is scaled up by this so the images land 1:1 on device
/// pixels.
const WRAPPER_SCALE: f32 = 1.08;

/// A live resize changes the window size every frame. Recreating the color,
/// depth and resolve textures at each of those sizes stalls the GPU in the
/// middle of the drag, so the targets keep their old size and the texture is
/// stretched until the window has been still for this long.
const RESIZE_SETTLE: Duration = Duration::from_millis(350);

/// Whether the GPU renderer is drawing the world. The CPU rasteriser keeps
/// drawing until the first GPU frame reaches the window, so a renderer without
/// wgpu leaves the fallback already on screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Gpu {
    Pending,
    Running,
    Failed,
}

/// The handover flag. The render callback and controller share the UI thread,
/// but the rasteriser's worker reads it too when a late frame arrives, so it is
/// an atomic behind an `Arc`.
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

/// What the render callback and the controller share; they never borrow each
/// other, so these are cells rather than part of the controller's `RefCell`.
pub struct Shared {
    pub gpu: Arc<Flag>,
    /// The GPU advances the camera as it draws, so a frame that never happens
    /// also never moves it.
    pub cam_z: Cell<f32>,
    pub moving: Cell<bool>,
    pub frames: Cell<u64>,
    /// A live resize is waiting for the targets to catch up; the controller
    /// keeps asking for frames until it does.
    pub resizing: Cell<bool>,
}

impl Shared {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            gpu: Flag::new(),
            cam_z: Cell::new(0.0),
            moving: Cell::new(false),
            frames: Cell::new(0),
            resizing: Cell::new(false),
        })
    }
}

pub type Handle = Rc<Shared>;

thread_local! {
    static RENDERER: RefCell<Option<Renderer>> = const { RefCell::new(None) };
}

/// Registers the notifier, and reports whether the renderer took it. A device
/// only arrives with the first frame, so a renderer that hands out none is
/// reported through [`Gpu::Failed`] rather than here.
pub fn install(ui: &App, shared: Handle) -> bool {
    // Exercises the rasteriser's path on a machine that does have a GPU.
    #[cfg(debug_assertions)]
    if std::env::var("CONIC_FORCE_CPU").is_ok() {
        return false;
    }
    let weak = ui.as_weak();
    let result = ui.window().set_rendering_notifier(move |state, api| {
        let Some(ui) = weak.upgrade() else { return };
        match state {
            RenderingState::RenderingSetup => {
                // The image in the layer was made on the previous device, so the
                // layer falls back to the sky until the first draw hands over
                // one from this device.
                ui.global::<Background>().set_world_image(Image::default());
                let GraphicsAPI::WGPU30 { device, queue, .. } = api else {
                    log::debug!(
                        target: "background",
                        "the renderer is not drawing through wgpu; the CPU rasteriser keeps \
                         the world",
                    );
                    shared.gpu.set(Gpu::Failed);
                    return;
                };
                let size = target_size(&ui);
                log::info!(
                    target: "background",
                    "drawing the world on the GPU at {}x{}",
                    size.0,
                    size.1,
                );
                let mut renderer = Renderer::new(device, queue, size);
                // A frame here as well as in one paint: wgpu hands out no
                // drawable for a window the system considers occluded, so the
                // paint that made this surface may be the only one for a while
                // and the handover cannot wait for the next.
                if renderer.draw(&ui, &shared) {
                    shared.gpu.set(Gpu::Running);
                }
                RENDERER.with(|slot| *slot.borrow_mut() = Some(renderer));
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
                // Dropping the renderer releases the device and queue this
                // module holds a reference to; the surface they belong to is
                // released right after this returns.
                RENDERER.with(|slot| *slot.borrow_mut() = None);
            }
            _ => {}
        }
    });
    // A renderer with no wgpu device is the ordinary "no GPU here" case, so this
    // is a debug message rather than a warning.
    let installed = result.is_ok();
    if !installed {
        log::debug!(target: "background", "the renderer takes no notifier; using the CPU path");
    }
    installed
}

/// The window's physical size times the wrapper's scale, with no downscaling:
/// the GPU does not care how many pixels these are.
fn target_size(ui: &App) -> (u32, u32) {
    let size = ui.window().size();
    (
        (size.width as f32 * WRAPPER_SCALE).round().max(1.0) as u32,
        (size.height as f32 * WRAPPER_SCALE).round().max(1.0) as u32,
    )
}

// The layer's 0.3 opacity is folded into the shader's passes: this image is one
// layer of Slint's composite rather than an element of its own, and Slint
// rounds the window's corners when it composites.
//
// The passes live in `shaders/world.wgsl` so that an editor highlights them, a
// diff shows the line that changed and a grep finds the pass that uses a
// uniform. `include_str!` keeps them inside the binary, so there is still no
// asset to ship or to find at runtime.
const WORLD: &str = include_str!("shaders/world.wgsl");

const FILL_ENTRY_VERTEX: &str = "fill_vertex";
const FILL_ENTRY_FRAGMENT: &str = "fill_fragment";
const EDGE_ENTRY_VERTEX: &str = "edge_vertex";
const EDGE_ENTRY_FRAGMENT: &str = "edge_fragment";
const RESOLVE_ENTRY_VERTEX: &str = "resolve_vertex";
const RESOLVE_ENTRY_FRAGMENT: &str = "resolve_fragment";

const FILL_STRIDE: u64 = 16;
const EDGE_STRIDE: u64 = 32;

/// `(position, alpha)` per fill vertex, and `(position, blend, other, side)` per
/// outline vertex: the same interleaving `scene` builds.
const FILL_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32];
const EDGE_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x3, 3 => Float32];

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// One frame's uniforms: the camera, the palette and the layer's opacity, in the
/// order and the alignment `Globals` in `world.wgsl` declares them.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    /// x, y and z of the camera, then its focal length in pixels.
    cam: [f32; 4],
    /// Half the target's width and height, then the near and far planes.
    plane: [f32; 4],
    crust: [f32; 4],
    outline: [f32; 4],
    /// The layer's opacity, then the outline's width in device pixels.
    opacity: [f32; 4],
}

/// A texture and the view every pass that draws into it binds.
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn new(device: &wgpu::Device, size: (u32, u32), label: &str) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            // `TEXTURE_BINDING` is what lets Slint sample the resolved one;
            // `COPY_SRC` is what the debug dump reads the frame back through.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

/// A vertex or index buffer that grows to fit the geometry instead of being
/// rebuilt: the world changes size only when the view distance does.
struct Storage {
    buffer: wgpu::Buffer,
    label: &'static str,
    usage: wgpu::BufferUsages,
    capacity: u64,
}

impl Storage {
    fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages) -> Self {
        let capacity = 1024;
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: capacity,
                usage,
                mapped_at_creation: false,
            }),
            label,
            usage,
            capacity,
        }
    }

    fn write(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) {
        if bytes.is_empty() {
            // Nothing is drawn from an empty buffer, so its old contents are
            // nobody's business.
            return;
        }
        let needed = bytes.len() as u64;
        if needed > self.capacity {
            self.capacity = needed.next_power_of_two();
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: self.capacity,
                usage: self.usage,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.buffer, 0, bytes);
    }
}

struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: (u32, u32),
    fill_pipeline: wgpu::RenderPipeline,
    edge_pipeline: wgpu::RenderPipeline,
    resolve_pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    /// The resolve pass's bind group; it names the texture being resolved, so it
    /// is rebuilt whenever that texture is.
    resolve_layout: wgpu::BindGroupLayout,
    resolve_bind_group: Option<wgpu::BindGroup>,
    sampler: wgpu::Sampler,
    fill_vertices: Storage,
    fill_indices: Storage,
    edge_vertices: Storage,
    /// The premultiplied world, then its straight-alpha copy that Slint samples.
    color: Option<Target>,
    resolved: Option<Target>,
    depth: Option<wgpu::TextureView>,
    image: Option<Image>,
    scene: Scene,
    started: Instant,
    last_draw: Option<Instant>,
    /// The size the window most recently asked for, and when it first asked for
    /// it. The targets are recreated only after the resize settles.
    pending_size: Option<(u32, u32)>,
    pending_since: Instant,
    dumped: bool,
}

impl Renderer {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) -> Self {
        let device = device.clone();
        let queue = queue.clone();

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("world"),
            source: wgpu::ShaderSource::Wgsl(WORLD.into()),
        });

        // The camera block, bound to every pass that projects something.
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });

        let resolve_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world resolve"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world"),
            bind_group_layouts: &[Some(&globals_layout)],
            immediate_size: 0,
        });
        let resolve_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("world resolve"),
                bind_group_layouts: &[Some(&resolve_layout)],
                immediate_size: 0,
            });

        let color_target = [Some(wgpu::ColorTargetState {
            format: COLOR_FORMAT,
            // The fill writes premultiplied colour, so the blend is the
            // premultiplied one: the faces stack over each other without a seam.
            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let opaque_target = [Some(wgpu::ColorTargetState {
            format: COLOR_FORMAT,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];

        let fill_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world fill"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some(FILL_ENTRY_VERTEX),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: FILL_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &FILL_ATTRIBUTES,
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(FILL_ENTRY_FRAGMENT),
                compilation_options: Default::default(),
                targets: &color_target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let edge_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world edge"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some(EDGE_ENTRY_VERTEX),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: EDGE_STRIDE,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &EDGE_ATTRIBUTES,
                })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                // The outlines test against the faces but must not occlude each
                // other, or a line drawn behind one would win on a tie.
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(EDGE_ENTRY_FRAGMENT),
                compilation_options: Default::default(),
                targets: &opaque_target,
            }),
            multiview_mask: None,
            cache: None,
        });
        let resolve_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world resolve"),
            layout: Some(&resolve_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some(RESOLVE_ENTRY_VERTEX),
                compilation_options: Default::default(),
                // The full-screen triangle comes from the vertex index alone.
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(RESOLVE_ENTRY_FRAGMENT),
                compilation_options: Default::default(),
                targets: &opaque_target,
            }),
            multiview_mask: None,
            cache: None,
        });

        // Nearest: the image lands on the wrapper 1:1, and a filter would soften
        // the terrain's finest lines for nothing.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("world resolve"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let mut renderer = Self {
            fill_pipeline,
            edge_pipeline,
            resolve_pipeline,
            globals,
            globals_bind_group,
            resolve_layout,
            resolve_bind_group: None,
            sampler,
            fill_vertices: Storage::new(
                &device,
                "world fill vertices",
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            fill_indices: Storage::new(
                &device,
                "world fill indices",
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            ),
            edge_vertices: Storage::new(
                &device,
                "world edge vertices",
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            device,
            queue,
            size: (0, 0),
            color: None,
            resolved: None,
            depth: None,
            image: None,
            scene: Scene::new(),
            started: Instant::now(),
            last_draw: None,
            pending_size: None,
            pending_since: Instant::now(),
            dumped: false,
        };
        renderer.resize(size);
        renderer
    }

    fn resize(&mut self, size: (u32, u32)) {
        if self.size == size && self.color.is_some() {
            return;
        }
        self.size = size;
        self.image = None;
        let color = Target::new(&self.device, size, "world");
        let resolved = Target::new(&self.device, size, "world resolve");
        let depth = self
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("world depth"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.resolve_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world resolve"),
            layout: &self.resolve_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        }));
        self.color = Some(color);
        self.resolved = Some(resolved);
        self.depth = Some(depth);
        log::debug!(
            target: "background",
            "GPU targets resized to {}x{}",
            size.0,
            size.1,
        );
    }

    fn draw(&mut self, ui: &App, shared: &Shared) -> bool {
        let target = target_size(ui);
        if target != self.size {
            if self.pending_size != Some(target) {
                self.pending_size = Some(target);
                self.pending_since = Instant::now();
            }
            shared.resizing.set(true);
            if self.pending_since.elapsed() >= RESIZE_SETTLE {
                self.resize(target);
                self.pending_size = None;
                shared.resizing.set(false);
            }
        } else {
            self.pending_size = None;
            shared.resizing.set(false);
        }

        let background = ui.global::<Background>();

        // While a custom background covers the world there is nothing to draw,
        // and reporting no frame keeps the handover honest.
        if !background.get_world_shown() {
            return false;
        }
        let crust = background.get_crust();
        let crust = [
            f32::from(crust.red()) / 255.0,
            f32::from(crust.green()) / 255.0,
            f32::from(crust.blue()) / 255.0,
        ];
        // Line colour: every flavour but Latte outlines in white
        // (`const c = latte ? 0 : 255`).
        let outline = if background.get_dark() {
            [1.0, 1.0, 1.0]
        } else {
            [0.0, 0.0, 0.0]
        };

        let (width, height) = (self.size.0 as f32, self.size.1 as f32);
        let aspect = height / width;

        let cam_z = self.advance(shared);
        let started = Instant::now();
        match self.scene.update(cam_z, aspect, true) {
            scene::Rebuilt::Nothing => {}
            scene::Rebuilt::Ground => self.upload_ground(),
            scene::Rebuilt::World => {
                self.upload();
                log::debug!(
                    target: "background",
                    "world rebuilt in {:.1}ms — {} faces, {} outlines",
                    started.elapsed().as_secs_f64() * 1000.0,
                    self.scene.faces.len(),
                    self.scene.edge_vertices.len() / 8 / 6,
                );
            }
        }

        let globals = Globals {
            cam: [
                scene::CAM_X,
                scene::CAM_Y,
                cam_z,
                width / 2.0 / scene::FOV_HALF_TAN,
            ],
            plane: [
                width / 2.0,
                height / 2.0,
                scene::NEAR_PLANE,
                scene::FADE_END,
            ],
            crust: [crust[0], crust[1], crust[2], 1.0],
            outline: [outline[0], outline[1], outline[2], 1.0],
            opacity: [scene::LAYER_ALPHA, scene::EDGE_WIDTH_DEVICE_PX, 0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        let (Some(color), Some(depth), Some(resolve_bind_group)) = (
            self.color.as_ref(),
            self.depth.as_ref(),
            self.resolve_bind_group.as_ref(),
        ) else {
            return false;
        };
        let Some(resolved) = self.resolved.as_ref() else {
            return false;
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("world"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color.view,
                    depth_slice: None,
                    resolve_target: None,
                    // Transparent, so the sky layer under this one shows
                    // through.
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            if !self.scene.fill_indices.is_empty() {
                pass.set_pipeline(&self.fill_pipeline);
                pass.set_vertex_buffer(0, self.fill_vertices.buffer.slice(..));
                pass.set_index_buffer(
                    self.fill_indices.buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(0..self.scene.fill_indices.len() as u32, 0, 0..1);
            }
            if !self.scene.edge_vertices.is_empty() {
                pass.set_pipeline(&self.edge_pipeline);
                pass.set_vertex_buffer(0, self.edge_vertices.buffer.slice(..));
                pass.draw(0..(self.scene.edge_vertices.len() / 8) as u32, 0..1);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world resolve"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &resolved.view,
                    depth_slice: None,
                    resolve_target: None,
                    // Every pixel is replaced, so what the clear wrote is
                    // irrelevant.
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.resolve_pipeline);
            pass.set_bind_group(0, resolve_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        // Skia samples this texture with an image it imported in the sampling
        // layout, so the frame ends where that import expects it: wgpu's own
        // record of the layout has to match what is really in memory.
        encoder.transition_resources(
            std::iter::empty(),
            std::iter::once(wgpu::TextureTransition {
                texture: &resolved.texture,
                selector: None,
                state: wgpu::TextureUses::RESOURCE,
            }),
        );
        self.queue.submit(Some(encoder.finish()));

        if !self.dumped {
            self.dumped = true;
            self.dump(cam_z);
        }

        if self.image.is_none() {
            match Image::try_from(resolved.texture.clone()) {
                Ok(image) => {
                    self.image = Some(image.clone());
                    background.set_world_image(image);
                }
                Err(error) => {
                    // Permanent for this device, and the controller would
                    // otherwise report a missing GPU renderer.
                    log::warn!(
                        target: "background",
                        "the world texture cannot be handed to Slint: {error}"
                    );
                    return false;
                }
            }
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

    /// Writes the first frame this renderer drew to `CONIC_GPU_DUMP`, read back
    /// from the resolved texture so it is the world alone without the UI over
    /// it.
    #[cfg(debug_assertions)]
    fn dump(&self, cam_z: f32) {
        let Ok(path) = std::env::var("CONIC_GPU_DUMP") else {
            return;
        };
        let Some(resolved) = self.resolved.as_ref() else {
            return;
        };
        let (width, height) = self.size;
        // A texture-to-buffer copy's rows are padded to this alignment and the
        // padding is not part of the image.
        let row = width * 4;
        let padded =
            row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world dump"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("world dump"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &resolved.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        match receiver.recv() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                log::warn!(target: "background", "could not map {path}: {error}");
                return;
            }
            Err(error) => {
                log::warn!(target: "background", "the {path} readback never finished: {error}");
                return;
            }
        }
        let mapped = match slice.get_mapped_range() {
            Ok(mapped) => mapped,
            Err(error) => {
                log::warn!(target: "background", "could not read {path} back: {error}");
                return;
            }
        };
        let mut pixels = Vec::with_capacity((row * height) as usize);
        for line in mapped.chunks_exact(padded as usize) {
            pixels.extend_from_slice(&line[..row as usize]);
        }
        drop(mapped);
        buffer.unmap();
        match image::save_buffer(&path, &pixels, width, height, image::ColorType::Rgba8) {
            Ok(()) => log::info!(
                target: "background",
                "wrote {width}x{height} at camera {cam_z} to {path}"
            ),
            Err(error) => log::warn!(target: "background", "could not write {path}: {error}"),
        }
    }

    #[cfg(not(debug_assertions))]
    fn dump(&self, _cam_z: f32) {}

    /// Steps the camera by the time since the last frame, so a window nobody is
    /// repainting does not move the world either.
    fn advance(&mut self, shared: &Shared) -> f32 {
        #[cfg(debug_assertions)]
        if let Ok(pinned) = std::env::var("CONIC_GPU_CAM_Z")
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

    /// An aspect change moves only the four ground vertices; the terrain
    /// buffers stay exactly as they were.
    fn upload_ground(&mut self) {
        let ground = bytemuck::cast_slice::<f32, u8>(&self.scene.fill_vertices[..16]);
        self.fill_vertices.write(&self.device, &self.queue, ground);
    }

    fn upload(&mut self) {
        let vertices = bytemuck::cast_slice::<f32, u8>(&self.scene.fill_vertices);
        self.fill_vertices
            .write(&self.device, &self.queue, vertices);
        let indices = bytemuck::cast_slice::<u32, u8>(&self.scene.fill_indices);
        self.fill_indices.write(&self.device, &self.queue, indices);
        let edges = bytemuck::cast_slice::<f32, u8>(&self.scene.edge_vertices);
        self.edge_vertices.write(&self.device, &self.queue, edges);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every entry point the pipelines name, as the shader declares it: a
    /// mistyped name is a pipeline that does not exist by the time the window
    /// first draws.
    #[test]
    fn every_pipeline_entry_point_is_declared() {
        for (stage, name) in [
            ("vertex", FILL_ENTRY_VERTEX),
            ("fragment", FILL_ENTRY_FRAGMENT),
            ("vertex", EDGE_ENTRY_VERTEX),
            ("fragment", EDGE_ENTRY_FRAGMENT),
            ("vertex", RESOLVE_ENTRY_VERTEX),
            ("fragment", RESOLVE_ENTRY_FRAGMENT),
        ] {
            assert!(
                WORLD.contains(&format!("@{stage}\nfn {name}(")),
                "`{name}` is not a {stage} entry point"
            );
        }
    }

    /// The uniform is byte-cast into the buffer, so the shader's block and the
    /// Rust struct have to describe the same memory: five `vec4`s, in order.
    #[test]
    fn the_globals_match_the_shaders_block() {
        let globals = <Globals as bytemuck::Zeroable>::zeroed();
        assert_eq!(std::mem::size_of::<Globals>(), 5 * 16);
        let base = std::ptr::addr_of!(globals) as usize;
        let fields = [
            ("cam", std::ptr::addr_of!(globals.cam) as usize),
            ("plane", std::ptr::addr_of!(globals.plane) as usize),
            ("crust", std::ptr::addr_of!(globals.crust) as usize),
            ("outline", std::ptr::addr_of!(globals.outline) as usize),
            ("opacity", std::ptr::addr_of!(globals.opacity) as usize),
        ];
        for (index, (name, offset)) in fields.into_iter().enumerate() {
            assert_eq!(offset - base, index * 16, "`{name}` is not at its slot");
            assert!(
                WORLD.contains(&format!("{name}: vec4<f32>")),
                "the shader has no `{name}` vec4"
            );
        }
    }

    /// `scene` builds the vertex streams the pipelines describe: a stride is the
    /// distance between two vertices, so it is the sum of the attributes' sizes.
    #[test]
    fn the_vertex_strides_match_the_attributes() {
        let span = |attributes: &[wgpu::VertexAttribute]| {
            attributes
                .iter()
                .map(|attribute| attribute.offset + attribute.format.size())
                .max()
                .unwrap_or(0)
        };
        assert_eq!(span(&FILL_ATTRIBUTES), FILL_STRIDE);
        assert_eq!(span(&EDGE_ATTRIBUTES), EDGE_STRIDE);
        assert_eq!(
            std::mem::size_of::<[f32; 4]>(),
            FILL_STRIDE as usize,
            "a fill vertex is four floats"
        );
        assert_eq!(
            std::mem::size_of::<[f32; 8]>(),
            EDGE_STRIDE as usize,
            "an outline vertex is eight floats"
        );
    }
}
