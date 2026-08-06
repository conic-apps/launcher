// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Data models for discovered Java runtimes.
//!
//! The classification scheme (major-version groups plus a normalized vendor) is
//! inspired by how HMCL presents Java toolchains in its Java management page:
//! runtimes are grouped by Java major version and shown with a friendly vendor
//! label, so the UI never has to re-parse raw version strings.

use std::{cmp::Ordering, collections::BTreeMap, path::PathBuf};

use serde::Serialize;

/// Normalized Java vendor, safe for the frontend to display directly.
///
/// Every value is lowercase snake_case when serialized to the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JavaVendor {
    Oracle,
    OpenJdk,
    EclipseAdoptium,
    Microsoft,
    AmazonCorretto,
    AzulZulu,
    BellsoftLiberica,
    Semeru,
    Sap,
    Dragonwell,
    Unknown,
}

impl JavaVendor {
    /// A human friendly vendor name.
    pub fn display_name(self) -> &'static str {
        match self {
            JavaVendor::Oracle => "Oracle",
            JavaVendor::OpenJdk => "OpenJDK",
            JavaVendor::EclipseAdoptium => "Eclipse Adoptium",
            JavaVendor::Microsoft => "Microsoft",
            JavaVendor::AmazonCorretto => "Amazon Corretto",
            JavaVendor::AzulZulu => "Azul Zulu",
            JavaVendor::BellsoftLiberica => "BellSoft Liberica",
            JavaVendor::Semeru => "IBM Semeru",
            JavaVendor::Sap => "SAP",
            JavaVendor::Dragonwell => "Alibaba Dragonwell",
            JavaVendor::Unknown => "Unknown",
        }
    }
}

/// CPU architecture of a Java runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JavaArch {
    X64,
    X86,
    Aarch64,
    Arm,
    Unknown,
}

impl JavaArch {
    /// A human friendly architecture name.
    pub fn display_name(self) -> &'static str {
        match self {
            JavaArch::X64 => "x86_64",
            JavaArch::X86 => "x86_32",
            JavaArch::Aarch64 => "aarch64",
            JavaArch::Arm => "arm",
            JavaArch::Unknown => "unknown",
        }
    }
}

/// A single discovered Java runtime, ready for display in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JavaRuntime {
    /// Absolute path to the `java` (or `java.exe`) executable.
    pub path: PathBuf,
    /// The resolved `JAVA_HOME` when the executable sits in a standard
    /// `<home>/bin/java` layout (including macOS `jre.bundle` bundles).
    pub java_home: Option<PathBuf>,
    /// Java major version: `8` for `1.8.x`, `17` for `17.x`, ...
    pub major_version: u32,
    /// Full version string, e.g. `17.0.9+9` or `1.8.0_392`.
    pub version: String,
    pub vendor: JavaVendor,
    pub arch: JavaArch,
    /// Whether a JDK (has `javac`/`jar`) instead of a plain JRE.
    pub is_jdk: bool,
    /// Whether this runtime lives under a launcher-managed runtime directory.
    pub is_managed: bool,
    /// Whether the executable actually ran and reported version information.
    ///
    /// A runtime can still be listed with `is_valid == false` when only its
    /// `release` file could be read, so the UI can surface broken installs.
    pub is_valid: bool,
}

/// `Ord` sorts newest major version first, then by full version and finally by
/// path, which matches how a launcher would present a Java list.
impl Ord for JavaRuntime {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .major_version
            .cmp(&self.major_version)
            .then_with(|| other.version.cmp(&self.version))
            .then_with(|| self.path.cmp(&other.path))
    }
}

impl PartialOrd for JavaRuntime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Runtimes sharing the same Java major version.
#[derive(Debug, Clone, Serialize)]
pub struct JavaVersionGroup {
    pub major_version: u32,
    pub runtimes: Vec<JavaRuntime>,
}

/// The full result of a Java scan, structured so the frontend can render it
/// without parsing any strings.
#[derive(Debug, Clone, Serialize)]
pub struct JavaScanResult {
    /// Flat list of all runtimes, sorted newest major version first.
    pub runtimes: Vec<JavaRuntime>,
    /// Runtimes grouped by major version (newest group first).
    pub groups: Vec<JavaVersionGroup>,
}

impl JavaScanResult {
    /// Builds a [`JavaScanResult`] from a flat runtime list, grouping it by
    /// major version for direct UI consumption.
    pub fn from_runtimes(mut runtimes: Vec<JavaRuntime>) -> Self {
        runtimes.sort();
        let mut by_version: BTreeMap<u32, Vec<JavaRuntime>> = BTreeMap::new();
        for runtime in runtimes.iter().cloned() {
            by_version
                .entry(runtime.major_version)
                .or_default()
                .push(runtime);
        }
        let groups = by_version
            .into_iter()
            .rev()
            .map(|(major_version, runtimes)| JavaVersionGroup {
                major_version,
                runtimes,
            })
            .collect();
        JavaScanResult { runtimes, groups }
    }
}

/// Options controlling a [`crate::scan_java_runtimes_with`] scan.
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Extra directories to treat as Java homes (each is checked for a
    /// `bin/java` executable). Reserved for future "manually add Java path".
    pub extra_home_dirs: Vec<PathBuf>,
    /// Launcher-managed runtime roots of the form `<root>/<platform>/<component>`.
    /// Runtimes found under these roots are flagged with `is_managed`.
    pub managed_dirs: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(path: &str, major_version: u32, version: &str) -> JavaRuntime {
        JavaRuntime {
            path: PathBuf::from(path),
            java_home: None,
            major_version,
            version: version.to_owned(),
            vendor: JavaVendor::Unknown,
            arch: JavaArch::Unknown,
            is_jdk: false,
            is_managed: false,
            is_valid: true,
        }
    }

    #[test]
    fn groups_sorted_newest_major_version_first() {
        let runtimes = vec![
            runtime("/opt/jdk-8", 8, "1.8.0_392"),
            runtime("/opt/jdk-21", 21, "21.0.1"),
            runtime("/opt/jdk-17", 17, "17.0.9+9"),
        ];
        let result = JavaScanResult::from_runtimes(runtimes);
        assert_eq!(
            result
                .groups
                .iter()
                .map(|g| g.major_version)
                .collect::<Vec<_>>(),
            vec![21, 17, 8]
        );
        assert_eq!(
            result.groups[1].runtimes[0].path,
            PathBuf::from("/opt/jdk-17")
        );
        assert_eq!(result.runtimes[0].path, PathBuf::from("/opt/jdk-21"));
    }

    #[test]
    fn runtime_ordering() {
        let mut runtimes = [
            runtime("/a", 17, "17.0.9"),
            runtime("/b", 17, "17.0.8"),
            runtime("/c", 21, "21.0.1"),
        ];
        runtimes.sort();
        assert_eq!(runtimes[0].path, PathBuf::from("/c"));
        assert_eq!(runtimes[1].path, PathBuf::from("/a"));
        assert_eq!(runtimes[2].path, PathBuf::from("/b"));
    }
}
