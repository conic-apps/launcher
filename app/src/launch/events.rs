// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The install and launch progress events, applied to `LaunchState`.

use super::*;

/// A scalar snapshot of an install event, for change detection and logging.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum InstallKey {
    Prepare,
    Game(DownloadPhase, u64, u64),
    Java(DownloadPhase, u64, u64),
    LoaderPrepare,
    LoaderDownload {
        installer: bool,
        phase: DownloadPhase,
        completed: u64,
        total: u64,
    },
    LoaderRun(String),
}

impl InstallKey {
    /// A coarse label used to decide whether a log line is a stage change
    /// (always logged) or just another byte count (logged at `debug`).
    fn stage(&self) -> &'static str {
        match self {
            InstallKey::Prepare => "prepare",
            InstallKey::Game(..) => "game",
            InstallKey::Java(..) => "java",
            InstallKey::LoaderPrepare | InstallKey::LoaderDownload { .. } => "loader",
            InstallKey::LoaderRun(_) => "loader-run",
        }
    }
}

pub(crate) fn install_key(event: &InstallEvent) -> InstallKey {
    match event {
        InstallEvent::Prepare => InstallKey::Prepare,
        InstallEvent::InstallGame(state) => {
            let (phase, completed, total) = state_snapshot(state);
            InstallKey::Game(phase, completed, total)
        }
        InstallEvent::InstallJava(state) => {
            let (phase, completed, total) = state_snapshot(state);
            InstallKey::Java(phase, completed, total)
        }
        InstallEvent::InstallModLoader(progress) => match progress {
            ModLoaderProgress::Prepare => InstallKey::LoaderPrepare,
            ModLoaderProgress::DownloadInstaller(state) => {
                let (phase, completed, total) = state_snapshot(state);
                InstallKey::LoaderDownload {
                    installer: true,
                    phase,
                    completed,
                    total,
                }
            }
            ModLoaderProgress::PrefetchDependencies(state) => {
                let (phase, completed, total) = state_snapshot(state);
                InstallKey::LoaderDownload {
                    installer: false,
                    phase,
                    completed,
                    total,
                }
            }
            ModLoaderProgress::RunInstaller { message } => InstallKey::LoaderRun(message.clone()),
        },
    }
}

/// Reads the plain values out of a `DownloadState` (whose counters are shared).
pub(crate) fn state_snapshot(state: &DownloadState) -> (DownloadPhase, u64, u64) {
    (
        phase_of(state),
        state.completed_bytes.load(Ordering::SeqCst),
        state.total_bytes.load(Ordering::SeqCst),
    )
}

/// Applies the current install status when it has moved on, and logs it.
pub(crate) fn flush_install(
    weak: &Weak<App>,
    run: &Run,
    status: &Arc<Mutex<InstallEvent>>,
    last: &mut Option<InstallKey>,
    last_log: &mut Option<Instant>,
    loader: &str,
) {
    let Ok(event) = status.lock().map(|guard| guard.clone()) else {
        return;
    };
    let key = install_key(&event);
    if last.as_ref() == Some(&key) {
        return;
    }
    log_progress(
        "install",
        &key,
        key.stage(),
        last.as_ref().map(InstallKey::stage),
        last_log,
    );
    *last = Some(key);
    apply_install_event(weak, run, &event, loader);
}

