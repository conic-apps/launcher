// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Drives the window background: source resolution, the cross-fade between
//! images, the camera clock and the parallax. Rendering runs on its own thread.

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime},
};

use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

use super::gl::{self, Gpu};
use super::scene::CAMERA_SPEED;
use super::sky::{self, SkyRequest};
use super::world::{FrameRequest, WorldRenderer};
use crate::slint_backend::{App, Background};
use crate::support::runtime;

/// The pixels one world frame is allowed to cost; the window is scaled down to
/// stay under it. The default window gets full resolution, a 2x or maximised one
/// loses a little sharpness.
const WORLD_PIXEL_BUDGET: u32 = 1_400_000;

/// Faster than any display: the camera step comes from real elapsed time, not
/// from the tick interval.
const TICK_MS: u64 = 8;

/// A resize settles for this long before the sky is redrawn at the new size.
/// The old image stretches in the meantime; a full-resolution sky pass for every
/// intermediate size is what makes a live resize crawl. The GPU framebuffer
/// uses the same settle, so the whole background updates once after the drag.
const SKY_SETTLE_MS: u64 = 350;

const FADE_MS: f32 = 400.0;

/// How many decoded backgrounds are kept. The fixed on-disk names mean a path
/// seen before is normally the same picture, so a kept decode makes clicking
/// back through instances instant. The bound keeps a long session from holding
/// every wallpaper it ever showed.
const CACHE_LIMIT: usize = 6;

/// Clicking through instances quickly should not start a fade per click.
const BURST_MS: u64 = 500;
const DEBOUNCE_MS: u64 = 200;

const MAX_OFFSET: f32 = 4.0;
/// The parallax time constant: every change eases, so a pointer flicking in and
/// out moves the background a little and brings it back rather than flashing.
const PARALLAX_EASE_SECS: f32 = 0.035;
/// The wrapper is scaled up by this so the images land 1:1 on device pixels.
const WRAPPER_SCALE: f32 = 1.08;

/// The longest edge a background image is kept at, bounding the memory an 8K
/// wallpaper costs.
const IMAGE_MAX_EDGE: u32 = 2560;

thread_local! {
    static CONTROLLER: RefCell<Option<Rc<RefCell<Controller>>>> = const { RefCell::new(None) };
    /// The pointer in physical window coordinates, `None` once it leaves.
    static POINTER: RefCell<Option<(f32, f32)>> = const { RefCell::new(None) };
}

/// The image's `modified` timestamp is part of its identity: the file is always
/// stored under a fixed name, so comparing only the path would miss a replaced
/// wallpaper.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Source {
    World,
    /// `global` marks the Settings → Appearance image, which decides whose
    /// darkness percentage dims it.
    Image {
        path: PathBuf,
        global: bool,
        modified: SystemTime,
    },
}

#[derive(Clone, Copy, PartialEq)]
enum Alpha {
    Opaque,
    Translucent,
    /// Not decoded yet: assumed translucent, so the world is kept behind it.
    Unknown,
}

/// One of the two custom-background slots.
#[derive(Clone)]
struct Slot {
    source: Option<Source>,
    image: Image,
    alpha: Alpha,
    /// The opacity last written to the window, and where the slot is heading.
    /// A fade interpolates `opacity` from the value it had when the fade began
    /// to `target`; at rest the two are equal.
    opacity: f32,
    target: f32,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            source: None,
            image: Image::default(),
            alpha: Alpha::Unknown,
            opacity: 0.0,
            target: 0.0,
        }
    }
}

/// A cross-fade in flight. Both halves share one clock so they move together;
/// either may be absent when only one side is an image.
#[derive(Clone, Copy)]
struct Fade {
    incoming: Option<usize>,
    outgoing: Option<usize>,
    incoming_from: f32,
    outgoing_from: f32,
    started: Instant,
}

fn ease_in_out(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The image [`Source`] for a path that is a readable file, carrying the
/// timestamp that decides whether a later resolution is the same picture.
fn image_source(path: PathBuf, global: bool) -> Option<Source> {
    // The user configured this picture, so failing to stat it is worth saying:
    // the fallback to the world would otherwise hide that the setting did nothing.
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            log::warn!(
                target: "background",
                "the configured background at {} cannot be read: {error}",
                path.display()
            );
            return None;
        }
    };
    if !metadata.is_file() {
        return None;
    }
    // A filesystem without a usable mtime still shows the image; it just cannot
    // detect a replacement at the same path.
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    Some(Source::Image {
        path,
        global,
        modified,
    })
}

/// One decoded background, with the timestamp it was decoded from. `Image` is a
/// ref-counted handle, so this and a showing slot share the pixels.
struct Cached {
    modified: SystemTime,
    alpha: Alpha,
    image: Image,
}

/// The decoded backgrounds, oldest evicted first.
#[derive(Default)]
struct ImageCache {
    entries: HashMap<PathBuf, Cached>,
    order: VecDeque<PathBuf>,
}

impl ImageCache {
    /// `None` when the file on disk is no longer the one that was decoded.
    fn get(&self, path: &Path, modified: SystemTime) -> Option<&Cached> {
        self.entries
            .get(path)
            .filter(|cached| cached.modified == modified)
    }

