// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch view's "script" (src/views/LaunchView.vue): it plays the Vue's
//! `launch()` — account refresh, install when needed, then the launch — against
//! the `slint-install` / `slint-launch` crates and pushes progress into the
//! `LaunchState` global.
//!
//! The Vue kept the two tasks as Tauri commands that reported progress over an
//! IPC channel; here the whole flow is one task on the app's runtime, which
//! polls the same `Arc<Mutex<…Event>>` the commands used and translates it into
//! `LaunchState` writes. Cancelling aborts the task, exactly like the Vue's
//! `cancel()` did through `cmd_cancel_*_task`.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use slint::{ComponentHandle, Weak};

use crate::slint_backend::{App, Dialogs, GameState, LaunchState, Navigation};
use account::Account;
use config::Config;
use download::progress::{DownloadPhase, DownloadState};
use install::{InstallEvent, ModLoaderProgress};
use instance::Instance;
use launch::LaunchEvent;

thread_local! {
    /// The app's shared config, for the event-loop half of the account refresh.
    /// The runtime task cannot hold the `Rc<RefCell<…>>` (it is neither `Send`
    /// nor `Sync`); it reports the refreshed account back, and the event-loop
    /// closure writes it through here (and persists it), exactly like the Vue's
    /// `configStore.current_account = …`.
    static SHARED_CONFIG: RefCell<Option<Rc<RefCell<Config>>>> = const { RefCell::new(None) };

    /// The one controller, for use on the UI thread.
    ///
    /// It lives in a thread-local rather than in a value `setup` owns because an
    /// `upgrade_in_event_loop` closure has to be `Send`, and the `Rc<RefCell<…>>`
    /// cannot cross into one. Slint's callbacks and the event-loop closures both
    /// run on the UI thread, so both reach it through [`launch_controller`]; the
    /// async task carries only owned values and a `Weak<App>`.
    static CONTROLLER: RefCell<Option<Rc<RefCell<LaunchController>>>> =
        const { RefCell::new(None) };
}

/// The controller, for use on the UI thread. Named apart from `setup`'s own
/// `controller` binding, which would otherwise shadow it.
fn launch_controller() -> Rc<RefCell<LaunchController>> {
    CONTROLLER
        .with(|cell| cell.borrow().clone())
        .expect("the launch controller is set up before any of its callbacks run")
}

/// One run of the launch flow. Everything the run schedules carries a clone of
/// it, so a late event from a cancelled run is dropped instead of writing into
/// a fresh one.
#[derive(Clone)]
struct Run {
    id: u64,
    cancelled: Arc<AtomicBool>,
    current_id: Arc<AtomicU64>,
}

impl Run {
    fn is_current(&self) -> bool {
        !self.cancelled.load(Ordering::SeqCst) && self.current_id.load(Ordering::SeqCst) == self.id
    }
}

struct LaunchController {
    /// The running flow, aborted by [`LaunchController::cancel`].
    task: Option<tokio::task::JoinHandle<()>>,
    current_id: Arc<AtomicU64>,
    next_id: u64,
    /// The cancelled flags of every run that was started, so `cancel` can stop
    /// their in-flight event deliveries as well as the task itself.
    runs: Vec<Arc<AtomicBool>>,
}

impl LaunchController {
    fn new() -> Self {
        Self {
            task: None,
            current_id: Arc::new(AtomicU64::new(0)),
            next_id: 1,
            runs: Vec::new(),
        }
    }

    fn begin(&mut self) -> Run {
        let id = self.next_id;
        self.next_id += 1;
        let cancelled = Arc::new(AtomicBool::new(false));
        self.runs.push(Arc::clone(&cancelled));
        self.current_id.store(id, Ordering::SeqCst);
        Run {
            id,
            cancelled,
            current_id: Arc::clone(&self.current_id),
        }
    }

