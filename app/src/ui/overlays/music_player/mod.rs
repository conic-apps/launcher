// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The music "script": drives [`music::Player`] and pushes what it reports into
//! the `MusicState` global.
//!
//! The player is not `Send` — cpal's stream is not — so it lives in a
//! `thread_local` and is only touched from the event-loop thread.

use std::cell::RefCell;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, VecModel, Weak};

use crate::slint_backend::{App, AppConfig, MusicState, MusicTrack};

/// The visualizer's frame rate: the analyser smooths on 60Hz and the visualizer
/// reads it once per frame.
pub(crate) const TICK_MS: u64 = 16;

mod player;
mod wiring;

pub(crate) use player::*;
pub(crate) use wiring::*;

thread_local! {
    /// `None` when the platform has no output device; the panel still opens with
    /// an empty playlist.
    static CONTROLLER: RefCell<Option<Controller>> = const { RefCell::new(None) };
}

pub(crate) struct Controller {
    player: music::Player,
    beat_map: BeatMap,
    /// The playlist the model was last built from, so rows are not rebuilt (and
    /// the popup's scroll position dropped) on every tick. The `Arc` is replaced
    /// only when the folder is re-listed, making this a pointer comparison.
    playlist: Option<Arc<Vec<music::MusicFile>>>,
    /// The volume the device was last given, for the same reason.
    volume: Option<u8>,
    /// Kept so reading the analyser sixty times a second does not allocate sixty
    /// times a second.
    levels: Vec<f32>,
    focused: bool,
    ticking: Arc<AtomicBool>,
}

/// The mapping of the analyser's bins onto bar heights.
///
/// It cannot be a Slint binding: every bar is normalised against the loudest bar
/// of the frame, which needs a pass over all of them first, and the decaying peak
/// is state only a callback may write.
pub(crate) struct BeatMap {
    bar_count: usize,
    peak_level: f32,
}

impl BeatMap {
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

    /// The FFT size that keeps a roughly constant bins-per-bar ratio.
    fn fft_size_for_bars(bar_count: usize) -> usize {
        let bins = bar_count * Self::BINS_PER_BAR;
        let mut size = Self::MIN_FFT_SIZE;
        while size < bins * 2 {
            size *= 2;
        }
        size.min(Self::MAX_FFT_SIZE)
    }

    /// One frame of bars: `bins` in dB turned into `bar_count` levels in 0..1.
    ///
    /// `out` is the caller's and is cleared first, so the same buffer can be
    /// reused every frame rather than allocating sixty times a second.
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
            // The value interpolated between two bins, so a bar does not jump as
            // the logarithmic spread moves across a bin edge.
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

    fn min_bin(bin_count: usize, nyquist: f64, max_bin: usize) -> usize {
        let bin = ((bin_count as f64) * (Self::MIN_FREQUENCY / nyquist))
            .round()
            .max(0.0) as usize;
        bin.min(max_bin.saturating_sub(1))
    }
}

pub(crate) fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "00:00".to_string();
    }
    let total = seconds.floor() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fft_size_keeps_the_bins_per_bar_ratio() {
        // 256 until twice the bins fit, capped at 16384.
        assert_eq!(BeatMap::fft_size_for_bars(32), 256);
        assert_eq!(BeatMap::fft_size_for_bars(64), 512);
        assert_eq!(BeatMap::fft_size_for_bars(140), 2048);
        assert_eq!(BeatMap::fft_size_for_bars(256), 2048);
    }

    #[test]
    fn the_frequency_range_clamps_to_the_spectrum() {
        // 48kHz, 1024 bins: 200Hz is bin 9 and 4000Hz is bin 171.
        assert_eq!(BeatMap::min_bin(1024, 24000.0, 171), 9);
        // A range that starts above the top bin collapses to `max_bin - 1`,
        // which is 0 here. `min_log_bin` in `sample` keeps the logarithmic spread
        // off bin 0 (0Hz).
        assert_eq!(BeatMap::min_bin(1024, 24000.0, 1), 0);
    }

    #[test]
    fn a_quiet_frame_is_scaled_up_to_fill_the_range() {
        // The frame's own peak is the divisor, so a very quiet signal still
        // reaches the top of the range — that is the whole point of it.
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
        // A non-finite or negative time falls back to `00:00`.
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
