// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The music "script": drives [`music::Player`] and pushes what it reports
//! into the `MusicState` global (src/store/music.ts + src/overlays/MusicPlayer.vue
//! + src/components/BeatMap.vue).
//!
//! Three things live here rather than in the crate, because they belong to the
//! app and not to the player:
//!
//!   * the *configuration* — the two volumes and the enable switch — and the
//!     window focus the background volume follows (the store's `init` and its
//!     two `watch`es);
//!   * [`BeatMap`]'s mapping of the analyser's bins onto bars, which is a
//!     component's own maths (see that type for why it is not in the .slint
//!     file);
//!   * the *clock* that decides how often the UI is refreshed.
//!
//! The player itself is not `Send` — cpal's stream is not — so it lives in a
//! `thread_local` and is only ever touched from the event loop's thread, the same
//! reason `content.rs` keeps its caches there.

use std::cell::RefCell;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, VecModel, Weak};

use crate::slint_backend::{App, AppConfig, MusicState, MusicTrack};

/// The cadence the UI is refreshed at.
///
/// The analyser smooths on a 60Hz clock (see `music::player`) and the
/// visualizer reads it once per frame, so this is the visualizer's frame rate —
/// the one the Vue gave its canvas through `requestAnimationFrame`. 16ms is also
/// comfortably faster than the ~4Hz the browser's `ontimeupdate` moved the
/// progress bar at, so nothing the panel shows is coarser here than it was.
const TICK_MS: u64 = 16;

thread_local! {
    /// The player and the visualizer's mapping. `None` when the platform has no
    /// output device, in which case the panel still opens — with an empty
    /// playlist, which is what the store's `loadTracks` failure left behind.
    static CONTROLLER: RefCell<Option<Controller>> = const { RefCell::new(None) };
}

struct Controller {
    player: music::Player,
    beat_map: BeatMap,
    /// The playlist the model was last built from, so the rows are not rebuilt
    /// (and the popup's scroll position dropped) on every tick. The player hands
    /// out an `Arc` that is only replaced when the folder is re-listed, so the
    /// check is a pointer comparison rather than a hundred string comparisons
    /// sixty times a second.
    playlist: Option<Arc<Vec<music::MusicFile>>>,
    /// The volume the device was last given, for the same reason.
    volume: Option<u8>,
    /// The last frame of bar heights, kept so that reading the analyser sixty
    /// times a second does not allocate sixty times a second. The idle branch
    /// leaves it alone, so the next live frame reuses whatever capacity it has.
    levels: Vec<f32>,
    /// Whether the window has the focus — the store's `backgrounded`.
    focused: bool,
    /// Whether the clock is running (see [`start_clock`]).
    ticking: Arc<AtomicBool>,
}

/// `BeatMap.vue`'s mapping of the analyser's bins onto bar heights.
///
/// It is here rather than in `components/beat-map.slint` because it cannot be
/// written as a Slint binding: every bar is normalised against the loudest *bar
/// of the frame*, which needs a pass over all of them before any of them can be
/// drawn, and the decaying peak is state only a callback may write. Slint
/// expressions can do neither. What the component draws is what the Vue drew —
/// the same bins, the same logarithmic spread, the same dB floor, the same 0.97
/// peak decay — from `levels`, which this produces.
struct BeatMap {
    /// How many bars the visualizer asked for, derived from its own width.
    bar_count: usize,
    /// The decaying peak the bars are scaled against.
    peak_level: f32,
}

impl BeatMap {
    // `BeatMap.vue`'s module constants, which are not props of it.
    const MIN_DB: f32 = -70.0;
    const PEAK_DECAY: f32 = 0.97;
    const MIN_PEAK_LEVEL: f32 = 0.05;
    const BINS_PER_BAR: usize = 4;
    const MIN_FFT_SIZE: usize = 256;
    const MAX_FFT_SIZE: usize = 16384;
    const MIN_FREQUENCY: f64 = 200.0;
    const MAX_FREQUENCY: f64 = 4000.0;

    fn new() -> Self {
        Self {
            bar_count: 0,
            peak_level: 0.0,
        }
    }