    fn insert(&mut self, path: PathBuf, cached: Cached) {
        if self.entries.insert(path.clone(), cached).is_none() {
            self.order.push_back(path);
        }
        while self.order.len() > CACHE_LIMIT {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
    }
}

/// What the renderer needs for one frame; everything in it is read on the UI
/// thread, because Slint is not thread-safe.
struct Request {
    width: u32,
    height: u32,
    edge_scale: f32,
    sky_scale: f32,
    cam_z: f32,
    crust: [f32; 3],
    dark: bool,
    /// Once the GPU is drawing the world only the sky is still ours, and a frame
    /// is worth asking for only when the sky's cache key has moved.
    draw_world: bool,
}

/// `Image` cannot be built off the drawing thread, so buffers travel back and
/// the UI makes the images.
struct RenderResult {
    world: Option<SharedPixelBuffer<Rgba8Pixel>>,
    sky: Option<SharedPixelBuffer<Rgba8Pixel>>,
}

struct Renderer {
    requests: mpsc::SyncSender<Request>,
    busy: Arc<AtomicBool>,
}

impl Renderer {
    fn spawn(ui: &App, dispatch_state: &Arc<gl::Flag>) -> Self {
        let (requests, incoming) = mpsc::sync_channel::<Request>(1);
        let busy = Arc::new(AtomicBool::new(false));
        let weak = ui.as_weak();
        let worker_busy = Arc::clone(&busy);
        // Read from the worker's delivering closure too, so a frame arriving
        // after the handover cannot put the CPU's world back on screen.
        let worker_state = Arc::clone(dispatch_state);
        std::thread::Builder::new()
            .name("conic-background".into())
            .spawn(move || {
                let mut world = WorldRenderer::new();
                let mut sky = SkyCache::default();
                while let Ok(request) = incoming.recv() {
                    let frame = if request.draw_world {
                        Some(world.render(&FrameRequest {
                            width: request.width,
                            height: request.height,
                            edge_scale: request.edge_scale,
                            cam_z: request.cam_z,
                            crust: request.crust,
                            dark: request.dark,
                        }))
                    } else {
                        None
                    };
                    let image = sky.render_if_changed(&request);
                    let result = RenderResult {
                        world: frame,
                        sky: image,
                    };
                    let weak = weak.clone();
                    let busy = Arc::clone(&worker_busy);
                    let state = Arc::clone(&worker_state);
                    crate::ui::services::report::report(&weak, move |ui| {
                        // The GPU may have taken over while this frame was being
                        // rasterised; its texture is the one the layers show.
                        let gpu = state.get() == Gpu::Running;
                        let background = ui.global::<Background>();
                        if let (Some(world), false) = (result.world, gpu) {
                            background.set_world_image(Image::from_rgba8_premultiplied(world));
                        }
                        if let Some(sky) = result.sky {
                            background.set_sky_image(Image::from_rgba8_premultiplied(sky));
                        }
                        busy.store(false, Ordering::Relaxed);
                    });
                }
            })
            .map(|_| {
                log::debug!(target: "background", "the background renderer started");
            })
            .unwrap_or_else(|error| {
                // Without this the failure would only show as a frozen background.
                log::error!(
                    target: "background",
                    "the background renderer could not be started: {error}; the window \
                     background will not update"
                );
            });
        Self { requests, busy }
    }

    fn busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }

    /// A frame already in flight means this one is skipped; the caller tries
    /// again on the next tick.
    fn request(&self, request: Request) -> bool {
        if self.busy.swap(true, Ordering::Relaxed) {
            return false;
        }
        if self.requests.try_send(request).is_err() {
            self.busy.store(false, Ordering::Relaxed);
            return false;
        }
        true
    }
}

/// The sky is redrawn only when its size or palette changes, not per frame.
#[derive(Default)]
struct SkyCache {
    key: Option<(u32, u32, u32, bool)>,
}

impl SkyCache {
    fn render_if_changed(&mut self, request: &Request) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
        let scale_key = (request.sky_scale * 1000.0) as u32;
        let key = (request.width, request.height, scale_key, request.dark);
        if self.key == Some(key) {
            return None;
        }
        self.key = Some(key);
        let started = Instant::now();
        let pixels = sky::render(&SkyRequest {
            width: request.width,
            height: request.height,
            scale: request.sky_scale,
            dark: request.dark,
        });
        log::debug!(
            target: "background",
            "sky redrawn at {}x{} in {:.1}ms",
            request.width,
            request.height,
            started.elapsed().as_secs_f64() * 1000.0,
        );
        Some(pixels)
    }
}

pub struct Controller {
    config: Rc<RefCell<config::Config>>,
    /// `(id, use_as_launcher_background, has_background, background_darkness)`.
    instance: Option<(String, bool, bool, u8)>,
    /// What the app resolves to, and what is fully on screen.
    target: Source,
    settled: Source,
    slots: [Slot; 2],
    shown: Option<usize>,
    fade: Option<Fade>,
    pending: Option<Source>,
    /// The first resolution is decoded on this thread and placed without a fade,
    /// so a configured background is already on the first frame.
    initialized: bool,
    last_change: Instant,
    debounce_until: Option<Instant>,
    cam_z: f32,
    last_tick: Instant,
    /// Set when something other than the camera needs a fresh frame.
    world_dirty: bool,
    /// A resize is still moving; the sky waits until this instant.
    resize_settle: Option<Instant>,
    gpu_shared: gl::Handle,
    gpu_engaged: bool,
    gpu_asked: Instant,
    /// No OpenGL, so the world is not drawn at all: only the sky remains, and a
    /// CPU-rasterised world is far too slow to be worth showing.
    software_only: bool,
    parallax: (f32, f32),
    cache: ImageCache,
    /// A `(path, modified)` that failed to decode, skipped so a corrupt file is
    /// not retried on every resolution; a new timestamp is tried again.
    failed: Option<(PathBuf, SystemTime)>,
    /// A load whose generation is stale when it lands is dropped.
    generation: u64,
    renderer: Renderer,
    /// The ticker's stop flag. Not a Slint `Timer`: those stop when rendering
    /// does, which is exactly what a camera clock must not do. This is a runtime
    /// task whose tick hands itself to the event loop.
    ticking: Arc<AtomicBool>,
}

