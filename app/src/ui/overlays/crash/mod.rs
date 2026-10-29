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
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::slint_backend::{App, CrashLogLine, CrashReason, CrashState};
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
    state.set_loader(crash.loader.into());
    state.set_minecraft_version(crash.minecraft_version.into());
    state.set_launcher_version(crash.launcher_version.into());
    state.set_allocated_memory(
        crate::support::formatting::bytes(crash.allocated_memory_mb as u64 * 1024 * 1024).into(),
    );
    state.set_total_memory(if crash.total_memory_bytes == 0 {
        "--".into()
    } else {
        crate::support::formatting::bytes(crash.total_memory_bytes).into()
    });
    state.set_java_version(if crash.java_version.is_empty() {
        "--".into()
    } else {
        crash.java_version.into()
    });
    state.set_summary(crash.summary.into());
    state.set_exit_code(
        match crash.exit_code {
            Some(code) => format!("exit code: {code} ({})", crash.exit_type.as_str()),
            None => crash.exit_type.as_str().to_string(),
        }
        .into(),
    );
    state.set_exit_code_number(
        crash
            .exit_code
            .map(|code| code.to_string())
            .unwrap_or_default()
            .into(),
    );
    state.set_exit_kind(crash.exit_type.as_str().into());
    state.set_reasons(format_reasons(&crash.reasons).into());
    state.set_reason_items(ModelRc::new(VecModel::from(reason_items(&crash.reasons))));
    state.set_keywords(crash.keywords.join(", ").into());
    state.set_report_path(crash.report_path.unwrap_or_default().into());
    state.set_detail_lines(detail_lines(&crash.details));
    state.set_mono_family(crate::ui::overlays::content::monospace_family().into());
    state.set_details(crash.details.into());
    state.set_game_console(crash.console_log.into());
    state.set_game_log(crash.game_log.into());
    state.set_visible(true);
}

/// The matched rules as translated rows, with their captured fields flattened.
fn reason_items(reasons: &[Reason]) -> Vec<CrashReason> {
    reasons
        .iter()
        // The manual-debug rule only repeats the summary the report already
        // showed, and an empty row would still leave a gap in the panel.
        .filter(|reason| reason.rule != "DEBUG_CRASH")
        .map(|reason| CrashReason {
            rule: SharedString::from(reason.rule),
            fields: SharedString::from(
                reason
                    .fields
                    .iter()
                    .filter(|field| !field.is_empty())
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        })
        .collect()
}

/// The width of a tab stop in the crash log's monospace font.
const TAB_WIDTH: usize = 4;

/// Splits the crash text into lines and tags each with the colour the panel
/// should draw it in.
fn detail_lines(details: &str) -> ModelRc<CrashLogLine> {
    let lines = details
        .lines()
        .map(|line| CrashLogLine {
            // Slint draws a tab as a missing glyph, so expand it before the
            // `Text` element sees it.
            text: SharedString::from(expand_tabs(line)),
            kind: SharedString::from(log_line_kind(line)),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(lines))
}

/// Expands tabs to the next [`TAB_WIDTH`] column, so indentations in a crash
/// report stay aligned in the panel.
fn expand_tabs(line: &str) -> String {
    if !line.contains('\t') {
        return line.to_string();
    }
    let mut expanded = String::with_capacity(line.len());
    let mut column = 0;
    for character in line.chars() {
        if character == '\t' {
            let spaces = TAB_WIDTH - column % TAB_WIDTH;
            expanded.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else {
            expanded.push(character);
            column += 1;
        }
    }
    expanded
}

/// The syntax class of one line of a Minecraft log or crash report.
///
/// Stack frames come first: they are the bulk of a report and the one thing
/// that must stay out of the way, which is why they are drawn in italic grey.
fn log_line_kind(line: &str) -> &'static str {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return "plain";
    }
    if trimmed.starts_with("at ") || (trimmed.starts_with("... ") && trimmed.ends_with(" more")) {
        return "stack";
    }
    if trimmed.starts_with("Caused by:") {
        return "caused";
    }
    if trimmed.starts_with("Suppressed:") || trimmed.starts_with("//") {
        return "muted";
    }
    if trimmed.starts_with("----")
        || trimmed.starts_with("#@!@#")
        || trimmed.starts_with("Description:")
        || trimmed.starts_with("Time:")
        || trimmed.starts_with("Stacktrace:")
        || trimmed.starts_with("-- ")
    {
        return "header";
    }
    if trimmed.contains("/ERROR]") || trimmed.contains("[ERROR]") || trimmed.contains("| ERROR") {
        return "error";
    }
    if trimmed.contains("/WARN]") || trimmed.contains("[WARN]") || trimmed.contains("| WARN") {
        return "warning";
    }
    if trimmed.contains("/INFO]") || trimmed.contains("[INFO]") || trimmed.contains("| INFO") {
        return "info";
    }
    if trimmed.contains("/DEBUG]") || trimmed.contains("[DEBUG]") || trimmed.contains("| DEBUG") {
        return "debug";
    }
    if trimmed.contains("/TRACE]") || trimmed.contains("[TRACE]") || trimmed.contains("| TRACE") {
        return "trace";
    }
    if trimmed.contains("Exception") || trimmed.contains("Error") {
        return "error";
    }
    "plain"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_frames_are_the_quiet_italic_lines() {
        assert_eq!(
            log_line_kind("\tat net.minecraft.Main.main(Main.java:1)"),
            "stack"
        );
        assert_eq!(log_line_kind("... 3 more"), "stack");
    }

    #[test]
    fn log_levels_and_exceptions_get_their_own_colours() {
        assert_eq!(
            log_line_kind("[12:00:00] [Render thread/ERROR]: boom"),
            "error"
        );
        assert_eq!(log_line_kind("[12:00:00] [main/WARN]: careful"), "warning");
        assert_eq!(log_line_kind("[12:00:00] [main/INFO]: hello"), "info");
        assert_eq!(
            log_line_kind("java.lang.NullPointerException: boom"),
            "error"
        );
        assert_eq!(log_line_kind("Caused by: java.io.IOException"), "caused");
        assert_eq!(
            log_line_kind("Description: Manually triggered debug crash"),
            "header"
        );
        assert_eq!(log_line_kind("ordinary message"), "plain");
    }

    #[test]
    fn tabs_are_expanded_to_the_next_tab_stop() {
        assert_eq!(
            expand_tabs("\tat net.minecraft.Main.main"),
            "    at net.minecraft.Main.main"
        );
        assert_eq!(expand_tabs("ab\tcd"), "ab  cd");
        assert_eq!(expand_tabs("abcd\te"), "abcd    e");
        assert_eq!(expand_tabs("plain"), "plain");
    }

    #[test]
    fn the_manual_debug_rule_is_not_drawn_twice() {
        assert!(
            reason_items(&[Reason {
                rule: "DEBUG_CRASH",
                fields: Vec::new(),
            }])
            .is_empty()
        );
    }

    #[test]
    fn reasons_keep_their_fields_as_one_row() {
        let rows = reason_items(&[Reason {
            rule: "TOO_OLD_JAVA",
            fields: vec!["61".to_string(), String::new(), "extra".to_string()],
        }]);
        assert_eq!(rows[0].rule.as_str(), "TOO_OLD_JAVA");
        assert_eq!(rows[0].fields.as_str(), "61, extra");
    }
}