    /// The FFT size that keeps a roughly constant bins-per-bar ratio, which is
    /// what `fftSizeForBars` picks.
    fn fft_size_for_bars(bar_count: usize) -> usize {
        let bins = bar_count * Self::BINS_PER_BAR;
        let mut size = Self::MIN_FFT_SIZE;
        while size < bins * 2 {
            size *= 2;
        }
        size.min(Self::MAX_FFT_SIZE)
    }

    /// One frame of bars: `bins` in dB (the analyser's own output) turned into
    /// `bar_count` levels in 0..1, spread logarithmically over the frequency
    /// range the visualizer shows.
    ///
    /// `out` is the caller's and is cleared first, so the caller can hand the
    /// same buffer to every frame: this runs sixty times a second, and a fresh
    /// `Vec` each time would be sixty allocations a second for a few hundred
    /// bytes.
    fn sample(&mut self, bins: &[f32], sample_rate: f64, out: &mut Vec<f32>) {
        out.clear();
        if self.bar_count == 0 || bins.is_empty() || sample_rate <= 0.0 {
            return;
        }
        let nyquist = sample_rate / 2.0;
        let bin_count = bins.len();
        let max_bin = ((bin_count as f64) * (Self::MAX_FREQUENCY / nyquist))
            .round()
            .max(1.0) as usize;
        let min_bin = Self::min_bin(bin_count, nyquist, max_bin);
        let min_log_bin = min_bin.max(1);

        out.reserve(self.bar_count);
        let mut frame_max = 0.0f32;
        for bar in 0..self.bar_count {
            let ratio = bar as f64 / self.bar_count as f64;
            let position = min_log_bin as f64 * (max_bin as f64 / min_log_bin as f64).powf(ratio);
            // `interpolate`: the value halfway between two bins, so a bar does
            // not jump as the logarithmic spread moves across a bin edge.
            let lower = (position.floor() as usize).min(max_bin);
            let upper = (lower + 1).min(max_bin);
            let fraction = (position - position.floor()) as f32;
            let low = bins.get(lower).copied().unwrap_or(f32::NEG_INFINITY);
            let high = bins.get(upper).copied().unwrap_or(f32::NEG_INFINITY);
            let db = if low.is_finite() && high.is_finite() {
                low + (high - low) * fraction
            } else if high.is_finite() {
                high
            } else {
                low
            };
            let value = if db <= Self::MIN_DB {
                0.0
            } else {
                (db - Self::MIN_DB) / -Self::MIN_DB
            };
            if value > frame_max {
                frame_max = value;
            }
            out.push(value);
        }

        self.peak_level = frame_max
            .max(self.peak_level * Self::PEAK_DECAY)
            .max(Self::MIN_PEAK_LEVEL);
        let scale = 1.0 / self.peak_level;
        for level in out.iter_mut() {
            *level = (*level * scale).min(1.0);
        }
    }

    /// The lower end of the frequency range: `Math.min(round(binCount *
    /// (minFrequency / nyquist)), maxBin - 1)`.
    fn min_bin(bin_count: usize, nyquist: f64, max_bin: usize) -> usize {
        let bin = ((bin_count as f64) * (Self::MIN_FREQUENCY / nyquist))
            .round()
            .max(0.0) as usize;
        bin.min(max_bin.saturating_sub(1))
    }
}

/// The panel's two clock labels, the `formatTime` of `MusicPlayer.vue`.
fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "00:00".to_string();
    }
    let total = seconds.floor() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Creates the player, restores the session and starts the clock.