impl Controller {
    fn new(ui: &App, config: Rc<RefCell<config::Config>>) -> Self {
        let gpu_shared = gl::Shared::new();
        let gpu = Arc::clone(&gpu_shared.gpu);
        Self {
            config,
            instance: None,
            target: Source::World,
            settled: Source::World,
            slots: [Slot::default(), Slot::default()],
            shown: None,
            fade: None,
            pending: None,
            initialized: false,
            last_change: Instant::now(),
            debounce_until: None,
            cam_z: 0.0,
            last_tick: Instant::now(),
            world_dirty: true,
            resize_settle: None,
            gpu_shared,
            gpu_engaged: false,
            gpu_asked: Instant::now(),
            software_only: false,
            parallax: (0.0, 0.0),
            cache: ImageCache::default(),
            failed: None,
            generation: 0,
            renderer: Renderer::spawn(ui, &gpu),
            ticking: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The current instance's background wins, then the global one, then the
    /// world.
    fn resolve(&self) -> Source {
        if let Some((id, use_as_launcher, has_background, _)) = &self.instance
            && *use_as_launcher
            && *has_background
            && let Some(source) = image_source(instance::get_background_path(id), false)
                .filter(|source| !self.is_failed(source))
        {
            return source;
        }
        let config = self.config.borrow();
        if let Some(name) = &config.appearance.background_image
            && let Some(source) = image_source(storage::LOCATIONS.launcher.root.join(name), true)
                .filter(|source| !self.is_failed(source))
        {
            return source;
        }
        Source::World
    }

    fn is_failed(&self, source: &Source) -> bool {
        let Source::Image { path, modified, .. } = source else {
            return false;
        };
        self.failed
            .as_ref()
            .is_some_and(|(failed, at)| failed == path && at == modified)
    }

    /// Re-resolves and applies on every config edit and instance refresh, so it
    /// has to be free when nothing changed; comparing the resolved source rather
    /// than its inputs is what makes that true.
    fn refresh(&mut self, ui: &App) {
        let target = self.resolve();
        let now = Instant::now();
        if !self.initialized {
            self.initialized = true;
            self.initialize(ui, target, now);
            return;
        }
        if target == self.target {
            // The darkness may have moved even when the picture did not.
            self.update_dim(ui);
            self.sync(ui, now);
            return;
        }
        self.target = target.clone();
        let burst = now.duration_since(self.last_change) < Duration::from_millis(BURST_MS);
        self.last_change = now;

        if self.fade.is_some() {
            if !self.retarget(ui, &target) {
                self.pending = Some(target);
            }
        } else if burst {
            self.debounce_until = Some(now + Duration::from_millis(DEBOUNCE_MS));
        } else {
            self.start_fade(ui, target);
        }
        self.sync(ui, now);
    }

    /// Applies the first resolution, decoded on this thread and placed without a
    /// cross-fade so a configured background is on the first frame rather than
    /// fading in over the world.
    fn initialize(&mut self, ui: &App, target: Source, now: Instant) {
        self.target = target.clone();
        let Source::Image { path, modified, .. } = &target else {
            self.settled = Source::World;
            self.update_dim(ui);
            self.sync(ui, now);
            return;
        };
        let cached = self
            .cache
            .get(path, *modified)
            .map(|cached| (cached.alpha, cached.image.clone()));
        let (alpha, image) = match cached {
            Some(pair) => pair,
            None => match decode(path) {
                Ok((decoded_mtime, buffer, alpha)) => {
                    let image = Image::from_rgba8_premultiplied(buffer);
                    self.cache.insert(
                        path.clone(),
                        Cached {
                            modified: decoded_mtime,
                            alpha,
                            image: image.clone(),
                        },
                    );
                    (alpha, image)
                }
                Err(error) => {
                    log::warn!("failed to load '{}': {error}", path.display());
                    self.failed = Some((path.clone(), *modified));
                    self.target = Source::World;
                    self.settled = Source::World;
                    self.update_dim(ui);
                    self.sync(ui, now);
                    return;
                }
            },
        };
        let slot = 0;
        self.slots[slot].source = Some(target.clone());
        self.slots[slot].image = image;
        self.slots[slot].alpha = alpha;
        self.slots[slot].opacity = 1.0;
        self.slots[slot].target = 1.0;
        self.shown = Some(slot);
        self.settled = target;
        self.set_slot_image(ui, slot);
        self.write_alpha(ui, slot);
        self.set_world_shown(ui, alpha != Alpha::Opaque);
        self.update_dim(ui);
        self.sync(ui, now);
    }

    /// Replaces the image of the layer still fading in early in its fade, and
    /// returns whether it took the target. An undecoded target waits, because
    /// the load in flight may be for a different path.
    fn retarget(&mut self, ui: &App, target: &Source) -> bool {
        let Some(fade) = self.fade else { return false };
        let Some(incoming) = fade.incoming else {
            return false;
        };
        let Source::Image { path, modified, .. } = target else {
            return false;
        };
        let progress = (Instant::now() - fade.started).as_secs_f32() * 1000.0 / FADE_MS;
        if progress > 0.5 {
            return false;
        }
        let Some(cached) = self.cache.get(path, *modified) else {
            return false;
        };
        let alpha = cached.alpha;
        let image = cached.image.clone();
        let slot = &mut self.slots[incoming];
        slot.source = Some(target.clone());
        slot.alpha = alpha;
        slot.image = image;
        if alpha != Alpha::Opaque {
            self.set_world_shown(ui, true);
            self.world_dirty = true;
        }
        self.set_slot_image(ui, incoming);
        self.update_dim(ui);
        true
    }

    /// Begins the cross-fade to `target`.
    fn start_fade(&mut self, ui: &App, target: Source) {
        log::debug!(
            target: "background",
            "showing {:?} -> {target:?}",
            self.settled,
        );
        match target {
            Source::World => {
                // The world goes back on screen before the image starts to
                // leave, so no frame ever shows neither.
                self.set_world_shown(ui, true);
                self.world_dirty = true;
                match self.shown {
                    Some(slot) => {
                        let from = self.slots[slot].opacity;
                        self.slots[slot].target = 0.0;
                        self.fade = Some(Fade {
                            incoming: None,
                            outgoing: Some(slot),
                            incoming_from: 0.0,
                            outgoing_from: from,
                            started: Instant::now(),
                        });
                        self.apply_fade(ui, Instant::now());
                    }
                    None => self.settled = Source::World,
                }
            }
            Source::Image {
                path,
                global,
                modified,
            } => {
                // The world may stop being drawn only when both layers are
                // opaque; a translucent one on either side leaves it visible.
                let outgoing = self.shown;
                let outgoing_covers =
                    outgoing.is_some_and(|slot| self.slots[slot].alpha == Alpha::Opaque);
                let slot = 1 - self.shown.unwrap_or(1);
                self.slots[slot].source = Some(Source::Image {
                    path: path.clone(),
                    global,
                    modified,
                });
                match self.cache.get(&path, modified) {
                    Some(cached) => {
                        let alpha = cached.alpha;
                        let image = cached.image.clone();
                        self.slots[slot].alpha = alpha;
                        self.slots[slot].image = image;
                        if !outgoing_covers {
                            self.set_world_shown(ui, true);
                            self.world_dirty = true;
                        }
                        self.begin_crossfade(ui, slot, outgoing);
                    }
                    None => {
                        // Assume translucent until decoded, so a translucent
                        // image does not have the world pulled from under it.
                        self.slots[slot].alpha = Alpha::Unknown;
                        if !outgoing_covers {
                            self.set_world_shown(ui, true);
                            self.world_dirty = true;
                        }
                        self.load(ui, path, modified, slot);
                    }
                }
            }
        }
        self.sync(ui, Instant::now());
    }

    /// Starts the cross-fade for a slot whose image is on hand.
    fn begin_crossfade(&mut self, ui: &App, incoming: usize, outgoing: Option<usize>) {
        if self.slots[incoming].alpha != Alpha::Opaque {
            self.set_world_shown(ui, true);
            self.world_dirty = true;
        }
        // An image does not animate, only its opacity; writing it first means
        // the first animated frame already has the pixels to show.
        self.set_slot_image(ui, incoming);
        self.slots[incoming].target = 1.0;
        let outgoing = outgoing.filter(|slot| *slot != incoming);
        if let Some(slot) = outgoing {
            self.set_slot_image(ui, slot);
            self.slots[slot].target = 0.0;
        }
        self.fade = Some(Fade {
            incoming: Some(incoming),
            outgoing,
            incoming_from: self.slots[incoming].opacity,
            outgoing_from: outgoing.map(|slot| self.slots[slot].opacity).unwrap_or(0.0),
            started: Instant::now(),
        });
        self.apply_fade(ui, Instant::now());
    }

    fn load(&mut self, ui: &App, path: PathBuf, modified: SystemTime, slot: usize) {
        self.generation += 1;
        let generation = self.generation;
        let weak = ui.as_weak();
        let requested = path.clone();
        let failed = (path.clone(), modified);
        runtime::spawn_blocking(move || {
            let decoded = decode(&requested);
            crate::ui::services::report::report(&weak, move |ui| {
                let controller = CONTROLLER.with(|slot| slot.borrow().clone());
                let Some(controller) = controller else { return };
                let decoded = match decoded {
                    Ok(decoded) => decoded,
                    Err(error) => {
                        log::warn!("failed to load '{}': {error}", path.display());
                        // Remember the file and resolve again, so a corrupt
                        // image shows the world instead of being retried forever.
                        let mut controller = controller.borrow_mut();
                        controller.failed = Some(failed);
                        controller.pending = None;
                        controller.debounce_until = None;
                        controller.refresh(&ui);
                        return;
                    }
                };
                let mut controller = controller.borrow_mut();
                if controller.generation != generation {
                    return;
                }
                let (mtime, buffer, alpha) = decoded;
                let image = Image::from_rgba8_premultiplied(buffer);
                controller.cache.insert(
                    path.clone(),
                    Cached {
                        modified: mtime,
                        alpha,
                        image: image.clone(),
                    },
                );
                controller.slots[slot].image = image;
                controller.slots[slot].alpha = alpha;
                if controller.fade.is_none() {
                    let outgoing = controller.shown;
                    controller.begin_crossfade(&ui, slot, outgoing);
                } else {
                    controller.set_slot_image(&ui, slot);
                }
                controller.sync(&ui, Instant::now());
            });
        });
    }

    /// Advances the camera, the fades and the parallax, and asks for a frame
    /// when the world needs one.
    fn tick(&mut self, ui: &App) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32().clamp(0.0, 0.05);
        self.last_tick = now;

        if let Some(until) = self.debounce_until
            && now >= until
        {
            self.debounce_until = None;
            let target = self.target.clone();
            if target != self.settled && self.fade.is_none() {
                self.start_fade(ui, target);
            }
        }

        self.apply_fade(ui, now);

        let moving = self.camera_moves();
        self.gpu_shared.moving.set(moving);
        ui.global::<Background>().set_camera_moving(moving);
        if moving {
            // The GPU steps the camera in its draw callback, so a window nobody
            // repaints does not move the world. The rasteriser is stepped here,
            // but only between frames.
            if self.gpu() != Gpu::Running && !self.renderer.busy() {
                self.cam_z += CAMERA_SPEED * dt;
                self.gpu_shared.cam_z.set(self.cam_z);
                self.world_dirty = true;
            }
        }

        // The first GPU frame has reached the window: the rasteriser now costs
        // more than it is worth. The sky stays here, at the renderer's size
        // rather than the rasteriser's budget, so it is redrawn once.
        if self.gpu() == Gpu::Running && !self.gpu_engaged {
            self.gpu_engaged = true;
            self.world_dirty = true;
        } else if self.gpu_engaged && self.gpu() != Gpu::Running {
            // The GPU gave up (a lost context, a framebuffer it cannot build).
            // An image that will never be redrawn is worse than a slow one.
            log::warn!(target: "background", "the GPU background stopped; back to the CPU");
            self.gpu_engaged = false;
            self.cam_z = self.gpu_shared.cam_z.get();
            self.world_dirty = true;
        }

        // No OpenGL, and none is coming: the world goes away for good.
        if !self.software_only && self.gpu() != Gpu::Running && !self.gpu_waiting() {
            log::info!(
                target: "background",
                "no OpenGL renderer: the background is the hyperbola sky alone",
            );
            self.software_only = true;
            let background = ui.global::<Background>();
            background.set_software_only(true);
            self.set_world_shown(ui, false);
            self.world_dirty = true;
        }

        self.push_parallax(ui, dt, now);

        let gpu_resizing = self.gpu_shared.resizing.get();
        if self.world_visible() || self.software_only {
            // The window is asked to redraw either way: the GPU draws inside
            // that frame's callback, and the rasteriser's new image needs a
            // frame to appear in.
            if self.world_dirty || self.camera_moves() || gpu_resizing {
                ui.window().request_redraw();
                // With the GPU drawing, only the sky is ours and it can wait
                // for the resize to settle; its old image is stretched by the
                // window until then.
                let sky_ready = self.gpu() != Gpu::Running
                    || self.resize_settle.is_none_or(|until| now >= until);
                if self.world_dirty && sky_ready && self.request_frame(ui) {
                    self.world_dirty = false;
                    self.resize_settle = None;
                }
            }
        } else if gpu_resizing {
            // The world is hidden, but the GPU still has to reach the frame where
            // it can drop the resize and stop the ticker.
            ui.window().request_redraw();
        } else {
            self.world_dirty = false;
            self.resize_settle = None;
        }

        self.sync(ui, now);
    }

    /// Moves both halves of the fade from one clock, so their curves and their
    /// instant agree, and finishes it when the clock runs out.
    fn apply_fade(&mut self, ui: &App, now: Instant) {
        let Some(fade) = self.fade else { return };
        let elapsed = now.duration_since(fade.started).as_secs_f32() * 1000.0;
        let eased = ease_in_out(elapsed / FADE_MS);
        if let Some(slot) = fade.incoming {
            let opacity = fade.incoming_from + (1.0 - fade.incoming_from) * eased;
            self.slots[slot].opacity = opacity;
            self.write_alpha(ui, slot);
        }
        if let Some(slot) = fade.outgoing {
            let opacity = fade.outgoing_from * (1.0 - eased);
            self.slots[slot].opacity = opacity;
            self.write_alpha(ui, slot);
        }
        self.update_dim(ui);
        if elapsed >= FADE_MS {
            self.on_fade_done(ui, fade);
        }
    }

    fn on_fade_done(&mut self, ui: &App, fade: Fade) {
        self.fade = None;
        if let Some(incoming) = fade.incoming {
            self.shown = Some(incoming);
            self.settled = self.slots[incoming].source.clone().unwrap_or(Source::World);
            self.slots[incoming].opacity = 1.0;
            self.slots[incoming].target = 1.0;
            self.write_alpha(ui, incoming);
            if let Some(outgoing) = fade.outgoing {
                self.clear_slot(ui, outgoing);
            }
            // An opaque image can stop the world; a translucent one still needs
            // it drawn behind.
            if self.slots[incoming].alpha == Alpha::Opaque {
                self.set_world_shown(ui, false);
            } else {
                self.set_world_shown(ui, true);
            }
        } else if let Some(outgoing) = fade.outgoing {
            self.shown = None;
            self.settled = Source::World;
            self.clear_slot(ui, outgoing);
        }
        self.update_dim(ui);

        if let Some(next) = self.pending.take()
            && next != self.settled
        {
            self.start_fade(ui, next);
        }
    }

    /// Returns whether the rasteriser took the request.
    fn request_frame(&mut self, ui: &App) -> bool {
        let logical = ui.window().size();
        let scale_factor = ui.window().scale_factor();
        // The window has no size until shown, and the setup asks before that:
        // declining keeps `world_dirty` set so the ticker comes back once there
        // is a window to fill.
        if logical.width == 0 || logical.height == 0 || scale_factor <= 0.0 {
            return false;
        }
        let device_width = (logical.width as f32 * WRAPPER_SCALE).round().max(1.0);
        let device_height = (logical.height as f32 * WRAPPER_SCALE).round().max(1.0);
        // With the GPU drawing the world only the sky remains, drawn once per
        // size and palette at the GPU's size so the layers stay matched.
        let draw_world = self.gpu() != Gpu::Running && !self.software_only;
        let downscale = if draw_world {
            let pixels = device_width * device_height;
            (WORLD_PIXEL_BUDGET as f32 / pixels).sqrt().min(1.0)
        } else {
            1.0
        };
        let width = (device_width * downscale).round().max(1.0) as u32;
        let height = (device_height * downscale).round().max(1.0) as u32;

        let background = ui.global::<Background>();
        let crust = background.get_crust();
        let dark = background.get_dark();
        log::debug!(
            target: "background",
            "frame requested at {width}x{height} (world {})",
            if draw_world { "yes" } else { "no" },
        );
        self.renderer.request(Request {
            width,
            height,
            edge_scale: downscale,
            draw_world,
            sky_scale: downscale * scale_factor * WRAPPER_SCALE,
            cam_z: self.cam_z,
            crust: [
                f32::from(crust.red()) / 255.0,
                f32::from(crust.green()) / 255.0,
                f32::from(crust.blue()) / 255.0,
            ],
            dark,
        })
    }

    fn gpu(&self) -> Gpu {
        self.gpu_shared.gpu.get()
    }

    /// Whether the GPU is still to report in; after two seconds the ticker stops
    /// waiting on a renderer that is never going to call back.
    fn gpu_waiting(&self) -> bool {
        self.gpu() == Gpu::Pending && self.gpu_asked.elapsed() < Duration::from_secs(2)
    }

    fn push_parallax(&mut self, ui: &App, dt: f32, now: Instant) {
        let enabled = self.config.borrow().appearance.background_parallax;
        let rectified = if enabled {
            POINTER.with(|pointer| {
                let (x, y) = (*pointer.borrow())?;
                let window = ui.window();
                let scale = window.scale_factor();
                let size = window.size();
                if size.width == 0 || size.height == 0 {
                    return Some((0.0, 0.0));
                }
                let normalized_x = (x / scale) / (size.width as f32 / scale) * 2.0 - 1.0;
                let normalized_y = (y / scale) / (size.height as f32 / scale) * 2.0 - 1.0;
                Some((
                    normalized_x.clamp(-1.0, 1.0) * MAX_OFFSET,
                    normalized_y.clamp(-1.0, 1.0) * MAX_OFFSET,
                ))
            })
        } else {
            Some((0.0, 0.0))
        };
        // The pointer being outside is a target of zero, not a position, so
        // entering, leaving and flicking across the edge all ease.
        let target = rectified.unwrap_or((0.0, 0.0));
        let k = 1.0 - (-dt / PARALLAX_EASE_SECS).exp();
        let offset = (
            self.parallax.0 + (target.0 - self.parallax.0) * k,
            self.parallax.1 + (target.1 - self.parallax.1) * k,
        );
        // Only a real move is written; Slint's `animate` smooths between writes.
        if (offset.0 - self.parallax.0).abs() < 0.01 && (offset.1 - self.parallax.1).abs() < 0.01 {
            return;
        }
        self.parallax = offset;
        let _ = now;
        let background = ui.global::<Background>();
        background.set_parallax_x(offset.0);
        background.set_parallax_y(offset.1);
    }

    fn camera_moves(&self) -> bool {
        // The camera only advances while the world is what the window shows.
        !self.software_only
            && self.shown.is_none()
            && self.config.borrow().appearance.background_camera_move
    }

    fn world_visible(&self) -> bool {
        // The world is also drawn behind a transition and behind a translucent
        // image, which does not cover it.
        self.shown.is_none()
            || self.fade.is_some()
            || self
                .shown
                .is_some_and(|slot| self.slots[slot].alpha != Alpha::Opaque)
    }

    fn set_world_shown(&self, ui: &App, shown: bool) {
        ui.global::<Background>().set_world_shown(shown);
    }

    fn set_slot_image(&self, ui: &App, slot: usize) {
        let background = ui.global::<Background>();
        match slot {
            0 => background.set_custom_a(self.slots[0].image.clone()),
            _ => background.set_custom_b(self.slots[1].image.clone()),
        }
    }

    fn write_alpha(&self, ui: &App, slot: usize) {
        let background = ui.global::<Background>();
        match slot {
            0 => background.set_custom_a_alpha(self.slots[0].opacity),
            _ => background.set_custom_b_alpha(self.slots[1].opacity),
        }
    }

    fn clear_slot(&mut self, ui: &App, slot: usize) {
        self.slots[slot].source = None;
        self.slots[slot].image = Image::default();
        self.slots[slot].opacity = 0.0;
        self.slots[slot].target = 0.0;
        self.set_slot_image(ui, slot);
        self.write_alpha(ui, slot);
    }

    /// Dims each image at its owner's percentage and weights it by the part of
    /// the picture it covers, so cross-fading between two percentages does not
    /// lighten in the middle. The world is never dimmed.
    fn update_dim(&self, ui: &App) {
        let global_darkness =
            f32::from(self.config.borrow().appearance.background_darkness) / 100.0;
        let instance_darkness = self
            .instance
            .as_ref()
            .map(|(_, _, _, darkness)| f32::from(*darkness) / 100.0)
            .unwrap_or(0.0);

        let mut remaining = 1.0f32;
        let mut dim = 0.0f32;
        for slot in self.slots.iter().rev() {
            let Some(Source::Image { global, .. }) = &slot.source else {
                continue;
            };
            let darkness = if *global {
                global_darkness
            } else {
                instance_darkness
            };
            dim += darkness * slot.opacity * remaining;
            remaining *= 1.0 - slot.opacity;
        }
        ui.global::<Background>().set_dim_alpha(dim);
    }

    fn sync(&mut self, ui: &App, now: Instant) {
        let parallax = self.config.borrow().appearance.background_parallax;
        let busy = parallax
            || self.fade.is_some()
            || self.debounce_until.is_some()
            || self.target != self.settled
            || self.world_dirty
            || self.gpu_shared.resizing.get()
            || (self.world_visible() && (self.camera_moves() || self.gpu_waiting()));
        if busy {
            if !self.ticking.swap(true, Ordering::Relaxed) {
                tick_on_the_runtime(ui, Arc::clone(&self.ticking));
            }
        } else {
            self.ticking.store(false, Ordering::Relaxed);
        }
        self.last_tick = now;
    }
}

/// Runs [`Controller::tick`] every [`TICK_MS`] until the flag is cleared. The
/// tick is handed to the event loop with `report`, which wakes it and keeps the
/// clock independent of whether anything is being painted.
fn tick_on_the_runtime(ui: &App, ticking: Arc<AtomicBool>) {
    let weak = ui.as_weak();
    runtime::spawn(async move {
        let interval = Duration::from_millis(TICK_MS);
        loop {
            tokio::time::sleep(interval).await;
            if !ticking.load(Ordering::Relaxed) {
                return;
            }
            let weak = weak.clone();
            let ticking = Arc::clone(&ticking);
            crate::ui::services::report::report(&weak, move |ui| {
                if !ticking.load(Ordering::Relaxed) {
                    return;
                }
                let controller = CONTROLLER.with(|slot| slot.borrow().clone());
                if let Some(controller) = controller {
                    controller.borrow_mut().tick(&ui);
                }
            });
        }
    });
}

type Decoded = (SystemTime, SharedPixelBuffer<Rgba8Pixel>, Alpha);

/// Decodes a background image on a worker thread: scaled down, checked for
/// transparency, and with the format taken from the bytes since the files have no
/// extension.
fn decode(path: &PathBuf) -> Result<Decoded, String> {
    let started = Instant::now();
    let mtime = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| error.to_string())?;
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
    let (width, height) = (image.width(), image.height());
    let scaled = if width.max(height) > IMAGE_MAX_EDGE {
        let ratio = IMAGE_MAX_EDGE as f32 / width.max(height) as f32;
        // The fast integer box down-scale; a Triangle filter is smoother but an
        // order of magnitude slower, and invisible under `image-fit: cover`.
        image.thumbnail_exact(
            ((width as f32 * ratio) as u32).max(1),
            ((height as f32 * ratio) as u32).max(1),
        )
    } else {
        image
    };
    let scaled = scaled.into_rgba8();
    let alpha = if scaled.pixels().any(|pixel| pixel.0[3] < 250) {
        Alpha::Translucent
    } else {
        Alpha::Opaque
    };
    let (width, height) = scaled.dimensions();
    log::debug!(
        target: "background",
        "decoded '{}' at {width}x{height} in {:?}",
        path.display(),
        started.elapsed(),
    );
    Ok((
        mtime,
        SharedPixelBuffer::clone_from_slice(scaled.as_raw(), width, height),
        alpha,
    ))
}

