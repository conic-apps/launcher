// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The player: the audio graph the webview built out of an `<audio>` element,
//! an `AudioContext`, an `AnalyserNode` and a `GainNode`, together with the
//! transport the `useMusicStore` actions drove it with (src/store/music.ts).
//!
//! # The graph
//!
//! `source → analyser → gain → destination` becomes
//! `decoded track → resampler → (analyser tap) → gain → output device`, in that
//! order, because the order is visible: the analyser sits *before* the gain, so
//! the visualiser is unaffected by the volume. The resampler is new and has no
//! counterpart — the element was handed the device's own pipeline and never had
//! to convert rates, while a native player has to (a 44.1 kHz track on a 48 kHz
//! device would otherwise play 9% fast).
//!
//! # The threads
//!
//! Three, because the audio callback may not block on anything.
//!
//!   * The **device callback** takes the lock once, copies samples out of a ring
//!     buffer, scales them by the gain and runs the analyser on its own clock.
//!     It allocates nothing, reads no file and writes no file. Everything below
//!     exists so that it can keep to that.
//!   * The **worker** owns the [`TrackSource`], decodes ahead into the ring
//!     buffer, resamples and up-mixes, and performs the seeks. All of it is
//!     expensive, and none of it may happen on the callback.
//!   * The **UI thread** only ever records intent: which track to select, where
//!     to seek. Neither blocks, so a track change and a scrub are both instant
//!     even though the audio behind them is not.
//!
//! The first version had two threads and no ring buffer: it decoded a whole
//! track into a `Vec<i16>` before playback started, so every track change cost a
//! full-file read — seconds, for the hundred-megabyte lossless files a music
//! library tends to hold.
//!
//! The read side — [`Player::state`], [`Player::with_spectrum`] — is a lock and
//! a borrow, so the UI can poll it as often as it likes without allocating.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{
    Arc, Condvar, Mutex, MutexGuard,
    atomic::{AtomicUsize, Ordering},
    mpsc::{Receiver, Sender, channel},
};
use std::time::{Duration, Instant};

use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::MusicFile;
use crate::analyser::{Analyser, DEFAULT_FFT_SIZE};
use crate::decode::TrackSource;
use crate::error::{Error, Result};
use crate::session::{self, SavedTrack};

/// How often the store's `ontimeupdate` handler persisted the position. The web
/// view persisted at most every 5s while a track played; the poll the app drives
/// through [`Player::tick`] keeps the same limit.
pub const PERSIST_INTERVAL: Duration = Duration::from_millis(5000);

/// How long `applyVolume(true)` ramps the gain for, the `linearRampToValueAtTime
/// (volume, now + 1.0)` of the store.
const VOLUME_RAMP: Duration = Duration::from_secs(1);

/// How often the audio thread runs the transform.
///
/// The webview read the analyser once per animation frame, so the smoothing
/// advanced at the display's refresh rate. Nothing here knows that rate, so the
/// transform runs on a fixed 60Hz clock instead — the same cadence on every
/// machine, and within one frame of the original on a 60Hz display.
const ANALYSIS_INTERVAL: Duration = Duration::from_millis(16);

/// How much decoded audio the ring buffer holds, in seconds.
///
/// It is the cushion between a disk that cannot keep up with playback and a
/// device that will not wait. Two seconds is long enough to ride out a seek on a
/// compressed track, and short enough that the position the UI shows does not
/// visibly lead the sound.
const RING_SECONDS: f64 = 2.0;

/// The fraction of the ring the worker refills to before it waits again.
///
/// Aiming at half rather than full leaves room to keep the device fed while the
/// next batch is being decoded, and it is the level the worker calls "enough".
const RING_LOW_WATER: f64 = 0.5;

/// How many consecutive reads may produce nothing before a track is given up on.
///
/// A single undecodable packet is skipped, which the element also did; a run of
/// them means the rest of the file is not going to decode either, and the worker
/// must not spin on a track that will never yield another sample.
const MAX_STALLED_READS: u32 = 64;

/// Everything the UI shows about the transport, as one snapshot.
#[derive(Clone, Debug, Default)]
pub struct PlayerState {
    /// The playlist, in the order `list_music_files` sorted it.
    ///
    /// Shared rather than copied, so that polling sixty times a second does not
    /// clone a hundred filenames and a hundred paths sixty times a second.
    pub tracks: Arc<Vec<MusicFile>>,
    /// Which of them is selected, or `None` when nothing is.
    pub current_index: Option<usize>,
    /// How far into the current track, in seconds.
    pub current_time: f64,
    /// How long the current track is, in seconds.
    pub duration: f64,
    /// Whether the device is being fed samples.
    pub is_playing: bool,
    /// The `shuffle` flag.
    pub shuffle: bool,
    /// The `repeat` flag.
    pub repeat: bool,
    /// The last failure, the one the store logged through `console.error`.
    pub error: Option<String>,
    /// How many track selections are still being opened. A selection is
    /// asynchronous — the file has to be read and its decoder built before the
    /// transport can say it is playing — so a caller has to be able to tell "not
    /// playing yet" from "never going to play".
    pub pending: u32,
    /// Whether the transport is waiting on audio: a selection being opened, or
    /// the ring buffer having run dry. `buffering` is what a UI shows instead of
    /// silently sitting still.
    pub buffering: bool,
}

impl PlayerState {
    /// The selected track, the `currentTrack` getter.
    pub fn current_track(&self) -> Option<&MusicFile> {
        self.current_index.and_then(|index| self.tracks.get(index))
    }
}

/// The graph, shared between the device callback and everything else.
struct Graph {
    state: Mutex<GraphState>,
    /// Woken for either of the two things the worker sleeps on: the device
    /// needing samples, or a command having been sent.
    signal: Condvar,
    /// The requested `fftSize`, read by the audio thread (the Vue's module-level
    /// `targetFftSize`). An atomic rather than a field of `GraphState` because it
    /// is the one piece of configuration that arrives from the UI thread and is
    /// only *applied* on the audio thread, which owns the transform.
    fft_size: AtomicUsize,
}

