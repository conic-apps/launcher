// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The registry of launched Minecraft sessions: each is recorded, reaped when
//! its watcher sees EOF, and exposed for termination plus a [`SessionEvent`]
//! stream through [`subscribe`]. An instance may have several live processes, so
//! each id maps to a list and a per-launch token identifies which entry a watcher
//! must drop (a pid can be reused).
//!
//! The tracked pid is not the game on every platform: the Unix launch script
//! `exec`s java, so the pid is the game itself, while on Windows the pid is the
//! `cmd.exe` running the launch `.bat` and the game is its child — see
//! `kill_tree`. Ending only the tracked process there would not merely leave the
//! game running: the child holds the output pipes, so the watcher would never see
//! EOF and the session would stay in the registry for good.

use std::{
    collections::{HashMap, VecDeque},
    io::{BufRead, BufReader, Read},
    process::{Child, ChildStderr, ChildStdout, ExitStatus},
    sync::{
        Arc, LazyLock, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use log::{debug, error, info, warn};
use shared::Sink;

use instance::Instance;

use crate::crash::{CrashReport, LaunchInfo};

/// Every session currently running, keyed by instance id. An instance can have
/// more than one live process, so each key holds a list.
static RUNNING: LazyLock<Mutex<HashMap<String, Vec<Session>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The app's subscriber, installed by [`subscribe`]. At most one: the app is a
/// single consumer that fans the events out to its own surfaces.
static SUBSCRIBER: LazyLock<Mutex<Option<SessionSink>>> = LazyLock::new(|| Mutex::new(None));

/// Hands each session a token no other session (past or future) has, so a
/// watcher can drop exactly its own entry and not one that reused its pid.
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

/// What a launch session reports to the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    Started(RunningInstance),
    Exited {
        instance: RunningInstance,
        /// Whether the exit was abnormal: it was not a success and the app did
        /// not ask for it.
        abnormal: bool,
        crash: Option<Box<CrashReport>>,
    },
}

pub type SessionSink = Sink<SessionEvent>;

/// The child's output pipes, taken before the child is wrapped so the watcher
/// can own them and read them to EOF without holding the process lock.
struct Pipes {
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
}

struct CrashContext {
    instance: Instance,
    launch: LaunchInfo,
}

/// One launched, still-running Minecraft session.
struct Session {
    /// Identifies this launch within its instance's list; see [`NEXT_TOKEN`].
    token: u64,
    pid: u32,
    /// The instance the process belongs to, kept for the crash analysis.
    instance: Instance,
    /// The process the launcher spawned, which on Windows is the shell running
    /// the launch script rather than the game. Shared with the stdout watcher,
    /// which reaps it on exit, while [`terminate`] only takes the lock long
    /// enough to read whether it is still alive.
    process: Arc<Mutex<Child>>,
    /// Set by [`terminate`]/[`terminate_all`]/[`shutdown`] before the signal, so
    /// the watcher can tell a stop the app asked for from a crash.
    stop_requested: Arc<AtomicBool>,
    /// The stdout watcher thread, kept so [`shutdown`] can wait for it. It is
    /// `None` only for the instant between the entry going in and the thread
    /// starting; see [`register`].
    watcher: Option<JoinHandle<()>>,
}

/// A running session as the rest of the app sees it: no handle, only enough to
/// name it and to ask [`terminate`] to stop it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningInstance {
    /// The instance's id, the key [`terminate_all`] and [`is_running`] take.
    pub id: String,
    pub name: String,
    /// The operating-system process id, unique among the live sessions.
    pub pid: u32,
}

/// Installs the app's subscriber and returns what is running right now.
///
/// The snapshot is what lets the app seed its view without a race: everything
/// live at subscription time is in the list, and every later change is a
/// [`SessionEvent`]. A session that slips between the install and the snapshot
/// can only make the app set the same entry twice, which is harmless.
pub fn subscribe(sink: SessionSink) -> Vec<RunningInstance> {
    *SUBSCRIBER.lock().unwrap_or_else(PoisonError::into_inner) = Some(sink);
    running_instances()
}

