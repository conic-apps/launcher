// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The crash page's adapter: it subscribes to the launch crate's session
//! events, fills `CrashState` and navigates to the `"crash"` page.
//!
//! The subscription is installed once for the whole app, not per launch: a game
//! outlives the launch flow that started it, so the crash it may produce arrives
//! long after that flow — and its generation token — are gone. This is the
//! single `Sink` the composition root installs (see `launch::running`).

use std::sync::Arc;

use slint::ComponentHandle;

use launch::crash::{CrashReport, Reason};
use launch::running::{self, SessionEvent};
use shared::Sink;

use crate::slint_backend::{App, CrashState, Navigation};

/// Installs the app's running-session subscriber and the page's "Close" button.
pub(crate) fn setup(ui: &App) {
    // "Close" returns to the page the crash interrupted. A crash during launch
    // leaves no `previous-page` behind (the launch view does not set one), so
    // the fallback is the game screen.
    let weak = ui.as_weak();
    ui.global::<CrashState>().on_close(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let navigation = ui.global::<Navigation>();
        if navigation.get_previous_page().is_empty() {
            navigation.set_current_page("game".into());
        } else {
            navigation.invoke_back();
        }
    });

    let weak = ui.as_weak();
    // `running::subscribe` keeps one slot, so a second `setup` would replace this
    // sink silently and no crash would ever reach the page. Worth a line, since
    // `main` wires this once and a future caller might not.
    log::debug!("subscribing the crash page to the launch session events");
    let sink: Sink<SessionEvent> = Arc::new(move |event| {
        let SessionEvent::Exited {
            crash: Some(crash), ..
        } = event
        else {
            return;
        };
        crate::ui::services::report::report(&weak, move |ui| show(&ui, crash));
    });
    running::subscribe(sink);
}

/// Pushes a collected crash into `CrashState` and opens the page.
fn show(ui: &App, crash: CrashReport) {
    let state = ui.global::<CrashState>();
    // The crash crate logs what it diagnosed; this is the line that says an
    // instance crashed and where to look, which is the first thing anyone reading
    // a support log wants and it was nowhere in it.
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
    ui.global::<Navigation>().invoke_navigate("crash".into());
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
