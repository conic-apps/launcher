// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Driving the audio player: volume, transport and the clock.

use super::*;

pub(crate) fn with_player(weak: &Weak<App>, action: impl FnOnce(&music::Player)) {
    let Some(ui) = weak.upgrade() else {
        return;
    };
    with_controller(|controller| action(&controller.player));
    apply(&ui);
}

/// Pauses playback, for the launch flow's `pause_on_launch`.
///
/// Owned by the music script because the launch flow runs without a view.
pub fn pause() {
    with_controller(|controller| controller.player.pause());
}

pub(crate) fn with_controller(action: impl FnOnce(&mut Controller)) {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow_mut().as_mut() {
            action(controller);
        }
    });
}

/// Reads the player and writes the global. One place, so the state can never be
/// half-updated.
pub(crate) fn apply(ui: &App) {
    let state = ui.global::<MusicState>();
    // The borrow cannot leave the closure — the `thread_local` cell does not
    // outlive it — so what the caller needs afterwards is handed back out.
    let needed = CONTROLLER.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(controller) = slot.as_mut() else {
            return false;
        };

        // `tick` is also what persists the position.
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

        // The visualizer draws only while a track is playing.
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
/// is handed over with `crate::ui::services::report::report`, which also wakes the event loop.
/// That is what makes it independent of whether anything is being painted: a Slint
/// `Timer` would stop the moment the window went quiet, which is exactly when the
/// progress bar has to keep moving. The task stops itself once nothing needs it
/// any more, and `start_clock` starts another one when something does.
pub(crate) fn start_clock(ui: &App, needed: bool) {
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

pub(crate) fn tick_on_the_runtime(ui: &App, ticking: Arc<AtomicBool>) {
    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let interval = Duration::from_millis(TICK_MS);
        loop {
            tokio::time::sleep(interval).await;
            if !ticking.load(Ordering::Relaxed) {
                return;
            }
            let weak = weak.clone();
            let ticking = Arc::clone(&ticking);
            crate::ui::services::report::report(&weak, move |ui| {
                if ticking.load(Ordering::Relaxed) {
                    apply(&ui);
                }
            });
        }
    });
}