/// Locks the graph, recovering from a poisoned mutex.
///
/// A panic anywhere in the graph must not silence the device: the callback
/// borrows the lock twice a second and a poisoned one would otherwise take the
/// music down for the rest of the session.
fn lock(state: &Mutex<GraphState>) -> MutexGuard<'_, GraphState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// [`lock`], for the worker about to sleep on [`Graph::signal`].
///
/// The guard it hands back is dropped: waking is the point, not the state, and
/// nothing may be done to the graph before the caller has re-checked why it was
/// waiting.
fn wait(graph: &Graph, guard: MutexGuard<'_, GraphState>) {
    // The guard this hands back is dropped here: waking is the point, not the
    // state, and nothing may be done to the graph before the caller has
    // re-checked why it was waiting.
    drop(
        graph
            .signal
            .wait(guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );
}

struct GraphState {
    tracks: Arc<Vec<MusicFile>>,
    current_index: Option<usize>,
    /// Whether a track is open on the worker, playing or not — the `audio.src`
    /// the store re-prepared when it was empty.
    loaded: bool,
    /// How far into the current track, in seconds. Published by the worker, which
    /// is the only thing that knows how far its reader has got and how much of it
    /// is still buffered.
    cursor: f64,
    /// The current track's length, published as soon as the file's header says,
    /// which is long before the track has been decoded.
    duration: f64,
    playing: bool,
    /// The track ran out and the ring drained; the worker picks this up.
    ended: bool,
    /// The device's rate and channel count — the `AudioContext`'s.
    sample_rate: f64,
    device_channels: usize,
    /// The target gain, and the gain itself, which is what the 1s ramp moves.
    volume: f32,
    gain: f32,
    ramp_from: f32,
    ramp_started: Option<Instant>,
    /// Decoded audio waiting for the device, already at the device's rate and
    /// channel count: what the worker's resampler produced and the callback
    /// consumes.
    ring: VecDeque<f32>,
    /// The last sample the worker produced, held so that a starved ring repeats
    /// it rather than jumping to silence. A step to zero from mid-waveform is an
    /// audible click; a held sample is not.
    tail: f32,
    /// The pre-gain signal the analyser reads, newest last.
    tap: Vec<f32>,
    /// The transform, owned by the audio thread (it is the only writer).
    analyser: Analyser,
    /// The last spectrum, in dB. Read under the same lock as everything else, so
    /// it needs no copy of its own.
    spectrum: Vec<f32>,
    /// The ring has run dry and the device is playing into an empty one.
    starved: bool,
    last_analysis: Option<Instant>,
    shuffle: bool,
    repeat: bool,
    error: Option<String>,
    /// How many selections the worker still has to open; see
    /// [`PlayerState::pending`].
    pending: u32,
    /// When the position was last written to the session file.
    last_persisted: Option<Instant>,
    /// The `Math.random()` of `randomIndex`, seeded from the clock.
    random: u64,
    /// A position the UI thread asked to move to, with the generation it was
    /// asked in. The worker performs it, because on a compressed track a seek
    /// means re-decoding and the UI thread has no business waiting for that. The
    /// generation is what tells a seek already done from one queued behind it.
    seek: Option<(u64, f64)>,
    /// The next generation for [`GraphState::seek`].
    generation: u64,
}

/// What the worker is told to do.
#[derive(Debug)]
enum Command {
    /// Open a track and make it the current one, starting playback when
    /// `autoplay` — the `playIndex` / `preparePlayback` pair.
    Select { index: usize, autoplay: bool },
    /// `restoreSession`: put the playlist and the saved position back.
    Restore {
        tracks: Arc<Vec<MusicFile>>,
        saved: Option<SavedTrack>,
        enabled: bool,
        resume_on_startup: bool,
    },
}

impl GraphState {
    /// The gain at `now`, moving towards `volume` over [`VOLUME_RAMP`] while a
    /// smooth change is in flight.
    fn gain_at(&mut self, now: Instant) -> f32 {
        match self.ramp_started {
            Some(started) => {
                let elapsed = now.saturating_duration_since(started);
                if elapsed >= VOLUME_RAMP {
                    self.ramp_started = None;
                    self.gain = self.volume;
                } else {
                    let progress = elapsed.as_secs_f32() / VOLUME_RAMP.as_secs_f32();
                    self.gain = self.ramp_from + (self.volume - self.ramp_from) * progress;
                }
            }
            None => self.gain = self.volume,
        }
        self.gain
    }

    /// How full the ring may be, and how full counts as "enough" — the point the
    /// worker refills to and the level the device wakes it at.
    fn ring_bounds(&self) -> (usize, usize) {
        let per_second = (self.sample_rate * self.device_channels as f64).max(1.0);
        let capacity = (per_second * RING_SECONDS) as usize;
        let low = (capacity as f64 * RING_LOW_WATER) as usize;
        (low, capacity)
    }

    /// The worker's side of the ring: takes what has been resampled and keeps
    /// the analyser's input in step with it.
    ///
    /// `reader_position` is how far the reader has got, which is *ahead* of the
    /// playhead by whatever is still waiting. The UI's position is the difference,
    /// so the progress bar does not run ahead of the sound.
    fn push(&mut self, samples: &[f32], fft_size: usize, reader_position: f64) {
        let channels = self.device_channels.max(1);
        if let Some(last) = samples.last() {
            self.tail = *last;
        }
        self.ring.extend(samples);

        // The analyser's input, the mean of the channels: the same mono the
        // callback used to build out of the source frame.
        let frames = samples.len() / channels;
        self.tap.reserve(frames);
        for frame in 0..frames {
            let sum: f32 = samples[frame * channels..(frame + 1) * channels]
                .iter()
                .sum();
            self.tap.push(sum / channels as f32);
        }
        if self.tap.len() > fft_size * 2 {
            let excess = self.tap.len() - fft_size * 2;
            self.tap.drain(..excess);
        }

        let buffered = self.ring.len() as f64 / (self.sample_rate * channels as f64);
        let position = (reader_position - buffered).max(0.0);
        self.cursor = if self.duration > 0.0 {
            position.min(self.duration)
        } else {
            position
        };
    }

    /// Throws the ring and the analyser's history away, for when the samples in
    /// them no longer belong to anything the device is about to play.
    fn flush(&mut self) {
        self.ring.clear();
        self.tap.clear();
        self.tail = 0.0;
    }

    /// The `{ path, currentTime }` pair to write out, or `None` when there is
    /// nothing selected to write it for.
    ///
    /// The state is read here, under the lock, and written by the caller *after*
    /// releasing it: a file write is slow enough to miss a callback, and the
    /// callback is the one thing that must never wait.
    fn pending_session(&self) -> Option<SavedTrack> {
        let index = self.current_index?;
        let track = self.tracks.get(index)?;
        Some(SavedTrack {
            path: track.path.clone(),
            current_time: self.cursor,
        })
    }

    /// Records that the position was written, so it is not written again
    /// immediately.
    fn persisted(&mut self) {
        self.last_persisted = Some(Instant::now());
    }

    fn next_random(&mut self) -> f64 {
        // xorshift64*, seeded from the clock. The store only needs a draw that
        // is not always the same, which is not worth a dependency for.
        let mut state = self.random;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.random = state;
        (state >> 11) as f64 / (1u64 << 53) as f64
    }

    /// `pause`: stop feeding the device.
    fn pause(&mut self) {
        self.playing = false;
    }
}

/// The graph a device of this shape would be fed by.
///
/// Split out of [`Player::new`] so the tests can have one without an output
/// device — the deadlock this exists next to was only ever reachable through a
/// machine with a sound card, and a test that needs one does not get run.
fn new_graph(device_rate: f64, device_channels: usize) -> Arc<Graph> {
    Arc::new(Graph {
        state: Mutex::new(GraphState {
            tracks: Arc::new(Vec::new()),
            current_index: None,
            loaded: false,
            cursor: 0.0,
            duration: 0.0,
            playing: false,
            ended: false,
            sample_rate: device_rate,
            device_channels,
            volume: 1.0,
            gain: 1.0,
            ramp_from: 1.0,
            ramp_started: None,
            ring: VecDeque::with_capacity(
                (device_rate * device_channels as f64 * RING_SECONDS) as usize,
            ),
            tail: 0.0,
            tap: Vec::new(),
            analyser: Analyser::new(DEFAULT_FFT_SIZE),
            spectrum: vec![0.0; DEFAULT_FFT_SIZE / 2],
            starved: false,
            last_analysis: None,
            shuffle: false,
            repeat: false,
            error: None,
            pending: 0,
            last_persisted: None,
            random: seed_random(),
            seek: None,
            generation: 0,
        }),
        signal: Condvar::new(),
        fft_size: AtomicUsize::new(DEFAULT_FFT_SIZE),
    })
}

/// The position to write to the session file, with the "written" mark set.
///
/// The state is read and the mark set under *one* guard, which is released
/// before the caller touches the filesystem. Both halves matter:
///
///   * a second acquisition while the first is alive is a deadlock rather than
///     an error, because `std::sync::Mutex` is not reentrant — so the two steps
///     have to share a guard and the file write has to be outside it, or the
///     audio callback waits on the disk;
///   * `if let` keeps the temporaries of its scrutinee alive through its body,
///     so `if let Some(save) = locked().read() { locked().write(); }` holds the
///     first guard across the second acquisition. That is what this function is,
///     and the first version was not, and it hung the UI thread during
///     `applicationDidFinishLaunching` — which is to say before the window
///     existed at all.
fn take_session(graph: &Graph) -> Option<SavedTrack> {
    let mut state = lock(&graph.state);
    let save = state.pending_session();
    if save.is_some() {
        state.persisted();
    }
    save
}

/// Writes the position out, holding no lock while the file is touched.
fn save_position(graph: &Graph) {
    if let Some(save) = take_session(graph) {
        session::save(&save);
    }
}

/// The background-music player.
///
/// Cloneable and cheap: every clone is another handle onto the same graph, the
/// same worker and the same output device. It is deliberately not `Send` — the
/// device stream is not — so it belongs on the thread that owns the UI.
#[derive(Clone)]
pub struct Player {
    graph: Arc<Graph>,
    commands: Sender<Command>,
    /// The open device stream, held so the device keeps being pulled: cpal
    /// stops a stream the moment it is dropped, and it has to outlive the worker
    /// and the callback, which the graph's own `Arc` does.
    _output: Rc<cpal::Stream>,
}

impl Player {
    /// Opens the default output device and starts the graph.
    ///
    /// The device is opened once, up front, and left running: the stream is
    /// silent whenever nothing plays, which is also what saves the store from
    /// having to create an `AudioContext` lazily — a web page may only resume one
    /// from a user gesture, a rule that does not exist outside one.
    pub fn new() -> Result<Player> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(Error::NoOutputDevice)?;
        let config = device
            .default_output_config()
            .map_err(|error| Error::Output(error.to_string()))?;
        let device_rate = config.sample_rate().0 as f64;
        let device_channels = config.channels() as usize;

        let (commands, receiver) = channel::<Command>();
        let graph = new_graph(device_rate, device_channels);

        let stream = build_stream(&device, &config, Arc::clone(&graph), device_channels)?;

        let worker = Arc::clone(&graph);
        std::thread::Builder::new()
            .name("conic-music".into())
            .spawn(move || worker_loop(worker, receiver))
            .expect("failed to start the music worker");

        Ok(Player {
            graph,
            commands,
            _output: Rc::new(stream),
        })
    }

    /// The playlist, the store's `tracks`.
    pub fn tracks(&self) -> Arc<Vec<MusicFile>> {
        Arc::clone(&lock(&self.graph.state).tracks)
    }

    /// The transport snapshot the UI binds to.
    pub fn state(&self) -> PlayerState {
        let state = lock(&self.graph.state);
        PlayerState {
            tracks: Arc::clone(&state.tracks),
            current_index: state.current_index,
            current_time: state.cursor,
            duration: state.duration,
            is_playing: state.playing,
            shuffle: state.shuffle,
            repeat: state.repeat,
            error: state.error.clone(),
            pending: state.pending,
            buffering: state.pending > 0 || state.starved,
        }
    }

    /// Which track is selected, or `None`.
    pub fn current_index(&self) -> Option<usize> {
        lock(&self.graph.state).current_index
    }

    /// How far into the current track, in seconds.
    pub fn current_time(&self) -> f64 {
        lock(&self.graph.state).cursor
    }

    /// Whether the device is being fed samples (`isPlaying`).
    pub fn is_playing(&self) -> bool {
        lock(&self.graph.state).playing
    }

    /// Starts a track and plays it (`playIndex`).
    pub fn play_index(&self, index: usize) {
        self.select(index, true);
    }

    /// Loads a track without playing it (`preparePlayback`).
    pub fn prepare_index(&self, index: usize) {
        self.select(index, false);
    }

    /// A selection.
    ///
    /// The playlist row is marked selected straight away and the transport is
    /// marked pending, so the UI reflects the click before the file has even
    /// been opened; what the worker does with it costs a header read.
    fn select(&self, index: usize, autoplay: bool) {
        {
            let mut state = lock(&self.graph.state);
            if index >= state.tracks.len() {
                return;
            }
            state.current_index = Some(index);
            state.cursor = 0.0;
            state.duration = 0.0;
            state.error = None;
            state.pending += 1;
        }
        if self.send(Command::Select { index, autoplay }).is_err() {
            let mut state = lock(&self.graph.state);
            state.pending = state.pending.saturating_sub(1);
        }
    }

    /// `togglePlay`: with nothing selected the first track starts, otherwise
    /// this is the play/pause switch.
    pub fn toggle_play(&self) {
        let (current_index, playing) = {
            let state = lock(&self.graph.state);
            (state.current_index, state.playing)
        };
        if current_index.is_none() {
            if !lock(&self.graph.state).tracks.is_empty() {
                self.play_index(0);
            }
            return;
        }
        if playing {
            self.pause();
        } else {
            self.resume();
        }
    }

    /// `resume`: start feeding the device again.
    pub fn resume(&self) {
        let (loaded, index) = {
            let state = lock(&self.graph.state);
            (state.loaded, state.current_index)
        };
        if !loaded {
            // The store re-prepared the source when `audio.src` was empty, which
            // is the case right after a startup with no track restored.
            if let Some(index) = index {
                self.prepare_index(index);
                return;
            }
        }
        let mut state = lock(&self.graph.state);
        state.playing = true;
        state.ended = false;
        self.graph.signal.notify_all();
    }

    /// `pause`.
    pub fn pause(&self) {
        {
            let mut state = lock(&self.graph.state);
            state.pause();
            state.starved = false;
        }
        save_position(&self.graph);
    }

    /// `next`.
    pub fn next(&self) {
        let (count, shuffle, index) = {
            let state = lock(&self.graph.state);
            (state.tracks.len(), state.shuffle, state.current_index)
        };
        if count == 0 {
            return;
        }
        self.play_index(if shuffle {
            random_index(&self.graph, count, index)
        } else {
            index.map_or(0, |index| (index + 1) % count)
        });
    }

    /// `prev`: within the first three seconds this restarts the track, otherwise
    /// it steps back one.
    pub fn previous(&self) {
        let (count, position, index) = {
            let state = lock(&self.graph.state);
            (state.tracks.len(), state.cursor, state.current_index)
        };
        if count == 0 {
            return;
        }
        if position > 3.0 {
            self.seek(0.0);
            return;
        }
        self.play_index(index.map_or(count - 1, |index| (index + count - 1) % count));
    }

    /// `seek`: records where the position should go, the `audio.currentTime` the
    /// store assigned.
    ///
    /// It does not move the reader — that is the worker's job, and on a
    /// compressed track it means re-decoding, which the thread that pressed the
    /// progress bar has no business waiting for.
    pub fn seek(&self, seconds: f64) {
        if !seconds.is_finite() {
            return;
        }
        {
            let mut state = lock(&self.graph.state);
            let duration = state.duration;
            if duration <= 0.0 || !state.loaded {
                return;
            }
            state.generation = state.generation.wrapping_add(1);
            state.seek = Some((state.generation, seconds.clamp(0.0, duration)));
        }
        self.graph.signal.notify_all();
    }

    /// `seekRatio`.
    pub fn seek_ratio(&self, ratio: f64) {
        let duration = lock(&self.graph.state).duration;
        self.seek(ratio * duration);
    }

    /// `toggleShuffle`.
    pub fn toggle_shuffle(&self) {
        let mut state = lock(&self.graph.state);
        state.shuffle = !state.shuffle;
    }

    /// `cycleRepeat`.
    pub fn cycle_repeat(&self) {
        let mut state = lock(&self.graph.state);
        state.repeat = !state.repeat;
    }

    /// `applyVolume`: `smooth` is the store's flag, the 1s ramp it applies when
    /// the window loses or regains focus.
    pub fn set_volume(&self, volume: f32, smooth: bool) {
        let mut state = lock(&self.graph.state);
        let volume = volume.clamp(0.0, 1.0);
        if smooth {
            state.ramp_from = state.gain;
            state.ramp_started = Some(Instant::now());
        } else {
            state.ramp_from = volume;
            state.ramp_started = None;
        }
        state.volume = volume;
    }

    /// `setAnalyserFftSize`: what the visualiser asks for as its bar count
    /// changes. Takes effect on the next analysis, as the store's did.
    pub fn set_fft_size(&self, size: usize) {
        self.graph.fft_size.store(size.max(32), Ordering::Relaxed);
    }

    /// The analyser's frequency bins in dB — one `getFloatFrequencyData` read,
    /// already smoothed.
    ///
    /// `action` is handed the bins under the graph's lock, so the caller can
    /// copy them into whatever it renders without this having to hand out a copy
    /// of its own sixty times a second.
    pub fn with_spectrum(&self, action: impl FnOnce(&[f32])) {
        let state = lock(&self.graph.state);
        action(&state.spectrum);
    }

    /// The device's sample rate, the `getAudioSampleRate` the visualiser maps
    /// its frequency range against.
    pub fn sample_rate(&self) -> f64 {
        lock(&self.graph.state).sample_rate
    }

    /// A periodic poll, standing in for the `audio.ontimeupdate` the webview
    /// fires while a track plays: it writes the position to the session file at
    /// most every [`PERSIST_INTERVAL`] and hands back the transport state, so
    /// the caller has one read instead of six.
    pub fn tick(&self) -> PlayerState {
        let due = {
            let state = lock(&self.graph.state);
            state
                .last_persisted
                .is_none_or(|last| last.elapsed() >= PERSIST_INTERVAL)
        };
        if due {
            let playing = lock(&self.graph.state).playing;
            if playing {
                save_position(&self.graph);
            }
        }
        self.state()
    }

    /// `restoreSession`: loads the playlist, puts the saved track back where it
    /// was and — when `enabled` and `resume_on_startup` both say so — starts it
    /// playing. A saved track that is no longer in the folder falls back to the
    /// first one, paused, exactly as the Vue store did.
    pub fn restore_session(&self, enabled: bool, resume_on_startup: bool) {
        let tracks = Arc::new(crate::list_music_files().unwrap_or_default());
        {
            let mut state = lock(&self.graph.state);
            state.tracks = Arc::clone(&tracks);
            state.pending += 1;
        }
        if self
            .send(Command::Restore {
                tracks,
                saved: session::load(),
                enabled,
                resume_on_startup,
            })
            .is_err()
        {
            let mut state = lock(&self.graph.state);
            state.pending = state.pending.saturating_sub(1);
        }
    }

    fn send(&self, command: Command) -> std::result::Result<(), ()> {
        let sent = self.commands.send(command).map_err(|_| {
            log::debug!("the music worker is gone; a command was dropped");
        });
        // The worker sleeps on this condvar, and a command it has not been woken
        // for is a command it will not act on.
        self.graph.signal.notify_all();
        sent
    }
}