/// A snapshot of every running session, ordered by name and then process id.
///
/// An instance launched twice appears twice, once per process.
pub fn running_instances() -> Vec<RunningInstance> {
    let mut running: Vec<RunningInstance> = registry()
        .values()
        .flatten()
        .map(|session| RunningInstance {
            id: session.instance.id.clone(),
            name: session.instance.config.name.clone(),
            pid: session.pid,
        })
        .collect();
    running.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then_with(|| a.id.cmp(&b.id))
            .then_with(|| a.pid.cmp(&b.pid))
    });
    running
}

pub fn is_running(id: &str) -> bool {
    registry()
        .get(id)
        .is_some_and(|sessions| !sessions.is_empty())
}

/// Asks the session with process id `pid` to stop.
///
/// Returns whether it was found. The stop takes the session's whole process tree
/// with it (see `signal`), because the game is not always the tracked process.
/// The watcher observes the exit, reaps the process and removes its entry, so the
/// session disappears from [`running_instances`] on its own once the processes
/// are really gone.
pub fn terminate(pid: u32) -> bool {
    let found = registry()
        .values()
        .flatten()
        .find(|session| session.pid == pid)
        .map(|session| {
            (
                session.pid,
                Arc::clone(&session.process),
                Arc::clone(&session.stop_requested),
            )
        });
    match found {
        Some((target, process, stop)) => {
            stop.store(true, Ordering::SeqCst);
            signal(target, &process, &format!("(pid {target})"));
            true
        }
        None => false,
    }
}

/// Asks every process of instance `id` to stop, however many it launched.
///
/// Returns whether any session was found. Each stop takes a whole process tree
/// with it (see `signal`), and each watcher observes its exit, reaps its process
/// and removes its entry, so the sessions disappear from [`running_instances`] on
/// their own once the processes are really gone.
pub fn terminate_all(id: &str) -> bool {
    let sessions: Vec<(u32, Arc<Mutex<Child>>, Arc<AtomicBool>)> = match registry().get(id) {
        Some(sessions) => sessions
            .iter()
            .map(|session| {
                (
                    session.pid,
                    Arc::clone(&session.process),
                    Arc::clone(&session.stop_requested),
                )
            })
            .collect(),
        None => return false,
    };
    if sessions.is_empty() {
        return false;
    }
    for (pid, process, stop) in &sessions {
        stop.store(true, Ordering::SeqCst);
        signal(*pid, process, &format!("for instance {id}"));
    }
    true
}

/// Stops one session, including the processes under it where the platform needs
/// that (see `kill_tree`).
///
/// A stop that finds nothing to stop is not an error: a process that exited
/// between the lookup and this call is the common case, and the watcher clears the
/// entry either way. That is decided here, from the process itself, rather than
/// read out of a failed kill — a `Child` whose exit was already collected refuses
/// to be killed, which would report a miss that never happened. The lock covers
/// only that read, never the platform call, so the watcher keeps polling the exit
/// in the meantime.
fn signal(pid: u32, process: &Arc<Mutex<Child>>, what: &str) {
    let polled = process
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .try_wait();
    match polled {
        Ok(Some(status)) => {
            debug!("The Minecraft process {what} had already exited ({status})");
            return;
        }
        Ok(None) => {}
        Err(error) => {
            // A status that cannot be read is not a reason to leave it running.
            debug!("Could not read the exit status of the Minecraft process {what}: {error}");
        }
    }

    if kill_tree(pid, what) {
        return;
    }
    match process
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .kill()
    {
        Ok(()) => info!("Asked the Minecraft process {what} to stop"),
        Err(error) => warn!("Could not terminate the Minecraft process {what}: {error}"),
    }
}