/// The Vue's `installGame()` `onProgress`.
pub(crate) fn apply_install_event(weak: &Weak<App>, run: &Run, event: &InstallEvent, loader: &str) {
    match event {
        InstallEvent::Prepare => push(weak, run, |state| {
            state.set_progress_kind("prepare-download".into());
            state.set_progress_loading(true);
        }),
        InstallEvent::InstallGame(progress) => {
            apply_install_download(weak, run, progress, "verify-files", "download-files");
        }
        InstallEvent::InstallJava(progress) => {
            apply_install_download(weak, run, progress, "check-java", "download-java");
        }
        InstallEvent::InstallModLoader(progress) => match progress {
            ModLoaderProgress::Prepare => {
                let loader = loader.to_string();
                push(weak, run, move |state| {
                    state.set_progress_kind("install-mod-loader".into());
                    state.set_progress_name(loader.into());
                    state.set_progress_loading(true);
                });
            }
            ModLoaderProgress::DownloadInstaller(detail) => {
                apply_loader_download(weak, run, detail, loader, true);
            }
            ModLoaderProgress::PrefetchDependencies(detail) => {
                apply_loader_download(weak, run, detail, loader, false);
            }
            ModLoaderProgress::RunInstaller { message } => {
                let loader = loader.to_string();
                let message = message.clone();
                push(weak, run, move |state| {
                    state.set_progress_kind("run-installer".into());
                    state.set_progress_name(loader.into());
                    state.set_progress_message(message.into());
                    state.set_progress_loading(true);
                });
            }
        },
    }
}

/// The `InstallGame`/`InstallJava` branches: a `VerifyExistingFiles` phase (or a
/// zero total) shows the stage text with no bar; `DownloadFiles` shows the byte
/// counters.
pub(crate) fn apply_install_download(
    weak: &Weak<App>,
    run: &Run,
    progress: &DownloadState,
    verify_kind: &str,
    download_kind: &str,
) {
    let phase = phase_of(progress);
    let total = progress.total_bytes.load(Ordering::SeqCst);
    if phase == DownloadPhase::VerifyExistingFiles || total == 0 {
        let kind = verify_kind.to_string();
        push(weak, run, move |state| {
            state.set_progress_kind(kind.into());
            state.set_progress_loading(true);
        });
        return;
    }
    let kind = download_kind.to_string();
    let completed = progress.completed_bytes.load(Ordering::SeqCst);
    let current = format_bytes(completed);
    let total_text = format_bytes(total);
    push(weak, run, move |state| {
        state.set_progress_kind(kind.into());
        state.set_progress_current(current.into());
        state.set_progress_total(total_text.into());
        state.set_progress_loading(false);
        state.set_progress_value(completed as f32);
        state.set_progress_max(total as f32);
    });
}

/// The mod-loader download branches. `installer` selects between the
/// "Downloading {name} installer" / "Downloading dependencies" texts; a
/// `VerifyExistingFiles` phase (or a zero total) shows the plain text, otherwise
/// the byte counters are appended (which needs its own kind).
pub(crate) fn apply_loader_download(
    weak: &Weak<App>,
    run: &Run,
    detail: &DownloadState,
    loader: &str,
    installer: bool,
) {
    let phase = phase_of(detail);
    let total = detail.total_bytes.load(Ordering::SeqCst);
    let plain_kind = if installer {
        "download-installer"
    } else {
        "download-deps"
    };
    let progress_kind = if installer {
        "download-installer-progress"
    } else {
        "download-deps-progress"
    };
    let name = loader.to_string();
    if phase == DownloadPhase::VerifyExistingFiles || total == 0 {
        let kind = plain_kind.to_string();
        push(weak, run, move |state| {
            state.set_progress_kind(kind.into());
            state.set_progress_name(name.into());
            state.set_progress_loading(true);
        });
        return;
    }
    let kind = progress_kind.to_string();
    let completed = detail.completed_bytes.load(Ordering::SeqCst);
    let current = format_bytes(completed);
    let total_text = format_bytes(total);
    push(weak, run, move |state| {
        state.set_progress_kind(kind.into());
        state.set_progress_name(name.into());
        state.set_progress_current(current.into());
        state.set_progress_total(total_text.into());
        state.set_progress_loading(false);
        state.set_progress_value(completed as f32);
        state.set_progress_max(total as f32);
    });
}