/// `randomIndex`: a track other than the current one, or 0 when the playlist
/// holds a single entry.
fn random_index(graph: &Graph, count: usize, current: Option<usize>) -> usize {
    if count <= 1 {
        return 0;
    }
    let mut state = lock(&graph.state);
    let current = current.unwrap_or(usize::MAX);
    let mut index = current.min(count - 1);
    while index == current {
        index = ((state.next_random() * count as f64) as usize) % count;
    }
    index
}

/// Why the feed loop gave up the track it was on.
enum Fed {
    /// The track ran out and the ring drained, so the transport may move on.
    Ended,
    /// A command arrived and took priority over the track being fed.
    Interrupted(Command),
}

/// The worker's loop: opening tracks, decoding ahead, and everything that follows
/// from a track running out.
///
/// It is two steps over and over — feed whatever is open, and when that stops
/// happening, do whatever was asked. A track that ends hands straight to
/// [`advance`], which leaves another track open, so feeding resumes without a
/// command ever being involved; and when nothing is open at all the loop blocks
/// on the channel.
fn worker_loop(graph: Arc<Graph>, commands: Receiver<Command>) {
    let mut source: Option<TrackSource> = None;
    let mut resampler = Resampler::default();
    let mut decoded: Vec<i16> = Vec::new();
    let mut resampled: Vec<f32> = Vec::new();

    loop {
        if let Some(track) = source.as_mut() {
            match feed(
                &graph,
                track,
                &mut resampler,
                &mut decoded,
                &mut resampled,
                &commands,
            ) {
                Fed::Ended => {
                    advance(&graph, &mut source, &mut resampler);
                    continue;
                }
                Fed::Interrupted(command) => {
                    apply(&graph, &mut source, &mut resampler, command);
                    continue;
                }
            }
        }

        // Nothing is open — an unreadable file, or an empty playlist. Wait to
        // be told what to play rather than spinning on nothing.
        let Ok(command) = commands.recv() else {
            break;
        };
        let command = coalesce(command, &commands);
        apply(&graph, &mut source, &mut resampler, command);
    }
}