/// Kills `pid` and every process it started, and reports whether that left
/// nothing for the caller to do.
///
/// The launch `.bat` makes the game a child of the tracked `cmd.exe`, and a
/// single-process kill orphans children, so the game would survive a stop and go
/// on holding the output pipes — the reason a session stopped that way never
/// leaves the registry. `taskkill /T` walks the parent chain from `pid` instead;
/// a job object per session would be the alternative and is not worth a handle to
/// keep alive for the one case this covers.
///
/// Known gap: if `cmd.exe` is gone before the stop, the chain it led is gone with
/// it and nothing is left to walk. Wrapper processes that detach themselves
/// (`start`) are outside the chain for the same reason.
#[cfg(target_os = "windows")]
fn kill_tree(pid: u32, what: &str) -> bool {
    use std::{os::windows::process::CommandExt, process::Stdio};

    // `CREATE_NO_WINDOW`: this is a launcher, not a terminal, so the console
    // `taskkill` would otherwise be given must not flash for the moment it runs.
    let mut taskkill = std::process::Command::new("taskkill");
    taskkill
        .args(["/F", "/T", "/PID"])
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000);
    match taskkill.status() {
        Ok(status) if status.success() => {
            info!("Asked the process tree of the Minecraft process {what} to stop");
            true
        }
        Ok(status) => {
            // Most likely "no such process": it exited on its own between the
            // lookup and the stop. Left at debug because the caller still signals
            // the direct child, and that is what reports a real failure.
            debug!("taskkill could not stop the process tree of {what}: {status}");
            false
        }
        Err(error) => {
            warn!("Could not run taskkill for the Minecraft process {what}: {error}");
            false
        }
    }
}

/// Unix has nothing between the launcher and the game to kill: the launch script
/// `exec`s java, so the tracked pid is the game and signalling it is the whole
/// stop.
#[cfg(not(target_os = "windows"))]
fn kill_tree(_pid: u32, _what: &str) -> bool {
    false
}

/// Terminates every session and waits for its watcher, for shutdown.
///
/// Killing before joining avoids waiting on a process that is still running;
/// polling for the exit in `wait` keeps the lock free, so the kill can land.
pub fn shutdown() {
    let sessions: Vec<Session> = registry()
        .drain()
        .flat_map(|(_, sessions)| sessions)
        .collect();
    for session in &sessions {
        session.stop_requested.store(true, Ordering::SeqCst);
        signal(
            session.pid,
            &session.process,
            &format!("for instance {}", session.instance.id),
        );
    }
    for session in sessions {
        if let Some(watcher) = session.watcher {
            info!(
                "Waiting for the watcher of instance {} to finish",
                session.instance.id
            );
            let _ = watcher.join();
        }
    }
}

/// Registers a freshly spawned process and starts watching its output.
///
/// `process` is the spawned child; its stdout and stderr pipes are taken here,
/// so the process must have been spawned with both piped. `on_line` sees every
/// trimmed stdout line while the game runs. This module only decides *when* the
/// process is gone and *how* it went; what the lines mean stays in the launch
/// pipeline.
///
/// It is `pub(crate)`: only the launch pipeline spawns processes to track.
/// The launched process's own exit, for a launch that is still waiting for its
/// first startup marker.
///
/// The session watcher reports the exit only after both output pipes reach EOF;
/// this asks the process itself. The status it collects stays cached in the
/// `Child`, so the watcher's own wait still reads it.
pub struct ExitWatch {
    process: Arc<Mutex<Child>>,
}

impl ExitWatch {
    /// Whether the process has exited. A status that cannot be read counts as an
    /// exit: a process we cannot call running is not one to keep waiting on.
    pub fn has_exited(&self) -> bool {
        let mut child = self.process.lock().unwrap_or_else(PoisonError::into_inner);
        match child.try_wait() {
            Ok(status) => status.is_some(),
            Err(error) => {
                warn!("Could not read the exit status of a launching process: {error}");
                true
            }
        }
    }
}

