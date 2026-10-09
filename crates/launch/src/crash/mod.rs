// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Turning an abnormal Minecraft exit into something a screen can show.
//!
//! A crash leaves its evidence in a few places: what the game printed (captured
//! while it ran), a `crash-reports/*.txt` it may have written, and the tail of
//! `logs/latest.log`. None is guaranteed — a launcher, Java or JVM failure can
//! exit before the game writes anything — so the analysis is best-effort and
//! always returns a report, even if it is only the exit kind and code.
//!
//! The flow is a port of HMCL's (GPL-3.0): classify the exit from the code,
//! signal and the words on the console; find the crash report from the game's
//! own `#@!@#` location line or carve it out of the console; then run the
//! [`analyzer`] rule table over the output. The report stays *raw* in the sense
//! that reasons are named by rule and carry their capture groups rather than a
//! translated sentence — the screen maps them.

pub mod analyzer;

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

pub use analyzer::Reason;
use log::warn;

use instance::Instance;
use storage::LOCATIONS;

/// The most raw detail kept for the UI. A crash report is often hundreds of
/// kilobytes of stack trace; the crude viewer shows a head of it.
const DETAILS_LIMIT: usize = 16 * 1024;

/// How many trailing lines of `logs/latest.log` are kept when the game wrote no
/// crash report.
const LOG_TAIL_LINES: usize = 100;

/// How the game's process ended, mirroring HMCL's `ExitWaiter.ExitType`.
///
/// The distinction matters: an `APPLICATION_ERROR` may have written a report and
/// a fix, while a `JVM_ERROR` usually means the arguments or the Java install.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitType {
    /// The JVM refused to start (bad heap size, unsupported option, 32-bit).
    JvmError,
    /// The game or a mod threw. Covers crashes that still exit `0`.
    ApplicationError,
    /// The process was killed (OOM killer, the user, the OS).
    Sigkill,
    /// A clean exit.
    Normal,
    /// We asked it to stop, or lost track of it.
    Interrupted,
}

impl ExitType {
    /// A stable, untranslated name for the screen and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            ExitType::JvmError => "JVM_ERROR",
            ExitType::ApplicationError => "APPLICATION_ERROR",
            ExitType::Sigkill => "SIGKILL",
            ExitType::Normal => "NORMAL",
            ExitType::Interrupted => "INTERRUPTED",
        }
    }
}

/// Words on the console that mean the JVM itself failed to start.
const JVM_ERROR_MARKERS: &[&str] = &[
    "Could not create the Java Virtual Machine.",
    "Error occurred during initialization of VM",
    "A fatal exception has occurred. Program will exit.",
    "Unrecognized option:",
    "Unrecognized VM option",
];

/// Words on the console that mean the game or a mod threw, even on exit `0`.
const APPLICATION_ERROR_MARKERS: &[&str] = &[
    "Crash report saved to",
    "Could not save crash report to",
    "This crash report has been saved to:",
    "Unable to launch",
    "An exception was thrown, the game will display an error screen and halt.",
];

/// Classifies how a process ended, the way HMCL's `ExitWaiter` does.
///
/// `exit_code` is the platform code (`None` on a Unix signal death), `signal`
/// the terminating signal when there was one, and `stopped` whether we asked the
/// process to stop. The console `log` is scanned for the crash markers so that a
/// game which prints a crash report and then exits `0` is still an error.
pub fn classify_exit(
    exit_code: Option<i32>,
    signal: Option<i32>,
    stopped: bool,
    log: &str,
) -> ExitType {
    if stopped {
        return ExitType::Interrupted;
    }
    let has = |markers: &[&str]| markers.iter().any(|marker| log.contains(marker));
    let failed = !matches!(exit_code, Some(0)) || signal.is_some();
    if failed && has(JVM_ERROR_MARKERS) {
        return ExitType::JvmError;
    }
    if failed || has(APPLICATION_ERROR_MARKERS) {
        // A SIGKILL is the OOM killer or the OS, not the game's fault; Linux and
        // the BSDs report it either as the 128+9 shell code or as signal 9.
        if signal == Some(9) || (cfg!(unix) && exit_code == Some(137)) {
            return ExitType::Sigkill;
        }
        return ExitType::ApplicationError;
    }
    ExitType::Normal
}

