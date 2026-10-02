// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The install-and-launch flow the launch button runs.

use super::*;

/// Resets `LaunchState` and seeds the instance and account fields.
pub(crate) fn reset_state(ui: &App, instance: Option<&Instance>, config: &Config) {
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

/// The launch flow: the checks, the account refresh, the install (when the
/// instance is not installed) and the launch.
pub(crate) async fn run_flow(
    weak: Weak<App>,
    token: Token,
    mut config: Config,
    instance: Option<Instance>,
) {
    log::info!(target: "launch", "launch flow started");
    // Only a zh_cn user may launch with no Microsoft account.
    let accounts = account::list_accounts();
    if config.language.as_deref() != Some("zh_cn") && accounts.microsoft.is_empty() {
        log::info!(target: "launch", "refused: no Microsoft account and the language is not zh_cn");
        show_dialog(&weak, &token, Dialog::NoMicrosoftAccount);
        return;
    }
    let Some(account) = config.current_account.clone() else {
        log::info!(target: "launch", "refused: no current account");
        show_dialog(&weak, &token, Dialog::NoAccount);
        return;
    };
    let Some(instance) = instance else {
        log::info!(target: "launch", "refused: no current instance");
        set_error(&weak, &token, "currentInstance is null".to_string());
        return;
    };

    if !config.launch.skip_refresh_account {
        log::info!(target: "launch", "refreshing the {} account", account.kind());
        match refresh_account(&weak, &token, &account).await {
            Ok(Some(refreshed)) => {
                config.current_account = Some(refreshed.clone());
                store_account(&weak, &token, refreshed);
            }
            Ok(None) => {}
            Err(error) => {
                log::error!("failed to refresh the account: {error}");
                show_dialog(&weak, &token, Dialog::AccountRefreshFailed);
                return;
            }
        }
    }

    if !instance.installed
        && let Err(error) = install_game(&weak, &token, config.clone(), instance.clone()).await
    {
        log::error!(target: "launch", "install failed: {error}");
        handle_failure(&weak, &token, Failure::from_install(error));
        return;
    }

    log::info!(target: "launch", "launching instance '{}'", instance.config.name);
    match launch_game(&weak, &token, config.clone(), instance.clone()).await {
        Ok(()) => {
            log::info!(target: "launch", "launch task finished");
            if config.music.pause_on_launch {
                crate::music::pause();
            }
            let quit = instance
                .config
                .launch_config
                .quit_app_after_launch
                .unwrap_or(config.launch.quit_app_after_launch);
            deliver(&weak, &token, move |ui| {
                if quit {
                    let _ = ui.hide();
                    let _ = slint::quit_event_loop();
                } else {
                    ui.global::<Navigation>().invoke_back();
                }
            });
        }
        Err(error) => {
            log::error!(target: "launch", "launch failed: {error}");
            handle_failure(&weak, &token, Failure::from_launch(error));
        }
    }
}

/// Runs the install task. The `install` crate owns the sampling; this only
/// hands it the port.
pub(crate) async fn install_game(
    weak: &Weak<App>,
    token: &Token,
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
    let sink = install_sink(weak.clone(), token.clone(), loader);
    install::install(config, instance, sink).await
}

/// Runs the launch task. The `launch` crate owns the sampling; this only hands
/// it the port.
pub(crate) async fn launch_game(
    weak: &Weak<App>,
    token: &Token,
    config: Config,
    instance: Instance,
) -> Result<(), launch::Error> {
    let sink = launch_sink(weak.clone(), token.clone());
    launch::launch(config, instance, sink).await.map(|_pid| ())
}

pub(crate) enum Dialog {
    NoAccount,
    NoMicrosoftAccount,
    AccountRefreshFailed,
    NoSuitableJava,
}

pub(crate) fn show_dialog(weak: &Weak<App>, token: &Token, dialog: Dialog) {
    deliver(weak, token, move |ui| {
        let dialogs = ui.global::<Dialogs>();
        match dialog {
            Dialog::NoAccount => dialogs.set_no_account_error_visible(true),
            Dialog::NoMicrosoftAccount => dialogs.set_no_microsoft_account_error_visible(true),
            Dialog::AccountRefreshFailed => dialogs.set_account_refresh_failed_visible(true),
            Dialog::NoSuitableJava => dialogs.set_no_suitable_java_error_visible(true),
        }
    });
}

pub(crate) enum Failure {
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

/// Shows the right dialog or error message for a failure.
pub(crate) fn handle_failure(weak: &Weak<App>, token: &Token, failure: Failure) {
    match failure {
        Failure::NoSuitableJava => show_dialog(weak, token, Dialog::NoSuitableJava),
        Failure::Message(message) => set_error(weak, token, message),
    }
}

/// Shows an error message on the launch screen.
pub(crate) fn set_error(weak: &Weak<App>, token: &Token, message: String) {
    deliver(weak, token, move |ui| {
        let state = ui.global::<LaunchState>();
        state.set_error(true);
        state.set_progress_kind("error".into());
        state.set_error_message(message.into());
        state.set_back_disabled(false);
    });
}

/// Pushes the refreshed account into the shared config and the view, on the
/// event loop.
pub(crate) fn store_account(weak: &Weak<App>, token: &Token, account: Account) {
    deliver(weak, token, move |ui| {
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

/// Pushes one `LaunchState` write on the event loop, dropping it if the run has
/// been superseded.
pub(crate) fn push(
    weak: &Weak<App>,
    token: &Token,
    update: impl FnOnce(&LaunchState) + Send + 'static,
) {
    deliver(weak, token, move |ui| {
        update(&ui.global::<LaunchState>());
    });
}

/// Refreshes the account's credentials before launch.
///
/// Returns the refreshed account when one was fetched (the caller stores it),
/// `None` when nothing had to change.
pub(crate) async fn refresh_account(
    weak: &Weak<App>,
    token: &Token,
    account: &Account,
) -> Result<Option<Account>, String> {
    if matches!(account, Account::Offline(_)) {
        return Ok(None);
    }
    push(weak, token, |state| {
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