pub(crate) fn register(
    instance: Instance,
    launch: LaunchInfo,
    mut process: Child,
    on_line: impl FnMut(&str) + Send + 'static,
) -> ExitWatch {
    let pipes = Pipes {
        stdout: process.stdout.take(),
        stderr: process.stderr.take(),
    };
    let id = instance.id.clone();
    let name = instance.config.name.clone();
    let token = NEXT_TOKEN.fetch_add(1, Ordering::Relaxed);
    let pid = process.id();
    let process = Arc::new(Mutex::new(process));
    let watch_process = Arc::clone(&process);
    let stop_requested = Arc::new(AtomicBool::new(false));
    let watch_context = CrashContext {
        instance: instance.clone(),
        launch,
    };
    let watch_stop = Arc::clone(&stop_requested);
    // The entry goes in before the watcher starts: a process that exits at once
    // must find its own entry to clean up, rather than race the insert and
    // leave a stale one behind.
    registry().entry(id.clone()).or_default().push(Session {
        token,
        pid,
        instance,
        process: Arc::clone(&process),
        stop_requested,
        watcher: None,
    });
    log::info!("Registered the Minecraft process {pid} for instance {id} ({name})");
    emit(SessionEvent::Started(RunningInstance {
        id: id.clone(),
        name,
        pid,
    }));
    let watcher_id = id.clone();
    let watcher = thread::spawn(move || {
        watch(
            watcher_id,
            token,
            watch_context,
            watch_stop,
            process,
            pipes,
            on_line,
        )
    });
    // The watcher may already have exited and removed the entry; storing into
    // nothing is fine, the thread is done either way.
    if let Some(session) = registry()
        .get_mut(&id)
        .and_then(|sessions| sessions.iter_mut().find(|session| session.token == token))
    {
        session.watcher = Some(watcher);
    } else {
        // The process exited before the handle could be stored, so the watcher
        // thread is detached rather than joined at shutdown. Harmless, but it is
        // also the shape of "the game died instantly".
        log::debug!(
            "The watcher for instance {id} had nowhere to be stored; the process had \
             already been reaped"
        );
    }
    ExitWatch {
        process: watch_process,
    }
}

/// Reads the process's output to the end, reaps it, classifies the exit,
/// collects any crash and forgets the session.
///
/// stdout and stderr are pumped in separate threads: reading them in sequence
/// can deadlock when the game fills one pipe while we block on the other, which
/// a mod that logs heavily to stderr does easily (this is what HMCL's
/// `StreamPump` pair and `ExitWaiter` do). Both threads end at EOF, once the
/// `exec`ed java behind the script has closed its streams, and only then is the
/// exit reaped so the captured console is complete.
fn watch(
    id: String,
    token: u64,
    context: CrashContext,
    stop_requested: Arc<AtomicBool>,
    process: Arc<Mutex<Child>>,
    pipes: Pipes,
    on_line: impl FnMut(&str) + Send + 'static,
) {
    let CrashContext { instance, launch } = context;
    let Pipes { stdout, stderr } = pipes;
    let pid = process.lock().unwrap_or_else(PoisonError::into_inner).id();
    let console = Arc::new(Mutex::new(VecDeque::new()));
    let stdout_console = Arc::clone(&console);
    let stdout_thread =
        stdout.map(|stdout| thread::spawn(move || pump(stdout, &stdout_console, on_line)));
    let stderr_console = Arc::clone(&console);
    let stderr_thread =
        stderr.map(|stderr| thread::spawn(move || pump(stderr, &stderr_console, |_| {})));
    if let Some(thread) = stdout_thread {
        let _ = thread.join();
    }
    if let Some(thread) = stderr_thread {
        let _ = thread.join();
    }
    let console = {
        let console = console.lock().unwrap_or_else(PoisonError::into_inner);
        console.iter().cloned().collect::<Vec<_>>().join("\n")
    };

    let status = wait(&process);
    let exit_code = status.as_ref().and_then(ExitStatus::code);
    let signal = status.as_ref().and_then(exit_signal);
    let stopped = stop_requested.load(Ordering::SeqCst);
    let exit_type = crate::crash::classify_exit(exit_code, signal, stopped, &console);
    // A clean exit, or a stop we asked for, is not a crash. Everything else is,
    // including an exit whose status we could not read: silence is not success.
    let abnormal = !stopped && exit_type != crate::crash::ExitType::Normal;
    let crash = if abnormal {
        if let Err(error) = instance::mark_last_exit_abnormal(&id) {
            warn!("could not mark instance {id} as crashed: {error}");
        }
        Some(Box::new(crate::crash::analyze(
            &instance, exit_type, exit_code, &console, &launch,
        )))
    } else {
        if let Err(error) = instance::clear_last_exit_abnormal(&id) {
            warn!("could not clear the crash marker of instance {id}: {error}");
        }
        None
    };

    match status.as_ref() {
        Some(status) if stopped => {
            info!("Minecraft process for instance {id} was stopped ({status})");
        }
        Some(status) if !abnormal => {
            info!("Minecraft process for instance {id} exited with {status}");
        }
        Some(status) => {
            error!(
                "Minecraft process for instance {id} exited with {status} ({})",
                exit_type.as_str()
            );
        }
        None => error!("Could not read the exit status of the Minecraft process for {id}"),
    }

    unregister(&id, token);
    emit(SessionEvent::Exited {
        instance: RunningInstance {
            id,
            name: instance.config.name,
            pid,
        },
        abnormal,
        crash,
    });
}

