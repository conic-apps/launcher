// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The combined crash report written from the crash dialog.
//!
//! This is UI-neutral: the dialog hands over the strings it collected and gets
//! back a file path. The report puts the diagnosis first, then the evidence a
//! helper actually needs — the game's own report, the game console and
//! `latest.log`, and the launcher log tail. Keeping the composition here means
//! the dialog adapter only maps Slint state to plain data and opens the result.

use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// How many trailing lines of the launcher log the exported report keeps.
const LAUNCHER_LOG_TAIL_LINES: usize = 500;

/// The plain strings the dialog collected for one crash.
pub(crate) struct Source {
    pub(crate) instance_name: String,
    pub(crate) summary: String,
    pub(crate) exit: String,
    pub(crate) report_path: String,
    pub(crate) reasons: String,
    pub(crate) keywords: String,
    pub(crate) details: String,
    pub(crate) game_console: String,
    pub(crate) game_log: String,
}

/// Writes the report into `directory` and returns the file that was created.
pub(crate) fn write(directory: &Path, source: &Source) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(directory)?;
    let file = directory.join(format!(
        "crash-report-{}-{}.txt",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default(),
        std::process::id()
    ));
    std::fs::write(&file, text(source, directory))?;
    Ok(file)
}

fn text(source: &Source, directory: &Path) -> String {
    let mut report = String::from("Conic Launcher crash report\n");
    if !source.instance_name.is_empty() {
        report.push_str(&format!("Instance: {}\n", source.instance_name));
    }
    if !source.summary.is_empty() {
        report.push_str(&format!("Summary: {}\n", source.summary));
    }
    if !source.exit.is_empty() {
        report.push_str(&format!("Exit: {}\n", source.exit));
    }
    if !source.report_path.is_empty() {
        report.push_str(&format!("Game report: {}\n", source.report_path));
    }
    if !source.reasons.is_empty() {
        report.push_str(&format!("Reasons:\n{}\n", source.reasons));
    }
    if !source.keywords.is_empty() {
        report.push_str(&format!("Possible mods: {}\n", source.keywords));
    }

    if !source.game_console.is_empty() {
        report.push_str("\n--- game console (before the crash) ---\n");
        report.push_str(&source.game_console);
        if !source.game_console.ends_with('\n') {
            report.push('\n');
        }
    }

    if !source.game_log.is_empty() {
        report.push_str("\n--- game logs/latest.log ---\n");
        report.push_str(&source.game_log);
        if !source.game_log.ends_with('\n') {
            report.push('\n');
        }
    }

    if source.game_console.is_empty() && source.game_log.is_empty() {
        report.push_str("\n--- game output ---\n(no output was captured)\n");
    }

    let game_report = game_report(source);
    if game_report.is_empty() {
        report.push_str("\n--- game crash report ---\n(no crash report was saved)\n");
    } else {
        report.push_str("\n--- game crash report ---\n");
        report.push_str(&game_report);
        if !game_report.ends_with('\n') {
            report.push('\n');
        }
    }

    report.push_str("\n--- launcher log ---\n");
    let launcher = launcher_log(directory);
    if launcher.is_empty() {
        report.push_str("(empty)\n");
    } else {
        report.push_str(&launcher);
        if !launcher.ends_with('\n') {
            report.push('\n');
        }
    }
    report
}

/// The game's own report, read fresh so a file that is still there is not
/// limited to the 16 KiB `details` copy. The collected details are the fallback
/// when the game wrote no file or it has since gone.
fn game_report(source: &Source) -> String {
    if !source.report_path.is_empty()
        && let Ok(contents) = std::fs::read_to_string(&source.report_path)
    {
        return contents;
    }
    source.details.clone()
}

/// The in-memory run is the primary source; the file is only a fallback for a
/// logger that never formatted a record (the tests).
fn launcher_log(directory: &Path) -> String {
    let recent = crate::support::logs::recent(LAUNCHER_LOG_TAIL_LINES);
    if !recent.is_empty() {
        return recent;
    }
    let path = crate::support::logs::log_path(directory);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            tail_lines(&text, LAUNCHER_LOG_TAIL_LINES)
        }
        Err(error) => format!("(no launcher log at {}: {error})", path.display()),
    }
}

fn tail_lines(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::tail_lines;

    #[test]
    fn the_report_tail_keeps_the_newest_lines() {
        assert_eq!(tail_lines("a\nb\nc\n", 2), "b\nc");
        assert_eq!(tail_lines("only\n", 500), "only");
    }
}
