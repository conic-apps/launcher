// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The crash dialog's adapter: it subscribes to the launch crate's session
//! events, fills `CrashState` and opens the dialog.
//!
//! The subscription is installed once for the whole app, not per launch: a game
//! outlives the launch flow that started it, so the crash it may produce arrives
//! long after that flow — and its generation token — are gone. This is the
//! single `Sink` the composition root installs (see `launch::running`).
//!
//! The report button hands the collected strings to `usecases::crash_report`,
//! which writes one combined report into the launcher's logs; this adapter then
//! opens that file. The report contains the game's own report (or the collected
//! details), the captured console, `logs/latest.log` and the launcher log. The
//! note in the dialog asks the user to send that file, not a screenshot.

use std::rc::Rc;
use std::sync::Arc;

use launch::crash::{CrashReport, Reason};
use launch::running::{self, SessionEvent};
use shared::Sink;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::slint_backend::{App, CrashState, Navigation};
use crate::ui::services::app_config;
use crate::usecases::crash_report;

/// Installs the app's running-session subscriber and the dialog's callbacks.
pub(crate) fn setup(ui: &App) {
    // "Close" hides the prompt and returns to the page the crash interrupted.
    // A crash during launch leaves no `previous-page` behind (the launch view
    // does not set one), so the fallback is the game screen.
    let weak = ui.as_weak();
    ui.global::<CrashState>().on_close(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        ui.global::<CrashState>().set_visible(false);
        let navigation = ui.global::<Navigation>();
        if navigation.get_previous_page().is_empty() {
            navigation.set_current_page("game".into());
        } else {
            navigation.invoke_back();
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
    // sink silently and no crash would ever reach the dialog. Worth a line,
    // since `main` wires this once and a future caller might not.
    log::debug!("subscribing the crash dialog to the launch session events");
    let sink: Sink<SessionEvent> = Arc::new(move |event| {
        let SessionEvent::Exited {
            crash: Some(crash), ..
        } = event
        else {
            return;
        };
        crate::ui::services::report::report(&weak, move |ui| show(&ui, *crash));
    });
    running::subscribe(sink);
}

/// Pushes a collected crash into `CrashState` and opens the dialog.
fn show(ui: &App, crash: CrashReport) {
    let state = ui.global::<CrashState>();
    // The crash crate logs what it diagnosed; this is the line that says an
    // instance crashed and where to look, which a support log is read for.
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
    let categories = reason_categories(&crash.reasons);
    state.set_reason_categories(ModelRc::from(Rc::new(VecModel::from(
        categories
            .iter()
            .map(|category| SharedString::from(*category))
            .collect::<Vec<_>>(),
    ))));
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

/// The user-facing family for each matched rule, deduplicated.
fn reason_categories(reasons: &[Reason]) -> Vec<&'static str> {
    let mut categories = Vec::new();
    for reason in reasons {
        let category = category(reason.rule);
        if !categories.contains(&category) {
            categories.push(category);
        }
    }
    categories
}

/// Maps the analyzer's rule names to the sentence family `CrashText.reason`
/// knows. The raw rule name stays in the report; this is what the player sees.
fn category(rule: &str) -> &'static str {
    match rule {
        "OUT_OF_MEMORY" | "MEMORY_EXCEEDED" => "memory",
        "TOO_OLD_JAVA"
        | "NEED_JDK11"
        | "JDK_9"
        | "JAVA_VERSION_IS_TOO_HIGH"
        | "JVM_32BIT"
        | "OPENJ9"
        | "MAC_JDK_8U261" => "java",
        "MOD_RESOLUTION"
        | "MOD_RESOLUTION0"
        | "MOD_RESOLUTION_COLLECTION"
        | "MOD_RESOLUTION_CONFLICT"
        | "MOD_RESOLUTION_MISSING"
        | "MOD_RESOLUTION_MISSING_MINECRAFT"
        | "FORGEMOD_RESOLUTION"
        | "FORGE_FOUND_DUPLICATE_MODS"
        | "DUPLICATED_MOD"
        | "FABRIC_WARNINGS"
        | "LOADING_CRASHED_FABRIC"
        | "LOADING_CRASHED_FORGE"
        | "MIXIN_APPLY_MOD_FAILED"
        | "MODMIXIN_FAILURE"
        | "NO_CLASS_DEF_FOUND_ERROR"
        | "NO_SUCH_METHOD_ERROR"
        | "UNSATISFIED_LINK_ERROR"
        | "OPTIFINE_IS_NOT_COMPATIBLE_WITH_FORGE"
        | "OPTIFINE_CAUSES_THE_WORLD_TO_FAIL_TO_LOAD"
        | "JADE_FOREST_OPTIFINE"
        | "MOD_FOREST_OPTIFINE"
        | "PERFORMANT_FOREST_OPTIFINE"
        | "TWILIGHT_FOREST_OPTIFINE"
        | "NEOFORGE_FOREST_OPTIFINE"
        | "RTSS_FOREST_SODIUM"
        | "TOO_MANY_MODS_LEAD_TO_EXCEEDING_THE_ID_LIMIT" => "mods",
        "GRAPHICS_DRIVER"
        | "OPENGL_NOT_SUPPORTED"
        | "GL_OPERATION_FAILURE"
        | "RESOLUTION_TOO_HIGH"
        | "SHADERS_MOD" => "graphics",
        "BOOTSTRAP_FAILED"
        | "INCOMPLETE_FORGE_INSTALLATION"
        | "FORGE_REPEAT_INSTALLATION"
        | "OPTIFINE_REPEAT_INSTALLATION"
        | "FILE_ALREADY_EXISTS"
        | "FILE_CHANGED"
        | "MOD_FILES_ARE_DECOMPRESSED"
        | "CONFIG"
        | "INSTALL_MIXINBOOTSTRAP" => "files",
        _ => "unknown",
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
    use super::category;

    #[test]
    fn memory_rules_become_the_memory_reason() {
        assert_eq!(category("OUT_OF_MEMORY"), "memory");
        assert_eq!(category("MEMORY_EXCEEDED"), "memory");
    }

    #[test]
    fn mod_and_java_rules_become_their_families() {
        assert_eq!(category("MOD_RESOLUTION_MISSING"), "mods");
        assert_eq!(
            category("TOO_MANY_MODS_LEAD_TO_EXCEEDING_THE_ID_LIMIT"),
            "mods"
        );
        assert_eq!(category("TOO_OLD_JAVA"), "java");
        assert_eq!(category("OPENGL_NOT_SUPPORTED"), "graphics");
        assert_eq!(category("INCOMPLETE_FORGE_INSTALLATION"), "files");
    }

    #[test]
    fn unrecognised_rules_stay_unknown() {
        assert_eq!(category("ENTITY"), "unknown");
        assert_eq!(category("not-a-rule"), "unknown");
    }
}