/// Reads `reader` a line at a time, keeping each in `console` and handing it to
/// `on_line`.
///
/// Lines are split on `\n` and decoded lossily, so a stray byte a mod wrote to
/// the console cannot stop the capture. The console buffer is bounded by
/// [`CONSOLE_LINE_LIMIT`] so a runaway logger cannot grow it without limit; a
/// crash's evidence is near the end, so dropping the oldest lines is safe.
fn pump(reader: impl Read, console: &Arc<Mutex<VecDeque<String>>>, mut on_line: impl FnMut(&str)) {
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                let text = String::from_utf8_lossy(&buffer);
                let line = text.trim_end_matches(['\r', '\n']);
                push_line(console, line);
                on_line(line);
            }
            Err(error) => {
                // Distinguished from EOF on purpose: a truncated read is exactly
                // when the tail of the output matters most.
                log::debug!("The game's output stream ended with an error: {error}");
                break;
            }
        }
    }
}

/// How many console lines are kept for crash analysis.
const CONSOLE_LINE_LIMIT: usize = 20_000;

fn push_line(console: &Arc<Mutex<VecDeque<String>>>, line: &str) {
    let mut console = console.lock().unwrap_or_else(PoisonError::into_inner);
    if console.len() == CONSOLE_LINE_LIMIT {
        console.pop_front();
    }
    console.push_back(line.to_string());
}