/// A crash, reduced to what a screen can render.
///
/// Every field is a plain `String` (or an `Option` thereof) so the report
/// crosses a thread and reaches Slint, which cannot format anything itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrashReport {
    /// The id of the instance that crashed.
    pub instance_id: String,
    /// The instance's display name.
    pub instance_name: String,
    /// How the process ended.
    pub exit_type: ExitType,
    /// The process's exit code, when the platform reported one. A Unix signal
    /// death has none.
    pub exit_code: Option<i32>,
    /// The crash report's `Description:` line, or the first exception line, or a
    /// fallback sentence built from the exit kind.
    pub summary: String,
    /// The raw crash report, or the tail of the console, capped to a bounded
    /// head.
    pub details: String,
    /// The crash report file the details came from, when one was found.
    pub report_path: Option<String>,
    /// The rules the output matched, in the table's order.
    pub reasons: Vec<Reason>,
    /// Likely offending mods, inferred from the report's stack trace.
    pub keywords: Vec<String>,
}

/// Collects the crash information left behind by `instance`'s last run.
///
/// `console_log` is everything the process printed (stdout and stderr), which
/// the running-instance watcher captured. It is the primary evidence: it holds
/// the game's crash-report location line and the exception text even when no
/// report file was written.
pub fn analyze(
    instance: &Instance,
    exit_type: ExitType,
    exit_code: Option<i32>,
    console_log: &str,
) -> CrashReport {
    let root = LOCATIONS.instances.get_instance_root(&instance.id);
    let latest_log_path = root.join("logs").join("latest.log");
    let latest_log = match std::fs::read_to_string(&latest_log_path) {
        Ok(log) => log,
        Err(error) => {
            // Not a missing file: a missing `latest.log` is normal for a game that
            // died before it could write one, which is worth saying — it is the
            // difference between "no crash report" and "no evidence at all".
            log::debug!(
                "No game log at {} ({error}); the crash report will rest on the console \
                 output alone",
                latest_log_path.display()
            );
            String::new()
        }
    };

    // Prefer the exact path the game printed, then the one in its log, then the
    // report carved out of either, then, as a last resort, the newest file.
    let named = analyzer::find_crash_report(console_log)
        .or_else(|| analyzer::find_crash_report(&latest_log));
    let extracted = analyzer::extract_crash_report(console_log)
        .or_else(|| analyzer::extract_crash_report(&latest_log));
    let (report_path, contents) = if let Some((path, contents)) = named {
        log::debug!("The game named its crash report: {}", path.display());
        (Some(path), contents)
    } else if let Some(contents) = extracted {
        log::debug!("The crash report was carved out of the output the game printed");
        (None, contents)
    } else if let Some((path, contents)) = newest_crash_report(&root) {
        log::debug!(
            "No crash report was named; using the newest one at {}",
            path.display()
        );
        (Some(path), contents)
    } else {
        log::debug!("No crash report at all; falling back to the tail of the game log");
        (None, latest_log.clone())
    };

    let mut reasons = analyzer::analyze(console_log);
    for reason in analyzer::analyze(&latest_log) {
        if !reasons.iter().any(|existing| existing.rule == reason.rule) {
            reasons.push(reason);
        }
    }

    let summary = description(&contents).unwrap_or_else(|| fallback_summary(exit_type, exit_code));
    let details = if contents.trim().is_empty() {
        truncate(console_log, DETAILS_LIMIT)
    } else {
        truncate(&contents, DETAILS_LIMIT)
    };
    // The one line that says what happened, to an instance, with an exit code —
    // `running.rs` logs the exit and the app logs the panel, but neither of those
    // says what was actually diagnosed.
    log::error!(
        "'{}' ended abnormally: {} ({}){}",
        instance.config.name,
        summary,
        exit_type.as_str(),
        match exit_code {
            Some(code) => format!(" [exit code {code}]"),
            None => String::new(),
        }
    );

    CrashReport {
        instance_id: instance.id.clone(),
        instance_name: instance.config.name.clone(),
        exit_type,
        exit_code,
        summary,
        details,
        report_path: report_path.map(|path| path.to_string_lossy().into_owned()),
        reasons,
        keywords: analyzer::find_keywords(&contents),
    }
}

/// A summary sentence for when the game left no crash report.
fn fallback_summary(exit_type: ExitType, exit_code: Option<i32>) -> String {
    match exit_code {
        Some(code) => format!("Minecraft exited with code {code} ({})", exit_type.as_str()),
        None => format!("Minecraft ended abnormally ({})", exit_type.as_str()),
    }
}