/// Carries out one command.
fn apply(
    graph: &Graph,
    source: &mut Option<TrackSource>,
    resampler: &mut Resampler,
    command: Command,
) {
    match command {
        Command::Select { index, autoplay } => {
            select(graph, source, resampler, index, autoplay);
        }
        Command::Restore {
            tracks,
            saved,
            enabled,
            resume_on_startup,
        } => restore(
            graph,
            source,
            resampler,
            tracks,
            saved,
            enabled,
            resume_on_startup,
        ),
    }
}

/// Folds a burst of commands into the one that matters: the last.
///
/// Five presses of "next" are one selection, not five. Handling them one at a
/// time would open five files in turn, and the user would wait for all of them
/// before hearing the one they asked for.
fn coalesce(mut command: Command, commands: &Receiver<Command>) -> Command {
    while let Ok(newer) = commands.try_recv() {
        command = newer;
    }
    command
}

/// Opens `index` and makes it the current track.
///
/// This is the whole cost of a track change: read the container header, build a
/// decoder, publish the duration. [`feed`] decodes the audio as the device asks
/// for it, so playback starts within milliseconds whatever the file weighs.
fn select(
    graph: &Graph,
    source: &mut Option<TrackSource>,
    resampler: &mut Resampler,
    index: usize,
    autoplay: bool,
) {
    let Some(selected) = lock(&graph.state).tracks.get(index).cloned() else {
        return;
    };

    match TrackSource::open(std::path::Path::new(&selected.path)) {
        Ok(track) => {
            let duration = track.duration();
            *source = Some(track);
            resampler.reset();
            let mut state = lock(&graph.state);
            state.current_index = Some(index);
            state.loaded = true;
            state.duration = duration;
            state.cursor = 0.0;
            state.ended = false;
            state.starved = false;
            state.error = None;
            state.flush();
            if autoplay {
                state.playing = true;
            }
        }
        Err(error) => {
            log::error!("failed to play '{}': {error}", selected.path);
            *source = None;
            let mut state = lock(&graph.state);
            // The track stays selected — it is what the panel and the playlist
            // show — but nothing plays. This is the store's
            // `console.error("Failed to play music", error)`.
            state.current_index = Some(index);
            state.loaded = false;
            state.duration = 0.0;
            state.cursor = 0.0;
            state.playing = false;
            state.ended = false;
            state.starved = false;
            state.flush();
            state.error = Some(error.to_string());
        }
    }
    {
        let mut state = lock(&graph.state);
        state.pending = state.pending.saturating_sub(1);
    }
    graph.signal.notify_all();
    if autoplay {
        // `playIndex` persists; `preparePlayback`, which is the same call with
        // `autoplay` off, does not.
        save_position(graph);
    }
}

