// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Decoding a track into PCM — what the webview did by pointing an
//! `<audio>` element's `src` at the file.
//!
//! The element decoded lazily and streamed, which is the right shape for a
//! long file the user may seek around in. It is the wrong one here for two
//! reasons: the analyser needs the signal the same way the element fed it
//! (`src → analyser → gain → destination`, see `analyser.rs`), and the position
//! has to be readable and seekable at any moment, since the store persists it
//! every few seconds and restores it at startup. So a track is decoded once, in
//! full, on the worker's thread; a `Vec` of frames then answers "where are we"
//! and "what is next" by arithmetic.
//!
//! The samples are kept as `i16`. Every format the player lists is 16-bit in
//! practice, and it halves what a five-minute stereo track costs to hold
//! (≈27 MB at 48 kHz against ≈55 MB of `f32`).

use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{Error, Result};

/// A decoded track: interleaved samples plus the rate they were decoded at.
///
/// The rate is the *track's*, not the output device's. The player resamples on
/// the way out, so the device is free to run at whatever it likes (see
/// `player.rs`).
pub struct DecodedTrack {
    /// Interleaved samples, `channels` of them per frame, in the range
    /// `-32768..=32767`.
    samples: Vec<i16>,
    sample_rate: u32,
    channels: u16,
}

impl DecodedTrack {
    /// Length in seconds, the `audio.duration` the store reads for the panel's
    /// right-hand time.
    pub fn duration(&self) -> f64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0.0;
        }
        self.frames() as f64 / self.channels as f64 / self.sample_rate as f64
    }

    /// Number of sample frames in the track.
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }

    /// The track's own sample rate, in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// How many channels the track has.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Reads `count` interleaved samples from `frame`, linearly interpolated
    /// when it lands between two frames — which it always does once the track
    /// and the device disagree on their rate.
    ///
    /// Returns `None` at (or past) the end of the track.
    pub fn read(&self, frame: f64, count: usize, out: &mut [f32]) -> Option<usize> {
        if self.channels == 0 || self.sample_rate == 0 {
            return None;
        }
        let channels = self.channels as usize;
        let index = frame.floor();
        if index < 0.0 || index >= self.frames() as f64 {
            return None;
        }
        let index = index as usize;
        let fraction = (frame - index as f64) as f32;
        // The last frame has no successor to interpolate towards; the end of the
        // track is the only place the cursor can land on a whole frame.
        let has_next = index + 1 < self.frames();
        let written = count.min(out.len());
        for frame_index in 0..written {
            for channel in 0..channels {
                let base = (index + frame_index) * channels + channel;
                let mut value = self.samples[base] as f32 / 32768.0;
                if has_next {
                    value += (self.samples[base + channels] as f32 / 32768.0 - value) * fraction;
                }
                out[frame_index * channels + channel] = value;
            }
        }
        Some(written)
    }
}

/// Decodes `path` in full.
///
/// The extension is only a probe hint, so a file whose name lies about its
/// contents is still read if its magic says otherwise; what the decoders do not
/// cover at all comes back as [`Error::Decode`].
pub fn decode(path: &Path) -> Result<DecodedTrack> {
    let decode_error = |message: String| Error::Decode {
        path: path.to_string_lossy().to_string(),
        message,
    };

    let file = std::fs::File::open(path)?;
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
        hint.with_extension(extension);
    }
    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|error| decode_error(error.to_string()))?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| decode_error("the file holds no decodable audio track".to_string()))?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|error| decode_error(error.to_string()))?;

    let mut samples: Vec<i16> = Vec::new();
    let mut sample_rate = 0;
    let mut channels = 0u16;
    let mut buffer: Option<SampleBuffer<i16>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            // The end of the stream — which is also how a file whose header
            // promises more data than it holds arrives — and a chained Ogg
            // physical stream asking for a decoder set the loop cannot rebuild.
            // Both end the decode with everything read so far intact.
            Err(SymphoniaError::ResetRequired) => break,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::IoError(error)) => {
                return Err(decode_error(error.to_string()));
            }
            Err(error) => return Err(decode_error(error.to_string())),
        };
        if packet.track_id() != track_id {
            continue;
        }

        // A packet the codec could not read is skipped rather than fatal, the
        // same way the element skipped it: one bad frame in an MP3 is not a
        // reason to refuse the track.
        let Ok(decoded) = decoder.decode(&packet) else {
            continue;
        };
        let spec = *decoded.spec();
        if channels == 0 {
            channels = spec.channels.count() as u16;
            sample_rate = spec.rate;
        }
        if channels == 0 || sample_rate == 0 {
            continue;
        }
        let frames = decoded.frames();
        if frames == 0 {
            continue;
        }
        if buffer
            .as_ref()
            .is_none_or(|buffer| buffer.capacity() < frames)
        {
            buffer = Some(SampleBuffer::new(frames as u64, spec));
        }
        let buffer = buffer.as_mut().expect("the sample buffer was just created");
        buffer.copy_interleaved_ref(decoded);
        samples.extend_from_slice(buffer.samples());
    }

    if channels == 0 || sample_rate == 0 {
        return Err(decode_error(
            "the file decoded to an empty signal".to_string(),
        ));
    }
    // A truncated stream still plays: drop whatever does not make a whole frame
    // so the reader can index it without a per-frame bounds check.
    samples.truncate(samples.len() - samples.len() % channels as usize);

    Ok(DecodedTrack {
        samples,
        sample_rate,
        channels,
    })
}
