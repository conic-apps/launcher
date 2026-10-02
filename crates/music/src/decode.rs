// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Decoding a track.
//!
//! [`TrackSource`] streams: opening it reads the container header and nothing
//! else, so the duration is known and playback can start immediately, and the
//! audio itself is pulled in only as the device consumes it
//! ([`TrackSource::decode_into`]). Seeking is a real seek on the container: the
//! decoder is reset and the reported position moves to the target.
//!
//! The streaming shape matters because the alternative — reading the whole file
//! into a `Vec` of frames before a single sample reaches the device — makes
//! every track change cost a full-file read. With a library of lossless
//! 16/44.1 kHz WAVs that is a hundred megabytes per switch, and a switch takes
//! seconds.
//!
//! One [`TrackSource`] belongs to one thread: it owns the reader and the
//! decoder, so it lives on the player's worker (see `player.rs`) and never on the
//! audio callback.

use std::path::{Path, PathBuf};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, CodecParameters, Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use crate::error::{Error, Result};

/// How many samples [`TrackSource::decode_into`] aims to append per call.
///
/// A call keeps decoding whole packets until it has appended at least this many
/// samples, so it returns a little more than this, or however much is left at
/// the end of the track.
const READ_SIZE: usize = 2048;

/// A track that is opened but not yet decoded: the reader, the decoder, and
/// enough of the header to answer "how long is this" and "where am I".
pub struct TrackSource {
    /// The file, kept only so a failure can name it.
    path: PathBuf,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    /// Reused across reads; re-made only when a packet needs a bigger one.
    buffer: Option<SampleBuffer<i16>>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    /// The track's own length in seconds, from the container. `0.0` when the
    /// container does not say, which the player reads as "unknown" rather than
    /// "empty".
    duration: f64,
    /// How far into the track the reader is, in seconds.
    position: f64,
    /// The reader has no more audio.
    finished: bool,
}

impl TrackSource {
    /// Opens `path` — its container header, its decoder, nothing more.
    ///
    /// This is what a track change waits on, so it deliberately stops short of
    /// decoding: on a hundred-megabyte file that is the difference between a few
    /// milliseconds and a few seconds.
    ///
    /// The extension is only a probe hint, so a file whose name lies about its
    /// contents is still read if its magic says otherwise; what the decoders do
    /// not cover at all comes back as [`Error::Decode`].
    pub fn open(path: &Path) -> Result<TrackSource> {
        let file = std::fs::File::open(path)?;
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|extension| extension.to_str()) {
            hint.with_extension(extension);
        }
        let source = MediaSourceStream::new(Box::new(file), Default::default());
        let format = symphonia::default::get_probe()
            .format(
                &hint,
                source,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|error| Error::Decode {
                path: path.to_string_lossy().to_string(),
                message: error.to_string(),
            })?
            .format;

        // The first track with a codec is the one played; the rest are artwork,
        // lyrics or an alternate language, and the player has no way to choose
        // between them.
        let (track_id, params) = {
            let track = format
                .tracks()
                .iter()
                .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
                .ok_or_else(|| Error::Decode {
                    path: path.to_string_lossy().to_string(),
                    message: "the file holds no decodable audio track".to_string(),
                })?;
            (track.id, track.codec_params.clone())
        };

        let decoder = symphonia::default::get_codecs()
            .make(&params, &DecoderOptions::default())
            .map_err(|error| Error::Decode {
                path: path.to_string_lossy().to_string(),
                message: error.to_string(),
            })?;

        // What the decoder was made from is what to believe to begin with. The
        // channel count has to come from here: `decode_into` only pulls packets
        // once `channels` is non-zero, so a container that does not say leaves
        // the source unable to start.
        let sample_rate = params.sample_rate.unwrap_or(0);
        let channels = params
            .channels
            .map_or(0, |channels| channels.count() as u16);

