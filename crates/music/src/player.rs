// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The player: the audio graph (decoded track → resampler → gain → output) and the
//! transport that drives it. The audio callback may not block, so decoding,
//! resampling and seeking happen elsewhere and the resampler matches the device's
//! rate.

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

/// [`Player::tick`] persists the position at most this often while a track plays.
pub const PERSIST_INTERVAL: Duration = Duration::from_millis(5000);

const VOLUME_RAMP: Duration = Duration::from_secs(1);

/// A fixed 16 ms clock (about 60 Hz) so the smoothing advances the same on every
/// machine, rather than being tied to a refresh rate the player cannot know.
const ANALYSIS_INTERVAL: Duration = Duration::from_millis(16);

/// The cushion between a disk that cannot keep up and a device that will not
/// wait: long enough to ride out a seek, short enough that the UI position does
/// not visibly lead the sound.
const RING_SECONDS: f64 = 2.0;

/// Waking at half rather than waiting for the ring to run dry leaves room to keep
/// the device fed while the next batch is decoded.
const RING_LOW_WATER: f64 = 0.5;

/// A run of empty reads means the rest of the file will not decode; the worker
/// must not spin on a track that will never yield another sample.
const MAX_STALLED_READS: u32 = 64;

#[derive(Clone, Debug, Default)]
pub struct PlayerState {
    /// Shared rather than copied, so polling sixty times a second does not clone
    /// the whole playlist each time.
    pub tracks: Arc<Vec<MusicFile>>,
    pub current_index: Option<usize>,
    pub current_time: f64,
    pub duration: f64,
    pub is_playing: bool,
    pub shuffle: bool,
    pub repeat: bool,
    pub error: Option<String>,
    /// Selections still being opened. A selection is asynchronous, so a caller
    /// has to tell "not playing yet" from "never going to play".
    pub pending: u32,
    /// A selection being opened or the ring having run short; what a UI shows
    /// instead of silently sitting still.
    pub buffering: bool,
}

impl PlayerState {
    pub fn current_track(&self) -> Option<&MusicFile> {
        self.current_index.and_then(|index| self.tracks.get(index))
    }
}

/// The graph, shared between the device callback and everything else.
struct Graph {
    state: Mutex<GraphState>,
    /// Raised both when the device needs samples and when a command is queued.
    signal: Condvar,
    /// Requested `fftSize`, set by the UI thread and only applied on the audio
    /// thread, which owns the transform.
    fft_size: AtomicUsize,
}

/// Locks the graph, recovering from a poisoned mutex.
///
/// A panic anywhere in the graph must not silence the device: the callback takes
/// the lock on every buffer, and a poisoned one would otherwise take the music
/// down for the rest of the session.
fn lock(state: &Mutex<GraphState>) -> MutexGuard<'_, GraphState> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// [`lock`], for the worker about to sleep on [`Graph::signal`]. The guard is
/// dropped: nothing may be done to the graph before re-checking why it woke.
fn wait(graph: &Graph, guard: MutexGuard<'_, GraphState>) {
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
    /// A track is open on the worker, playing or not.
    loaded: bool,
    /// Published by the worker, which is the only thing that knows how far its
    /// reader has got and how much is still buffered.
    cursor: f64,
    /// Published as soon as the header says, long before the track is decoded.
    duration: f64,
    playing: bool,
    /// The track ran out and the ring drained; the worker picks this up.
    ended: bool,
    sample_rate: f64,
    device_channels: usize,
    volume: f32,
    gain: f32,
    ramp_from: f32,
    ramp_started: Option<Instant>,
    /// Already at the device's rate and channel count.
    ring: VecDeque<f32>,
    /// The last sample the worker produced. A starved ring repeats it rather than
    /// jumping to silence, which would be an audible click.
    tail: f32,
    /// The post-gain output the analyser reads, newest last.
    tap: Vec<f32>,
    /// Owned by the audio thread, its only writer.
    analyser: Analyser,
    /// In dB, read under the same lock so it needs no copy of its own.
    spectrum: Vec<f32>,
    starved: bool,
    last_analysis: Option<Instant>,
    shuffle: bool,
    repeat: bool,
    error: Option<String>,
    pending: u32,
    last_persisted: Option<Instant>,
    random: u64,
    /// A position the UI asked for, with the generation it was asked in. The
    /// worker performs it because a compressed-track seek means re-decoding; the
    /// generation distinguishes a done seek from one queued behind it.
    seek: Option<(u64, f64)>,
    generation: u64,
}