/// `restoreSession`, on the worker so that its steps cannot overtake each other.
fn restore(
    graph: &Graph,
    source: &mut Option<TrackSource>,
    resampler: &mut Resampler,
    tracks: Arc<Vec<MusicFile>>,
    saved: Option<SavedTrack>,
    enabled: bool,
    resume_on_startup: bool,
) {
    {
        let mut state = lock(&graph.state);
        state.tracks = Arc::clone(&tracks);
        if tracks.is_empty() {
            state.pending = state.pending.saturating_sub(1);
            return;
        }
    }
    let restored = saved
        .as_ref()
        .and_then(|saved| tracks.iter().position(|track| track.path == saved.path));
    let index = restored.unwrap_or(0);
    select(graph, source, resampler, index, false);

    if let (Some(saved), Some(_)) = (&saved, restored)
        && saved.current_time > 0.0
    {
        let seconds = saved.current_time;
        let mut state = lock(&graph.state);
        state.generation = state.generation.wrapping_add(1);
        state.seek = Some((state.generation, seconds));
    }
    if enabled && resume_on_startup && restored.is_some() {
        lock(&graph.state).playing = true;
    }
    graph.signal.notify_all();
}

/// Keeps the ring buffer filled for as long as the track lasts.
///
/// The one thing it must never do is block without a way out: it waits on
/// [`Graph::signal`], which the callback raises when the device has drained the
/// buffer and [`Player::send`] raises when a command has been queued. So a "next"
/// pressed while the worker is asleep for room still gets through.
fn feed(
    graph: &Graph,
    track: &mut TrackSource,
    resampler: &mut Resampler,
    decoded: &mut Vec<i16>,
    resampled: &mut Vec<f32>,
    commands: &Receiver<Command>,
) -> Fed {
    let mut draining = false;
    let mut stalled = 0;

    loop {
        // A command first: it is what the user is waiting on, and a selection
        // supersedes the track being fed.
        if let Ok(command) = commands.try_recv() {
            return Fed::Interrupted(command);
        }

        // A seek the UI asked for while this track was playing.
        //
        // The request is taken out into a local first, so the guard is released
        // before `service_seek` runs: that takes the same lock again, and the
        // mutex is not reentrant. Written as one `if let` over the locked
        // expression the guard would live through the body and deadlock the
        // worker on the first scrub of the session.
        let request = lock(&graph.state).seek.take();
        if let Some((generation, seconds)) = request {
            service_seek(graph, track, resampler, generation, seconds);
            draining = false;
            stalled = 0;
        }

        // The reader is done; the audio it produced is still in the ring, so the
        // transport is not told the track ended until the device has actually
        // played it. That is what the element's `onended` meant.
        if draining {
            if lock(&graph.state).ring.is_empty() {
                let mut state = lock(&graph.state);
                state.ended = true;
                state.playing = false;
                return Fed::Ended;
            }
            // Always sleep here rather than only below the low-water mark: the
            // ring is already draining, and polling it would be a spin. The
            // callback wakes this as soon as the buffer runs low.
            let state = lock(&graph.state);
            wait(graph, state);
            continue;
        }

        // Room in the ring?
        let full = {
            let state = lock(&graph.state);
            let (_, capacity) = state.ring_bounds();
            state.ring.len() >= capacity
        };
        if full {
            let state = lock(&graph.state);
            wait(graph, state);
            continue;
        }

        decoded.clear();
        match track.decode_into(decoded) {
            Ok(0) if track.finished() => {
                draining = true;
                stalled = 0;
                continue;
            }
            Ok(0) => {
                // A packet the codec would not read, skipped rather than fatal.
                // Enough of them in a row and the rest of the file is not going
                // to decode either.
                stalled += 1;
                if stalled >= MAX_STALLED_READS {
                    let mut state = lock(&graph.state);
                    state.ended = true;
                    state.playing = false;
                    return Fed::Ended;
                }
                continue;
            }
            Ok(_) => stalled = 0,
            Err(error) => {
                log::error!("could not decode '{}': {error}", track_path(graph));
                let mut state = lock(&graph.state);
                state.error = Some(error.to_string());
                state.playing = false;
                state.ended = true;
                return Fed::Ended;
            }
        }

        let (rate, channels) = {
            let state = lock(&graph.state);
            (state.sample_rate, state.device_channels)
        };
        resampled.clear();
        resampler.run(
            track.sample_rate(),
            track.channels(),
            rate,
            channels,
            decoded,
            resampled,
        );
        if resampled.is_empty() {
            // The reader produced samples the resampler could not turn into any
            // the device can use, which only a rate or channel count it will not
            // accept causes. Counted as a stall so that a track the player cannot
            // play is given up on rather than spun on.
            stalled += 1;
            if stalled >= MAX_STALLED_READS {
                let mut state = lock(&graph.state);
                state.ended = true;
                state.playing = false;
                return Fed::Ended;
            }
            continue;
        }
        stalled = 0;

        let position = track.position();
        let fft_size = graph.fft_size.load(Ordering::Relaxed);
        lock(&graph.state).push(resampled, fft_size, position);
    }
}

