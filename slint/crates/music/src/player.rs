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
//! The device pulls samples on cpal's callback thread, which only ever touches
//! the graph under its one lock: everything that has to be instantaneous — the
//! cursor, the gain ramp, the seek — happens there. Everything expensive happens
//! on the worker's thread, which decodes a track and swaps it in. The two talk
//! over a channel, in both directions: the worker is told to select a track, and
//! the callback tells the worker when the cursor ran off the end of one (the
//! `audio.onended` the store handled in `handleTrackEnded`).
//!
//! The read side — [`Player::state`], [`Player::spectrum`] — is a lock and a
//! clone, so the UI can poll it as often as it likes.

use std::rc::Rc;
use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicUsize, Ordering},
    mpsc::{Receiver, Sender, channel},
};
use std::time::{Duration, Instant};

use cpal::StreamConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::MusicFile;
use crate::analyser::{Analyser, DEFAULT_FFT_SIZE};
use crate::decode::{self, DecodedTrack};
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

/// Everything the UI shows about the transport, as one snapshot.
#[derive(Clone, Debug, Default)]
pub struct PlayerState {
    /// The playlist, in the order `list_music_files` sorted it.
    pub tracks: Vec<MusicFile>,
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
    /// How many track selections are still queued on the worker. A selection is
    /// asynchronous — the file has to be decoded before the transport can say it is
    /// playing — so a caller has to be able to tell "not playing yet" from "never
    /// going to play".
    pub pending: u32,
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

struct GraphState {
    tracks: Vec<MusicFile>,
    current_index: Option<usize>,
    /// The decoded track the cursor walks, and the cursor itself: source frames
    /// from its start, which is what `audio.currentTime` times the track's rate.
    track: Option<Arc<DecodedTrack>>,
    cursor: f64,
    playing: bool,
    /// The track ran out; the worker picks this up as `Ended`.
    ended: bool,
    /// The device's rate — the `AudioContext`'s.
    sample_rate: f64,
    /// The target gain, and the gain itself, which is what the 1s ramp moves.
    volume: f32,
    gain: f32,
    ramp_from: f32,
    ramp_started: Option<Instant>,
    /// The pre-gain signal the analyser reads, newest last.
    tap: Vec<f32>,
    /// The transform, owned by the audio thread (it is the only writer).
    analyser: Analyser,
    /// The last spectrum, in dB.
    spectrum: Arc<Vec<f32>>,
    last_analysis: Option<Instant>,
    shuffle: bool,
    repeat: bool,
    error: Option<String>,
    /// How many selections the worker still has to do; see
    /// [`PlayerState::pending`].
    pending: u32,
    /// When the position was last written to the session file.
    last_persisted: Option<Instant>,
    /// The `Math.random()` of `randomIndex`, seeded from the clock.
    random: u64,
}

/// What the worker is told to do.
enum Command {
    /// Decode a track and make it the current one, starting playback when
    /// `autoplay` — the `playIndex` / `preparePlayback` pair.
    Select { index: usize, autoplay: bool },
    /// The current track reached its end.
    Ended,
    /// `restoreSession`: put the playlist and the saved position back.
    Restore {
        tracks: Vec<MusicFile>,
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

    /// The current position in seconds: the `audio.currentTime` the store reads
    /// for the panel's left-hand time and for the progress bar.
    fn position(&self) -> f64 {
        let Some(track) = self.track.as_ref() else {
            return 0.0;
        };
        if track.sample_rate() == 0 {
            return 0.0;
        }
        (self.cursor / track.sample_rate() as f64).clamp(0.0, track.duration())
    }

    fn duration(&self) -> f64 {
        self.track
            .as_ref()
            .map(|track| track.duration())
            .unwrap_or(0.0)
    }

    /// The path of the selected track, the `currentTrack` the session is written
    /// from.
    fn current_path(&self) -> Option<String> {
        let index = self.current_index?;
        self.tracks.get(index).map(|track| track.path.clone())
    }

    /// Writes the position to the session file, the store's `persistState`.
    fn persist(&mut self) {
        let Some(path) = self.current_path() else {
            return;
        };
        session::save(&SavedTrack {
            path,
            current_time: self.position(),
        });
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

    /// `pause`: stop feeding the device and write the position out, which is
    /// what the store's `pause` did through the element's `onpause` handler.
    fn pause(&mut self) {
        self.playing = false;
        self.persist();
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
        let graph = Arc::new(Graph {
            state: Mutex::new(GraphState {
                tracks: Vec::new(),
                current_index: None,
                track: None,
                cursor: 0.0,
                playing: false,
                ended: false,
                sample_rate: device_rate,
                volume: 1.0,
                gain: 1.0,
                ramp_from: 1.0,
                ramp_started: None,
                tap: Vec::new(),
                analyser: Analyser::new(DEFAULT_FFT_SIZE),
                spectrum: Arc::new(vec![0.0; DEFAULT_FFT_SIZE / 2]),
                last_analysis: None,
                shuffle: false,
                repeat: false,
                error: None,
                pending: 0,
                last_persisted: None,
                random: seed_random(),
            }),
            fft_size: AtomicUsize::new(DEFAULT_FFT_SIZE),
        });

        let stream = build_stream(
            &device,
            &config,
            Arc::clone(&graph),
            commands.clone(),
            device_rate,
            device_channels,
        )?;

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
    pub fn tracks(&self) -> Vec<MusicFile> {
        lock(&self.graph.state).tracks.clone()
    }

    /// The transport snapshot the UI binds to.
    pub fn state(&self) -> PlayerState {
        let state = lock(&self.graph.state);
        PlayerState {
            tracks: state.tracks.clone(),
            current_index: state.current_index,
            current_time: state.position(),
            duration: state.duration(),
            is_playing: state.playing,
            shuffle: state.shuffle,
            repeat: state.repeat,
            error: state.error.clone(),
            pending: state.pending,
        }
    }

    /// Which track is selected, or `None`.
    pub fn current_index(&self) -> Option<usize> {
        lock(&self.graph.state).current_index
    }

    /// How far into the current track, in seconds.
    pub fn current_time(&self) -> f64 {
        lock(&self.graph.state).position()
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

    /// A selection, counted as pending until the worker has decoded it.
    fn select(&self, index: usize, autoplay: bool) {
        lock(&self.graph.state).pending += 1;
        if self.send(Command::Select { index, autoplay }).is_err() {
            lock(&self.graph.state).pending = lock(&self.graph.state).pending.saturating_sub(1);
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
        let has_track = lock(&self.graph.state).track.is_some();
        if !has_track {
            // The store re-prepared the source when `audio.src` was empty, which
            // is the case right after a startup with no track restored.
            if let Some(index) = self.current_index() {
                self.prepare_index(index);
                return;
            }
        }
        let mut state = lock(&self.graph.state);
        state.playing = true;
        state.ended = false;
    }

    /// `pause`.
    pub fn pause(&self) {
        lock(&self.graph.state).pause();
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
            (state.tracks.len(), state.position(), state.current_index)
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

    /// `seek`: moves the cursor, clamped to the track (`HTMLMediaElement` clamps
    /// a seek outside the media too).
    pub fn seek(&self, seconds: f64) {
        seek(&self.graph, seconds);
    }

    /// `seekRatio`.
    pub fn seek_ratio(&self, ratio: f64) {
        let duration = lock(&self.graph.state).duration();
        seek(&self.graph, ratio * duration);
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
    /// already smoothed. Empty until the first transform has run.
    pub fn spectrum(&self) -> Arc<Vec<f32>> {
        Arc::clone(&lock(&self.graph.state).spectrum)
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
        {
            let mut state = lock(&self.graph.state);
            let due = state
                .last_persisted
                .is_none_or(|last| last.elapsed() >= PERSIST_INTERVAL);
            if due && state.playing {
                state.persist();
            }
        }
        self.state()
    }

    /// `restoreSession`: loads the playlist, puts the saved track back where it
    /// was and — when `enabled` and `resume_on_startup` both say so — starts it
    /// playing. A saved track that is no longer in the folder falls back to the
    /// first one, paused, exactly as the Vue store did.
    ///
    /// The whole sequence runs on the worker: it has to, because the position is
    /// only meaningful once the track it belongs to is decoded.
    pub fn restore_session(&self, enabled: bool, resume_on_startup: bool) {
        let tracks = crate::list_music_files().unwrap_or_default();
        lock(&self.graph.state).pending += 1;
        if self
            .send(Command::Restore {
                tracks,
                saved: session::load(),
                enabled,
                resume_on_startup,
            })
            .is_err()
        {
            lock(&self.graph.state).pending = lock(&self.graph.state).pending.saturating_sub(1);
        }
    }

    fn send(&self, command: Command) -> std::result::Result<(), ()> {
        self.commands.send(command).map_err(|_| {
            log::debug!("the music worker is gone; a command was dropped");
        })
    }
}

/// Moves the cursor, clamped to the track.
fn seek(graph: &Graph, seconds: f64) {
    let mut state = lock(&graph.state);
    let Some(track) = state.track.as_ref() else {
        return;
    };
    let duration = state.duration();
    if duration <= 0.0 || !seconds.is_finite() {
        return;
    }
    state.cursor = (seconds.clamp(0.0, duration) * track.sample_rate() as f64).max(0.0);
    state.ended = false;
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

/// The worker's loop: the decoding, and everything that follows from a track
/// running out.
fn worker_loop(graph: Arc<Graph>, commands: Receiver<Command>) {
    while let Ok(command) = commands.recv() {
        match command {
            Command::Select { index, autoplay } => select(&graph, index, autoplay),
            Command::Ended => ended(&graph),
            Command::Restore {
                tracks,
                saved,
                enabled,
                resume_on_startup,
            } => restore_session(&graph, tracks, saved, enabled, resume_on_startup),
        }
    }
}

/// `restoreSession`, on the worker so that its steps cannot overtake each other.
fn restore_session(
    graph: &Graph,
    tracks: Vec<MusicFile>,
    saved: Option<SavedTrack>,
    enabled: bool,
    resume_on_startup: bool,
) {
    lock(&graph.state).tracks = tracks;
    if lock(&graph.state).tracks.is_empty() {
        return;
    }
    let restored = saved.as_ref().and_then(|saved| {
        lock(&graph.state)
            .tracks
            .iter()
            .position(|track| track.path == saved.path)
    });
    let index = restored.unwrap_or(0);
    select(graph, index, false);
    if let (Some(saved), Some(_)) = (&saved, restored)
        && saved.current_time > 0.0
    {
        seek(graph, saved.current_time);
    }
    if enabled && resume_on_startup && restored.is_some() {
        lock(&graph.state).playing = true;
    }
}

/// Decodes `index` and makes it the current track.
fn select(graph: &Graph, index: usize, autoplay: bool) {
    // The decode happens outside the lock — it is the slow part, and the audio
    // callback must not wait on it. Two queued selections are therefore
    // possible, and the last one to finish is the one that wins: the state it
    // writes is the one the other just overwrote.
    let selected = lock(&graph.state).tracks.get(index).cloned();
    let decoded = selected.as_ref().map(|track| {
        let path = std::path::PathBuf::from(&track.path);
        let result = decode::decode(&path);
        (path, result)
    });

    if let Some((path, result)) = decoded {
        match result {
            Ok(decoded) => {
                let mut state = lock(&graph.state);
                state.current_index = Some(index);
                state.track = Some(Arc::new(decoded));
                state.cursor = 0.0;
                state.ended = false;
                state.playing = autoplay;
                state.error = None;
                if autoplay {
                    // `playIndex` persists; `preparePlayback`, which is the same
                    // call with `autoplay` off, does not.
                    state.persist();
                }
            }
            Err(error) => {
                let name = selected.map(|track| track.name).unwrap_or_default();
                // The store's `console.error("Failed to play music", error)`: the
                // track stays selected — it is what the panel and the playlist
                // show — but nothing plays.
                log::error!("failed to play '{name}': {error} ({})", path.display());
                let mut state = lock(&graph.state);
                state.current_index = Some(index);
                state.track = None;
                state.cursor = 0.0;
                state.playing = false;
                state.ended = false;
                state.error = Some(error.to_string());
            }
        }
    }
    let mut state = lock(&graph.state);
    state.pending = state.pending.saturating_sub(1);
}

/// `handleTrackEnded`: repeat plays the same track again, otherwise the playlist
/// advances — shuffled when the flag is on. It runs on the worker, so the next
/// track can be selected here rather than through another command.
fn ended(graph: &Graph) {
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
        let mut state = lock(&graph.state);
        state.cursor = 0.0;
        state.ended = false;
        state.playing = true;
        return;
    }
    let next = if shuffle {
        random_index(graph, count, index)
    } else {
        index.map_or(0, |index| (index + 1) % count)
    };
    select(graph, next, true);
}

/// Opens the device stream and its callback.
fn build_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    graph: Arc<Graph>,
    commands: Sender<Command>,
    sample_rate: f64,
    channels: usize,
) -> Result<cpal::Stream> {
    let on_error = |error: cpal::StreamError| log::error!("audio output failed: {error}");
    let stream_config = StreamConfig::from(config.clone());
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &stream_config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                render(out, &graph, &commands, sample_rate, channels)
            },
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            &stream_config,
            move |out: &mut [i16], _: &cpal::OutputCallbackInfo| {
                let mut samples = vec![0.0f32; out.len()];
                render(&mut samples, &graph, &commands, sample_rate, channels);
                for (target, source) in out.iter_mut().zip(samples) {
                    *target = (source * 32767.0).clamp(-32768.0, 32767.0) as i16;
                }
            },
            on_error,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            &stream_config,
            move |out: &mut [u16], _: &cpal::OutputCallbackInfo| {
                let mut samples = vec![0.0f32; out.len()];
                render(&mut samples, &graph, &commands, sample_rate, channels);
                for (target, source) in out.iter_mut().zip(samples) {
                    *target = (((source + 1.0) * 0.5 * 65535.0).clamp(0.0, 65535.0)) as u16;
                }
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

/// Fills one output buffer: walks the cursor through the track, resampling as it
/// goes, taps the analyser, applies the gain, and reports the end of the track.
fn render(
    out: &mut [f32],
    graph: &Arc<Graph>,
    commands: &Sender<Command>,
    sample_rate: f64,
    channels: usize,
) {
    let now = Instant::now();
    let fft_size = graph.fft_size.load(Ordering::Relaxed);
    let mut state = lock(&graph.state);
    let gain = state.gain_at(now);
    if !state.playing {
        out.fill(0.0);
        return;
    }
    let Some(track) = state.track.clone() else {
        out.fill(0.0);
        return;
    };
    if sample_rate <= 0.0 || channels == 0 {
        out.fill(0.0);
        return;
    }

    let source_channels = (track.channels() as usize).max(1);
    // Source frames per output frame: the only place the two rates meet.
    let step = track.sample_rate() as f64 / sample_rate;
    let frames = out.len() / channels;
    // Two frames of headroom: `read` interpolates into the next one, and a
    // mono track is read into the same buffer.
    let mut frame_buffer = vec![0.0f32; source_channels * 2];
    let mut mono = vec![0.0f32; frames];
    let mut cursor = state.cursor;
    let mut produced = 0usize;

    for frame in 0..frames {
        if track.read(cursor, 1, &mut frame_buffer).is_none() {
            break;
        }
        let mono_sum: f32 = frame_buffer[..source_channels].iter().sum();
        mono[frame] = mono_sum / source_channels as f32;
        for channel in 0..channels {
            // A device with more channels than the track repeats its last one,
            // the way a browser up-mixes; a mono track reaches all of them.
            out[frame * channels + channel] = frame_buffer[channel.min(source_channels - 1)] * gain;
        }
        produced = frame + 1;
        cursor += step;
    }

    // Whatever produced nothing is silence rather than stale samples.
    for sample in out[produced * channels..].iter_mut() {
        *sample = 0.0;
    }

    state.cursor = cursor;
    state.tap.extend_from_slice(&mono[..produced]);
    if state.tap.len() > fft_size * 2 {
        let excess = state.tap.len() - fft_size * 2;
        state.tap.drain(..excess);
    }

    // The transform runs on its own clock rather than once per buffer, because
    // the smoothing has to advance at a rate the visualiser's frames can rely on.
    let due = state
        .last_analysis
        .is_none_or(|last| now.saturating_duration_since(last) >= ANALYSIS_INTERVAL);
    if due {
        state.last_analysis = Some(now);
        state.analyser.set_fft_size(fft_size);
        let spectrum = {
            let GraphState { analyser, tap, .. } = &mut *state;
            analyser.read(tap)
        };
        state.spectrum = Arc::new(spectrum);
    }

    if cursor >= track.frames() as f64 && !state.ended {
        state.ended = true;
        state.playing = false;
        let _ = commands.send(Command::Ended);
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