#[derive(Debug)]
enum Command {
    Select {
        index: usize,
        autoplay: bool,
    },
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

    /// How full counts as "enough" (the level the callback wakes the worker at),
    /// and the most the ring may hold (the point the worker stops refilling).
    fn ring_bounds(&self) -> (usize, usize) {
        let per_second = (self.sample_rate * self.device_channels as f64).max(1.0);
        let capacity = (per_second * RING_SECONDS) as usize;
        let low = (capacity as f64 * RING_LOW_WATER) as usize;
        (low, capacity)
    }

    /// `reader_position` is ahead of the playhead by whatever is still buffered;
    /// the UI's position is the difference, so the progress bar does not run
    /// ahead of the sound.
    fn push(&mut self, samples: &[f32], reader_position: f64) {
        if let Some(last) = samples.last() {
            self.tail = *last;
        }
        self.ring.extend(samples);

        let channels = self.device_channels.max(1);
        let buffered = self.ring.len() as f64 / (self.sample_rate * channels as f64);
        let position = (reader_position - buffered).max(0.0);
        self.cursor = if self.duration > 0.0 {
            position.min(self.duration)
        } else {
            position
        };
    }

    /// The mean of the channel samples the device has just been given, after the
    /// gain. The analyser must look at what is playing, not at the up-to
    /// [`RING_SECONDS`] of future the worker has read ahead into the ring.
    fn tap_out(&mut self, out: &[f32], fft_size: usize) {
        let channels = self.device_channels.max(1);
        let frames = out.len() / channels;
        // Reserved once per size change rather than per buffer, so the common
        // path cannot allocate on the audio thread.
        if self.tap.capacity() < fft_size * 2 {
            self.tap.reserve(fft_size * 2 - self.tap.len());
        }
        self.tap.reserve(frames);
        for frame in 0..frames {
            let sum: f32 = out[frame * channels..(frame + 1) * channels].iter().sum();
            self.tap.push(sum / channels as f32);
        }
        if self.tap.len() > fft_size * 2 {
            let excess = self.tap.len() - fft_size * 2;
            self.tap.drain(..excess);
        }
    }

    fn flush(&mut self) {
        self.ring.clear();
        self.tap.clear();
        self.tail = 0.0;
    }

    /// The state is read here, under the lock, and written by the caller after
    /// releasing it: a file write is slow enough to miss a callback.
    fn pending_session(&self) -> Option<SavedTrack> {
        let index = self.current_index?;
        let track = self.tracks.get(index)?;
        Some(SavedTrack {
            path: track.path.clone(),
            current_time: self.cursor,
        })
    }

    fn mark_persisted(&mut self) {
        self.last_persisted = Some(Instant::now());
    }

    fn next_random(&mut self) -> f64 {
        // xorshift64*, seeded from the clock. The shuffle pick only needs a draw
        // that is not always the same, which is not worth a dependency for.
        let mut state = self.random;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.random = state;
        (state >> 11) as f64 / (1u64 << 53) as f64
    }

    fn pause(&mut self) {
        self.playing = false;
    }
}

/// Split out of [`Player::new`] so the tests can have a graph without an output
/// device.
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
/// The read and the mark share *one* guard, released before the caller touches
/// the filesystem. Taking the lock a second time while the first guard is alive
/// deadlocks, because `std::sync::Mutex` is not reentrant; writing it as one
/// `if let` over the lock would hold the guard through the body.
fn take_session(graph: &Graph) -> Option<SavedTrack> {
    let mut state = lock(&graph.state);
    let save = state.pending_session();
    if save.is_some() {
        state.mark_persisted();
    }
    save
}

/// Writes the position out, holding no lock while the file is touched.
fn save_position(graph: &Graph) {
    if let Some(save) = take_session(graph) {
        session::save(&save);
    }
}

