// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Filesystem scanning for installed Java runtimes.
//!
//! Design inspired by HMCL's `JavaManager#searchPotentialJavaExecutables`:
//! candidates are collected from the environment (`JAVA_HOME`, `PATH`),
//! platform specific locations (Windows registry and Program Files, Linux
//! `/usr/lib/jvm`, macOS `JavaVirtualMachines` and Homebrew), Minecraft's
//! bundled runtime folders and launcher-managed runtime roots. Each candidate
//! is validated by reading its `release` file and running
//! `java -XshowSettings:properties -version`, then deduplicated by its
//! canonical path.

use std::{
    collections::HashMap,
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use log::{debug, warn};

use crate::{
    error::Result,
    models::{JavaRuntime, ScanOptions},
    parser::{
        JavaInfoRaw, normalize_arch, normalize_vendor, parse_java_output, parse_major_version,
        parse_release_file,
    },
};

/// How long a single `java -XshowSettings:properties -version` probe may run
/// before it is killed.
const JAVA_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// Scans the system for installed Java runtimes using default options.
///
/// See [`scan_java_runtimes_with`] for customization.
pub fn scan_java_runtimes() -> Result<Vec<JavaRuntime>> {
    scan_java_runtimes_with(&ScanOptions::default())
}

/// Scans the system for installed Java runtimes.
///
/// Returns runtimes sorted by major version (newest first). Runtimes that could
/// be identified (a version string was obtained) are returned; broken or
/// misconfigured executables are still included with `is_valid == false` so the
/// UI can surface them.
pub fn scan_java_runtimes_with(options: &ScanOptions) -> Result<Vec<JavaRuntime>> {
    let candidates = collect_candidates(options);
    debug!("Found {} Java candidates", candidates.len());

    let mut runtimes = Vec::new();
    let mut seen: HashMap<PathBuf, ()> = HashMap::new();
    for candidate in candidates {
        let Some(canonical) = canonicalize(&candidate) else {
            continue;
        };
        if seen.contains_key(&canonical) {
            continue;
        }
        if let Some(runtime) = probe_java(&canonical, options) {
            seen.insert(canonical, ());
            runtimes.push(runtime);
        }
    }

    runtimes.sort();
    Ok(runtimes)
}

/// Resolves the canonical (symlink-free) path of `path`.
fn canonicalize(path: &Path) -> Option<PathBuf> {
    match fs::canonicalize(path) {
        Ok(path) => Some(path),
        Err(error) => {
            debug!("Cannot canonicalize {}: {error}", path.display());
            None
        }
    }
}

/// Validates a candidate executable and gathers its metadata.
///
/// Mirrors HMCL's `tryAddJavaHome` flow: the `release` file in the Java home is
/// read first (cheap), then the executable is run to confirm it works and to
/// fill in anything the release file lacked.
fn probe_java(executable: &Path, options: &ScanOptions) -> Option<JavaRuntime> {
    let resolved_home = resolve_java_home(executable);

    let mut release_raw = JavaInfoRaw::default();
    if let Some(home) = &resolved_home {
        let release = home.join("release");
        if release.is_file() {
            match fs::read_to_string(&release) {
                Ok(content) => release_raw.merge(parse_release_file(&content)),
                Err(error) => warn!("Failed to read release file {}: {error}", release.display()),
            }
        }
    }

    let probe_raw = run_java_probe(executable);

    let mut raw = release_raw;
    if let Some(probe) = &probe_raw {
        raw.merge(probe.clone());
    }

    let version = raw.version?;
    let major_version = raw
        .major_version
        .or_else(|| parse_major_version(&version))?;

    let java_home = raw.java_home.clone().or(resolved_home);
    let is_jdk = java_home.as_ref().is_some_and(|home| {
        home.join("bin").join(javac_executable_name()).is_file()
            || home.join("bin").join("jar").is_file()
    });
    let is_managed = options
        .managed_dirs
        .iter()
        .any(|dir| java_home.as_ref().is_some_and(|home| home.starts_with(dir)));

    Some(JavaRuntime {
        path: executable.to_path_buf(),
        java_home,
        major_version,
        version,
        vendor: normalize_vendor(raw.vendor.as_deref().unwrap_or("")),
        arch: normalize_arch(raw.arch.as_deref().unwrap_or("")),
        is_jdk,
        is_managed,
        is_valid: probe_raw.is_some(),
    })
}

/// Runs `java -XshowSettings:properties -version` on the candidate and parses
/// its output. Returns `None` when the executable cannot run or its output does
/// not look like a Java runtime.
fn run_java_probe(executable: &Path) -> Option<JavaInfoRaw> {
    let mut child = match Command::new(executable)
        .args(["-XshowSettings:properties", "-version"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            debug!(
                "Cannot execute Java probe {}: {error}",
                executable.display()
            );
            return None;
        }
    };

    // Drain both pipes on background threads so the probe can never deadlock on
    // a full pipe buffer.
    let stdout_reader = child
        .stdout
        .take()
        .map(|out| thread::spawn(move || read_all(out)));
    let stderr_reader = child
        .stderr
        .take()
        .map(|err| thread::spawn(move || read_all(err)));

    let start = Instant::now();
    let exited = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(error) => {
                debug!(
                    "Failed to wait on Java probe {}: {error}",
                    executable.display()
                );
                break None;
            }
        }
        if start.elapsed() > JAVA_PROBE_TIMEOUT {
            debug!("Java probe timed out for {}", executable.display());
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(20));
    };
    let _ = exited?;

    let stdout = stdout_reader
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let stderr = stderr_reader
        .and_then(|h| h.join().ok())
        .unwrap_or_default();
    let output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );

    let info = parse_java_output(&output);
    let is_java_output = info.version.is_some()
        || info.java_home.is_some()
        || info.vendor.is_some()
        || info.arch.is_some();
    if is_java_output { Some(info) } else { None }
}