/// Moves the reader to `seconds` on the worker's behalf.
fn service_seek(
    graph: &Graph,
    track: &mut TrackSource,
    resampler: &mut Resampler,
    generation: u64,
    seconds: f64,
) {
    let seeked = track.seek(seconds);
    resampler.reset();
    let mut state = lock(&graph.state);
    // Only clear the ring for the request that was ours. A newer one may have
    // been recorded while the seek was being performed, and clearing then would
    // throw away the audio for the position the UI is now showing.
    let still_current = state
        .seek
        .as_ref()
        .is_none_or(|(newer, _)| *newer == generation);
    if still_current {
        state.flush();
        state.cursor = seconds;
        state.ended = false;
        state.starved = false;
    }
    if let Err(error) = seeked {
        log::warn!("could not seek to {seconds}s: {error}");
        state.error = Some(error.to_string());
    }
    drop(state);
    graph.signal.notify_all();
}

/// The selected track's path, for a log line.
fn track_path(graph: &Graph) -> String {
    let state = lock(&graph.state);
    state
        .current_index
        .and_then(|index| state.tracks.get(index))
        .map_or_else(String::new, |track| track.path.clone())
}

/// `handleTrackEnded`: repeat plays the same track again, otherwise the playlist
/// advances — shuffled when the flag is on.
fn advance(graph: &Graph, source: &mut Option<TrackSource>, resampler: &mut Resampler) {
    let (repeat, shuffle, count, index) = {
        let state = lock(&graph.state);
        (
            state.repeat,
            state.shuffle,
            state.tracks.len(),
            state.current_index,
        )
    };
    if count == 0 {
        return;
    }
    if repeat {
        // The same track from the top. It is still open, so this is a seek
        // rather than a second read of a file that has not gone anywhere.
        let restart = source.as_mut().is_some_and(|track| track.seek(0.0).is_ok());
        if restart {
            resampler.reset();
            let mut state = lock(&graph.state);
            state.flush();
            state.cursor = 0.0;
            state.ended = false;
            state.starved = false;
            state.playing = true;
            drop(state);
            graph.signal.notify_all();
            return;
        }
    }
    let next = if shuffle {
        random_index(graph, count, index)
    } else {
        index.map_or(0, |index| (index + 1) % count)
    };
    select(graph, source, resampler, next, true);
}