/// Every clone is another handle onto the same graph, worker and output device.
/// It is deliberately not `Send` — the device stream is not — so it belongs on
/// the thread that owns the UI.
#[derive(Clone)]
pub struct Player {
    graph: Arc<Graph>,
    commands: Sender<Command>,
    /// Held so the device keeps being pulled: cpal stops a stream the moment it
    /// is dropped, and it has to outlive the worker and the callback.
    _output: Rc<cpal::Stream>,
}

impl Player {
    /// The device is opened once, up front, and left running; the stream is
    /// silent whenever nothing plays.
    pub fn new() -> Result<Player> {
        let host = cpal::default_host();
        // The app's own line ("background music is unavailable") names neither
        // the device, its rate nor its format, which is what decides whether a
        // particular audio setup can work at all.
        let device = host.default_output_device().ok_or_else(|| {
            log::error!("no default audio output device was found");
            Error::NoOutputDevice
        })?;
        let config = device.default_output_config().map_err(|error| {
            log::error!(
                "the audio device {} would not give its configuration: {error}",
                device.name().unwrap_or_else(|_| "<unnamed>".to_string())
            );
            Error::Output(error.to_string())
        })?;
        let device_rate = config.sample_rate().0 as f64;
        let device_channels = config.channels() as usize;
        log::info!(
            "Audio output: {} at {} Hz, {} channel(s), format {:?}",
            device.name().unwrap_or_else(|_| "<unnamed>".to_string()),
            config.sample_rate().0,
            device_channels,
            config.sample_format()
        );

        let (commands, receiver) = channel::<Command>();
        let graph = new_graph(device_rate, device_channels);

        let stream = build_stream(&device, &config, Arc::clone(&graph), device_channels)?;

        let worker = Arc::clone(&graph);
        // Returned as an error rather than panicking: a worker that could not
        // start is a silent background feature, not a reason to take the app down.
        // The handle is detached; the worker ends when its command channel closes,
        // and it holds the audio graph.
        std::thread::Builder::new()
            .name("conic-music".into())
            .spawn(move || worker_loop(worker, receiver))
            .map_err(|error| {
                log::error!("the music worker could not be started: {error}");
                Error::Output(error.to_string())
            })?;

        Ok(Player {
            graph,
            commands,
            _output: Rc::new(stream),
        })
    }

    pub fn tracks(&self) -> Arc<Vec<MusicFile>> {
        Arc::clone(&lock(&self.graph.state).tracks)
    }

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

    pub fn current_index(&self) -> Option<usize> {
        lock(&self.graph.state).current_index
    }

    pub fn current_time(&self) -> f64 {
        lock(&self.graph.state).cursor
    }

    pub fn is_playing(&self) -> bool {
        lock(&self.graph.state).playing
    }

    pub fn play_index(&self, index: usize) {
        self.select(index, true);
    }

    pub fn prepare_index(&self, index: usize) {
        self.select(index, false);
    }

    /// The playlist row is marked selected straight away and the transport
    /// pending, so the UI reflects the click before the file has been opened.
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

    /// With nothing selected the first track starts, otherwise this is the
    /// play/pause switch.
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

