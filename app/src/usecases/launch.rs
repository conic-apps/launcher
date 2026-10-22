// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch use case: the checks, the account refresh, the install (when the
//! instance is not installed) and the launch.
//!
//! UI-neutral. It reports through a [`LaunchSink`] of [`LaunchUpdate`]s; the
//! frontend turns each into screen state. Nothing here knows Slint, the
//! `LaunchState` global or which dialog a refusal shows.

use std::sync::Arc;

use account::Account;
use config::Config;
use instance::Instance;
use shared::Sink;

/// The output port [`run`] reports through.
pub type LaunchSink = Sink<LaunchUpdate>;

/// A refusal or failure the frontend shows as a dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchDialog {
    NoAccount,
    NoMicrosoftAccount,
    AccountRefreshFailed,
    NoSuitableJava,
}

/// What one launch run reports, in the order it reports it.
pub enum LaunchUpdate {
    /// The `install` crate's progress, passed straight through.
    Install(install::InstallProgress),
    /// The `launch` crate's progress, passed straight through.
    Launch(launch::LaunchProgress),
    /// The account is being refreshed, before a stage bar is set.
    RefreshAccount,
    Dialog(LaunchDialog),
    /// A failure shown as a message on the screen rather than a dialog.
    Error(String),
    /// A refreshed account the frontend stores back into the config.
    Account(Account),
    /// The run is over. `quit_app` closes the launcher; `pause_music` is the
    /// user's "pause on launch" setting.
    Finished {
        quit_app: bool,
        pause_music: bool,
    },
}

/// Runs the flow and reports every step through `out`.
pub async fn run(mut config: Config, instance: Option<Instance>, out: LaunchSink) {
    log::info!(target: "launch", "launch flow started");
    // Only a zh_cn user may launch with no Microsoft account.
    let accounts = account::list_accounts();
    if config.language.as_deref() != Some("zh_cn") && accounts.microsoft.is_empty() {
        log::info!(target: "launch", "refused: no Microsoft account and the language is not zh_cn");
        out(LaunchUpdate::Dialog(LaunchDialog::NoMicrosoftAccount));
        return;
    }
    let Some(account) = config.current_account.clone() else {
        log::info!(target: "launch", "refused: no current account");
        out(LaunchUpdate::Dialog(LaunchDialog::NoAccount));
        return;
    };
    let Some(instance) = instance else {
        log::info!(target: "launch", "refused: no current instance");
        out(LaunchUpdate::Error("currentInstance is null".to_string()));
        return;
    };

    if !config.launch.skip_refresh_account {
        log::info!(target: "launch", "refreshing the {} account", account.kind());
        out(LaunchUpdate::RefreshAccount);
        match refresh_account(&account).await {
            Ok(Some(refreshed)) => {
                config.current_account = Some(refreshed.clone());
                out(LaunchUpdate::Account(refreshed));
            }
            Ok(None) => {}
            Err(error) => {
                log::error!(target: "launch", "failed to refresh the account: {error}");
                out(LaunchUpdate::Dialog(LaunchDialog::AccountRefreshFailed));
                return;
            }
        }
    }

    if !instance.installed {
        log::info!(
            target: "launch",
            "instance '{}' is not installed, starting the install",
            instance.config.name
        );
        let sink: install::InstallSink = {
            let out = out.clone();
            Arc::new(move |progress| out(LaunchUpdate::Install(progress)))
        };
        if let Err(error) = install::install(config.clone(), instance.clone(), sink).await {
            log::error!(target: "launch", "install failed: {error}");
            out(LaunchUpdate::Error(error.to_string()));
            return;
        }
    }

    log::info!(target: "launch", "launching instance '{}'", instance.config.name);
    let sink: launch::LaunchSink = {
        let out = out.clone();
        Arc::new(move |progress| out(LaunchUpdate::Launch(progress)))
    };
    match launch::launch(config.clone(), instance.clone(), sink).await {
        Ok(pid) => {
            // The process id is the one handle a bug report can be correlated
            // with, against `running.rs`'s own lines about the same process.
            log::info!(target: "launch", "launch task finished (pid {pid})");
            let quit_app = instance
                .config
                .launch_config
                .quit_app_after_launch
                .unwrap_or(config.launch.quit_app_after_launch);
            log::info!(
                target: "launch",
                "the launch finished; the app will {} and the music is {}",
                if quit_app { "quit" } else { "stay open" },
                if config.music.pause_on_launch { "paused" } else { "left alone" }
            );
            out(LaunchUpdate::Finished {
                quit_app,
                pause_music: config.music.pause_on_launch,
            });
        }
        Err(error) => {
            log::error!(target: "launch", "launch failed: {error}");
            match error {
                launch::Error::NoSuitableJavaRuntime => {
                    out(LaunchUpdate::Dialog(LaunchDialog::NoSuitableJava));
                }
                other => out(LaunchUpdate::Error(other.to_string())),
            }
        }
    }
}

/// Refreshes the account's credentials before launch.
///
/// Returns the refreshed account when one was fetched, `None` when nothing had
/// to change.
async fn refresh_account(account: &Account) -> Result<Option<Account>, String> {
    if matches!(account, Account::Offline(_)) {
        return Ok(None);
    }
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