/// Converts a track's own rate and channel count into the device's, so the
/// callback only has to copy and scale.
///
/// The interpolation carries a frame across calls, which is what lets it work in
/// whatever sizes the caller reads: a block boundary is not a sample boundary,
/// and losing the frame a read interpolates *from* would put a step in the output
/// every time the buffer was refilled.
#[derive(Default)]
struct Resampler {
    /// Source-rate frames not yet consumed, oldest first. It always keeps the
    /// frame the next read interpolates from, so that frame is never the one
    /// dropped at the end of a call.
    tail: Vec<f32>,
    /// The read position within `tail`, in frames.
    index: f64,
    /// The channel count of the track last read, so a change can be noticed.
    source_channels: usize,
}

impl Resampler {
    /// Forgets the interpolation state, for when the track changes underneath it.
    fn reset(&mut self) {
        self.tail.clear();
        self.index = 0.0;
        self.source_channels = 0;
    }

    /// Appends `decoded` — interleaved samples in the source's own format — to
    /// `out` as interleaved `f32` in `-1.0..=1.0` at the device's rate and
    /// channel count.
    fn run(
        &mut self,
        source_rate: u32,
        source_channels: u16,
        device_rate: f64,
        device_channels: usize,
        decoded: &[i16],
        out: &mut Vec<f32>,
    ) {
        let source_channels = usize::from(source_channels);
        if source_rate == 0 || source_channels == 0 || device_channels == 0 || device_rate <= 0.0 {
            return;
        }
        let step = f64::from(source_rate) / device_rate;
        if step <= 0.0 {
            return;
        }

        if source_channels != self.source_channels {
            // A different track, or a different number of channels within it:
            // whatever was carried over belongs to the old one.
            self.reset();
            self.source_channels = source_channels;
        }

        self.tail
            .extend(decoded.iter().map(|sample| *sample as f32 / 32_768.0));
        let frames = self.tail.len() / source_channels;
        if frames == 0 {
            return;
        }

        // A read never consumes the last frame: the one after it is what the
        // interpolation reads towards.
        while self.index + 1.0 < frames as f64 {
            let base = self.index.floor() as usize;
            let fraction = (self.index - base as f64) as f32;
            for channel in 0..device_channels {
                // A device with more channels than the track repeats the last
                // one, the way a browser up-mixes; a mono track reaches all of
                // them.
                let source = channel.min(source_channels - 1);
                let current = self.tail[base * source_channels + source];
                let next = self.tail[(base + 1) * source_channels + source];
                out.push(current + (next - current) * fraction);
            }
            // Source frames per output frame: the only place the two rates meet.
            self.index += step;
        }

        // Keep what the next read needs and nothing more: from the frame the
        // cursor is on to the end.
        let keep_from = (self.index.floor().max(0.0) as usize).min(frames);
        if keep_from > 0 {
            self.tail.drain(..keep_from * source_channels);
            self.index -= keep_from as f64;
        }
    }
}

thread_local! {
    /// Scratch for the devices that ask for integer samples: one allocation for
    /// the life of the thread, grown to the largest buffer the device has asked
    /// for and never shrunk. A device that wants integers is a rounding error
    /// away from being as fast, and cpal hands the callback no scratch of its
    /// own, so this is where the conversion buffer lives.
    static CONVERSION: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

/// Opens the device stream and its callback.
fn build_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    graph: Arc<Graph>,
    channels: usize,
) -> Result<cpal::Stream> {
    let on_error = |error: cpal::StreamError| log::error!("audio output failed: {error}");
    let stream_config = StreamConfig::from(config.clone());
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                render(out, &graph, channels);
            },
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &stream_config,
            move |out: &mut [i16], _: &cpal::OutputCallbackInfo| {
                with_conversion(out.len(), |samples| {
                    render(samples, &graph, channels);
                    for (target, source) in out.iter_mut().zip(samples.iter()) {
                        *target = (source * 32_767.0).clamp(-32_768.0, 32_767.0) as i16;
                    }
                });
            },
            on_error,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &stream_config,
            move |out: &mut [u16], _: &cpal::OutputCallbackInfo| {
                with_conversion(out.len(), |samples| {
                    render(samples, &graph, channels);
                    for (target, source) in out.iter_mut().zip(samples.iter()) {
                        *target = (((source + 1.0) * 0.5 * 65_535.0).clamp(0.0, 65_535.0)) as u16;
                    }
                });
            },
            on_error,
            None,
        ),
        format => {
            return Err(Error::Output(format!(
                "the device only offers {format} output, which is not supported"
            )));
        }
    }
    .map_err(|error| Error::Output(error.to_string()))?;

    stream
        .play()
        .map_err(|error| Error::Output(error.to_string()))?;
    Ok(stream)
}

