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
//! cross-fade itself is driven here, on the tick: the outgoing layer's opacity
//! eases to zero while the incoming one eases to one, both from the same clock,
//! so the two move together. The component is only told the two opacities (see
//! `window-background.slint`); owning the curve in Rust rather than in Slint's
//! `animate` is what lets a test read the mid-fade value back and assert that
//! the halves really overlap.
//!
//! Rendering runs on a thread of its own (see [`Renderer`]): a frame of the
//! world costs several milliseconds of CPU and it must not be the UI's
//! milliseconds. One frame is in flight at a time — a request arriving while
//! the previous frame is still being rasterised is dropped, which paces the
//! loop to whatever the machine can do and keeps the UI's own frames smooth.

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
const FADE_MS: f32 = 400.0;

/// How many decoded backgrounds are kept. The fixed on-disk file names
/// (`background_image`, an instance's `background`) mean a path seen before is
/// normally the same picture, so keeping its pixels skips the decode the next
/// time the slot fills — which is what makes clicking back through instances
/// instant rather than a fresh decode each time. A few is plenty (the visible
/// slot holds its own copy, so an evicted image never blanks the window), and
/// the bound keeps a long session from holding every wallpaper it ever showed.
const CACHE_LIMIT: usize = 6;

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
///
/// The image's `modified` timestamp is part of its identity on purpose. The
/// background is always stored under a fixed name — `background_image`, or an
/// instance's `background` — so picking a new wallpaper replaces the file at
/// the same path. Comparing only the path would call that "unchanged" and leave
/// the old picture on screen; the timestamp is what tells a replacement from a
/// re-resolution of the same one.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Source {
    World,
    /// A custom background image. `global` marks the one from Settings →
    /// Appearance, the only one the darkness setting dims.
    Image {
        path: PathBuf,
        global: bool,
        modified: SystemTime,
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

/// A cross-fade in flight.
///
/// Both halves share one clock so they move together: the incoming layer eases
/// from the opacity it had when the fade began to 1, the outgoing one from its
/// opacity to 0. Either half may be absent — a background appearing over the
/// world has no outgoing image, and one leaving to reveal the world has no
/// incoming one.
#[derive(Clone, Copy)]
struct Fade {
    /// The slot whose opacity rises to 1, if the incoming source is an image.
    incoming: Option<usize>,
    /// The slot whose opacity falls to 0, if an image is being replaced.
    outgoing: Option<usize>,
    /// The incoming slot's opacity when the fade began.
    incoming_from: f32,
    /// The outgoing slot's opacity when the fade began.
    outgoing_from: f32,
    started: Instant,
}

/// The eased position within a fade: a smooth in-out so neither end of the
/// cross-fade starts or stops abruptly.
fn ease_in_out(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The image [`Source`] for a path that is a readable file, carrying the
/// timestamp that decides whether a later resolution is the same picture.
fn image_source(path: PathBuf, global: bool) -> Option<Source> {
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    // A filesystem without a usable mtime still shows the image; it just cannot
    // tell a replacement at the same path from the first read, which is the
    // floor for detection rather than a reason to hide the picture.
    let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    Some(Source::Image {
        path,
        global,
        modified,
    })
}

/// One decoded background, with the timestamp it was decoded from. `Image` is a
/// ref-counted handle, so the slot that is currently showing one and this cache
/// hold the same pixels rather than a second copy.
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
    /// The decoded image for `path`, or `None` when the file on disk is no
    /// longer the one that was decoded — a replacement at the same path.
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
                    crate::ui::services::report::report(&weak, move |ui| {
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
    /// Whether the first resolution has run. The first one is decoded on this
    /// thread and placed without a cross-fade, so a configured background is
    /// already on the first frame instead of the world appearing and then
    /// fading into it.
    initialized: bool,
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
    /// Decoded background images by path, so a path already seen does not have
    /// to be decoded again and a released slot can be refilled without a gap.
    cache: ImageCache,
    /// A `(path, modified)` that failed to decode. `resolve` skips it so the
    /// window settles on the next source instead of retrying a corrupt file on
    /// every resolution; a replacement (a new timestamp) is tried again.
    failed: Option<(PathBuf, SystemTime)>,
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
            initialized: false,
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
            cache: ImageCache::default(),
            failed: None,
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

    /// Whether `source` is the file that failed to decode.
    fn is_failed(&self, source: &Source) -> bool {
        let Source::Image { path, modified, .. } = source else {
            return false;
        };
        self.failed
            .as_ref()
            .is_some_and(|(failed, at)| failed == path && at == modified)
    }

    /// Re-resolves and applies. Called on every config edit and instance
    /// refresh, so it has to be free when nothing changed — which is the point
    /// of comparing the resolved source rather than its inputs.
    fn refresh(&mut self, ui: &App) {
        let target = self.resolve();
        let now = Instant::now();
        if !self.initialized {
            self.initialized = true;
            self.initialize(ui, target, now);
            return;
        }
        if target == self.target {
            // The source did not move, but the darkness and the parallax are
            // read *through* this call and may have — the dim has to follow a
            // slider that changes neither the picture nor which one shows.
            self.update_dim(ui);
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

    /// Applies the very first resolution.
    ///
    /// A configured background is decoded on this thread and placed without a
    /// cross-fade, unlike every later change. The window has not been shown
    /// yet, so the decode only delays the first frame; doing it on the worker
    /// would instead put the world on screen and then ease the picture in over
    /// it, which is the "shows the default for a while" a launch should not
    /// have. Nothing is lost by not fading from the world here: there is no
    /// previous picture to move out.
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
        // An opaque picture is the whole background; a translucent one lets the
        // world show through it.
        self.set_world_shown(ui, alpha != Alpha::Opaque);
        self.update_dim(ui);
        self.sync(ui, now);
    }

    /// Replaces the image of the layer still fading in, when it is early enough
    /// in its fade for the swap not to be visible. Returns whether it took the
    /// target.
    ///
    /// The pixels come from the cache, which now holds them: the layer being
    /// replaced is the one still on screen, so its image cannot be borrowed to
    /// draw the new target. A target that has not been decoded waits — the load
    /// in flight may be for a different path, so taking it here would be wrong.
    fn retarget(&mut self, ui: &App, target: &Source) -> bool {
        let Some(fade) = self.fade else { return false };
        let Some(incoming) = fade.incoming else {
            // The incoming layer is the world; there is no image to swap.
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
        // The replacement may be translucent where the one it displaces was
        // not, so the world still needs to be behind it.
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
                // The world goes back on screen *before* the image starts to
                // leave, so no frame ever shows neither: it is rendered under
                // the outgoing image and revealed by its fade.
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
                // The world may stop being drawn only when *both* layers are
                // opaque: the outgoing has to cover it until the incoming has
                // arrived, and the incoming has to keep covering it after. A
                // translucent layer on either side leaves the world visible
                // through the cross-fade, so it has to be there.
                let outgoing = self.shown;
                let outgoing_covers =
                    outgoing.is_some_and(|slot| self.slots[slot].alpha == Alpha::Opaque);
                // The slot that is not on screen takes the new image.
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
                        // Not decoded yet: assume the worst until it is, so a
                        // translucent image does not have the world pulled out
                        // from under it.
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

    /// Starts the cross-fade for a slot whose image is on hand. `outgoing` is
    /// the slot being replaced, if any: it eases out while `incoming` eases in.
    fn begin_crossfade(&mut self, ui: &App, incoming: usize, outgoing: Option<usize>) {
        // A translucent incoming layer is no cover for the world, whether it
        // was cached or just decoded, so the world has to be under it.
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
                        // Remember the file that would not decode, and resolve
                        // again. `refresh` will skip it and settle on the next
                        // source — a corrupt image shows the world instead of
                        // being retried forever on every resolution.
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
                // The fade may have been started by something else in the
                // meantime, or not started at all.
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

        // Move the cross-fade on, and finish it at its end.
        self.apply_fade(ui, now);

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

    /// Writes the two opacities for the position `now` is at within the fade,
    /// then finishes the fade when the clock has run out. Both layers are moved
    /// from the one call, so the outgoing's fall and the incoming's rise are the
    /// same curve and the same instant.
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
            // The incoming layer has arrived: it is what the window shows now.
            self.shown = Some(incoming);
            self.settled = self.slots[incoming].source.clone().unwrap_or(Source::World);
            self.slots[incoming].opacity = 1.0;
            self.slots[incoming].target = 1.0;
            self.write_alpha(ui, incoming);
            if let Some(outgoing) = fade.outgoing {
                self.clear_slot(ui, outgoing);
            }
            // With the image fully in place and opaque, the world has nothing
            // to show and can stop rendering. A translucent one stays over the
            // world, which has to keep being drawn behind it.
            if self.slots[incoming].alpha == Alpha::Opaque {
                self.set_world_shown(ui, false);
            } else {
                self.set_world_shown(ui, true);
            }
        } else if let Some(outgoing) = fade.outgoing {
            // The world is back.
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
        // The world is drawn when it is what the window shows, while it is
        // still behind a transition, and behind a translucent custom image —
        // which is not an opaque cover, so the world shows through it and has
        // to be kept up to date. `world-shown` is the authority the layers act
        // on; this is the same question asked of the controller's own state.
        self.shown.is_none()
            || self.fade.is_some()
            || self
                .shown
                .is_some_and(|slot| self.slots[slot].alpha != Alpha::Opaque)
    }

    fn set_world_shown(&self, ui: &App, shown: bool) {
        ui.global::<Background>().set_world_shown(shown);
    }

    /// Hands the slot's image to the window. Images do not animate, so this is
    /// only ever called for a slot arriving at rest or just before a fade.
    fn set_slot_image(&self, ui: &App, slot: usize) {
        let background = ui.global::<Background>();
        match slot {
            0 => background.set_custom_a(self.slots[0].image.clone()),
            _ => background.set_custom_b(self.slots[1].image.clone()),
        }
    }

    /// Writes one slot's opacity. The cross-fade calls this for both slots from
    /// the same tick, which is what makes them move together.
    fn write_alpha(&self, ui: &App, slot: usize) {
        let background = ui.global::<Background>();
        match slot {
            0 => background.set_custom_a_alpha(self.slots[0].opacity),
            _ => background.set_custom_b_alpha(self.slots[1].opacity),
        }
    }

    /// Puts a slot back to empty at rest, both the image and its opacity.
    fn clear_slot(&mut self, ui: &App, slot: usize) {
        self.slots[slot].source = None;
        self.slots[slot].image = Image::default();
        self.slots[slot].opacity = 0.0;
        self.slots[slot].target = 0.0;
        self.set_slot_image(ui, slot);
        self.write_alpha(ui, slot);
    }

    /// Only the *global* custom background is dimmed, at the user's percentage
    /// — an instance background and the world are never dimmed. It follows the
    /// layer's *displayed* opacity, so the dim eases in and out with it, and
    /// the two slots are combined as they are composited (`1 - (1-a)(1-b)`),
    /// so replacing one global image with another does not lighten in the
    /// middle of the cross-fade.
    fn update_dim(&self, ui: &App) {
        let darkness = f32::from(self.config.borrow().appearance.background_darkness) / 100.0;
        let uncovered = self
            .slots
            .iter()
            .filter(|slot| matches!(&slot.source, Some(Source::Image { global: true, .. })))
            .fold(1.0f32, |uncovered, slot| uncovered * (1.0 - slot.opacity));
        ui.global::<Background>()
            .set_dim_alpha(darkness * (1.0 - uncovered));
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
/// is handed over with `crate::ui::services::report::report`, which also wakes the event loop
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

/// The image the UI thread has to make from what a worker decoded.
type Decoded = (SystemTime, SharedPixelBuffer<Rgba8Pixel>, Alpha);

/// Reads and decodes a background image on a worker thread.
///
/// The files are stored without an extension (`background_image`), so the
/// format is guessed from the bytes rather than from the name. Anything larger
/// than [`IMAGE_MAX_EDGE`] is scaled down once here, which is also the only
/// point at which it is cheap to ask whether the image has any transparency —
/// the answer decides whether the world stays behind it.
///
/// The scale is `DynamicImage::thumbnail_exact`, not `imageops::thumbnail`
/// directly, on purpose. The image-ops functions are generic and are
/// instantiated in the crate that calls them, so calling them here would put
/// the per-pixel loop in this (unoptimised) crate; going through the concrete
/// `DynamicImage` method keeps it inside `image`, which the dev profile
/// optimises (see the root `Cargo.toml`).
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
        // `thumbnail_exact` is the fast integer box down-scale; `resize` with a
        // Triangle filter gives a slightly smoother edge but costs an order of
        // magnitude more on a 4K image, and the difference is invisible under
        // `image-fit: cover`.
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
    // It refreshes even when the tuple did not move: an instance's background
    // keeps its file name, so replacing the image leaves the facts unchanged.
    // The resolution compares the file's timestamp and is cheap when nothing
    // moved, which is the guard that keeps this from doing work on every
    // unrelated refresh.
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

    /// The background is stored under a fixed name, so replacing it changes the
    /// file at the same path. The resolved source has to tell those apart or a
    /// new wallpaper is dismissed as "the same one" and never loaded — which is
    /// exactly the "it does not take effect" this fixes.
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

    /// A cache hit means "the pixels in it are the file on disk". Once the file
    /// is replaced, the entry is stale and has to miss so the load happens
    /// again — otherwise the old picture is reused forever.
    #[test]
    fn the_cache_only_returns_the_decoded_revision() {
        let path = PathBuf::from("/tmp/conic-background-test");
        let mut cache = ImageCache::default();
        cache.insert(path.clone(), decoded());
        assert!(cache.get(&path, mtime(1)).is_some());
        assert!(cache.get(&path, mtime(2)).is_none());
    }

    /// The source reads the file's timestamp, and reports nothing for a path
    /// that is not a readable file.
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

    /// The cross-fade has to move *both* layers at once: at its midpoint the
    /// outgoing and incoming layers are both half-way, rather than one waiting
    /// for the other to finish. The real component is built with Slint's test
    /// backend and the controller's own clock is driven by hand, so the two
    /// opacities the view would draw can be read straight back.
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
        // Seed the pixels so each fade starts without a worker decode: the load
        // path needs a live event loop, which the display-less backend has not
        // got.
        for name in ["/tmp/conic-a.png", "/tmp/conic-b.png"] {
            controller.cache.insert(PathBuf::from(name), decoded());
        }

        // A first background arrives over the world.
        controller.start_fade(&ui, source("/tmp/conic-a.png"));
        let first = controller.fade.expect("the first fade");
        assert_eq!(first.incoming, Some(0));
        assert_eq!(first.outgoing, None);
        // Part-way through, the incoming layer is really moving.
        controller.apply_fade(&ui, first.started + Duration::from_millis(100));
        assert!(ui.global::<Background>().get_custom_a_alpha() > 0.1);
        // Let it land at rest.
        controller.apply_fade(
            &ui,
            first.started + Duration::from_millis(FADE_MS as u64 + 1),
        );
        assert!(controller.fade.is_none(), "the first fade did not finish");
        assert_eq!(controller.shown, Some(0));

        // Cross-fade to a second background: the shown layer leaves while the
        // free one arrives.
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

        // The ticker is a runtime task; leave it stopped so the test does not
        // keep the process's worker awake.
        controller.ticking.store(false, Ordering::Relaxed);
    }

    /// The configured background has to be on the very first frame, not eased
    /// in from the world: the first resolution decodes it on this thread and
    /// places it whole, so there is no fade at all.
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
        // It is opaque, so the world has nothing to do behind it.
        assert!(!background.get_world_shown());
        controller.ticking.store(false, Ordering::Relaxed);
        std::fs::remove_dir_all(&dir).ok();
    }
}
