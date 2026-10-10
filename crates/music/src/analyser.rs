// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The frequency analyser the footer visualizer reads, reproducing the Web Audio
//! `AnalyserNode` step for step.
//!
//! The Web Audio spec (and every implementation of it) defines one
//! `getFloatFrequencyData` read as:
//!
//!   1. Blackman-window the last `fftSize` samples of the signal,
//!   2. a DFT of that size, normalised by `1/fftSize`,
//!   3. an exponential smoothing of the *magnitudes* across reads, with
//!      `smoothingTimeConstant` as the carry-over factor,
//!   4. and finally the `20·log10` the float accessor reports.
//!
//! Smoothing the magnitudes and only then taking the logarithm matters: doing it
//! the other way round (a plain average of the previous frame's dB) is a
//! different filter, and the visualizer is the only thing that reads this.
//!
//! The analyser taps what the device is given, after the gain has been applied,
//! so the volume scales the bars along with the sound.

use std::sync::Arc;

use rustfft::{Fft, FftPlanner, num_complex::Complex32};

/// The default `fftSize`.
pub const DEFAULT_FFT_SIZE: usize = 2048;

/// The `smoothingTimeConstant`, matching the Web Audio default.
pub const SMOOTHING_TIME_CONSTANT: f32 = 0.8;

/// A Web Audio `AnalyserNode`.
///
/// `fftSize` is a power of two in the range 32..32768 in Web Audio; the
/// visualiser is the only thing that ever sets it (through the bar count) and it
/// always rounds to a power of two itself, so an out-of-range value is clamped
/// rather than rejected.
pub struct Analyser {
    fft_size: usize,
    window: Vec<f32>,
    fft: Arc<dyn Fft<f32>>,
    /// The smoothed magnitudes, in linear units — not dB (see the module docs).
    smoothed: Vec<f32>,
    /// The transform's input, kept between reads. A fresh `Vec` per read would be
    /// an allocation on the audio thread sixty times a second.
    scratch: Vec<Complex32>,
}

impl Analyser {
    pub fn new(fft_size: usize) -> Self {
        let fft_size = normalize_fft_size(fft_size);
        let mut analyser = Self {
            fft_size,
            window: blackman_window(fft_size),
            fft: plan_fft(fft_size),
            smoothed: Vec::new(),
            scratch: Vec::new(),
        };
        analyser
            .smoothed
            .resize(analyser.frequency_bin_count(), 0.0);
        analyser
    }

    /// The `fftSize`: the number of samples every read transforms, and twice
    /// [`Self::frequency_bin_count`].
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    /// The `frequencyBinCount`: the number of values a read reports.
    pub fn frequency_bin_count(&self) -> usize {
        self.fft_size / 2
    }

    /// Resizes the transform, as `analyser.fftSize = size` does. A no-op when
    /// the size is already what the analyser is using.
    pub fn set_fft_size(&mut self, fft_size: usize) {
        let fft_size = normalize_fft_size(fft_size);
        if fft_size == self.fft_size {
            return;
        }
        self.fft_size = fft_size;
        self.window = blackman_window(fft_size);
        self.fft = plan_fft(fft_size);
        // The smoothed magnitudes belong to the old transform, so they start
        // over, which is what changing `fftSize` does under Web Audio too.
        self.smoothed.clear();
        self.smoothed.resize(self.frequency_bin_count(), 0.0);
    }

    /// One `getFloatFrequencyData`: the spectrum of `input` (the samples the
    /// analyser has been fed, newest last), in dB, one value per frequency bin.
    ///
    /// Fewer samples than `fftSize` are zero-padded at the front and more are
    /// truncated to the newest `fftSize`, matching the Web Audio ring buffer.
    pub fn read(&mut self, input: &[f32]) -> Vec<f32> {
        let mut bins = Vec::with_capacity(self.frequency_bin_count());
        self.read_into(input, &mut bins);
        bins
    }

    /// [`Self::read`] writing into `bins`, which is cleared first.
    ///
    /// The audio callback uses this rather than `read` so that a read allocates
    /// nothing: handing out a fresh `Vec` every read would put an allocation on
    /// the one thread that cannot afford one.
    pub fn read_into(&mut self, input: &[f32], bins: &mut Vec<f32>) {
        let size = self.fft_size;
        let skip = input.len().saturating_sub(size);
        self.scratch.clear();
        self.scratch
            .reserve(size.saturating_sub(self.scratch.capacity()));
        for index in 0..size {
            let sample = input.get(skip + index).copied().unwrap_or(0.0);
            self.scratch
                .push(Complex32::new(sample * self.window[index], 0.0));
        }
        self.fft.process(&mut self.scratch);

        // The spec's `1/N` normalisation, applied to the magnitudes.
        let scale = 1.0 / size as f32;
        let carry = SMOOTHING_TIME_CONSTANT;
        let keep = 1.0 - carry;
        for (slot, spectrum) in self.smoothed.iter_mut().zip(self.scratch.iter()) {
            // A silent bin is clamped to `f32::MIN_POSITIVE`, not left at 0: the
            // logarithm of 0 is `-inf`, and the visualiser treats anything at or
            // below its own `MIN_DB` as silence anyway.
            let magnitude = (spectrum.norm() * scale).max(f32::MIN_POSITIVE);
            *slot = carry * *slot + keep * magnitude;
        }

        bins.clear();
        bins.extend(
            self.smoothed
                .iter()
                .map(|magnitude| 20.0 * magnitude.log10()),
        );
    }
}

