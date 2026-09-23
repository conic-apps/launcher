// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Minimal Java runtime discovery for the settings page.
//!
//! The full scanner lives in `crates/java-runtime` (a Tauri plugin, ~1500 LOC).
//! This is a lightweight, Tauri-free subset: it probes `JAVA_HOME`, `PATH`, the
//! launcher-managed runtime folder and the common installation directories per
//! platform, then reads `java -version` for the major version, vendor and
//! architecture. It exists so the JVM settings page is functional without
//! pulling in the whole domain crate.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone)]
pub struct DetectedJava {
    pub major: i32,
    pub vendor: String,
    pub version: String,
    pub arch: String,
    pub path: PathBuf,
}

impl DetectedJava {
    /// Raw executable path as stored in `disabled_java_runtime`.
    pub fn path_string(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

fn exe_name() -> &'static str {
    if cfg!(windows) { "java.exe" } else { "java" }
}

fn push_if_file(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() {
        candidates.push(path);
    }
}

fn push_from_home(candidates: &mut Vec<PathBuf>, home: &Path) {
    push_if_file(candidates, home.join("bin").join(exe_name()));
}

fn push_glob(candidates: &mut Vec<PathBuf>, dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        push_from_home(candidates, &entry.path());
    }
}

/// Lists the Java executables found on the system.
pub fn scan(managed_dir: &Path) -> Vec<DetectedJava> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        push_from_home(&mut candidates, Path::new(&java_home));
    }

    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            push_if_file(&mut candidates, dir.join(exe_name()));
        }
    }

    push_glob(&mut candidates, managed_dir);

    #[cfg(target_os = "macos")]
    {
        push_glob(
            &mut candidates,
            Path::new("/Library/Java/JavaVirtualMachines"),
        );
        for entry in std::fs::read_dir("/Library/Java/JavaVirtualMachines")
            .into_iter()
            .flatten()
            .flatten()
        {
            push_from_home(&mut candidates, &entry.path().join("Contents").join("Home"));
        }
        push_glob(
            &mut candidates,
            Path::new("/System/Library/Java/JavaVirtualMachines"),
        );
    }

    #[cfg(target_os = "linux")]
    {
        push_glob(&mut candidates, Path::new("/usr/lib/jvm"));
        push_glob(&mut candidates, Path::new("/usr/java"));
        if let Ok(home) = std::env::var("HOME") {
            push_glob(&mut candidates, &Path::new(&home).join(".jdks"));
            push_glob(
                &mut candidates,
                &Path::new(&home).join(".sdkman/candidates/java"),
            );
        }
    }

    #[cfg(target_os = "windows")]
    {
        for key in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
            if let Ok(base) = std::env::var(key) {
                let base = PathBuf::from(base);
                push_glob(&mut candidates, &base.join("Java"));
                push_glob(&mut candidates, &base.join("Eclipse Adoptium"));
                push_glob(&mut candidates, &base.join("Microsoft"));
                push_glob(&mut candidates, &base.join("Amazon Corretto"));
                push_glob(&mut candidates, &base.join("Zulu"));
                push_glob(&mut candidates, &base.join("BellSoft"));
            }
        }
    }

    // Deduplicate by canonical path and probe each executable.
    let mut seen = HashSet::new();
    let mut runtimes = Vec::new();
    for candidate in candidates {
        let canonical = std::fs::canonicalize(&candidate).unwrap_or(candidate.clone());
        if !seen.insert(canonical.clone()) {
            continue;
        }
        if let Some(runtime) = probe(&canonical) {
            runtimes.push(runtime);
        }
    }

    runtimes.sort_by(|a, b| b.major.cmp(&a.major).then_with(|| a.path.cmp(&b.path)));
    runtimes
}

fn probe(executable: &Path) -> Option<DetectedJava> {
    let output = Command::new(executable)
        .arg("-XshowSettings:properties")
        .arg("-version")
        .output()
        .or_else(|_| Command::new(executable).arg("-version").output())
        .ok()?;
    // `java -version` writes to stderr.
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );

    let version = parse_quoted_version(&text);
    let major = parse_major(version.as_deref().unwrap_or(""));
    if major <= 0 {
        return None;
    }

    Some(DetectedJava {
        major,
        vendor: parse_vendor(&text),
        version: version.unwrap_or_else(|| major.to_string()),
        arch: parse_arch(&text),
        path: executable.to_path_buf(),
    })
}

fn parse_quoted_version(text: &str) -> Option<String> {
    let marker = "version \"";
    let start = text.find(marker)? + marker.len();
    let rest = &text[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_major(version: &str) -> i32 {
    let mut parts = version.split(['.', '_', '-']);
    match parts.next() {
        Some("1") => parts.next().and_then(|p| p.parse().ok()).unwrap_or(0),
        Some(first) => first.parse().unwrap_or(0),
        None => 0,
    }
}

fn parse_arch(text: &str) -> String {
    if let Some(line) = text
        .lines()
        .find(|line| line.trim_start().starts_with("sun.arch.data.model"))
        && let Some(value) = line.split('=').nth(1)
    {
        return match value.trim() {
            "64" => "x64".to_string(),
            "32" => "x86".to_string(),
            other => other.to_string(),
        };
    }
    if text.contains("64-Bit") {
        "x64".to_string()
    } else if text.contains("32-Bit") {
        "x86".to_string()
    } else {
        String::new()
    }
}

fn parse_vendor(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let vendor = if lower.contains("temurin") || lower.contains("adoptium") {
        "Eclipse Adoptium"
    } else if lower.contains("corretto") {
        "Amazon Corretto"
    } else if lower.contains("zulu") {
        "Azul Zulu"
    } else if lower.contains("liberica") {
        "BellSoft Liberica"
    } else if lower.contains("semeru") {
        "IBM Semeru"
    } else if lower.contains("microsoft") {
        "Microsoft"
    } else if lower.contains("dragonwell") {
        "Alibaba Dragonwell"
    } else if lower.contains("sapmachine") {
        "SAP"
    } else if lower.contains("openjdk") {
        "OpenJDK"
    } else if lower.contains("oracle") || lower.contains("java(tm)") {
        "Oracle"
    } else {
        "Unknown"
    };
    vendor.to_string()
}
