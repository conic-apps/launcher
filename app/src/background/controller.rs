// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The background's controller: what the window is showing, the cross-fade
//! between backgrounds, the camera's clock and the parallax.
//!
//! Three things decide what is on screen, in this order: the current instance's
//! background (when it asks to be the launcher's), the global custom background
//! from Settings → Appearance, and the 3D world. Every config edit and every
//! instance refresh re-resolves that and compares the *result* — a refresh that
//! does not change the answer does nothing at all, which is what keeps an
//! unrelated update from flickering the background.
//!
//! A change starts a cross-fade unless one is already on screen, in which case
//! the latest target is remembered and applied when the current one lands. The
//! fades themselves are Slint's: the component animates its own layer alphas
//! and this only says which slot holds what, so a fade in flight is never
//! restarted by an unrelated refresh (see `window-background.slint`).
//!
//! Rendering runs on a thread of its own (see [`Renderer`]): a frame of the
//! world costs several milliseconds of CPU and it must not be the UI's
//! milliseconds. One frame is in flight at a time — a request arriving while
//! the previous frame is still being rasterised is dropped, which paces the
//! loop to whatever the machine can do and keeps the UI's own frames smooth.

use std::{
    cell::RefCell,
    collections::HashMap,
    path::PathBuf,
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
use crate::runtime;
use crate::slint_backend::{App, Background};

/// The pixels one world frame is allowed to cost. The renderer rasterises on
/// the CPU and the cost is very nearly linear in the target's area, so the
/// window is drawn at whatever scale brings it under this. The default window
/// (840×532 at 1x) is comfortably under it and gets the full device
/// resolution; a 2x display or a maximised window is scaled down, which costs
/// it a little sharpness and keeps the frame rate.
const WORLD_PIXEL_BUDGET: u32 = 1_400_000;

/// How often the background looks at its clock. Faster than any display, like
/// the app's scroll containers — the camera's step comes from the *real*
/// elapsed time, not from the tick interval.
const TICK_MS: u64 = 8;

/// The cross-fade length: 0.4s on the background images.
const FADE_MS: u64 = 400;

/// A target arriving within this of the previous one is part of the same burst:
/// clicking through instances quickly should not start a fade per click.
const BURST_MS: u64 = 500;
/// How long a burst has to be quiet before the target it settled on is applied.
const DEBOUNCE_MS: u64 = 200;

/// How far the background shifts at the window's edge.
const MAX_OFFSET: f32 = 4.0;
/// The parallax's time constant: the background is within a couple of percent of
/// where it belongs about 140ms after anything changes, which is what an eased
/// short transition looks like. Every change is eased — the pointer entering the
/// window as much as leaving it — and because the *position* always approaches
/// its target rather than being set to it, a pointer flicking in and out moves
/// the background a little and then brings it back, with nothing to flash.
const PARALLAX_EASE_SECS: f32 = 0.035;
/// The parallax wrapper's scale; the images are rendered oversized by it so the
/// scaling lands them 1:1 on device pixels.
const WRAPPER_SCALE: f32 = 1.08;

/// The longest edge a background image is kept at, before Slint scales it to
/// the window. Bounds the memory an 8K wallpaper costs.
const IMAGE_MAX_EDGE: u32 = 2560;

thread_local! {
    /// The controller, for the modules that can only report changes.
    static CONTROLLER: RefCell<Option<Rc<RefCell<Controller>>>> = const { RefCell::new(None) };
    /// The pointer in *physical* window coordinates, `None` once it leaves.
    static POINTER: RefCell<Option<(f32, f32)>> = const { RefCell::new(None) };
}

/// What the window should be showing.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Source {
    World,
    /// A custom background image. `global` marks the one from Settings →
    /// Appearance, the only one the darkness setting dims.
    Image {
        path: PathBuf,
        global: bool,
    },
}

/// Whether an image's own pixels let the layers under it show through.
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
    /// The opacity target handed to Slint (which animates its own copy).
    target: f32,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            source: None,
            image: Image::default(),
            alpha: Alpha::Unknown,
            target: 0.0,
        }
    }
}