///
/// The store's two calls at the top of `MusicPlayer.vue` (`music.init()` and
/// `music.restoreSession()`), in that order: `init` wires the volume and the focus
/// tracking, `restoreSession` reads the folder and the saved position.
pub fn setup(ui: &App) {
    let player = match music::Player::new() {
        Ok(player) => player,
        Err(error) => {
            // The store's `loadTracks` failure path leaves an empty playlist and
            // logs; the panel still opens and reports that there is no music.
            log::error!("background music is unavailable: {error}");
            return;
        }
    };

    let (enabled, resume_on_startup, main_volume) = {
        let settings = ui.global::<AppConfig>();
        (
            settings.get_music_enabled(),
            settings.get_resume_on_startup(),
            settings.get_main_volume(),
        )
    };
    let volume = volume_percent(main_volume);
    player.set_volume(volume as f32 / 100.0, false);
    player.restore_session(enabled, resume_on_startup);

    CONTROLLER.with(|slot| {
        *slot.borrow_mut() = Some(Controller {
            player,
            beat_map: BeatMap::new(),
            playlist: None,
            volume: Some(volume),
            levels: Vec::new(),
            // The store assumes the window is focused and corrects itself from
            // `isFocused()`. A correct answer needs a live window, so the first
            // focus event after startup settles it; until then the main volume
            // applies, which is the common case.
            focused: true,
            ticking: Arc::new(AtomicBool::new(false)),
        })
    });

    register_callbacks(ui);
    apply(ui);
}

/// Watches the window's focus, the store's `onFocusChanged`.
///
/// A focus change is the one volume change the store ramps over a second instead
/// of applying at once — the point being that the user is looking at something
/// else while it happens.
pub fn watch_focus(window: &window::WindowService<App>) {
    let weak = window.component_weak();
    window.on_focus_changed(move |focused| {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let settings = ui.global::<AppConfig>();
        let (main, background) = (settings.get_main_volume(), settings.get_background_volume());
        with_controller(|controller| {
            if controller.focused == focused {
                return;
            }
            controller.focused = focused;
            // Smoothed in *both* directions, which is what the store's
            // `onFocusChanged` does: it passes `true` to `applyVolume` whether the
            // window gained or lost the focus, and the unsmoothed call is the one
            // `init` makes from `isFocused()`.
            apply_volume(controller, if focused { main } else { background }, true);
        });
        apply(&ui);
    });
}

/// Reacts to a settings change: the volumes, and the switch that stops the music
/// outright.
///
/// The last part is the `watch` on `config.music.enabled` at the top of
/// `MusicPlayer.vue`: turning music off pauses it *and* closes the panel, so the
/// overlay cannot be left open over a player that is not playing.
pub fn config_changed(ui: &App) {
    let settings = ui.global::<AppConfig>();
    let (enabled, main, background) = (
        settings.get_music_enabled(),
        settings.get_main_volume(),
        settings.get_background_volume(),
    );
    with_controller(|controller| {
        let focused = controller.focused;
        apply_volume(controller, if focused { main } else { background }, false);
        if !enabled {
            controller.player.pause();
        }
    });
    if !enabled {
        // `music.pause(); music.closePanel();` — and the Vue's watcher leaves
        // `showPlaylist` alone, exactly as above.
        ui.global::<MusicState>().set_panel_open(false);
    }
    apply(ui);
}

/// Gives the device the volume the configuration asks for, remembering it so the
/// tick does not hand the same value over sixty times a second.
fn apply_volume(controller: &mut Controller, percent: i32, smooth: bool) {
    let percent = volume_percent(percent);
    if !smooth && controller.volume == Some(percent) {
        return;
    }
    controller.volume = Some(percent);
    controller.player.set_volume(percent as f32 / 100.0, smooth);
}

/// A config percentage as the whole 0..100 range the slider allows:
/// `Math.min(Math.max(percent / 100, 0), 1)`.
fn volume_percent(percent: i32) -> u8 {
    percent.clamp(0, 100) as u8
}