    /// Cancels the running flow. Returns whether one was running, which is the
    /// Vue's `onUnmounted` `instanceStore.loadInstances()` trigger.
    fn cancel(&mut self) -> bool {
        let had_task = self.task.is_some();
        for cancelled in self.runs.drain(..) {
            cancelled.store(true, Ordering::SeqCst);
        }
        self.current_id.store(0, Ordering::SeqCst);
        if let Some(task) = self.task.take() {
            task.abort();
        }
        had_task
    }
}

/// Registers the launch view's callbacks.
pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    SHARED_CONFIG.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&config)));
    let controller = Rc::new(RefCell::new(LaunchController::new()));
    CONTROLLER.with(|cell| *cell.borrow_mut() = Some(Rc::clone(&controller)));
    let state = ui.global::<LaunchState>();

    // `LaunchView.vue`'s `onMounted(launch)`.
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_start(move || {
            let Some(ui) = weak.upgrade() else { return };
            controller.borrow_mut().cancel();
            let run = controller.borrow_mut().begin();

            // Everything the flow needs that is not on disk is gathered on the
            // UI thread, because `GameState`/`LaunchState` and the config are
            // not reachable from the runtime.
            let current_id = ui.global::<GameState>().get_current_id().to_string();
            let config_snapshot = SHARED_CONFIG.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .map(|config| config.borrow().clone())
                    .unwrap_or_default()
            });

            // `instance.toml` is read on the runtime, as the original read it
            // in a Tauri command, and the state is reset behind that read.
            let weak = weak.clone();
            crate::runtime::spawn(async move {
                let instance = instance::get_instance_by_id(&current_id).await;
                let flow_weak = weak.clone();
                let flow_config = config_snapshot.clone();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    reset_state(&ui, instance.as_ref(), &flow_config);
                    let task = crate::runtime::spawn(async move {
                        run_flow(flow_weak, run, config_snapshot, instance).await;
                    });
                    launch_controller().borrow_mut().task = Some(task);
                });
            });
        });
    }

    // The back button (the Vue's `back()`; `onUnmounted` cancels too).
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        state.on_cancel(move || {
            let had_task = controller.borrow_mut().cancel();
            if let Some(ui) = weak.upgrade() {
                ui.global::<Navigation>().invoke_back();
                if had_task {
                    ui.global::<GameState>().invoke_refresh();
                }
            }
        });
    }

    // Leaving the page (title bar Home/Settings) cancels the flow, like the
    // Vue's `onUnmounted`.
    {
        let controller = Rc::clone(&controller);
        let weak = ui.as_weak();
        ui.global::<Navigation>().on_page_changed(move |page| {
            if page.as_str() != "launch" && controller.borrow_mut().cancel() {
                // The Vue's `onUnmounted` for the launch view: the instance
                // list is reloaded, so `.install.lock` written by a fresh
                // install shows up on the game screen.
                if let Some(ui) = weak.upgrade() {
                    ui.global::<GameState>().invoke_refresh();
                }
            }
        });
    }
}