/// The signal that killed the process, on platforms that have signals.
#[cfg(unix)]
fn exit_signal(status: &ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

/// Windows reports a signal death through the exit code, not a signal.
#[cfg(not(unix))]
fn exit_signal(_status: &ExitStatus) -> Option<i32> {
    None
}

/// Waits for the process to exit without holding its lock across the wait, so
/// [`terminate`] can still signal it.
fn wait(process: &Arc<Mutex<Child>>) -> Option<ExitStatus> {
    loop {
        {
            let mut child = process.lock().unwrap_or_else(PoisonError::into_inner);
            match child.try_wait() {
                Ok(Some(status)) => return Some(status),
                Ok(None) => {}
                Err(error) => {
                    error!("Could not wait for the Minecraft process: {error}");
                    return None;
                }
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn unregister(id: &str, token: u64) -> bool {
    let mut registry = registry();
    let Some(sessions) = registry.get_mut(id) else {
        return false;
    };
    let before = sessions.len();
    sessions.retain(|session| session.token != token);
    let removed = sessions.len() != before;
    if sessions.is_empty() {
        registry.remove(id);
    }
    removed
}

/// Reports an event to the app, if one subscribed.
///
/// The subscriber is called with no registry lock held — a subscriber that
/// then calls [`running_instances`] would otherwise deadlock — and a panicking
/// one is caught, so it cannot take the watcher thread down before it reaps its
/// process.
fn emit(event: SessionEvent) {
    let sink = SUBSCRIBER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let Some(sink) = sink else {
        return;
    };
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sink(event))).is_err() {
        warn!("a running-session subscriber panicked");
    }
}

/// The registry, with poisoning ignored: a panic elsewhere must not cost the
/// mapping of what is running.
fn registry() -> MutexGuard<'static, HashMap<String, Vec<Session>>> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::{
        process::{Command, Stdio},
        sync::mpsc,
        time::Duration,
    };

    use super::*;

    /// [`SUBSCRIBER`] holds one sink for the whole process, so two tests that
    /// install one would replace each other's and each would then see the other
    /// test's events. Every test that calls [`subscribe`] takes this first.
    fn subscriber_lock() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[test]
    fn an_unknown_instance_is_not_running() {
        assert!(!is_running("no-such-instance"));
        assert!(!terminate_all("no-such-instance"));
        assert!(!terminate(u32::MAX));
    }

    #[test]
    fn a_clean_exit_is_not_a_crash() {
        let _lock = subscriber_lock();
        let child = if cfg!(windows) {
            Command::new("cmd")
                .args(["/C", "exit", "0"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        } else {
            Command::new("sh")
                .args(["-c", "exit 0"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        }
        .expect("a shell to spawn");
        let instance = Instance {
            id: "conic-exit-test".to_string(),
            ..Instance::default()
        };

        let (sender, receiver) = mpsc::channel();
        subscribe(Arc::new(move |event| {
            let _ = sender.send(event);
        }));
        register(instance, LaunchInfo::default(), child, |_| {});

        let exited = loop {
            let event = receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("an exit event");
            if let SessionEvent::Exited {
                abnormal, crash, ..
            } = event
            {
                break (abnormal, crash);
            }
        };
        assert!(!exited.0, "a clean exit was flagged as a crash");
        assert!(exited.1.is_none(), "a clean exit carried a crash report");
    }

    /// The shape the launcher actually spawns on Windows: a `cmd.exe` running the
    /// launch script, with the game as its child. The child is what holds the
    /// output pipes, so a stop that ends only the shell reports no exit at all and
    /// leaves the session in the registry — both assertions below are what a
    /// single-process kill fails.
    #[cfg(target_os = "windows")]
    #[test]
    fn stopping_a_session_takes_its_child_with_it() {
        let _lock = subscriber_lock();
        // A shell that starts a child and waits for it, like `cmd /c` on the
        // launch script. The child writes to the inherited stdout, which is what
        // keeps the pipes open after the shell is gone.
        let child = Command::new("cmd")
            .args(["/C", "ping", "-n", "60", "127.0.0.1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("a shell to spawn");
        let instance = Instance {
            id: "conic-tree-kill-test".to_string(),
            ..Instance::default()
        };

        let (sender, receiver) = mpsc::channel();
        subscribe(Arc::new(move |event| {
            let _ = sender.send(event);
        }));
        register(instance.clone(), LaunchInfo::default(), child, |_| {});
        let pid = running_instances()
            .into_iter()
            .find(|running| running.id == instance.id)
            .expect("the session to be registered")
            .pid;
        // Otherwise the shell could be stopped before it starts its child, which
        // would pass on the bug this test exists to catch.
        thread::sleep(Duration::from_secs(1));

        assert!(terminate(pid));

        let (abnormal, crash) = loop {
            let event = receiver
                .recv_timeout(Duration::from_secs(30))
                .expect("an exit event");
            if let SessionEvent::Exited {
                abnormal, crash, ..
            } = event
            {
                break (abnormal, crash);
            }
        };
        assert!(!abnormal, "a stop we asked for was flagged as a crash");
        assert!(
            crash.is_none(),
            "a stop we asked for carried a crash report"
        );
        assert!(
            !is_running(&instance.id),
            "the session stayed in the registry"
        );
    }
}