/// The Rust entry points for the rest of the app.
pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    let controller = Rc::new(RefCell::new(Controller::new(ui, config)));
    CONTROLLER.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&controller)));
    install_window_hook(ui);
    // A palette change animates `Theme.crust`; the world follows it so the
    // whole window changes colour together.
    ui.global::<Background>().on_theme_changed({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                palette_changed(&ui);
            }
        }
    });
    log::debug!(
        target: "background",
        "the camera {} and the parallax {}; the GPU is {:?}",
        if controller.borrow().config.borrow().appearance.background_camera_move {
            "moves"
        } else {
            "stands still"
        },
        if controller.borrow().config.borrow().appearance.background_parallax {
            "follows the pointer"
        } else {
            "does not follow the pointer"
        },
        controller.borrow().gpu(),
    );
    controller.borrow_mut().refresh(ui);
    controller.borrow_mut().request_frame(ui);
    // The world is handed to the GPU only after it has drawn one, so a renderer
    // without OpenGL keeps this frame.
    let shared = Rc::clone(&controller.borrow().gpu_shared);
    super::gl::install(ui, shared);
}

pub fn set_instance(ui: &App, current: Option<(&str, bool, bool, u8)>) {
    let controller = CONTROLLER.with(|slot| slot.borrow().clone());
    let Some(controller) = controller else { return };
    let current = current.map(|(id, use_as_launcher, has_background, darkness)| {
        (id.to_string(), use_as_launcher, has_background, darkness)
    });
    let mut controller = controller.borrow_mut();
    // Refresh even when the tuple did not move: replacing an instance's image
    // leaves its facts unchanged, but changes the file's timestamp.
    controller.instance = current;
    controller.refresh(ui);
}

