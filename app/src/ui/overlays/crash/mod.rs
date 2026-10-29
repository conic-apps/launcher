// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The crash overlay's adapter: it subscribes to the launch crate's session
//! events, posts a notification for the crash and raises the overlay when that
//! notification is activated.
//!
//! The subscription is installed once for the whole app, not per launch: a game
//! outlives the launch flow that started it, so the crash it may produce arrives
//! long after that flow — and its generation token — are gone. This is the
//! single `Sink` the composition root installs (see `launch::running`).
//!
//! The report callback hands the collected strings to `usecases::crash_report`,
//! which writes one combined report into the launcher's logs; this adapter then
//! opens that file. The report contains the game's own report (or the collected
//! details), the captured console, `logs/latest.log` and the launcher log. The
//! callback is wired here so the writer stays reachable while the overlay's
//! body is being rewritten; the panel does not draw a button for it yet.

use std::sync::Arc;

use launch::crash::{CrashReport, Reason};
use launch::running::{self, SessionEvent};
use shared::Sink;
use shared::notification::{Notice, NoticeKind};
use slint::ComponentHandle;

use crate::slint_backend::{App, CrashState};
use crate::ui::services::{app_config, notifications};
use crate::usecases::crash_report;

/// Installs the app's running-session subscriber and the overlay's callbacks.
pub(crate) fn setup(ui: &App) {
    // "Close" hides the overlay and leaves the page it interrupted in place.
    let weak = ui.as_weak();
    ui.global::<CrashState>().on_close(move || {
        if let Some(ui) = weak.upgrade() {
            ui.global::<CrashState>().set_visible(false);
        }
    });

    let weak = ui.as_weak();
    ui.global::<CrashState>().on_open_report(move || {
        if let Some(ui) = weak.upgrade() {
            open_report(&ui);
        }
    });

    let weak = ui.as_weak();
    // `running::subscribe` keeps one slot, so a second `setup` would replace this
    // sink silently and no crash would ever reach the notification. Worth a
    // line, since `main` wires this once and a future caller might not.
    log::debug!("subscribing the crash overlay to the launch session events");
    let sink: Sink<SessionEvent> = Arc::new(move |event| {
        let SessionEvent::Exited {
            crash: Some(crash), ..
        } = event
        else {
            return;
        };
        let crash = *crash;
        let action_weak = weak.clone();
        crate::ui::services::report::report(&weak, move |ui| {
            // The crash crate logs what it diagnosed; this is the line that
            // says an instance crashed and where to look, which a support log
            // is read for. Logged when the crash is posted, not when the user
            // opens the report.
            log::error!(
                "'{}' crashed: {} ({}){}",
                crash.instance_name,
                crash.summary,
                crash.exit_type.as_str(),
                match crash.report_path {
                    Some(ref path) => format!(" — report at {path}"),
                    None => String::new(),
                }
            );
            let instance_name = crash.instance_name.clone();
            notifications::send(
                &ui,
                Notice::new(NoticeKind::GameCrashed)
                    .with_arg(instance_name)
                    .on_click(move || {
                        if let Some(ui) = action_weak.upgrade() {
                            show(&ui, crash);
                        }
                    }),
            );
        });
    });
    running::subscribe(sink);
}

/// Pushes a collected crash into `CrashState` and raises the overlay.
fn show(ui: &App, crash: CrashReport) {
    let state = ui.global::<CrashState>();
    state.set_instance_name(crash.instance_name.into());
    state.set_summary(crash.summary.into());
    state.set_exit_code(
        match crash.exit_code {
            Some(code) => format!("exit code: {code} ({})", crash.exit_type.as_str()),
            None => crash.exit_type.as_str().to_string(),
        }
        .into(),
    );
    state.set_reasons(format_reasons(&crash.reasons).into());
    state.set_keywords(crash.keywords.join(", ").into());
    state.set_report_path(crash.report_path.unwrap_or_default().into());
    state.set_details(crash.details.into());
    state.set_game_console(crash.console_log.into());
    state.set_game_log(crash.game_log.into());
    state.set_visible(true);
}

/// Writes one report with everything a helper needs and opens it.
///
/// The game may have written its own crash report; opening that file alone
/// throws away the console output and the launcher log that say what led to it.
/// So this always writes the combined report instead, embedding the game's file
/// (or the collected details when it wrote none) beside both logs.
fn open_report(ui: &App) {
    let state = ui.global::<CrashState>();
    let source = crash_report::Source {
        instance_name: state.get_instance_name().to_string(),
        summary: state.get_summary().to_string(),
        exit: state.get_exit_code().to_string(),
        report_path: state.get_report_path().to_string(),
        reasons: state.get_reasons().to_string(),
        keywords: state.get_keywords().to_string(),
        details: state.get_details().to_string(),
        game_console: state.get_game_console().to_string(),
        game_log: state.get_game_log().to_string(),
    };
    let directory = storage::LOCATIONS.launcher.logs.clone();
    let file = match crash_report::write(&directory, &source) {
        Ok(file) => file,
        Err(error) => {
            log::error!(
                "could not write the crash report in {}: {error}",
                directory.display()
            );
            return;
        }
    };
    if let Err(error) = app_config::open_external(file.to_string_lossy().as_ref()) {
        log::warn!(
            "could not open the crash report at {}: {error}",
            file.display()
        );
    }
}

/// Renders the matched rules as one line each: the rule's name, and the fields
/// it captured when it has any (a mod id, an expected version, a file).
fn format_reasons(reasons: &[Reason]) -> String {
    reasons
        .iter()
        .map(|reason| {
            let fields: Vec<&str> = reason
                .fields
                .iter()
                .map(String::as_str)
                .filter(|field| !field.is_empty())
                .collect();
            if fields.is_empty() {
                reason.rule.to_string()
            } else {
                format!("{}: {}", reason.rule, fields.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