/// The Blackman window the spec applies before every transform.
fn blackman_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|index| {
            let n = index as f32 / size as f32;
            0.42 - 0.5 * (2.0 * std::f32::consts::PI * n).cos()
                + 0.08 * (4.0 * std::f32::consts::PI * n).cos()
        })
        .collect()
}

fn plan_fft(size: usize) -> Arc<dyn Fft<f32>> {
    FftPlanner::<f32>::new().plan_fft_forward(size)
}

/// Clamps a requested FFT size the way the `fftSize` attribute does: a power of
/// two, at least 32 and at most 32768.
fn normalize_fft_size(size: usize) -> usize {
    size.clamp(32, 32768).next_power_of_two()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 48000.0;

    /// A sine wave at `frequency`, at the analyser's own size.
    fn sine(frequency: f32) -> Vec<f32> {
        (0..DEFAULT_FFT_SIZE)
            .map(|index| {
                (2.0 * std::f32::consts::PI * frequency * index as f32 / SAMPLE_RATE).sin()
            })
            .collect()
    }

    /// Reads until the smoothing has settled, the way the visualiser does: it
    /// reads once per frame, and the carry-over is 0.8 of the previous value.
    fn settled(mut analyser: Analyser, input: &[f32]) -> Vec<f32> {
        let mut spectrum = Vec::new();
        for _ in 0..64 {
            spectrum = analyser.read(input);
        }
        spectrum
    }

    #[test]
    fn fft_size_is_a_power_of_two_in_range() {
        assert_eq!(normalize_fft_size(2048), 2048);
        assert_eq!(normalize_fft_size(1000), 1024);
        assert_eq!(normalize_fft_size(4), 32);
        assert_eq!(normalize_fft_size(1 << 20), 32768);
    }

    #[test]
    fn frequency_bin_count_is_half_the_fft_size() {
        let analyser = Analyser::new(2048);
        assert_eq!(analyser.frequency_bin_count(), 1024);
        assert_eq!(analyser.fft_size(), 2048);
    }

    #[test]
    fn a_tone_peaks_in_its_own_bin() {
        // 1kHz over a 2048-point transform of a 48kHz signal lands 42.7 bins up,
        // so the peak has to be the bin that 1kHz rounds to.
        let frequency = 1000.0;
        let spectrum = settled(Analyser::new(DEFAULT_FFT_SIZE), &sine(frequency));
        let expected = (frequency * DEFAULT_FFT_SIZE as f32 / SAMPLE_RATE).round() as usize;
        let peak = spectrum
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(index, _)| index)
            .expect("the spectrum is never empty");
        assert!(
            peak.abs_diff(expected) <= 1,
            "the peak landed in bin {peak}, expected {expected}"
        );
    }

    #[test]
    fn a_tone_is_loud_and_silence_is_quiet() {
        let loud = settled(Analyser::new(DEFAULT_FFT_SIZE), &sine(1000.0));
        let quiet = settled(
            Analyser::new(DEFAULT_FFT_SIZE),
            &vec![0.0; DEFAULT_FFT_SIZE],
        );
        let loudest = loud.iter().cloned().fold(f32::MIN, f32::max);
        let quietest = quiet.iter().cloned().fold(f32::MAX, f32::min);
        // A full-scale tone is well above the visualizer's -70dB floor and
        // silence is well below it, which is what makes the two branches
        // distinguishable at all.
        assert!(loudest > -20.0, "a full-scale tone measured {loudest}dB");
        assert!(quietest < -70.0, "silence measured {quietest}dB");
    }

    #[test]
    fn a_short_read_is_zero_padded() {
        // Fewer samples than the transform is what happens for the first frames
        // after a track starts; the missing ones are silence, not garbage.
        let short = vec![0.5; 128];
        let spectrum = settled(Analyser::new(DEFAULT_FFT_SIZE), &short);
        assert_eq!(spectrum.len(), DEFAULT_FFT_SIZE / 2);
        assert!(spectrum.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn a_longer_read_keeps_only_its_newest_samples() {
        let mut input = sine(1000.0);
        input.extend(std::iter::repeat_n(0.0, DEFAULT_FFT_SIZE));
        let spectrum = settled(Analyser::new(DEFAULT_FFT_SIZE), &input);
        let loudest = spectrum.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            loudest < -60.0,
            "the silence at the end measured {loudest}dB"
        );
    }
}