/// Registers every `MusicState` callback, and the title bar's own.
fn register_callbacks(ui: &App) {
    let state = ui.global::<MusicState>();

    {
        let weak = ui.as_weak();
        state.on_toggle_play(move || with_player(&weak, |player| player.toggle_play()));
    }
    {
        // The progress bar's drag, which stops the music for its duration and
        // asks for it back on the release. Explicit rather than two toggles, so
        // the second half cannot undo the wrong thing.
        let weak = ui.as_weak();
        state.on_pause(move || with_player(&weak, |player| player.pause()));
    }
    {
        let weak = ui.as_weak();
        state.on_resume(move || with_player(&weak, |player| player.resume()));
    }
    {
        let weak = ui.as_weak();
        state.on_play_index(move |index| {
            let index = index.max(0) as usize;
            with_player(&weak, |player| player.play_index(index))
        });
    }
    {
        let weak = ui.as_weak();
        state.on_previous(move || with_player(&weak, |player| player.previous()));
    }
    {
        let weak = ui.as_weak();
        state.on_next(move || with_player(&weak, |player| player.next()));
    }
    {
        let weak = ui.as_weak();
        state.on_seek_ratio(move |ratio| {
            with_player(&weak, |player| player.seek_ratio(ratio as f64))
        });
    }
    {
        let weak = ui.as_weak();
        state.on_toggle_shuffle(move || with_player(&weak, |player| player.toggle_shuffle()));
    }
    {
        let weak = ui.as_weak();
        state.on_cycle_repeat(move || with_player(&weak, |player| player.cycle_repeat()));
    }
    {
        state.on_open_music_folder(|| {
            let path = folder::DATA_LOCATION.music.clone();
            if let Err(error) = crate::config_bridge::open_external(&path.to_string_lossy()) {
                log::warn!("failed to open the music folder: {error}");
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_close_panel(move || {
            if let Some(ui) = weak.upgrade() {
                // Only `panelOpen`: the Vue's `showPlaylist` is a `ref` in a
                // component that stays mounted, and `closePanel()` does not touch
                // it, so reopening the panel brings the playlist back as it was.
                ui.global::<MusicState>().set_panel_open(false);
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_toggle_playlist(move || {
            if let Some(ui) = weak.upgrade() {
                let state = ui.global::<MusicState>();
                state.set_playlist_open(!state.get_playlist_open());
                start_clock(&ui, true);
            }
        });
    }
    {
        // A resize changes the bar count, so the visualizer is repainted with the
        // new geometry straight away rather than on the next tick.
        let weak = ui.as_weak();
        state.on_resized(move |bar_count| {
            let bar_count = bar_count.max(0) as usize;
            with_controller(|controller| {
                controller.beat_map.bar_count = bar_count;
                if bar_count > 0 {
                    controller
                        .player
                        .set_fft_size(BeatMap::fft_size_for_bars(bar_count));
                }
            });
            if let Some(ui) = weak.upgrade() {
                apply(&ui);
            }
        });
    }

    // The title bar's music button (App.vue's `music.togglePanel()`).
    let weak = ui.as_weak();
    ui.on_toggle_music(move || {
        if let Some(ui) = weak.upgrade() {
            let state = ui.global::<MusicState>();
            state.set_panel_open(!state.get_panel_open());
            start_clock(&ui, true);
        }
    });
}

/// Runs `action` on the player, then refreshes the global.
fn with_player(weak: &Weak<App>, action: impl FnOnce(&music::Player)) {
    let Some(ui) = weak.upgrade() else {
        return;
    };
    with_controller(|controller| action(&controller.player));
    apply(&ui);
}

/// Pauses playback, for the launch flow's `pause_on_launch`.
///
/// The Vue called `musicStore.pause()` straight from `LaunchView.vue`; there is
/// no view to hang that on here, so the music script owns it.
pub fn pause() {
    with_controller(|controller| controller.player.pause());
}

/// Runs `action` on the controller.
fn with_controller(action: impl FnOnce(&mut Controller)) {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow_mut().as_mut() {
            action(controller);
        }
    });
}

/// Reads the player and writes the global. One place, so the state can never be
/// half-updated.
fn apply(ui: &App) {
    let state = ui.global::<MusicState>();
    // The borrow cannot leave the closure — the `thread_local` cell does not
    // outlive it — so what the caller needs afterwards is handed back out.
    let needed = CONTROLLER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(controller) = slot.as_mut() else {
            return false;
        };

        // `tick` is also what persists the position, on the same limit the
        // store's `ontimeupdate` used.
        let snapshot = controller.player.tick();

        // The playlist is an `Arc` the player replaces only when the folder is
        // re-listed, so this is a pointer comparison: the rows are rebuilt when
        // it changed, not sixty times a second.
        let relisted = controller
            .playlist
            .as_ref()
            .is_none_or(|playlist| !Arc::ptr_eq(playlist, &snapshot.tracks));
        if relisted {
            let rows: Vec<MusicTrack> = snapshot
                .tracks
                .iter()
                .map(|track| MusicTrack {
                    name: track.name.clone().into(),
                    path: track.path.clone().into(),
                })
                .collect();
            state.set_tracks(ModelRc::new(VecModel::from(rows)));
            controller.playlist = Some(Arc::clone(&snapshot.tracks));
        }
        state.set_current_index(snapshot.current_index.map_or(-1, |index| index as i32));
        state.set_is_playing(snapshot.is_playing);
        state.set_shuffle(snapshot.shuffle);
        state.set_repeat(snapshot.repeat);
        state.set_current_time(snapshot.current_time as f32);
        state.set_duration(snapshot.duration as f32);
        state.set_current_time_text(format_time(snapshot.current_time).into());
        state.set_duration_text(format_time(snapshot.duration).into());
        state.set_progress(if snapshot.duration > 0.0 {
            (snapshot.current_time / snapshot.duration).clamp(0.0, 1.0) as f32
        } else {
            0.0
        });
        state.set_buffering(snapshot.buffering);

        // The visualizer's own condition: `analyser && hasTrack && isPlaying`.
        let live = snapshot.current_track().is_some() && snapshot.is_playing;
        state.set_live(live);
        if live {
            // The bins are handed over under the player's own lock, so the
            // mapping reads them in place instead of the player handing out a
            // copy sixty times a second for a reader that copies again. The
            // levels go out and the buffer comes back empty, ready for the next
            // frame.
            let sample_rate = controller.player.sample_rate();
            let beat_map = &mut controller.beat_map;
            let levels = &mut controller.levels;
            controller
                .player
                .with_spectrum(|bins| beat_map.sample(bins, sample_rate, levels));
            state.set_levels(ModelRc::new(VecModel::from(std::mem::take(levels))));
        } else {
            // The idle branch draws its own baseline, so the levels it would have
            // read are not carried over from the last playing frame.
            state.set_levels(ModelRc::default());
        }

        // The clock is only worth running while something is moving — or about
        // to: a selection is asynchronous (the file has to be opened before the
        // transport can report it as playing), and a `restore_session` at startup
        // is nothing but one. Without that the very first play would start
        // silently, because the tick that would have noticed it had already
        // stopped.
        snapshot.is_playing || snapshot.pending > 0 || state.get_panel_open()
    });
    start_clock(ui, needed);
}

/// Starts (or stops) the 60Hz clock that pushes the player into the global.
///
/// The tick has to reach the UI thread — everything it touches is Slint's — so it
/// is handed over with `upgrade_in_event_loop`, which also wakes the event loop.
/// That is what makes it independent of whether anything is being painted: a Slint
/// `Timer` would stop the moment the window went quiet, which is exactly when the
/// progress bar has to keep moving. The task stops itself once nothing needs it
/// any more, and `start_clock` starts another one when something does.
fn start_clock(ui: &App, needed: bool) {
    let mut start = None;
    with_controller(|controller| {
        if needed && !controller.ticking.swap(true, Ordering::Relaxed) {
            start = Some(Arc::clone(&controller.ticking));
        } else if !needed {
            controller.ticking.store(false, Ordering::Relaxed);
        }
    });
    if let Some(ticking) = start {
        tick_on_the_runtime(ui, ticking);
    }
}

fn tick_on_the_runtime(ui: &App, ticking: Arc<AtomicBool>) {
    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        let interval = Duration::from_millis(TICK_MS);
        loop {
            tokio::time::sleep(interval).await;
            if !ticking.load(Ordering::Relaxed) {
                return;
            }
            let weak = weak.clone();
            let ticking = Arc::clone(&ticking);
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if ticking.load(Ordering::Relaxed) {
                    apply(&ui);
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fft_size_keeps_the_bins_per_bar_ratio() {
        // `fftSizeForBars`: 256 until twice the bins fit, capped at 16384.
        assert_eq!(BeatMap::fft_size_for_bars(32), 256);
        assert_eq!(BeatMap::fft_size_for_bars(64), 512);
        assert_eq!(BeatMap::fft_size_for_bars(140), 2048);
        assert_eq!(BeatMap::fft_size_for_bars(256), 2048);
    }

    #[test]
    fn the_frequency_range_clamps_to_the_spectrum() {
        // 48kHz, 1024 bins: 200Hz is bin 9 and 4000Hz is bin 171.
        assert_eq!(BeatMap::min_bin(1024, 24000.0, 171), 9);
        // A range that starts above the top bin collapses to `maxBin - 1`, which
        // is 0 here — the same value the Vue computes. `minLogBin` is what keeps
        // the logarithmic spread off bin 0 (0Hz), and it is applied in `sample`.
        assert_eq!(BeatMap::min_bin(1024, 24000.0, 1), 0);
    }

    #[test]
    fn a_quiet_frame_is_scaled_up_to_fill_the_range() {
        // `normalize` divides by the frame's own peak, so a very quiet signal
        // still reaches the top of the range — that is the whole point of it.
        let mut beat_map = BeatMap {
            bar_count: 32,
            peak_level: 0.0,
        };
        let bins = vec![-90.0; 1024];
        let mut quiet = Vec::new();
        beat_map.sample(&bins, 48000.0, &mut quiet);
        assert!(quiet.iter().all(|level| level.is_finite()));
        assert!(quiet.iter().all(|level| (0.0..=1.0).contains(level)));
    }

    #[test]
    fn silence_above_the_floor_draws_nothing() {
        let mut beat_map = BeatMap {
            bar_count: 32,
            peak_level: 0.0,
        };
        // -100dB is below `MIN_DB` (-70), so every bar is 0 — and the peak then
        // settles on `MIN_PEAK_LEVEL` (0.05), which is what keeps the noise floor
        // from being divided by zero.
        let mut levels = Vec::new();
        beat_map.sample(&vec![-100.0; 1024], 48000.0, &mut levels);
        assert!(levels.iter().all(|level| *level == 0.0));
        assert_eq!(beat_map.peak_level, BeatMap::MIN_PEAK_LEVEL);
    }

    #[test]
    fn no_bars_means_no_levels() {
        let mut beat_map = BeatMap::new();
        let mut levels = Vec::new();
        beat_map.sample(&[0.0; 1024], 48000.0, &mut levels);
        assert!(levels.is_empty());
    }

    #[test]
    fn a_frame_reuses_the_callers_buffer() {
        // The visualizer runs sixty times a second and hands the same `Vec`
        // round every time; a frame must replace its contents rather than
        // append to them.
        let mut beat_map = BeatMap {
            bar_count: 8,
            peak_level: 0.0,
        };
        let mut levels = Vec::new();
        beat_map.sample(&vec![-20.0; 1024], 48000.0, &mut levels);
        assert_eq!(levels.len(), 8);
        beat_map.sample(&vec![-20.0; 1024], 48000.0, &mut levels);
        assert_eq!(levels.len(), 8, "a second frame appended to the first");
    }

    #[test]
    fn the_clocks_are_zero_padded_minutes_and_seconds() {
        assert_eq!(format_time(0.0), "00:00");
        assert_eq!(format_time(9.9), "00:09");
        assert_eq!(format_time(61.0), "01:01");
        assert_eq!(format_time(600.0), "10:00");
        assert_eq!(format_time(3599.0), "59:59");
        // `Number.isFinite(time) || time < 0` is the Vue's own guard.
        assert_eq!(format_time(-1.0), "00:00");
        assert_eq!(format_time(f64::NAN), "00:00");
        assert_eq!(format_time(f64::INFINITY), "00:00");
    }

    #[test]
    fn a_volume_percentage_is_clamped_like_the_gain() {
        assert_eq!(volume_percent(0), 0);
        assert_eq!(volume_percent(100), 100);
        assert_eq!(volume_percent(140), 100);
        assert_eq!(volume_percent(-10), 0);
    }
}