        Ok(TrackSource {
            path: path.to_path_buf(),
            track_id,
            format,
            decoder,
            buffer: None,
            sample_rate,
            channels,
            duration: duration_of(&params, sample_rate),
            position: 0.0,
            finished: false,
        })
    }

    /// How long the track is, in seconds, from the container rather than from
    /// having decoded all of it. `0.0` when the container does not say.
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// The track's own sample rate, in Hz — what the player resamples from.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// How many channels the track has, from the container's codec parameters.
    /// `0` when the container did not say, which the player reads as "cannot
    /// start".
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// How far into the track the reader is, in seconds. This runs *ahead* of
    /// what the device has been fed, by however much of the track is buffered.
    pub fn position(&self) -> f64 {
        self.position
    }

    /// Whether the reader has run out of audio.
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// Decodes the next stretch of the track into `out`, appending interleaved
    /// samples in `-32768..=32767` and returning how many it wrote.
    ///
    /// `0` means the end of the track, but it may also mean a run of undecodable
    /// packets was skipped — one bad frame in an MP3 is not a reason to refuse
    /// the track. Callers should therefore keep reading until the source reports
    /// itself [`finished`](Self::finished) rather than stopping at the first `0`.
    ///
    /// `out` is the caller's so that a read allocates nothing per call. It is
    /// trimmed to whole frames on the way in and on the way out, so its contents
    /// can be indexed by frame without a per-frame bounds check.
    pub fn decode_into(&mut self, out: &mut Vec<i16>) -> Result<usize> {
        let channels = usize::from(self.channels);
        out.truncate(out.len() - out.len() % channels.max(1));
        let written = out.len();

        while self.channels > 0 && out.len() - written < READ_SIZE {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                // The end of the stream — which is also how a file whose header
                // promises more data than it holds arrives — and a chained Ogg
                // physical stream asking for a decoder set this source cannot
                // rebuild. Both end the track with everything read so far intact.
                Err(SymphoniaError::ResetRequired) => {
                    self.finished = true;
                    break;
                }
                Err(SymphoniaError::IoError(error))
                    if error.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    self.finished = true;
                    break;
                }
                Err(error) => {
                    self.finished = true;
                    return Err(Error::Decode {
                        path: self.path.to_string_lossy().to_string(),
                        message: error.to_string(),
                    });
                }
            };
            if packet.track_id() != self.track_id {
                continue;
            }

            // The decoder's own parameters are what its packets will agree with,
            // and the packet that has not been decoded yet cannot say anything
            // about the track. Reading them first also keeps the decoder from
            // being borrowed twice at once below.
            let first = self.channels == 0;
            let params = first.then(|| self.decoder.codec_params().clone());

            let Ok(decoded) = self.decoder.decode(&packet) else {
                continue;
            };
            let spec = *decoded.spec();
            let frames = decoded.frames();
            if let Some(params) = params {
                self.channels = spec.channels.count() as u16;
                self.sample_rate = spec.rate;
                self.duration = duration_of(&params, spec.rate);
            }
            if self.channels == 0 || self.sample_rate == 0 || frames == 0 {
                continue;
            }
            if self
                .buffer
                .as_ref()
                .is_none_or(|buffer| buffer.capacity() < frames)
            {
                self.buffer = Some(SampleBuffer::new(frames as u64, spec));
            }
            let buffer = self
                .buffer
                .as_mut()
                .expect("the sample buffer was just made");
            buffer.copy_interleaved_ref(decoded);
            out.extend_from_slice(buffer.samples());
            // The reader is now this far in, which is what the player's progress
            // bar reads; it leads the playhead by whatever is still buffered.
            self.position += frames as f64 / f64::from(self.sample_rate);
        }

        // A truncated stream still plays: drop whatever does not make a whole
        // frame so the player can index the result without a per-frame bounds
        // check.
        let channels = channels.max(1);
        out.truncate(out.len() - out.len() % channels);
        Ok(out.len() - written)
    }

    /// Moves the reader to `seconds`, clamped to the track.
    ///
    /// The container is asked for an accurate seek — which on a compressed track
    /// can mean decoding from a frame just before the target — and the decoder is
    /// reset, because a seek makes the next packet discontinuous with the last.
    /// That re-decode is why this runs on the worker rather than on the thread
    /// that asked for it.
    pub fn seek(&mut self, seconds: f64) -> Result<()> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Ok(());
        }
        let seconds = if self.duration > 0.0 {
            seconds.min(self.duration)
        } else {
            seconds
        };

        // A reader that cannot seek is one that is already at the end; the seek
        // itself is what says so, so it is allowed to fail.
        self.format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    // `Time` is whole seconds plus a fraction, not a float.
                    time: Time::new(seconds as u64, seconds - seconds.trunc()),
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|message| Error::Decode {
                path: self.path.to_string_lossy().to_string(),
                message: message.to_string(),
            })?;

        // A decoder must be reset when the next packet is discontinuous with the
        // last one, which a seek guarantees.
        self.decoder.reset();
        self.position = seconds;
        self.finished = false;
        Ok(())
    }
}