/// The Vue's `launchGame()`.
pub(crate) async fn launch_game(
    weak: &Weak<App>,
    run: &Run,
    config: Config,
    instance: Instance,
) -> Result<(), launch::Error> {
    let status = Arc::new(Mutex::new(LaunchEvent::Prepare));
    let future = launch::launch(config, instance, Arc::clone(&status));
    tokio::pin!(future);
    let mut ticker = tokio::time::interval(Duration::from_millis(100));
    let mut last: Option<LaunchKey> = None;
    let mut last_log: Option<Instant> = None;
    // Whether one of the game's own startup markers has been seen. Latched per
    // run, and read by `flush_launch` before the event dedupe — see there.
    let mut game_up = false;
    loop {
        tokio::select! {
            result = &mut future => {
                flush_launch(weak, run, &status, &mut last, &mut last_log, &mut game_up);
                return result.map(|_pid| ());
            }
            _ = ticker.tick() => {
                flush_launch(weak, run, &status, &mut last, &mut last_log, &mut game_up);
            }
        }
    }
}

/// A scalar snapshot of a launch event, for change detection and logging (see
/// `InstallKey` for why the event itself cannot be compared).
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum LaunchKey {
    Prepare,
    CompleteFiles(DownloadPhase, u64, u64),
    GenerateScriptlet,
    /// `WaitForLaunch` and the three startup log markers share one screen state.
    Waiting,
    GameStarted,
    /// The authlib-injector step, which the Vue does not show either.
    Other,
}

impl LaunchKey {
    fn stage(&self) -> &'static str {
        match self {
            LaunchKey::Prepare => "prepare",
            LaunchKey::CompleteFiles(..) => "complete-files",
            LaunchKey::GenerateScriptlet => "generate-scriptlet",
            LaunchKey::Waiting => "waiting",
            LaunchKey::GameStarted => "game-started",
            LaunchKey::Other => "authlib",
        }
    }
}

pub(crate) fn launch_key(event: &LaunchEvent) -> LaunchKey {
    match event {
        LaunchEvent::Prepare => LaunchKey::Prepare,
        LaunchEvent::CompleteFiles(state) => {
            let (phase, completed, total) = state_snapshot(state);
            LaunchKey::CompleteFiles(phase, completed, total)
        }
        LaunchEvent::GenerateScriptlet => LaunchKey::GenerateScriptlet,
        LaunchEvent::WaitForLaunch
        | LaunchEvent::LogSettingUser
        | LaunchEvent::LogLwjglVersion
        | LaunchEvent::LogOpenALLoaded => LaunchKey::Waiting,
        LaunchEvent::LogTextureLoaded => LaunchKey::GameStarted,
        LaunchEvent::InstallAuthlibInjector(_) => LaunchKey::Other,
    }
}

/// Applies the current launch status when it has moved on, and logs it.
pub(crate) fn flush_launch(
    weak: &Weak<App>,
    run: &Run,
    status: &Arc<Mutex<LaunchEvent>>,
    last: &mut Option<LaunchKey>,
    last_log: &mut Option<Instant>,
    game_up: &mut bool,
) {
    let Ok(event) = status.lock().map(|guard| guard.clone()) else {
        return;
    };
    // "The game is up" is read here, off the raw event and *before* the dedupe
    // below, because the three startup markers collapse into one `LaunchKey`:
    // `WaitForLaunch` has usually arrived first, so the other two are dropped and
    // `apply_launch_event` never sees them at all. The set is the launch crate's
    // own — the three it breaks its twenty-second wait on.
    if !*game_up
        && matches!(
            event,
            LaunchEvent::LogLwjglVersion
                | LaunchEvent::LogOpenALLoaded
                | LaunchEvent::LogTextureLoaded
        )
    {
        *game_up = true;
        dismiss_quit_dialog(weak);
    }
    let key = launch_key(&event);
    if last.as_ref() == Some(&key) {
        return;
    }
    log_progress(
        "launch",
        &key,
        key.stage(),
        last.as_ref().map(LaunchKey::stage),
        last_log,
    );
    *last = Some(key);
    apply_launch_event(weak, run, &event);
}