fn read_all(mut reader: impl Read) -> Vec<u8> {
    let mut buffer = Vec::new();
    let _ = reader.read_to_end(&mut buffer);
    buffer
}

/// Derives a Java home directory from an executable path.
///
/// Handles the standard `<home>/bin/java` layout as well as macOS
/// `<home>/jre.bundle/Contents/Home/bin/java` (where the home is the `Home`
/// directory that contains the `release` file).
pub fn resolve_java_home(executable: &Path) -> Option<PathBuf> {
    let bin = executable.parent()?;
    if bin.file_name()?.to_string_lossy() != "bin" {
        return None;
    }
    Some(bin.parent()?.to_path_buf())
}

fn java_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "java.exe"
    } else {
        "java"
    }
}

fn javac_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "javac.exe"
    } else {
        "javac"
    }
}

fn path_separator() -> char {
    if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    }
}

#[allow(dead_code)]
fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let home = env::var("USERPROFILE").ok();
    #[cfg(not(target_os = "windows"))]
    let home = env::var("HOME").ok();
    home.filter(|home| !home.is_empty()).map(PathBuf::from)
}

fn collect_candidates(options: &ScanOptions) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(java_home) = env::var("JAVA_HOME")
        && !java_home.is_empty()
    {
        push_home_candidate(&mut candidates, Path::new(&java_home));
    }

    if let Ok(path) = env::var("PATH") {
        for dir in path.split(path_separator()) {
            if !dir.is_empty() {
                candidates.push(Path::new(dir).join(java_executable_name()));
            }
        }
    }

    #[cfg(target_os = "windows")]
    push_windows_candidates(&mut candidates);
    #[cfg(target_os = "macos")]
    push_macos_candidates(&mut candidates);
    #[cfg(target_os = "linux")]
    push_linux_candidates(&mut candidates);

    push_minecraft_candidates(&mut candidates);

    for home in &options.extra_home_dirs {
        push_home_candidate(&mut candidates, home);
    }
    for managed in &options.managed_dirs {
        push_homes_recursive(&mut candidates, managed, 4);
    }

    candidates
}

/// Adds `<home>/bin/java` to the candidate list if it exists.
fn push_home_candidate(candidates: &mut Vec<PathBuf>, home: &Path) {
    let executable = home.join("bin").join(java_executable_name());
    if executable.is_file() {
        candidates.push(executable);
    }
}

/// Pushes `<home>/bin/java` (or the macOS `jre.bundle` variant) to
/// `candidates` when `dir` is a Java home. Returns `true` if a home was found.
fn try_push_home(candidates: &mut Vec<PathBuf>, dir: &Path) -> bool {
    let direct = dir.join("bin").join(java_executable_name());
    if direct.is_file() {
        candidates.push(direct);
        return true;
    }
    let bundle = dir
        .join("jre.bundle")
        .join("Contents")
        .join("Home")
        .join("bin")
        .join(java_executable_name());
    if bundle.is_file() {
        candidates.push(bundle);
        return true;
    }
    false
}

/// Recursively collects Java homes below `root`, stopping a branch as soon as a
/// home is found. `max_depth` bounds how far each branch is descended.
fn push_homes_recursive(candidates: &mut Vec<PathBuf>, root: &Path, max_depth: usize) {
    if max_depth == 0 {
        return;
    }
    if try_push_home(candidates, root) {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            push_homes_recursive(candidates, &path, max_depth - 1);
        }
    }
}

/// Treats every immediate sub-directory of `root` as a potential Java home.
fn push_home_subdirs(candidates: &mut Vec<PathBuf>, root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            push_home_candidate(candidates, &path);
        }
    }
}

/// Treats every immediate sub-directory of `root` as a macOS Java home bundle.
#[cfg(target_os = "macos")]
fn push_mac_jvm_candidates(candidates: &mut Vec<PathBuf>, root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            push_home_candidate(candidates, &path.join("Contents").join("Home"));
        }
    }
}