/// A cross-fade in flight.
#[derive(Clone, Copy)]
struct Fade {
    /// The slot whose opacity is animating.
    slot: usize,
    started: Instant,
    /// The target the slot is heading for: 1 for an incoming background, 0 for
    /// one that is leaving to reveal the world.
    towards: f32,
}

/// What the renderer needs for one frame. Everything in it is something only
/// the UI thread can read — Slint is not thread-safe.
struct Request {
    width: u32,
    height: u32,
    /// Buffer pixels per device pixel, for the outline width.
    edge_scale: f32,
    /// Buffer pixels per CSS pixel, for the sky.
    sky_scale: f32,
    cam_z: f32,
    crust: [f32; 3],
    dark: bool,
    /// Whether to rasterise the world. Once the GPU is drawing it, only the
    /// sky is still ours, and a frame is then worth asking for only when the
    /// sky's own cache key has moved.
    draw_world: bool,
}

/// A rendered frame, on its way back to the UI thread. `Image` cannot be built
/// off the drawing thread, so the buffers travel and the UI makes the images.
struct RenderResult {
    world: Option<SharedPixelBuffer<Rgba8Pixel>>,
    sky: Option<SharedPixelBuffer<Rgba8Pixel>>,
}

/// The renderer's thread and the channel to it.
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
        // The handover flag, read from the worker's delivering closure too, so
        // a frame that arrives after the handover cannot put the CPU's world
        // back on screen.
        let worker_state = Arc::clone(dispatch_state);
        std::thread::Builder::new()
            .name("conic-background".into())
            .spawn(move || {
                let mut world = WorldRenderer::new();
                let mut sky = SkyCache::default();
                // The thread owns the renderer and lives as long as the
                // process, like the single-instance watcher's.
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
                    crate::report::report(&weak, move |ui| {
                        // The GPU may have taken over while this frame was being
                        // rasterised; its texture is the one the layers show.
                        let gpu = state.get() == Gpu::Running;
                        let background = ui.global::<Background>();
                        // The GPU's texture is the world now; only the sky is
                        // still this thread's to deliver.
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
            .expect("failed to start the background renderer");
        Self { requests, busy }
    }

    /// Whether a frame is still being rasterised.
    fn busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }

    /// Asks for a frame. A frame already in flight means this one is skipped —
    /// the caller is told so, and asks again on the next tick.
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

/// The sky is drawn once per size and palette: it only changes when the window
/// does, and it is the one part of the background that is not re-rendered per
/// frame.
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
        log::debug!(
            target: "background",
            "sky redrawn at {}x{}",
            request.width,
            request.height,
        );
        Some(sky::render(&SkyRequest {
            width: request.width,
            height: request.height,
            scale: request.sky_scale,
            dark: request.dark,
        }))
    }
}