    pub fn resume(&self) {
        let (loaded, index) = {
            let state = lock(&self.graph.state);
            (state.loaded, state.current_index)
        };
        if !loaded {
            // The track is selected but not open, which is the case right after
            // a startup that restored no source.
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

    pub fn pause(&self) {
        {
            let mut state = lock(&self.graph.state);
            state.pause();
            state.starved = false;
        }
        save_position(&self.graph);
    }

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

    /// Past the first three seconds this restarts the track from the top,
    /// otherwise it steps back to the previous one.
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

    /// Only records the position; the worker performs it, because a compressed
    /// track means re-decoding.
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

    pub fn seek_ratio(&self, ratio: f64) {
        let duration = lock(&self.graph.state).duration;
        self.seek(ratio * duration);
    }

    pub fn toggle_shuffle(&self) {
        let mut state = lock(&self.graph.state);
        state.shuffle = !state.shuffle;
    }

    pub fn cycle_repeat(&self) {
        let mut state = lock(&self.graph.state);
        state.repeat = !state.repeat;
    }

    /// Sets the target volume; `smooth` ramps it over `VOLUME_RAMP` instead of
    /// jumping.
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

    /// Sets the `fftSize` the analyser transforms, as the visualiser asks when
    /// its bar count changes. Takes effect on the next analysis.
    pub fn set_fft_size(&self, size: usize) {
        self.graph.fft_size.store(size.max(32), Ordering::Relaxed);
    }

    /// The analyser's frequency bins in dB, already smoothed. Handed over under
    /// the graph's lock so no copy is made sixty times a second.
    pub fn with_spectrum(&self, action: impl FnOnce(&[f32])) {
        let state = lock(&self.graph.state);
        action(&state.spectrum);
    }

    /// The visualiser maps its frequency range against this.
    pub fn sample_rate(&self) -> f64 {
        lock(&self.graph.state).sample_rate
    }

    /// Writes the position at most every [`PERSIST_INTERVAL`], then hands back
    /// the transport state so the caller has one read instead of six.
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

    /// A saved track missing from the folder falls back to the first one,
    /// paused.
    pub fn restore_session(&self, enabled: bool, resume_on_startup: bool) {
        // A missing music folder is ordinary and reads as an empty playlist; one
        // that exists and cannot be read is not, and is logged.
        let tracks = Arc::new(crate::list_music_files().unwrap_or_else(|error| {
            log::warn!(
                "the music folder at {} could not be listed: {error}",
                storage::LOCATIONS.launcher.music.display()
            );
            Vec::new()
        }));
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

/// A track other than `current`, or 0 when the playlist holds a single entry.
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

/// Feed whatever is open; when that stops, do whatever was asked. A track that
/// ends hands straight to [`advance`], which leaves another track open, so
/// feeding resumes without a command; when nothing is open the loop blocks.
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
                    apply_command(&graph, &mut source, &mut resampler, command);
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
        apply_command(&graph, &mut source, &mut resampler, command);
    }
}

fn apply_command(
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

/// Folds a burst into the one that matters, the last: five presses of "next" are
/// one selection, not five files opened in turn.
fn coalesce(mut command: Command, commands: &Receiver<Command>) -> Command {
    while let Ok(newer) = commands.try_recv() {
        command = newer;
    }
    command
}

/// The whole cost of a track change: read the container header, build a decoder,
/// publish the duration. [`feed`] decodes the rest as the device asks.
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
            // show — but nothing plays.
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
        // A selection that starts playback persists the position; one that only
        // prepares the track does not.
        save_position(graph);
    }
}

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

/// Keeps the ring filled for as long as the track lasts.
///
/// It must never block without a way out: it waits on [`Graph::signal`], raised
/// both when the device drains the buffer and when [`Player::send`] queues a
/// command, so a "next" pressed while the worker sleeps for room still gets
/// through.
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

        // A seek the UI asked for while this track was playing. The request is taken
        // out into a local first, so the guard is released before `service_seek` takes
        // the same lock again — a non-reentrant mutex would deadlock the worker.
        let request = lock(&graph.state).seek.take();
        if let Some((generation, seconds)) = request {
            service_seek(graph, track, resampler, generation, seconds);
            draining = false;
            stalled = 0;
        }

        // The reader is done; the audio it produced is still in the ring, so the
        // transport is not told the track ended until the device has actually
        // played it.
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
                    // Abandoning the track is a real outcome — a corrupt file or
                    // a codec the device refuses — and it was indistinguishable
                    // from reaching the end of a good one.
                    log::warn!(
                        "{MAX_STALLED_READS} consecutive packets could not be decoded; giving \
                         up on the current track"
                    );
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
                // The device rejected this rate or channel layout repeatedly: the
                // resampler is producing something the output cannot take, which
                // is a different problem from a file that will not decode.
                log::warn!(
                    "{MAX_STALLED_READS} consecutive reads produced nothing the device \
                     would accept; giving up on the current track"
                );
                let mut state = lock(&graph.state);
                state.ended = true;
                state.playing = false;
                return Fed::Ended;
            }
            continue;
        }
        stalled = 0;

        let position = track.position();
        lock(&graph.state).push(resampled, position);
    }
}

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