#[cfg(target_os = "windows")]
fn push_windows_candidates(candidates: &mut Vec<PathBuf>) {
    query_windows_registry(candidates);

    for env_name in ["ProgramFiles", "ProgramFiles(x86)", "ProgramFiles(ARM)"] {
        let Some(program_files) = env::var(env_name).ok() else {
            continue;
        };
        if program_files.is_empty() {
            continue;
        }
        let root = PathBuf::from(program_files);
        for vendor in [
            "Java",
            "BellSoft",
            "AdoptOpenJDK",
            "Zulu",
            "Microsoft",
            "Eclipse Foundation",
            "Semeru",
        ] {
            push_home_subdirs(candidates, &root.join(vendor));
        }
    }
}

/// Queries the well-known JavaSoft registry keys through `reg.exe`, which is
/// part of every supported Windows version (HMCL performs the same queries via
/// the Win32 registry API).
#[cfg(target_os = "windows")]
fn query_windows_registry(candidates: &mut Vec<PathBuf>) {
    let output = match Command::new("reg")
        .args(["query", "HKLM\\SOFTWARE\\JavaSoft", "/s"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    {
        Ok(output) => output,
        Err(_) => return,
    };
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let line = line.trim();
        // e.g. `JavaHome    REG_SZ    C:\Program Files\Java\jre-17.0.9`
        if !line.to_ascii_uppercase().contains("JAVAHOME") {
            continue;
        }
        let Some(value) = line.split("REG_SZ").nth(1) else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        if !value.is_empty() {
            push_home_candidate(candidates, Path::new(value));
        }
    }
}

#[cfg(target_os = "linux")]
fn push_linux_candidates(candidates: &mut Vec<PathBuf>) {
    for root in [
        Path::new("/usr/java"), // Oracle RPM installs
        Path::new("/usr/lib/jvm"),
        Path::new("/usr/lib32/jvm"),
        Path::new("/usr/lib64/jvm"),
    ] {
        push_home_subdirs(candidates, root);
    }
    if let Some(home) = home_dir() {
        // SDKMAN!
        push_home_subdirs(
            candidates,
            &home.join(".sdkman").join("candidates").join("java"),
        );
        // JetBrains Toolbox / IntelliJ SDKs
        push_home_subdirs(candidates, &home.join(".jdks"));
    }
}

#[cfg(target_os = "macos")]
fn push_macos_candidates(candidates: &mut Vec<PathBuf>) {
    push_mac_jvm_candidates(candidates, Path::new("/Library/Java/JavaVirtualMachines"));
    if let Some(home) = home_dir() {
        push_mac_jvm_candidates(
            candidates,
            &home
                .join("Library")
                .join("Java")
                .join("JavaVirtualMachines"),
        );
    }

    // Legacy Java applet plugin
    push_home_candidate(
        candidates,
        Path::new("/Library/Internet Plug-Ins/JavaAppletPlugin.plugin/Contents/Home"),
    );

    // Homebrew: `Cellar/<formula>/<version>` and the `opt/openjdk@*` symlinks
    push_homes_recursive(candidates, Path::new("/opt/homebrew/Cellar"), 3);
    push_homes_recursive(candidates, Path::new("/usr/local/Cellar"), 3);
    push_home_subdirs(candidates, Path::new("/opt/homebrew/opt"));
    push_home_subdirs(candidates, Path::new("/usr/local/opt"));
}

fn minecraft_runtime_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            roots.push(
                PathBuf::from(local_app_data)
                    .join("Packages")
                    .join("Microsoft.4297127D64EC6_8wekyb3d8bbwe")
                    .join("LocalCache")
                    .join("Local")
                    .join("runtime"),
            );
        }
        if let Ok(program_files) = env::var("ProgramFiles(x86)") {
            roots.push(
                PathBuf::from(program_files)
                    .join("Minecraft Launcher")
                    .join("runtime"),
            );
        }
    }
    #[cfg(target_os = "linux")]
    if let Some(home) = home_dir() {
        roots.push(home.join(".minecraft").join("runtime"));
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = home_dir() {
        roots.push(
            home.join("Library")
                .join("Application Support")
                .join("minecraft")
                .join("runtime"),
        );
    }
    roots
}

/// Minecraft's own bundled Java runtimes, in HMCL's official-runtime layout:
/// `<root>/<component>/<platform>/<component>` (macOS bundles live under
/// `<home>/jre.bundle/Contents/Home`).
fn push_minecraft_candidates(candidates: &mut Vec<PathBuf>) {
    for root in minecraft_runtime_roots() {
        push_homes_recursive(candidates, &root, 4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_standard_home() {
        let home = resolve_java_home(Path::new("/usr/lib/jvm/java-17/bin/java"));
        assert_eq!(home, Some(PathBuf::from("/usr/lib/jvm/java-17")));
    }

    #[test]
    fn resolves_mac_bundle_home() {
        let home = resolve_java_home(Path::new(
            "/Library/Java/JavaVirtualMachines/jdk-17.jdk/Contents/Home/bin/java",
        ));
        assert_eq!(
            home,
            Some(PathBuf::from(
                "/Library/Java/JavaVirtualMachines/jdk-17.jdk/Contents/Home"
            ))
        );
    }

    #[test]
    fn does_not_resolve_non_standard_home() {
        assert_eq!(
            resolve_java_home(Path::new("/opt/custom/javabin/java")),
            None
        );
    }
}
