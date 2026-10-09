// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! How this copy of the launcher was installed, and therefore whether it may
//! update itself and which bundle the server should hand back.
//!
//! Guessing from `$APPIMAGE` or the executable's path is not enough — a Linux
//! build may be an AppImage, a package-manager install *or* a portable tarball,
//! and those need different answers. Each packaging format therefore ships an
//! `update-policy` marker that names its own form; the environment override and
//! the per-OS fallback below only cover a build that was run straight out of
//! `target/` or an installer that has not written one yet.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The file a package or a portable archive leaves beside the executable (or
/// under `/usr/share`, or in a macOS bundle's `Resources`) naming its form.
pub const POLICY_FILE: &str = "update-policy";

/// A self-updatable bundle form. The string form is what the update server's
/// `?kind=` parameter takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallKind {
    #[serde(rename = "appimage")]
    AppImage,
    #[serde(rename = "app")]
    App,
    #[serde(rename = "msi")]
    Msi,
    #[serde(rename = "nsis")]
    Nsis,
    #[serde(rename = "portable")]
    Portable,
}

impl InstallKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            InstallKind::AppImage => "appimage",
            InstallKind::App => "app",
            InstallKind::Msi => "msi",
            InstallKind::Nsis => "nsis",
            InstallKind::Portable => "portable",
        }
    }

    pub fn from_slug(value: &str) -> Option<Self> {
        match value {
            "appimage" => Some(InstallKind::AppImage),
            "app" => Some(InstallKind::App),
            "msi" => Some(InstallKind::Msi),
            "nsis" => Some(InstallKind::Nsis),
            "portable" => Some(InstallKind::Portable),
            _ => None,
        }
    }
}

/// The detected install form. [`PackageManager`](InstallForm::PackageManager)
/// covers deb, rpm and Arch, which are updated through the package manager and
/// whose settings toggle is greyed out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallForm {
    SelfUpdating(InstallKind),
    PackageManager,
}

impl InstallForm {
    pub const fn kind(self) -> Option<InstallKind> {
        match self {
            InstallForm::SelfUpdating(kind) => Some(kind),
            InstallForm::PackageManager => None,
        }
    }

    pub const fn is_self_updating(self) -> bool {
        matches!(self, InstallForm::SelfUpdating(_))
    }
}

impl std::fmt::Display for InstallForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallForm::SelfUpdating(kind) => f.write_str(kind.as_str()),
            InstallForm::PackageManager => f.write_str("package-manager"),
        }
    }
}

/// Detects the install form once, for the current process.
pub fn detect() -> InstallForm {
    if let Ok(value) = std::env::var("CONIC_UPDATE_POLICY")
        && let Some(form) = parse_policy(&value)
    {
        return form;
    }

    for path in policy_candidates() {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Some(form) = parse_policy(&text)
        {
            log::debug!("update policy read from {}", path.display());
            return form;
        }
    }

    // The resolved form and *why* it was reached — the one line that answers "the
    // updater is greyed out", which is otherwise reported only as an absence.
    let form = platform_default();
    log::debug!("no update policy marker was found, so this install is treated as {form}");
    form
}

fn parse_policy(value: &str) -> Option<InstallForm> {
    let value = value.trim().to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    if matches!(value.as_str(), "package-manager" | "package") {
        return Some(InstallForm::PackageManager);
    }
    InstallKind::from_slug(&value).map(InstallForm::SelfUpdating)
}

fn policy_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        paths.push(dir.join(POLICY_FILE));
        // A macOS bundle keeps the marker next to the executable by convention,
        // but a hand-built one may put it in `Resources`; both are checked.
        paths.push(dir.join("../Resources").join(POLICY_FILE));
    }
    #[cfg(target_os = "linux")]
    paths.push(PathBuf::from("/usr/share/conic-launcher").join(POLICY_FILE));
    paths
}

fn platform_default() -> InstallForm {
    #[cfg(target_os = "macos")]
    {
        InstallForm::SelfUpdating(InstallKind::App)
    }
    #[cfg(windows)]
    {
        // A packaged MSI lands under Program Files and can be repaired through
        // `msiexec`; anything else is a portable executable. The marker above
        // is the real answer, and this only covers a build that lacks one.
        if under_program_files() {
            InstallForm::SelfUpdating(InstallKind::Msi)
        } else {
            InstallForm::SelfUpdating(InstallKind::Portable)
        }
    }
    #[cfg(target_os = "linux")]
    {
        // An AppImage exports its own path. Everything else defaults to a
        // package-manager install, which is the safer guess: a portable tarball
        // ships its own `update-policy` marker, and wrongly offering a
        // self-update over a deb/rpm copy is worse than wrongly greying it out.
        if std::env::var_os("APPIMAGE").is_some() {
            InstallForm::SelfUpdating(InstallKind::AppImage)
        } else {
            InstallForm::PackageManager
        }
    }
    #[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
    {
        InstallForm::PackageManager
    }
}

#[cfg(windows)]
fn under_program_files() -> bool {
    let Some(exe) = std::env::current_exe().ok() else {
        return false;
    };
    for key in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Some(root) = std::env::var_os(key)
            && exe.starts_with(PathBuf::from(root))
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_round_trips_through_its_slug() {
        for kind in [
            InstallKind::AppImage,
            InstallKind::App,
            InstallKind::Msi,
            InstallKind::Nsis,
            InstallKind::Portable,
        ] {
            assert_eq!(InstallKind::from_slug(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn a_package_manager_policy_is_not_self_updating() {
        assert_eq!(
            parse_policy("package-manager"),
            Some(InstallForm::PackageManager)
        );
        assert!(!InstallForm::PackageManager.is_self_updating());
    }

    #[test]
    fn a_kind_policy_is_self_updating() {
        assert_eq!(
            parse_policy("appimage\n"),
            Some(InstallForm::SelfUpdating(InstallKind::AppImage))
        );
        assert!(InstallForm::SelfUpdating(InstallKind::Portable).is_self_updating());
    }

    #[test]
    fn an_unknown_policy_is_ignored() {
        assert_eq!(parse_policy("banana"), None);
        assert_eq!(parse_policy(""), None);
    }
}