/// The newest `crash-reports/*.txt`, with its contents.
///
/// Only a fallback, for when the console named no report (a very old game, or
/// output we failed to capture). Newest by modified time rather than by name:
/// the game's own names are timestamped, but a copied or restored report would
/// sort wrong.
fn newest_crash_report(root: &Path) -> Option<(PathBuf, String)> {
    let entries = std::fs::read_dir(root.join("crash-reports")).ok()?;
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("txt") {
            continue;
        }
        let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) else {
            continue;
        };
        if newest.as_ref().is_none_or(|(best, _)| modified > *best) {
            newest = Some((modified, path));
        }
    }
    let (_, path) = newest?;
    match std::fs::read_to_string(&path) {
        Ok(contents) => Some((path, contents)),
        Err(error) => {
            warn!("could not read crash report {}: {error}", path.display());
            None
        }
    }
}

/// The crash report's `Description:` line, or the first line that names an
/// exception or error.
fn description(contents: &str) -> Option<String> {
    for line in contents.lines() {
        if let Some(rest) = line.trim().strip_prefix("Description:") {
            return Some(rest.trim().to_string());
        }
    }
    for line in contents.lines().take(200) {
        let trimmed = line.trim();
        if trimmed.contains("Exception") || trimmed.contains("Error") {
            return Some(trimmed.to_string());
        }
    }
    None
}

/// The last [`LOG_TAIL_LINES`] lines of `<root>/logs/latest.log`, oldest first.
///
/// Kept for tests and callers that have no captured console; [`analyze`] reads
/// the whole log instead so the analyzer can scan it.
#[allow(dead_code)]
pub(crate) fn log_tail(root: &Path) -> Vec<String> {
    let Ok(file) = File::open(root.join("logs").join("latest.log")) else {
        return Vec::new();
    };
    let reader = BufReader::new(file);
    let mut tail: Vec<String> = Vec::with_capacity(LOG_TAIL_LINES);
    for line in reader.lines().map_while(Result::ok) {
        if tail.len() == LOG_TAIL_LINES {
            tail.remove(0);
        }
        tail.push(line);
    }
    tail
}

/// Caps `text` to `limit` bytes without splitting a character.
fn truncate(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n… (truncated)", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_line_is_preferred() {
        let report = "---- Minecraft Crash Report ----\n// Oops\n\nDescription: Rendering overlay\n\njava.lang.RuntimeException: boom\n";
        assert_eq!(description(report), Some("Rendering overlay".to_string()));
    }

    #[test]
    fn the_first_exception_is_the_fallback_summary() {
        let report = "---- Minecraft Crash Report ----\n\njava.lang.NullPointerException: boom\n\tat a.b(C)\n";
        assert_eq!(
            description(report),
            Some("java.lang.NullPointerException: boom".to_string())
        );
    }

    #[test]
    fn a_clean_exit_is_normal() {
        assert_eq!(classify_exit(Some(0), None, false, ""), ExitType::Normal);
    }

    #[test]
    fn a_nonzero_exit_without_markers_is_an_application_error() {
        assert_eq!(
            classify_exit(Some(1), None, false, "boom"),
            ExitType::ApplicationError
        );
    }

    #[test]
    fn a_bad_vm_option_is_a_jvm_error() {
        let log =
            "Unrecognized option: -XX:+NotARealFlag\nCould not create the Java Virtual Machine.";
        assert_eq!(classify_exit(Some(1), None, false, log), ExitType::JvmError);
    }

    #[test]
    fn an_exit_zero_with_a_crash_marker_is_an_application_error() {
        let log = "#@!@# Game crashed! Crash report saved to: #@!@# /tmp/x.txt";
        assert_eq!(
            classify_exit(Some(0), None, false, log),
            ExitType::ApplicationError
        );
    }

    #[test]
    fn a_signal_nine_is_sigkill() {
        assert_eq!(classify_exit(None, Some(9), false, ""), ExitType::Sigkill);
    }

    #[test]
    fn a_stop_request_is_interrupted() {
        assert_eq!(
            classify_exit(Some(1), None, true, "boom"),
            ExitType::Interrupted
        );
    }

    #[test]
    fn truncation_keeps_character_boundaries() {
        let text = "äöü";
        // Cut mid-'ö' (after the first byte of the second character) must back
        // off to the previous boundary rather than panic.
        let truncated = truncate(text, 3);
        assert!(truncated.starts_with('ä'));
        assert!(truncated.ends_with("… (truncated)"));
    }
}