pub fn config_changed(ui: &App) {
    let controller = CONTROLLER.with(|slot| slot.borrow().clone());
    if let Some(controller) = controller {
        log::debug!(
            target: "background",
            "the camera {} and the parallax {}",
            if controller.borrow().config.borrow().appearance.background_camera_move {
                "moves"
            } else {
                "stands still"
            },
            if controller.borrow().config.borrow().appearance.background_parallax {
                "follows the pointer"
            } else {
                "does not follow the pointer"
            },
        );
        controller.borrow_mut().refresh(ui);
    }
}

pub fn palette_changed(ui: &App) {
    let controller = CONTROLLER.with(|slot| slot.borrow().clone());
    if let Some(controller) = controller {
        let mut controller = controller.borrow_mut();
        controller.world_dirty = true;
        controller.sync(ui, Instant::now());
    }
}

/// Watches the pointer and size through the platform, not a `TouchArea`: one
/// covering the window swallows every click, and one underneath never sees a
/// move because the `TouchArea` above it accepts the event.
fn install_window_hook(ui: &App) {
    use winit::event::WindowEvent;

    let weak = ui.as_weak();
    window::on_window_event(ui, move |event| {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                POINTER.with(|pointer| {
                    *pointer.borrow_mut() = Some((position.x as f32, position.y as f32));
                });
            }
            WindowEvent::CursorLeft { .. } => {
                POINTER.with(|pointer| *pointer.borrow_mut() = None);
            }
            // A resize or scale change makes the images stale, and with the
            // camera still nothing else may redraw them for a long time.
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                let controller = CONTROLLER.with(|slot| slot.borrow().clone());
                let (Some(controller), Some(ui)) = (controller, weak.upgrade()) else {
                    return;
                };
                let mut controller = controller.borrow_mut();
                controller.world_dirty = true;
                controller.resize_settle =
                    Some(Instant::now() + Duration::from_millis(SKY_SETTLE_MS));
                controller.sync(&ui, Instant::now());
            }
            _ => {}
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mtime(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn decoded() -> Cached {
        Cached {
            modified: mtime(1),
            alpha: Alpha::Opaque,
            image: Image::default(),
        }
    }

    /// A replaced file at the same path is a new source, or the new wallpaper is
    /// dismissed as unchanged.
    #[test]
    fn a_replaced_file_is_a_new_source() {
        let path = PathBuf::from("/tmp/conic-background-test");
        let first = Source::Image {
            path: path.clone(),
            global: true,
            modified: mtime(10),
        };
        let second = Source::Image {
            path,
            global: true,
            modified: mtime(20),
        };
        assert_ne!(first, second);
        assert_eq!(first.clone(), first);
    }

    /// A replaced file must miss the cache, or the old picture is reused forever.
    #[test]
    fn the_cache_only_returns_the_decoded_revision() {
        let path = PathBuf::from("/tmp/conic-background-test");
        let mut cache = ImageCache::default();
        cache.insert(path.clone(), decoded());
        assert!(cache.get(&path, mtime(1)).is_some());
        assert!(cache.get(&path, mtime(2)).is_none());
    }

    /// Nothing for a path that is not a readable file.
    #[test]
    fn image_source_reads_the_timestamp() {
        let dir = std::env::temp_dir().join(format!("conic-bg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let file = dir.join("background_image");
        assert!(image_source(file.clone(), true).is_none());
        std::fs::write(&file, b"not really an image").expect("a file");
        match image_source(file.clone(), true) {
            Some(Source::Image {
                path,
                global,
                modified,
            }) => {
                assert_eq!(path, file);
                assert!(global);
                assert!(modified > SystemTime::UNIX_EPOCH);
            }
            other => panic!("expected an image source, got {other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// At the midpoint both layers are half-way, not one waiting for the other.
    #[test]
    fn the_cross_fade_moves_both_layers_together() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = App::new().expect("the app");
        let config = Rc::new(RefCell::new(config::Config::default()));
        let mut controller = Controller::new(&ui, Rc::clone(&config));

        let source = |name: &str| Source::Image {
            path: PathBuf::from(name),
            global: true,
            modified: mtime(1),
        };
        // Seeded so each fade starts without a worker decode, which needs a live
        // event loop.
        for name in ["/tmp/conic-a.png", "/tmp/conic-b.png"] {
            controller.cache.insert(PathBuf::from(name), decoded());
        }

        controller.start_fade(&ui, source("/tmp/conic-a.png"));
        let first = controller.fade.expect("the first fade");
        assert_eq!(first.incoming, Some(0));
        assert_eq!(first.outgoing, None);
        controller.apply_fade(&ui, first.started + Duration::from_millis(100));
        assert!(ui.global::<Background>().get_custom_a_alpha() > 0.1);
        controller.apply_fade(
            &ui,
            first.started + Duration::from_millis(FADE_MS as u64 + 1),
        );
        assert!(controller.fade.is_none(), "the first fade did not finish");
        assert_eq!(controller.shown, Some(0));

        controller.start_fade(&ui, source("/tmp/conic-b.png"));
        let fade = controller.fade.expect("the second fade");
        assert_eq!(fade.incoming, Some(1));
        assert_eq!(fade.outgoing, Some(0));

        controller.apply_fade(
            &ui,
            fade.started + Duration::from_millis((FADE_MS / 2.0) as u64),
        );
        let background = ui.global::<Background>();
        let (a, b) = (
            background.get_custom_a_alpha(),
            background.get_custom_b_alpha(),
        );
        assert!(
            (a - b).abs() < 0.2,
            "the cross-fade is lopsided: outgoing {a}, incoming {b}"
        );
        assert!(
            a > 0.1 && b > 0.1,
            "one layer is not moving: outgoing {a}, incoming {b}"
        );

        controller.apply_fade(
            &ui,
            fade.started + Duration::from_millis(FADE_MS as u64 + 1),
        );
        assert!(controller.fade.is_none(), "the second fade did not finish");
        assert_eq!(controller.shown, Some(1));
        assert_eq!(controller.slots[0].opacity, 0.0);
        assert_eq!(controller.slots[1].opacity, 1.0);

        controller.ticking.store(false, Ordering::Relaxed);
    }

    /// The first background is placed whole, with no fade from the world.
    #[test]
    fn the_first_background_is_placed_without_a_fade() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = App::new().expect("the app");
        let config = Rc::new(RefCell::new(config::Config::default()));
        let mut controller = Controller::new(&ui, Rc::clone(&config));

        let dir = std::env::temp_dir().join(format!("conic-bg-init-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let path = dir.join("background_image");
        image::RgbaImage::from_pixel(4, 4, image::Rgba([10, 20, 30, 255]))
            .save_with_format(&path, image::ImageFormat::Png)
            .expect("a png");
        let modified = std::fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .expect("a timestamp");

        controller.initialize(
            &ui,
            Source::Image {
                path,
                global: true,
                modified,
            },
            Instant::now(),
        );

        let background = ui.global::<Background>();
        assert!(controller.fade.is_none(), "the first background faded");
        assert_eq!(controller.shown, Some(0));
        assert_eq!(background.get_custom_a_alpha(), 1.0);
        assert!(!background.get_world_shown());
        controller.ticking.store(false, Ordering::Relaxed);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// An instance image uses the instance's percentage; the global one uses the
    /// global's.
    #[test]
    fn a_background_is_dimmed_at_its_owners_percentage() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = App::new().expect("the app");
        let config = Rc::new(RefCell::new(config::Config::default()));
        config.borrow_mut().appearance.background_darkness = 40;
        let mut controller = Controller::new(&ui, Rc::clone(&config));
        let background = ui.global::<Background>();

        let image = |name: &str, global: bool| Source::Image {
            path: PathBuf::from(name),
            global,
            modified: mtime(1),
        };

        // An instance background, fully on screen at 80%.
        controller.instance = Some(("id".to_string(), true, true, 80));
        controller.slots[0].source = Some(image("/tmp/conic-instance", false));
        controller.slots[0].opacity = 1.0;
        controller.update_dim(&ui);
        let dim = background.get_dim_alpha();
        assert!(
            (dim - 0.8).abs() < 1e-6,
            "instance darkness not used: {dim}"
        );

        // The global image at the same opacity uses the global 40% instead.
        controller.instance = None;
        controller.slots[0].source = Some(image("/tmp/conic-global", true));
        controller.update_dim(&ui);
        let dim = background.get_dim_alpha();
        assert!((dim - 0.4).abs() < 1e-6, "global darkness not used: {dim}");
    }
}