/// Logs a progress change: every stage change, and at most once a second for the
/// byte-count updates in between, so a log at the default level shows the
/// download moving without a line per 100 ms tick.
pub(crate) fn log_progress(
    flow: &str,
    key: &impl std::fmt::Debug,
    stage: &'static str,
    previous_stage: Option<&'static str>,
    last_log: &mut Option<Instant>,
) {
    let now = Instant::now();
    let stage_changed = previous_stage != Some(stage);
    let due = last_log
        .map(|last| now.duration_since(last) >= Duration::from_secs(1))
        .unwrap_or(true);
    if stage_changed || due {
        log::info!(target: "launch", "{flow} progress: {key:?}");
        *last_log = Some(now);
    } else {
        log::debug!(target: "launch", "{flow} progress: {key:?}");
    }
}

/// The game is up, so there is nothing left in progress to abort and
/// `ConfirmQuitApp` has stopped having anything to warn about. A user who opened
/// it and then watched the game start should find it gone rather than still being
/// asked.
///
/// It is dismissed and *not* treated as a cancel: the flow is deliberately still
/// running — the back button is disabled and the screen waits for the process to
/// exit — and only the dialog's own flag goes down, which is all its "Wait a
/// moment…" button does.
pub(crate) fn dismiss_quit_dialog(weak: &Weak<App>) {
    let _ = weak.upgrade_in_event_loop(move |ui| {
        ui.global::<Dialogs>().set_confirm_quit_app_visible(false);
    });
}

/// The Vue's `launchGame()` `onProgress`.
pub(crate) fn apply_launch_event(weak: &Weak<App>, run: &Run, event: &LaunchEvent) {
    match event {
        LaunchEvent::Prepare => push(weak, run, |state| {
            state.set_progress_kind("preparing-launch".into());
            state.set_progress_loading(true);
        }),
        LaunchEvent::CompleteFiles(progress) => {
            let phase = phase_of(progress);
            let total = progress.total_bytes.load(Ordering::SeqCst);
            if phase == DownloadPhase::VerifyExistingFiles {
                push(weak, run, |state| {
                    state.set_progress_kind("verify-files".into());
                    state.set_progress_loading(true);
                });
            } else if phase == DownloadPhase::DownloadFiles {
                // The Vue passes the raw byte counts here (unlike the install
                // path, which formats them with `formatBytes`).
                let completed = progress.completed_bytes.load(Ordering::SeqCst);
                let current = completed.to_string();
                let total_text = total.to_string();
                push(weak, run, move |state| {
                    state.set_progress_kind("download-files".into());
                    state.set_progress_current(current.into());
                    state.set_progress_total(total_text.into());
                    state.set_progress_loading(false);
                    state.set_progress_value(completed as f32);
                    state.set_progress_max(total as f32);
                });
            }
        }
        LaunchEvent::GenerateScriptlet => push(weak, run, |state| {
            state.set_progress_kind("generate-script".into());
            state.set_progress_loading(true);
        }),
        // `LogTextureLoaded` alone gets the "Game started" *description*, which is
        // what the Vue gave it. That is a separate concern from the launch being
        // over, which any of the three markers means — see `flush_launch`.
        LaunchEvent::WaitForLaunch
        | LaunchEvent::LogSettingUser
        | LaunchEvent::LogLwjglVersion
        | LaunchEvent::LogOpenALLoaded => push(weak, run, |state| {
            state.set_progress_kind("wait-for-launch".into());
            state.set_progress_loading(true);
            state.set_back_disabled(true);
        }),
        LaunchEvent::LogTextureLoaded => push(weak, run, |state| {
            state.set_progress_kind("game-started".into());
            state.set_progress_loading(true);
            state.set_back_disabled(true);
        }),
        // No Vue branch updates the screen for this event.
        LaunchEvent::InstallAuthlibInjector(_) => {}
    }
}

pub(crate) fn phase_of(state: &DownloadState) -> DownloadPhase {
    state
        .phase
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// The Vue frontend's `formatBytes` (`crates/download/index.ts`).
pub(crate) fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.2} {}", UNITS[unit])
}