fn track_path(graph: &Graph) -> String {
    let state = lock(&graph.state);
    state
        .current_index
        .and_then(|index| state.tracks.get(index))
        .map_or_else(String::new, |track| track.path.clone())
}

/// Repeat plays the same track again, otherwise the playlist advances — shuffled
/// when the flag is on.
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

/// Converts a track's rate and channel count into the device's, so the callback
/// only has to copy and scale.
///
/// The interpolation carries a frame across calls: a block boundary is not a
/// sample boundary, and losing the frame a read interpolates *from* would put a
/// step in the output whenever the buffer is refilled.
#[derive(Default)]
struct Resampler {
    /// Source-rate frames not yet consumed, oldest first. It always keeps the
    /// frame the next read interpolates from.
    tail: Vec<f32>,
    index: f64,
    source_channels: usize,
}

impl Resampler {
    fn reset(&mut self) {
        self.tail.clear();
        self.index = 0.0;
        self.source_channels = 0;
    }

    /// Appends interleaved `i16` samples in the source's format to `out` as
    /// interleaved `f32` in `-1.0..=1.0` at the device's rate and channel count.
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
    /// Scratch for devices that ask for integer samples: one allocation for the
    /// life of the thread, grown to the largest buffer requested and never
    /// shrunk. cpal gives the callback no scratch of its own.
    static CONVERSION: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

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
/// It allocates nothing, reads no file and takes the lock once. The cursor walk,
/// resampling and interpolation belong to the worker: a callback that misses its
/// deadline clicks.
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

    // The analyser's input, taken from what the device has just been given. It
    // has to be here and not on the worker: the ring holds up to `RING_SECONDS`
    // of audio the device has not played yet, and a spectrum taken from the
    // leading edge of that describes the wrong moment — and, since the worker
    // refills in one go after the device has drained half of it, arrives in
    // steps rather than as the music does.
    state.tap_out(out, fft_size);

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

/// A clock-seeded xorshift state for the shuffle pick.
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
        // Read the state under the graph's lock, then take the same lock again. A
        // `std::sync::Mutex` is not reentrant, so a guard that outlives its statement
        // waits forever on a thread holding it — hence the reading and the writing
        // being separate functions.
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
    fn the_analyser_is_fed_what_is_played_not_what_is_queued() {
        // The worker fills the ring, and the ring holds up to `RING_SECONDS` of audio
        // the device has not played yet: an analyser input taken from there describes
        // the leading edge of the queue, in the worker's refill steps. Only the
        // callback knows what is actually playing.
        let graph = new_graph(48_000.0, 2);
        {
            let mut state = lock(&graph.state);
            state.push(&vec![0.25; 4_096], 0.0);
            assert!(
                state.tap.is_empty(),
                "the worker fed the analyser from the queue"
            );
            assert_eq!(state.ring.len(), 4_096, "…but it did fill the ring");
        }

        // The callback is the one that moves it, and it moves it by what it was
        // given. The input is interleaved, so 2048 samples of stereo is 1024
        // frames of the mono the transform is given.
        lock(&graph.state).tap_out(&vec![1.0; 2_048], 1_024);
        assert_eq!(lock(&graph.state).tap.len(), 1_024);
        assert!(lock(&graph.state).tap.iter().all(|s| *s == 1.0));
    }

    #[test]
    fn the_analysers_input_stays_bounded_to_two_transforms() {
        // The callback runs forever, so the tap is trimmed to the two most recent
        // transforms' worth: unbounded, it would grow for as long as the music
        // played.
        let graph = new_graph(48_000.0, 2);
        for _ in 0..64 {
            lock(&graph.state).tap_out(&vec![0.5; 512], 1_024);
        }
        let tap = lock(&graph.state);
        assert_eq!(tap.tap.len(), 2_048, "the tap grew past the trim");
    }

    #[test]
    fn a_starved_ring_reports_buffering_rather_than_playing() {
        // What the panel shows: the transport waiting on audio, either because a
        // selection is being opened or because the device has run the buffer dry.
        let (graph, _) = graph_with_a_track();
        assert!(!lock(&graph.state).starved);
        lock(&graph.state).starved = true;
        assert!(lock(&graph.state).starved, "the flag did not take");
    }
}