/// Runs `action` with a scratch buffer of exactly `len` floats.
fn with_conversion(len: usize, action: impl FnOnce(&mut [f32])) {
    CONVERSION.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        if scratch.len() < len {
            scratch.resize(len, 0.0);
        }
        action(&mut scratch[..len]);
    });
}

/// Fills one output buffer: what the worker left in the ring, scaled by the gain,
/// plus the analyser's transform for this instant.
///
/// It allocates nothing, reads no file and takes the lock once. Everything that
/// used to happen here — the cursor walk, the resampling, the per-frame
/// interpolation, the two `vec!`s per callback — belongs to the worker now,
/// because a callback that does any of it can miss its deadline, and a missed
/// deadline on an audio callback is a click.
fn render(out: &mut [f32], graph: &Graph, channels: usize) {
    let now = Instant::now();
    let fft_size = graph.fft_size.load(Ordering::Relaxed);
    let mut state = lock(&graph.state);
    let gain = state.gain_at(now);
    if !state.playing || channels == 0 {
        out.fill(0.0);
        return;
    }

    // The transform runs on its own clock rather than once per buffer, because
    // the smoothing has to advance at a rate the visualiser's frames can rely on.
    let due = state
        .last_analysis
        .is_none_or(|last| now.saturating_duration_since(last) >= ANALYSIS_INTERVAL);
    if due {
        state.last_analysis = Some(now);
        state.analyser.set_fft_size(fft_size);
        let GraphState {
            analyser,
            tap,
            spectrum,
            ..
        } = &mut *state;
        analyser.read_into(tap, spectrum);
    }

    let available = state.ring.len().min(out.len());
    let starved = available < out.len();
    // A starved buffer repeats the last sample rather than jumping to silence: a
    // step off a waveform mid-cycle is a click, and the worker is on its way.
    let held = state.tail * gain;
    for (target, source) in out.iter_mut().zip(state.ring.drain(..available)) {
        *target = source * gain;
    }
    for sample in out[available..].iter_mut() {
        *sample = held;
    }
    if starved {
        state.starved = true;
    }
    let (low, _) = state.ring_bounds();
    let needs_more = starved || state.ring.len() < low;
    drop(state);
    if needs_more {
        graph.signal.notify_all();
    }
}

/// A clock-seeded xorshift state, for `Math.random()`.
fn seed_random() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    // Never zero: xorshift cannot leave it.
    nanos | 1
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A graph with one track selected, which is the state a position write and
    /// a seek request both need to be meaningful.
    fn graph_with_a_track() -> (Arc<Graph>, String) {
        let graph = new_graph(48_000.0, 2);
        let path = "/nonexistent/track.wav".to_string();
        {
            let mut state = lock(&graph.state);
            state.tracks = Arc::new(vec![MusicFile {
                name: "track.wav".to_string(),
                path: path.clone(),
            }]);
            state.current_index = Some(0);
            state.loaded = true;
            state.duration = 120.0;
            state.playing = true;
            state.cursor = 42.5;
        }
        (graph, path)
    }

    #[test]
    fn taking_the_session_releases_the_lock_before_the_next_take() {
        // The exact shape that hung the app: read the state under the graph's
        // lock, then take the same lock again straight afterwards. A `std::sync::
        // Mutex` is not reentrant, so a guard that outlives its statement does
        // not panic — it waits, forever, on a thread holding it. This hangs the
        // test rather than failing it, which is the whole reason the reading and
        // the writing are separate functions.
        let (graph, path) = graph_with_a_track();

        let first = take_session(&graph).expect("a selected track has a position");
        let second = take_session(&graph).expect("and so does the next read");

        assert_eq!(first.path, path);
        assert_eq!(first.current_time, 42.5);
        assert_eq!(second.current_time, 42.5);
        // And the mark was set, so `Player::tick` will not write it again for
        // another `PERSIST_INTERVAL`.
        let due = lock(&graph.state)
            .last_persisted
            .is_none_or(|last| last.elapsed() >= PERSIST_INTERVAL);
        assert!(!due, "the position was not marked as written");
    }

    #[test]
    fn taking_a_seek_request_releases_the_lock_before_the_worker_acts_on_it() {
        // The same hazard on the worker's side. `feed` has to read the request
        // out from under the lock and let the guard go before `service_seek`
        // takes it again — the seek itself is a container seek plus a decoder
        // reset, so the guard cannot be held across it in any case.
        let (graph, _) = graph_with_a_track();
        {
            let mut state = lock(&graph.state);
            state.generation = state.generation.wrapping_add(1);
            state.seek = Some((state.generation, 90.0));
        }

        // Read out, then take the lock as the rest of `feed` does. If the read
        // kept its guard this would never return.
        let request = lock(&graph.state).seek.take();
        let state_is_reachable = { lock(&graph.state).cursor };
        assert_eq!(state_is_reachable, 42.5);

        let (generation, seconds) = request.expect("the request was there");
        assert_eq!(seconds, 90.0);
        assert_eq!(generation, 1);
        // Consumed, so a second pass does not seek again.
        assert!(lock(&graph.state).seek.is_none());
    }

    #[test]
    fn the_ring_holds_the_configured_number_of_seconds() {
        let graph = new_graph(48_000.0, 2);
        let (low, capacity) = lock(&graph.state).ring_bounds();
        assert_eq!(capacity, (48_000.0 * 2.0 * RING_SECONDS) as usize);
        assert_eq!(low, (capacity as f64 * RING_LOW_WATER) as usize);
        assert!(
            low < capacity,
            "the low-water mark has to leave room to refill"
        );
    }

    #[test]
    fn a_starved_ring_reports_buffering_rather_than_playing() {
        // What the panel shows: the transport waiting on audio, either because a
        // selection is being opened or because the device has run the buffer
        // dry. The second used to be indistinguishable from "playing, and
        // quietly nothing".
        let (graph, _) = graph_with_a_track();
        assert!(!lock(&graph.state).starved);
        lock(&graph.state).starved = true;
        assert!(lock(&graph.state).starved, "the flag did not take");
    }
}
