// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Bakes in the Conic Nexus app API key and registers it with the API server.
//!
//! The key is derived from `BUILD_API_KEY` and the build's identity rather than
//! generated randomly: the release builds one version on six separate runners,
//! and a random key per runner would leave five binaries holding a key the KV
//! store no longer has. Deriving it makes every runner of one build compute the
//! same key, so registering is idempotent.
//!
//! Without `BUILD_API_KEY` — a local `cargo build`, `cargo test`, `cargo clippy`
//! — there is nothing to derive from and no registration is attempted; the crate
//! is told so through the same empty `CONIC_NEXUS_API_KEY` the CurseForge crate
//! uses for its missing key.

use std::{env, fmt::Write as _, fs, path::Path, process::Command};

use sha2::{Digest, Sha256};

/// The internal endpoint that adds a key to the KV store.
const REGISTER_ENDPOINT: &str = "https://api.conicmc.app/internal/apikeys";

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_API_KEY");
    println!("cargo:rerun-if-env-changed=CONIC_APP_VERSION");
    println!("cargo:rerun-if-env-changed=CONIC_COMMIT");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");

    let build_key = env::var("BUILD_API_KEY").unwrap_or_default();
    let build_key = build_key.trim();
    if build_key.is_empty() {
        println!(
            "cargo:warning=BUILD_API_KEY is not set; the Conic Nexus node list will be requested without an API key"
        );
        println!("cargo:rustc-env=CONIC_NEXUS_API_KEY=");
        return;
    }

    let name = build_name(&app_version());
    let key = derive_key(build_key, &name);
    println!("cargo:rustc-env=CONIC_NEXUS_API_KEY={key}");

    match register(build_key, &name, &key) {
        Ok(()) => println!("cargo:warning=registered the Conic Nexus API key for {name}"),
        Err(error) => {
            println!("cargo:warning=could not register the Conic Nexus API key for {name}: {error}")
        }
    }
}

/// `<version>-<date>-<commit>`, the identity every runner of one build agrees
/// on. The version is unchanged between releases, so the date and commit are
/// what keep two builds of the same version distinct.
fn build_name(version: &str) -> String {
    format!("{version}-{}-{}", utc_date(), commit())
}

/// Today's date in UTC as `YYYYMMDD`.
///
/// Hand-rolled rather than reached for a date crate: `chrono`'s `clock` feature
/// drags `iana-time-zone` and its per-platform shims into the build script for
/// one `format!`, and a build script is the wrong place to grow a dependency
/// tree. This is Howard Hinnant's `civil_from_days`.
fn utc_date() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}{month:02}{day:02}")
}

/// The app version, read from `app/Cargo.toml` — the same file the release
/// workflow reads it from. `CONIC_APP_VERSION` overrides it for a caller that
/// already knows.
fn app_version() -> String {
    if let Ok(version) = env::var("CONIC_APP_VERSION") {
        let version = version.trim();
        if !version.is_empty() {
            return version.to_string();
        }
    }

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/Cargo.toml");
    // A version bump is exactly what a release is, so the rebuilt name has to
    // follow it.
    println!("cargo:rerun-if-changed={}", manifest.display());

    let version = fs::read_to_string(&manifest)
        .ok()
        .and_then(|text| package_version(&text));
    version.unwrap_or_else(|| {
        println!(
            "cargo:warning=could not read a version from {}",
            manifest.display()
        );
        "0.0.0".to_string()
    })
}

/// The `version` key of the `[package]` table, ignoring every other section so
/// a dependency's inline `version = "…"` cannot be mistaken for it.
fn package_version(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if in_package
            && let Some(value) = line.strip_prefix("version")
            && let Some(value) = value.trim_start().strip_prefix('=')
        {
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// The commit the build is from. CI exports `GITHUB_SHA`; a local build asks
/// git. A tree without either (a source tarball) yields `unknown`.
fn commit() -> String {
    for variable in ["CONIC_COMMIT", "GITHUB_SHA"] {
        if let Ok(value) = env::var(variable) {
            let short: String = value.trim().chars().take(7).collect();
            if !short.is_empty() {
                return short;
            }
        }
    }
    Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// The app key for `name`: `SHA-256(BUILD_API_KEY || ":" || name)` in hex. It is
/// public the moment it ships inside a binary, so it is a revocable identifier
/// rather than a secret — and without `BUILD_API_KEY` a leaked key reveals
/// nothing about any other build.
fn derive_key(build_key: &str, name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(build_key.as_bytes());
    hasher.update(b":");
    hasher.update(name.as_bytes());
    hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// Adds the key to the KV store. `curl` rather than an HTTP crate: it is on
/// every runner and in every developer's PATH, and a build script cannot use the
/// crate's own `reqwest` anyway. Best-effort — a failure is reported and the
/// build continues.
fn register(build_key: &str, name: &str, key: &str) -> Result<(), String> {
    let body = serde_json::json!({ "name": name, "key": key }).to_string();
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            "10",
            "--request",
            "POST",
            REGISTER_ENDPOINT,
            "--header",
            &format!("X-API-Key: {build_key}"),
            "--header",
            "Content-Type: application/json",
            "--data-binary",
            &body,
        ])
        .output()
        .map_err(|error| format!("could not run curl: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        Err(if stderr.is_empty() {
            format!("curl exited with {}", output.status)
        } else {
            stderr.to_string()
        })
    }
}