/// The track's length in seconds, from the container's own count of frames.
///
/// A file that declares no length reports `0.0`, which the player shows as an
/// unknown duration rather than an empty track.
fn duration_of(params: &CodecParameters, sample_rate: u32) -> f64 {
    if sample_rate == 0 {
        return 0.0;
    }
    match params.n_frames {
        Some(frames) => frames as f64 / f64::from(sample_rate),
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 44_100;
    const CHANNELS: u16 = 2;

    /// A directory that cleans itself up, so a failing test does not leave its
    /// audio behind in the temp dir.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let path = std::env::temp_dir().join(format!("conic-music-decode-{name}"));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("the test could not make its temp dir");
            TempDir(path)
        }

        /// Writes a `seconds`-long 16-bit stereo sine WAV and returns its path.
        ///
        /// The tests generate their own audio rather than carrying a fixture:
        /// what is being checked is that the frames read match the frames the
        /// file declares, and a checked-in binary could only be trusted to still
        /// say so.
        fn sine_wav(&self, name: &str, seconds: f64) -> PathBuf {
            let frames = (f64::from(RATE) * seconds) as u32;
            let data_len = frames * u32::from(CHANNELS) * 2;

            let mut bytes = Vec::with_capacity(44 + data_len as usize);
            bytes.extend_from_slice(b"RIFF");
            bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
            bytes.extend_from_slice(b"WAVEfmt ");
            bytes.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
            bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
            bytes.extend_from_slice(&CHANNELS.to_le_bytes());
            bytes.extend_from_slice(&RATE.to_le_bytes());
            // byte rate
            bytes.extend_from_slice(&(RATE * u32::from(CHANNELS) * 2).to_le_bytes());
            bytes.extend_from_slice(&4u16.to_le_bytes()); // block align
            bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
            bytes.extend_from_slice(b"data");
            bytes.extend_from_slice(&data_len.to_le_bytes());
            for frame in 0..frames {
                let sample = ((f64::from(frame) / f64::from(RATE) * 440.0)
                    .to_radians()
                    .sin()
                    * 8_000.0) as i16;
                bytes.extend_from_slice(&sample.to_le_bytes());
                bytes.extend_from_slice(&sample.to_le_bytes());
            }

            let path = self.0.join(name);
            std::fs::write(&path, bytes).expect("the test could not write its WAV");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Reads a whole track, in whatever stretches `decode_into` yields, and
    /// returns the samples.
    fn drain(source: &mut TrackSource) -> Vec<i16> {
        let mut out = Vec::new();
        while !source.finished() {
            let before = out.len();
            source
                .decode_into(&mut out)
                .expect("the generated WAV always decodes");
            assert!(
                out.len() > before || source.finished(),
                "a read that was neither progress nor the end"
            );
        }
        out
    }

    #[test]
    fn opening_reads_the_duration_without_decoding() {
        let dir = TempDir::new("duration");
        let path = dir.sine_wav("one-second.wav", 1.0);

        let mut source = TrackSource::open(&path).expect("the generated WAV opens");
        // The whole point of the streaming shape: the length is answerable
        // before a single sample has been read, which is what lets a track change
        // finish in milliseconds rather than after a full-file decode.
        assert!(
            (source.duration() - 1.0).abs() < 0.001,
            "the duration was {}",
            source.duration()
        );
        assert_eq!(source.sample_rate(), RATE);
        assert_eq!(source.channels(), CHANNELS);
        assert!(!source.finished());

        // And a read is a *batch*, not the rest of the file: the player takes
        // what it can use and comes back, which is the other half of not having
        // to decode a hundred megabytes before the first sound.
        let mut out = Vec::new();
        let first = source.decode_into(&mut out).expect("a first read");
        assert!(first >= READ_SIZE, "a read returned only {first} samples");
        assert!(
            out.len() < 8 * READ_SIZE,
            "a read returned {} samples, which is not a batch",
            out.len()
        );

        out.clear();
        drain(&mut source);
        assert!(
            (source.position() - 1.0).abs() < 0.001,
            "the reader stopped at {}",
            source.position()
        );
    }

    #[test]
    fn a_track_decodes_to_exactly_as_many_frames_as_it_declares() {
        let dir = TempDir::new("frames");
        let path = dir.sine_wav("half-second.wav", 0.5);
        let mut source = TrackSource::open(&path).expect("the generated WAV opens");

        let samples = drain(&mut source);
        assert_eq!(
            samples.len() / usize::from(CHANNELS),
            RATE as usize / 2,
            "a half-second track is half a second of frames"
        );
        assert!(source.finished());
    }

    #[test]
    fn reading_in_stretches_or_all_at_once_gives_the_same_samples() {
        // The player pulls from this in whatever sizes its ring buffer asks for,
        // so a decode split across calls has to be indistinguishable from one
        // that was not — that equivalence is what keeps the audio seamless.
        let dir = TempDir::new("resume");
        let path = dir.sine_wav("resume.wav", 0.2);

        let mut whole = TrackSource::open(&path).expect("the generated WAV opens");
        let expected = drain(&mut whole);

        let mut piecewise = TrackSource::open(&path).expect("the generated WAV opens");
        let mut actual = Vec::new();
        for _ in 0..8 {
            let before = actual.len();
            piecewise.decode_into(&mut actual).expect("a partial read");
            if actual.len() == before {
                break;
            }
        }
        // Whatever the eight reads did not cover, the rest read on.
        while !piecewise.finished() {
            let before = actual.len();
            piecewise
                .decode_into(&mut actual)
                .expect("the last partial read");
            if actual.len() == before {
                break;
            }
        }

        assert_eq!(actual, expected, "a split decode changed the samples");
    }

    #[test]
    fn a_partial_read_leaves_the_output_on_a_whole_frame() {
        // `decode_into` appends to whatever the caller already holds, and the
        // player indexes the result by frame, so a read that stopped on a
        // half-frame would shift every channel after it.
        let dir = TempDir::new("aligned");
        let path = dir.sine_wav("aligned.wav", 0.2);
        let mut source = TrackSource::open(&path).expect("the generated WAV opens");

        let mut out = vec![1i16, 2, 3, 4, 5];
        source.decode_into(&mut out).expect("a first read");
        assert_eq!(
            out.len() % usize::from(CHANNELS),
            0,
            "the read left a half frame behind"
        );
    }

    #[test]
    fn a_seek_lands_near_the_target_and_the_track_keeps_playing() {
        let dir = TempDir::new("seek");
        let path = dir.sine_wav("seek.wav", 2.0);
        let mut source = TrackSource::open(&path).expect("the generated WAV opens");

        source.seek(1.5).expect("a lossless file seeks");
        assert!(
            (source.position() - 1.5).abs() < 0.05,
            "the seek landed at {} instead of 1.5",
            source.position()
        );
        assert!(!source.finished(), "a seek into the middle is not the end");

        // And what comes back is the tail of the file, not silence: a reader
        // that lost its samples, or resumed at the wrong place, would read flat
        // or wrong. The file's own peak is 8000, and half a second of a 440 Hz
        // tone covers 220 cycles, so the read has to reach it.
        let mut out = Vec::new();
        let mut peak = 0i32;
        for _ in 0..40 {
            out.clear();
            if source.decode_into(&mut out).expect("a read after seeking") == 0 {
                break;
            }
            for sample in &out {
                peak = peak.max(i32::from(sample.abs()));
            }
        }
        assert!(
            peak > 7_900,
            "the audio after a seek did not reach the file's own peak (got {peak})"
        );
    }

    #[test]
    fn a_seek_past_the_end_clamps_instead_of_failing() {
        let dir = TempDir::new("clamp");
        let path = dir.sine_wav("clamp.wav", 1.0);
        let mut source = TrackSource::open(&path).expect("the generated WAV opens");

        source
            .seek(999.0)
            .expect("an out-of-range seek is not an error");
        assert!(source.position() <= 1.0, "the position ran past the track");
    }

    #[test]
    fn a_file_that_is_not_audio_is_reported_rather_than_panicking() {
        let dir = TempDir::new("garbage");
        let path = dir.0.join("not-audio.mp3");
        std::fs::write(&path, b"this is not a music file").expect("the test could not write");
        match TrackSource::open(&path) {
            Ok(_) => panic!("garbage opened"),
            Err(error) => {
                assert!(matches!(error, Error::Decode { .. }), "got {error:?}");
                assert!(
                    error.to_string().contains("not-audio.mp3"),
                    "the error did not name the file: {error}"
                );
            }
        }
    }
}