/// Resets `LaunchState` and seeds the instance/account fields — the Vue's
/// template computed values plus its `onMounted` initial `ref`s.
fn reset_state(ui: &App, instance: Option<&Instance>, config: &Config) {
    let state = ui.global::<LaunchState>();
    state.set_error(false);
    state.set_progress_kind("preparing".into());
    state.set_progress_name("".into());
    state.set_progress_current("".into());
    state.set_progress_total("".into());
    state.set_progress_message("".into());
    state.set_error_message("".into());
    state.set_progress_loading(true);
    state.set_progress_value(0.0);
    state.set_progress_max(0.0);
    state.set_back_disabled(false);

    match instance {
        Some(instance) => {
            state.set_has_instance(true);
            state.set_instance_name(instance.config.name.clone().into());
            state.set_minecraft_version(instance.config.runtime.minecraft.clone().into());
            state.set_has_loader(instance.config.runtime.mod_loader_type.is_some());
            state.set_loader_type(
                instance
                    .config
                    .runtime
                    .mod_loader_type
                    .as_ref()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default()
                    .into(),
            );
            state.set_loader_version(
                instance
                    .config
                    .runtime
                    .mod_loader_version
                    .clone()
                    .unwrap_or_default()
                    .into(),
            );
        }
        None => {
            state.set_has_instance(false);
            state.set_instance_name("".into());
            state.set_minecraft_version("".into());
            state.set_has_loader(false);
            state.set_loader_type("".into());
            state.set_loader_version("".into());
        }
    }

    let account = config.current_account.clone();
    match account.as_ref() {
        Some(account) => {
            state.set_has_account(true);
            state.set_account_kind(account.kind().into());
            state.set_profile_name(account.get_profile_name().into());
            state.set_account_avatar(
                crate::account_avatar::account_head(Some(account), 48).unwrap_or_default(),
            );
        }
        None => {
            state.set_has_account(false);
            state.set_account_kind("".into());
            state.set_profile_name("".into());
            state.set_account_avatar(Default::default());
        }
    }
}

/// The Vue's `launch()`: the checks, the account refresh, the install (when the
/// instance is not installed) and the launch.
async fn run_flow(weak: Weak<App>, run: Run, mut config: Config, instance: Option<Instance>) {
    log::info!(target: "launch", "launch flow started");
    // `if (configStore.language !== "zh_cn" && accountStore.microsoft.length === 0)`.
    let accounts = account::list_accounts();
    if config.language.as_deref() != Some("zh_cn") && accounts.microsoft.is_empty() {
        log::info!(target: "launch", "refused: no Microsoft account and the language is not zh_cn");
        show_dialog(&weak, &run, Dialog::NoMicrosoftAccount);
        return;
    }
    let Some(account) = config.current_account.clone() else {
        log::info!(target: "launch", "refused: no current account");
        show_dialog(&weak, &run, Dialog::NoAccount);
        return;
    };
    let Some(instance) = instance else {
        log::info!(target: "launch", "refused: no current instance");
        set_error(&weak, &run, "currentInstance is null".to_string());
        return;
    };

    if !config.launch.skip_refresh_account {
        log::info!(target: "launch", "refreshing the {} account", account.kind());
        match refresh_account(&weak, &run, &account).await {
            Ok(Some(refreshed)) => {
                config.current_account = Some(refreshed.clone());
                store_account(&weak, &run, refreshed);
            }
            Ok(None) => {}
            Err(error) => {
                log::error!("failed to refresh the account: {error}");
                show_dialog(&weak, &run, Dialog::AccountRefreshFailed);
                return;
            }
        }
    }

    if !instance.installed
        && let Err(error) = install_game(&weak, &run, config.clone(), instance.clone()).await
    {
        log::error!(target: "launch", "install failed: {error}");
        handle_failure(&weak, &run, Failure::from_install(error));
        return;
    }

    log::info!(target: "launch", "launching instance '{}'", instance.config.name);
    match launch_game(&weak, &run, config.clone(), instance.clone()).await {
        Ok(()) => {
            log::info!(target: "launch", "launch task finished");
            // `if (configStore.music.pause_on_launch) musicStore.pause()`.
            if config.music.pause_on_launch {
                crate::music::pause();
            }
            let quit = instance
                .config
                .launch_config
                .quit_app_after_launch
                .unwrap_or(config.launch.quit_app_after_launch);
            let weak = weak.clone();
            let run = run.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                if !run.is_current() {
                    return;
                }
                if quit {
                    // `appWindow.getCurrentWindow().close()`.
                    let _ = ui.hide();
                    let _ = slint::quit_event_loop();
                } else {
                    ui.global::<Navigation>().invoke_back();
                }
            });
        }
        Err(error) => {
            log::error!(target: "launch", "launch failed: {error}");
            handle_failure(&weak, &run, Failure::from_launch(error));
        }
    }
}