pub struct Controller {
    config: Rc<RefCell<config::Config>>,
    /// The current instance, as `(id, use_as_launcher_background, has_background)`.
    instance: Option<(String, bool, bool)>,
    /// What the app's state resolves to, and what is fully on screen.
    target: Source,
    settled: Source,
    slots: [Slot; 2],
    /// The slot that is the visible custom background, if one is.
    shown: Option<usize>,
    fade: Option<Fade>,
    /// The latest target seen while a fade was in flight.
    pending: Option<Source>,
    last_change: Instant,
    /// The burst debounce, while one is pending.
    debounce_until: Option<Instant>,
    /// The camera, and the clock the tick advances it by.
    cam_z: f32,
    last_tick: Instant,
    /// Set when something other than the camera needs a fresh frame: the
    /// window's size, a palette change or a layer coming back.
    world_dirty: bool,
    /// The GPU renderer, whether it has taken the world over already, and when
    /// it was asked to.
    gpu_shared: gl::Handle,
    gpu_engaged: bool,
    gpu_asked: Instant,
    /// The renderer has no OpenGL, so the world is not drawn at all: the
    /// background is the hyperbola sky, and the camera setting is disabled in
    /// the settings page. A CPU-rasterised world is far too slow to be worth
    /// showing, and the sky is the part of the picture that costs nothing.
    software_only: bool,
    /// The parallax offsets last written.
    parallax: (f32, f32),
    /// Background images by path, with the time they were read at and whether
    /// they have transparency. The pixels themselves live in the slots.
    cache: HashMap<PathBuf, (SystemTime, Alpha)>,
    /// Bumped per load; a load whose generation is stale when it lands was for
    /// a target the user has already moved past, and is dropped.
    generation: u64,
    renderer: Renderer,
    /// The ticker's stop flag. It is *not* a Slint `Timer`: those are ticked by
    /// `update_timers_and_animations`, which only runs while the window is being
    /// rendered — so a Slint timer stops exactly when rendering does, and a
    /// camera clock built on one cannot be what restarts it. This is a task on
    /// the app's own runtime, and each tick hands itself to the event loop,
    /// which wakes it.
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
            last_change: Instant::now(),
            debounce_until: None,
            cam_z: 0.0,
            last_tick: Instant::now(),
            world_dirty: true,
            gpu_shared,
            gpu_engaged: false,
            gpu_asked: Instant::now(),
            software_only: false,
            parallax: (0.0, 0.0),
            cache: HashMap::new(),
            generation: 0,
            renderer: Renderer::spawn(ui, &gpu),
            ticking: Arc::new(AtomicBool::new(false)),
        }
    }

    // ----- the source -----

    /// Which of the three sources the app is asking for.
    fn resolve(&self) -> Source {
        if let Some((id, use_as_launcher, has_background)) = &self.instance
            && *use_as_launcher
            && *has_background
        {
            let path = instance::get_background_path(id);
            if path.is_file() {
                return Source::Image {
                    path,
                    global: false,
                };
            }
        }
        let config = self.config.borrow();
        if let Some(name) = &config.appearance.background_image {
            let path = storage::LOCATIONS.launcher.root.join(name);
            if path.is_file() {
                return Source::Image { path, global: true };
            }
        }
        Source::World
    }

    /// Re-resolves and applies. Called on every config edit and instance
    /// refresh, so it has to be free when nothing changed — which is the point
    /// of comparing the resolved source rather than its inputs.
    fn refresh(&mut self, ui: &App) {
        let target = self.resolve();
        let now = Instant::now();
        if target == self.target {
            self.sync(ui, now);
            return;
        }
        self.target = target.clone();
        let burst = now.duration_since(self.last_change) < Duration::from_millis(BURST_MS);
        self.last_change = now;

        if self.fade.is_some() {
            // A transition is on screen. If the incoming layer has barely
            // arrived its image can simply be replaced — the common case when
            // clicking through instances, and it avoids showing a background
            // the user has already moved past. Otherwise the target waits for
            // the current fade to land.
            if !self.retarget(ui, &target) {
                self.pending = Some(target);
            }
        } else if burst {
            // Mid-burst: wait for the clicking to stop before starting.
            self.debounce_until = Some(now + Duration::from_millis(DEBOUNCE_MS));
        } else {
            self.start_fade(ui, target);
        }
        self.sync(ui, now);
    }

    /// Replaces the image of the layer still fading in, when it is early enough
    /// in its fade for the swap not to be visible. Returns whether it took the
    /// target.
    fn retarget(&mut self, ui: &App, target: &Source) -> bool {
        let Some(fade) = self.fade else { return false };
        let Source::Image { path, .. } = target else {
            // The world cannot be swapped into an image slot; it waits.
            return false;
        };
        let progress = (Instant::now() - fade.started).as_secs_f32() * 1000.0 / FADE_MS as f32;
        if progress > 0.5 {
            return false;
        }
        match self.cache.get(path) {
            Some((_, alpha)) => {
                let alpha = *alpha;
                self.slots[fade.slot].source = Some(target.clone());
                self.slots[fade.slot].alpha = alpha;
                self.set_slot(ui, fade.slot);
                self.update_dim(ui);
                true
            }
            // Not loaded yet: the load in flight will land on the new target
            // anyway, since its generation is stale.
            None => false,
        }
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
                // The world goes back on screen *before* the image starts to
                // leave, so no frame ever shows neither: it is rendered under
                // the outgoing image and revealed by its fade.
                self.set_world_shown(ui, true);
                self.world_dirty = true;
                match self.shown {
                    Some(slot) => {
                        self.slots[slot].target = 0.0;
                        self.fade = Some(Fade {
                            slot,
                            started: Instant::now(),
                            towards: 0.0,
                        });
                        self.set_slot(ui, slot);
                    }
                    None => self.settled = Source::World,
                }
            }
            Source::Image { path, global } => {
                // The world has to stay under the fade unless the layer being
                // replaced is already opaque and covers it.
                let outgoing_covers = self
                    .shown
                    .is_some_and(|slot| self.slots[slot].alpha == Alpha::Opaque);
                if !outgoing_covers {
                    self.set_world_shown(ui, true);
                    self.world_dirty = true;
                }
                // The slot that is not on screen takes the new image.
                let slot = 1 - self.shown.unwrap_or(1);
                self.slots[slot].source = Some(Source::Image {
                    path: path.clone(),
                    global,
                });
                match self.cache.get(&path) {
                    Some((_, alpha)) => {
                        self.slots[slot].alpha = *alpha;
                        self.begin_incoming(ui, slot);
                    }
                    None => {
                        // Assume the worst until it is decoded: a translucent
                        // image must not have the world pulled out from under
                        // it.
                        self.slots[slot].alpha = Alpha::Unknown;
                        self.load(ui, path, slot);
                    }
                }
            }
        }
        self.sync(ui, Instant::now());
    }

    /// Starts the fade for a slot whose image is on hand.
    fn begin_incoming(&mut self, ui: &App, slot: usize) {
        self.slots[slot].target = 1.0;
        self.fade = Some(Fade {
            slot,
            started: Instant::now(),
            towards: 1.0,
        });
        self.set_slot(ui, slot);
    }

    fn load(&mut self, ui: &App, path: PathBuf, slot: usize) {
        self.generation += 1;
        let generation = self.generation;
        let weak = ui.as_weak();
        let requested = path.clone();
        runtime::spawn_blocking(move || {
            let decoded = decode(&requested);
            crate::report::report(&weak, move |ui| {
                let controller = CONTROLLER.with(|slot| slot.borrow().clone());
                let Some(controller) = controller else { return };
                let decoded = match decoded {
                    Ok(decoded) => decoded,
                    Err(error) => {
                        log::warn!("failed to load '{}': {error}", path.display());
                        // Fall back to the world rather than leaving a slot
                        // pointing at nothing.
                        controller.borrow_mut().target = Source::World;
                        controller.borrow_mut().pending = None;
                        controller.borrow_mut().debounce_until = None;
                        controller.borrow_mut().refresh(&ui);
                        return;
                    }
                };
                let mut controller = controller.borrow_mut();
                if controller.generation != generation {
                    return;
                }
                let (mtime, buffer, alpha) = decoded;
                controller.cache.insert(path.clone(), (mtime, alpha));
                controller.slots[slot].image = Image::from_rgba8_premultiplied(buffer);
                controller.slots[slot].alpha = alpha;
                // The fade may have been started by something else in the
                // meantime, or not started at all.
                if controller.fade.map(|fade| fade.slot) == Some(slot) || controller.fade.is_none()
                {
                    controller.begin_incoming(&ui, slot);
                } else {
                    controller.set_slot(&ui, slot);
                }
                controller.sync(&ui, Instant::now());
            });
        });
    }

    // ----- the tick -----

    /// Advances the camera, the fades and the parallax, and asks for a frame
    /// when the world needs one.
    fn tick(&mut self, ui: &App) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32().clamp(0.0, 0.05);
        self.last_tick = now;

        // The burst has gone quiet: apply what it settled on.
        if let Some(until) = self.debounce_until
            && now >= until
        {
            self.debounce_until = None;
            let target = self.target.clone();
            if target != self.settled && self.fade.is_none() {
                self.start_fade(ui, target);
            }
        }

        // A fade that has run its course.
        if let Some(fade) = self.fade
            && now.duration_since(fade.started) >= Duration::from_millis(FADE_MS)
        {
            self.on_fade_done(ui, fade);
        }

        let moving = self.camera_moves();
        self.gpu_shared.moving.set(moving);
        ui.global::<Background>().set_camera_moving(moving);
        if moving {
            // The GPU renderer steps the camera itself as it draws, so that a
            // window nobody is repainting does not move the world either. The
            // rasteriser is stepped here, but only between frames — moving it
            // while one is in flight would outrun the frames.
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

        // The sky is the whole background when there is no world renderer, so
        // it is kept fed there too.
        if self.world_visible() || self.software_only {
            // The rasteriser still owns the sky, and owns the whole world when
            // the renderer has no OpenGL, so it is asked for a frame whenever
            // the window needs one. The window is asked to redraw either way:
            // the GPU draws inside that frame's callback, and the rasteriser's
            // new image needs a frame to appear in.
            if self.world_dirty || self.camera_moves() {
                ui.window().request_redraw();
                if self.world_dirty && self.request_frame(ui) {
                    self.world_dirty = false;
                }
            }
        } else {
            // Nothing will be drawn, so there is nothing to be dirty about; the
            // world is marked again when it comes back.
            self.world_dirty = false;
        }

        self.sync(ui, now);
    }

    fn on_fade_done(&mut self, ui: &App, fade: Fade) {
        self.fade = None;
        let slot = fade.slot;
        if fade.towards > 0.5 {
            // The incoming layer has arrived: it is what the window shows now.
            self.shown = Some(slot);
            self.settled = self.slots[slot].source.clone().unwrap_or(Source::World);
            let other = 1 - slot;
            if self.slots[other].target > 0.0 {
                if self.slots[slot].alpha == Alpha::Opaque {
                    // Fully covered, so dropping the layer it replaced is
                    // invisible.
                    self.slots[other].target = 0.0;
                    self.slots[other].source = None;
                    self.slots[other].image = Image::default();
                    self.set_slot(ui, other);
                } else {
                    // A translucent one would show the replacement appearing
                    // under it, so it is faded out instead.
                    self.slots[other].target = 0.0;
                    self.set_slot(ui, other);
                    self.fade = Some(Fade {
                        slot: other,
                        started: Instant::now(),
                        towards: 0.0,
                    });
                    return;
                }
            }
            // With the image fully in place and opaque, the world has nothing
            // to show and can stop rendering.
            if self.slots[slot].alpha == Alpha::Opaque {
                self.set_world_shown(ui, false);
            }
        } else {
            // The world is back.
            self.shown = None;
            self.settled = Source::World;
            self.slots[slot].source = None;
            self.slots[slot].image = Image::default();
            self.set_slot(ui, slot);
        }

        if let Some(next) = self.pending.take()
            && next != self.settled
        {
            self.start_fade(ui, next);
        }
    }

    // ----- the frame -----

    /// Asks the rasteriser for a frame. Returns whether it took the request.
    fn request_frame(&mut self, ui: &App) -> bool {
        let logical = ui.window().size();
        let scale_factor = ui.window().scale_factor();
        // The window has no size until it has been shown, and the setup asks
        // for the first frame before that: a frame at that point is a single
        // pixel, and the one the window needed would never be asked for again
        // (nothing else marks the world dirty). Declining keeps `world_dirty`
        // set, so the ticker comes back to it once there is a window to fill.
        if logical.width == 0 || logical.height == 0 || scale_factor <= 0.0 {
            return false;
        }
        // The images cover the parallax wrapper, which is 1.08x the window, and
        // are then scaled back down — so they are drawn that much larger than
        // the window and land 1:1 on device pixels.
        let device_width = (logical.width as f32 * WRAPPER_SCALE).round().max(1.0);
        let device_height = (logical.height as f32 * WRAPPER_SCALE).round().max(1.0);
        // The budget is what the rasteriser can afford. With the GPU drawing
        // the world this thread only has the sky to do, which is drawn once per
        // size and palette — so it is drawn at the GPU's size instead, and the
        // two layers stay matched.
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

    /// Whether the GPU is still to report in. The renderer is set up on the
    /// first frame, so this is over in milliseconds; after that long the
    /// ticker stops rather than waiting on a renderer that is never going to
    /// call back.
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
                // Normalized to -1 at the left edge, +1 at the right.
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
        // The pointer being outside is a target of zero, not a position: every
        // change eases, so entering, leaving and a pointer flicking across the
        // edge all move the background rather than teleporting it.
        let target = rectified.unwrap_or((0.0, 0.0));
        let k = 1.0 - (-dt / PARALLAX_EASE_SECS).exp();
        let offset = (
            self.parallax.0 + (target.0 - self.parallax.0) * k,
            self.parallax.1 + (target.1 - self.parallax.1) * k,
        );
        // A still pointer costs nothing: only a real move is written out, and
        // Slint's `animate` does the smoothing between writes.
        if (offset.0 - self.parallax.0).abs() < 0.01 && (offset.1 - self.parallax.1).abs() < 0.01 {
            return;
        }
        self.parallax = offset;
        let _ = now;
        let background = ui.global::<Background>();
        background.set_parallax_x(offset.0);
        background.set_parallax_y(offset.1);
    }

    // ----- the layers -----

    fn camera_moves(&self) -> bool {
        // The camera only advances while the world is what the window shows: it
        // stops while a custom background is up. With no renderer for it, the
        // world is not on screen at all.
        !self.software_only
            && self.shown.is_none()
            && self.config.borrow().appearance.background_camera_move
    }

    fn world_visible(&self) -> bool {
        // The world is drawn when it is what the window shows, and while it is
        // still behind a transition. `world-shown` is the authority — it is
        // what the layers actually do.
        self.shown.is_none() || self.fade.is_some()
    }

    fn set_world_shown(&self, ui: &App, shown: bool) {
        let background = ui.global::<Background>();
        background.set_snap(true);
        background.set_world_shown(shown);
        background.set_snap(false);
    }

    fn set_slot(&self, ui: &App, slot: usize) {
        let background = ui.global::<Background>();
        background.set_snap(true);
        match slot {
            0 => {
                background.set_custom_a(self.slots[0].image.clone());
                background.set_custom_a_alpha(self.slots[0].target);
            }
            _ => {
                background.set_custom_b(self.slots[1].image.clone());
                background.set_custom_b_alpha(self.slots[1].target);
            }
        }
        background.set_snap(false);
        self.update_dim(ui);
    }

    /// Only the *global* custom background is dimmed, at the user's percentage
    /// — an instance background and the world are never dimmed.
    fn update_dim(&self, ui: &App) {
        let darkness = f32::from(self.config.borrow().appearance.background_darkness) / 100.0;
        let global_alpha = self
            .shown
            .into_iter()
            .chain(self.fade.map(|fade| fade.slot))
            .filter(|slot| {
                matches!(
                    &self.slots[*slot].source,
                    Some(Source::Image { global: true, .. })
                )
            })
            .map(|slot| self.slots[slot].target)
            .fold(0.0f32, f32::max);
        ui.global::<Background>()
            .set_dim_alpha(darkness * global_alpha);
    }

    /// Keeps the tick running only while there is something to do.
    fn sync(&mut self, ui: &App, now: Instant) {
        let parallax = self.config.borrow().appearance.background_parallax;
        let busy = parallax
            || self.fade.is_some()
            || self.debounce_until.is_some()
            || self.target != self.settled
            // Something is waiting for a frame, or for the GPU to report in.
            || self.world_dirty
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

/// Runs [`Controller::tick`] every [`TICK_MS`] until the flag is cleared.
///
/// The tick has to reach the UI thread (everything it touches is Slint's), so it
/// is handed over with `crate::report::report`, which also wakes the event loop
/// — that is what makes the clock independent of whether anything is being
/// painted. The task stops itself when `ticking` goes false, and a new one is
/// started if the controller needs the clock again.
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
            crate::report::report(&weak, move |ui| {
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

/// The image the UI thread has to make from what a worker decoded.
type Decoded = (SystemTime, SharedPixelBuffer<Rgba8Pixel>, Alpha);

/// Reads and decodes a background image on a worker thread.
///
/// The files are stored without an extension (`background_image`), so the
/// format is guessed from the bytes rather than from the name. Anything larger
/// than [`IMAGE_MAX_EDGE`] is scaled down once here, which is also the only
/// point at which it is cheap to ask whether the image has any transparency —
/// the answer decides whether the world stays behind it.
fn decode(path: &PathBuf) -> Result<Decoded, String> {
    let mtime = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| error.to_string())?;
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| error.to_string())?
        .into_rgba8();
    let (width, height) = image.dimensions();
    let scaled = if width.max(height) > IMAGE_MAX_EDGE {
        let ratio = IMAGE_MAX_EDGE as f32 / width.max(height) as f32;
        image::imageops::resize(
            &image,
            ((width as f32 * ratio) as u32).max(1),
            ((height as f32 * ratio) as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        image
    };
    let alpha = if scaled.pixels().any(|pixel| pixel.0[3] < 250) {
        Alpha::Translucent
    } else {
        Alpha::Opaque
    };
    let (width, height) = scaled.dimensions();
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
    // A first frame is asked for now, but the window has no size until it is
    // shown, so `request_frame` declines here; the ticker comes back to it once
    // there is a window to fill.
    controller.borrow_mut().request_frame(ui);
    // It hands the world over to the GPU once the GPU has actually drawn one
    // (see `background::gl`) — so a renderer without OpenGL keeps this frame.
    let shared = Rc::clone(&controller.borrow().gpu_shared);
    super::gl::install(ui, shared);
}

/// Reports the current instance's background facts, from `game.rs`'s refresh.
pub fn set_instance(ui: &App, current: Option<(&str, bool, bool)>) {
    let controller = CONTROLLER.with(|slot| slot.borrow().clone());
    let Some(controller) = controller else { return };
    let current = current.map(|(id, use_as_launcher, has_background)| {
        (id.to_string(), use_as_launcher, has_background)
    });
    let mut controller = controller.borrow_mut();
    if controller.instance == current {
        return;
    }
    controller.instance = current;
    controller.refresh(ui);
}

/// A config edit went through, from `settings.rs`.
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

/// The palette moved: the world's fill and outline colours have to be redrawn.
pub fn palette_changed(ui: &App) {
    let controller = CONTROLLER.with(|slot| slot.borrow().clone());
    if let Some(controller) = controller {
        let mut controller = controller.borrow_mut();
        controller.world_dirty = true;
        controller.sync(ui, Instant::now());
    }
}

/// Watches the window: the pointer for the parallax, the size for the two
/// layers.
///
/// It has to come from the platform rather than from a `TouchArea`: a
/// `TouchArea` that covers the window swallows every click in the app, and one
/// underneath the content never sees a move, because any `TouchArea` the
/// pointer is over accepts the event and ends the walk. The winit backend
/// hands out its window events instead, and these are window-wide — through
/// `window::on_window_event`, which shares the backend's single event
/// filter with the window controls and the music player rather than taking it
/// over from them.
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
            // Both layers are images of a size the window decides, so a resize
            // or a scale change makes them stale until something else redraws
            // them — which, with the camera still, may be a long time.
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                let controller = CONTROLLER.with(|slot| slot.borrow().clone());
                let (Some(controller), Some(ui)) = (controller, weak.upgrade()) else {
                    return;
                };
                let mut controller = controller.borrow_mut();
                controller.world_dirty = true;
                controller.sync(&ui, Instant::now());
            }
            _ => {}
        }
    });
}
