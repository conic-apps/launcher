// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The music player's callbacks, registered on `MusicState`.

use super::*;

/// Creates the player, restores the session and starts the clock.
///
/// It wires the initial volume from the settings; `restore_session` reads the
/// folder and the saved position. Focus tracking is wired separately by
/// `watch_focus`.
pub fn setup(ui: &App) {
    let player = match music::Player::new() {
        Ok(player) => player,
        Err(error) => {
            // A failed load leaves an empty playlist and logs; the panel still
            // opens and reports that there is no music.
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
            // Assume the window is focused until the first focus event settles
            // it; a correct answer needs a live window. Until then the main
            // volume applies, which is the common case.
            focused: true,
            ticking: Arc::new(AtomicBool::new(false)),
        })
    });

    register_callbacks(ui);
    apply(ui);
}

/// Watches the window's focus.
///
/// A focus change is the one volume change that ramps over a second instead of
/// applying at once — the point being that the user is looking at something else
/// while it happens.
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
            // Smoothed in *both* directions, whether the window gained or lost
            // the focus; the unsmoothed call is the one `setup` makes at startup.
            apply_volume(controller, if focused { main } else { background }, true);
        });
        apply(&ui);
    });
}

/// Reacts to a settings change: the volumes, and the switch that stops the music
/// outright.
///
/// Turning music off pauses it *and* closes the panel, so the overlay cannot be
/// left open over a player that is not playing.
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
        ui.global::<MusicState>().set_panel_open(false);
    }
    apply(ui);
}

/// Gives the device the volume the configuration asks for, remembering it so an
/// unchanged unsmoothed value is not handed over again.
pub(crate) fn apply_volume(controller: &mut Controller, percent: i32, smooth: bool) {
    let percent = volume_percent(percent);
    if !smooth && controller.volume == Some(percent) {
        return;
    }
    controller.volume = Some(percent);
    controller.player.set_volume(percent as f32 / 100.0, smooth);
}

/// A config percentage clamped to the 0..100 range the slider allows.
pub(crate) fn volume_percent(percent: i32) -> u8 {
    percent.clamp(0, 100) as u8
}

pub(crate) fn register_callbacks(ui: &App) {
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
            let path = storage::LOCATIONS.launcher.music.clone();
            if let Err(error) =
                crate::ui::services::app_config::open_external(&path.to_string_lossy())
            {
                log::warn!("failed to open the music folder: {error}");
            }
        });
    }
    {
        let weak = ui.as_weak();
        state.on_close_panel(move || {
            if let Some(ui) = weak.upgrade() {
                // Only `panelOpen`: the playlist stays as it was, so reopening
                // the panel brings it back unchanged.
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

    let weak = ui.as_weak();
    ui.on_toggle_music(move || {
        if let Some(ui) = weak.upgrade() {
            let state = ui.global::<MusicState>();
            state.set_panel_open(!state.get_panel_open());
            start_clock(&ui, true);
        }
    });
}