enum Dialog {
    NoAccount,
    NoMicrosoftAccount,
    AccountRefreshFailed,
    NoSuitableJava,
}

fn show_dialog(weak: &Weak<App>, run: &Run, dialog: Dialog) {
    let run = run.clone();
    let _ = weak.upgrade_in_event_loop(move |ui| {
        if !run.is_current() {
            return;
        }
        let dialogs = ui.global::<Dialogs>();
        match dialog {
            Dialog::NoAccount => dialogs.set_no_account_error_visible(true),
            Dialog::NoMicrosoftAccount => dialogs.set_no_microsoft_account_error_visible(true),
            Dialog::AccountRefreshFailed => dialogs.set_account_refresh_failed_visible(true),
            Dialog::NoSuitableJava => dialogs.set_no_suitable_java_error_visible(true),
        }
    });
}

enum Failure {
    NoSuitableJava,
    Message(String),
}

impl Failure {
    fn from_install(error: install::Error) -> Self {
        Self::Message(error.to_string())
    }

    fn from_launch(error: launch::Error) -> Self {
        match error {
            launch::Error::NoSuitableJavaRuntime => Self::NoSuitableJava,
            other => Self::Message(other.to_string()),
        }
    }
}

/// The Vue's `isNoSuitableJavaError` branch of `handleError`.
fn handle_failure(weak: &Weak<App>, run: &Run, failure: Failure) {
    match failure {
        Failure::NoSuitableJava => show_dialog(weak, run, Dialog::NoSuitableJava),
        Failure::Message(message) => set_error(weak, run, message),
    }
}

/// The Vue's `handleError`.
fn set_error(weak: &Weak<App>, run: &Run, message: String) {
    let run = run.clone();
    let _ = weak.upgrade_in_event_loop(move |ui| {
        if !run.is_current() {
            return;
        }
        let state = ui.global::<LaunchState>();
        state.set_error(true);
        state.set_progress_kind("error".into());
        state.set_error_message(message.into());
        state.set_back_disabled(false);
    });
}

/// Pushes the refreshed account into the shared config and the view, on the
/// event loop.
fn store_account(weak: &Weak<App>, run: &Run, account: Account) {
    let run = run.clone();
    let _ = weak.upgrade_in_event_loop(move |ui| {
        if !run.is_current() {
            return;
        }
        SHARED_CONFIG.with(|slot| {
            if let Some(config) = slot.borrow().as_ref() {
                config.borrow_mut().current_account = Some(account.clone());
                if let Err(error) = config::save_config(&config.borrow()) {
                    log::warn!("failed to save the refreshed account: {error}");
                }
            }
        });
        let state = ui.global::<LaunchState>();
        state.set_has_account(true);
        state.set_account_kind(account.kind().into());
        state.set_profile_name(account.get_profile_name().into());
        state.set_account_avatar(
            crate::account_avatar::account_head(Some(&account), 48).unwrap_or_default(),
        );
    });
}

/// Pushes one `LaunchState` write on the event loop, dropping it if the run is
/// no longer the current one.
fn push(weak: &Weak<App>, run: &Run, update: impl FnOnce(&LaunchState) + Send + 'static) {
    let run = run.clone();
    let _ = weak.upgrade_in_event_loop(move |ui| {
        if !run.is_current() {
            return;
        }
        update(&ui.global::<LaunchState>());
    });
}

/// The Vue's `refreshAccountCredentials()`.
///
/// Returns the refreshed account when one was fetched (the caller stores it),
/// `None` when nothing had to change.
async fn refresh_account(
    weak: &Weak<App>,
    run: &Run,
    account: &Account,
) -> Result<Option<Account>, String> {
    if matches!(account, Account::Offline(_)) {
        return Ok(None);
    }
    push(weak, run, |state| {
        state.set_progress_kind("refresh-account".into());
        state.set_progress_loading(true);
    });

    match account {
        Account::Microsoft(account) => {
            let refreshed = account::microsoft::refresh_account(account.profile.uuid, false)
                .await
                .map_err(|error| error.to_string())?;
            Ok(Some(Account::Microsoft(refreshed)))
        }
        Account::Yggdrasil(account) => {
            if account::yggdrasil::yggdrasil_user_api::validate(account.clone())
                .await
                .map_err(|error| error.to_string())?
            {
                return Ok(None);
            }
            let refreshed = account::yggdrasil::yggdrasil_user_api::refresh(account.clone())
                .await
                .map_err(|error| error.to_string())?;
            account::yggdrasil::update_account(refreshed.identifier, refreshed.clone())
                .await
                .map_err(|error| error.to_string())?;
            Ok(Some(Account::Yggdrasil(refreshed)))
        }
        Account::Offline(_) => Ok(None),
    }
}

/// The Vue's `installGame()`: runs the install task while polling its progress
/// (the Tauri command's channel thread, folded into the flow).
async fn install_game(
    weak: &Weak<App>,
    run: &Run,
    config: Config,
    instance: Instance,
) -> Result<(), install::Error> {
    let loader = instance
        .config
        .runtime
        .mod_loader_type
        .as_ref()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();
    log::info!(
        target: "launch",
        "instance '{}' is not installed, starting the install (loader: '{}')",
        instance.config.name,
        loader
    );
    let status = Arc::new(Mutex::new(InstallEvent::Prepare));
    let future = install::install(config, instance, Arc::clone(&status));
    tokio::pin!(future);
    let mut ticker = tokio::time::interval(Duration::from_millis(100));
    // The change detection has to run on plain numbers: an `InstallEvent` holds
    // a `DownloadState`, whose counters sit behind `Arc`s — so a saved clone and
    // the "current" event share the same atomics and always compare equal, which
    // is what froze the progress bar and the byte counters.
    let mut last: Option<InstallKey> = None;
    let mut last_log: Option<Instant> = None;
    loop {
        tokio::select! {
            result = &mut future => {
                // Flush the last status the poll may have missed before the
                // future resolved (the Vue's channel thread sends once more
                // before it is joined).
                flush_install(weak, run, &status, &mut last, &mut last_log, &loader);
                log::info!(target: "launch", "install finished: {result:?}");
                return result;
            }
            _ = ticker.tick() => {
                flush_install(weak, run, &status, &mut last, &mut last_log, &loader);
            }
        }
    }
}

/// A scalar snapshot of an install event, for change detection and logging.
#[derive(Clone, PartialEq, Debug)]
enum InstallKey {
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

fn install_key(event: &InstallEvent) -> InstallKey {
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
fn state_snapshot(state: &DownloadState) -> (DownloadPhase, u64, u64) {
    (
        phase_of(state),
        state.completed_bytes.load(Ordering::SeqCst),
        state.total_bytes.load(Ordering::SeqCst),
    )
}

/// Applies the current install status when it has moved on, and logs it.
fn flush_install(
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
fn apply_install_event(weak: &Weak<App>, run: &Run, event: &InstallEvent, loader: &str) {
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
fn apply_install_download(
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
fn apply_loader_download(
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
async fn launch_game(
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
enum LaunchKey {
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

fn launch_key(event: &LaunchEvent) -> LaunchKey {
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
fn flush_launch(
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
fn log_progress(
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
fn dismiss_quit_dialog(weak: &Weak<App>) {
    let _ = weak.upgrade_in_event_loop(move |ui| {
        ui.global::<Dialogs>().set_confirm_quit_app_visible(false);
    });
}

/// The Vue's `launchGame()` `onProgress`.
fn apply_launch_event(weak: &Weak<App>, run: &Run, event: &LaunchEvent) {
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

fn phase_of(state: &DownloadState) -> DownloadPhase {
    state
        .phase
        .lock()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// The Vue frontend's `formatBytes` (`crates/download/index.ts`).
fn format_bytes(bytes: u64) -> String {
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
