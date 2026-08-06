// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Parsing of Java runtime metadata.
//!
//! Design inspired by HMCL's `JavaInfo` / `JavaInfoUtils`:
//! two complementary sources of truth are combined — the `release` properties
//! file shipped with every JDK/JRE home (cheap to read) and the output of
//! `java -XshowSettings:properties -version` (authoritative for the actual
//! executable). Both are normalized into a single [`JavaInfoRaw`] that the
//! scanner can merge.

use std::path::PathBuf;

use crate::models::{JavaArch, JavaVendor};

/// Raw, still-unvalidated Java metadata gathered from a single source.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct JavaInfoRaw {
    /// Full version string, e.g. `17.0.9+9` or `1.8.0_392`.
    pub version: Option<String>,
    /// Java major version (`8` for `1.8.x`, `17` for `17.x`, ...).
    pub major_version: Option<u32>,
    /// Raw vendor string before normalization.
    pub vendor: Option<String>,
    /// Raw architecture string, e.g. `amd64`.
    pub arch: Option<String>,
    /// `java.home` reported by the runtime.
    pub java_home: Option<PathBuf>,
}

impl JavaInfoRaw {
    /// Fills in any fields that are still `None` from `other`.
    pub fn merge(&mut self, other: JavaInfoRaw) {
        if self.version.is_none() {
            self.version = other.version;
        }
        if self.major_version.is_none() {
            self.major_version = other.major_version;
        }
        if self.vendor.is_none() {
            self.vendor = other.vendor;
        }
        if self.arch.is_none() {
            self.arch = other.arch;
        }
        if self.java_home.is_none() {
            self.java_home = other.java_home;
        }
    }
}

/// Extracts the Java major version from a full version string.
///
/// - `1.8.0_392` → `8`
/// - `17.0.9+9` → `17`
/// - `21.0.1` → `21`
/// - `22-ea` → `22`
pub fn parse_major_version(version: &str) -> Option<u32> {
    let version = version.trim();
    let version = version
        .split(['-', '+'])
        .next()
        .map(str::trim)
        .unwrap_or(version);
    let version = version.strip_prefix("1.").unwrap_or(version);
    version.split('.').next()?.parse().ok()
}

/// Extracts the quoted version string from a `java -version` line.
///
/// Handles both `java version "1.8.0_392"` and `openjdk version "17.0.9+9"`.
pub fn parse_version_line(line: &str) -> Option<String> {
    let line = line.trim();
    let version_pos = line.to_ascii_lowercase().find("version")?;
    let rest = &line[version_pos + "version".len()..];
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_owned())
}

/// Splits a single `key = value` line into a trimmed `(key, value)` pair.
fn parse_property_line(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key, value.trim()))
}

/// Parses the combined output of `java -XshowSettings:properties -version`.
///
/// The property settings are printed first (including `java.home`,
/// `java.vendor`, `java.runtime.version` and `os.arch`), followed by the usual
/// `openjdk version "..."` lines.
pub fn parse_java_output(output: &str) -> JavaInfoRaw {
    let mut info = JavaInfoRaw::default();
    for line in output.lines() {
        let line = line.trim();
        if let Some((key, value)) = parse_property_line(line) {
            let value = value.trim_matches('"');
            match key {
                // Prefer the build-qualified version (e.g. `17.0.9+9`); it is
                // printed before the plain `java.version` property.
                "java.runtime.version" | "java.version" if info.version.is_none() => {
                    info.version = Some(value.to_owned());
                }
                "java.home" if info.java_home.is_none() => {
                    info.java_home = Some(PathBuf::from(value));
                }
                "java.vendor" if info.vendor.is_none() => {
                    info.vendor = Some(value.to_owned());
                }
                "os.arch" if info.arch.is_none() => {
                    info.arch = Some(value.to_owned());
                }
                _ => {}
            }
            continue;
        }
        if info.version.is_none()
            && let Some(version) = parse_version_line(line)
        {
            info.version = Some(version);
        }
    }
    if let (Some(version), None) = (&info.version, info.major_version) {
        info.major_version = parse_major_version(version);
    }
    info
}

/// Parses the JDK `release` properties file.
///
/// This file ships with every JDK/JRE home and looks like:
///
/// ```text
/// JAVA_VERSION="17.0.9+9"
/// OS_NAME="Linux"
/// OS_ARCH="amd64"
/// IMPLEMENTOR="Eclipse Adoptium"
/// ```
pub fn parse_release_file(content: &str) -> JavaInfoRaw {
    let mut info = JavaInfoRaw::default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = parse_property_line(line) else {
            continue;
        };
        let value = value.trim_matches('"');
        match key {
            "JAVA_VERSION" if info.version.is_none() => {
                info.version = Some(value.to_owned());
            }
            "IMPLEMENTOR" if info.vendor.is_none() => {
                info.vendor = Some(value.to_owned());
            }
            "OS_ARCH" if info.arch.is_none() => {
                info.arch = Some(value.to_owned());
            }
            _ => {}
        }
    }
    if let (Some(version), None) = (&info.version, info.major_version) {
        info.major_version = parse_major_version(version);
    }
    info
}

/// Maps a raw vendor string to the normalized [`JavaVendor`] classification.
///
/// Order matters: some strings are ambiguous (e.g. `Oracle OpenJDK`), so the
/// more specific distributors are matched before the generic `OpenJDK` bucket.
pub fn normalize_vendor(raw: &str) -> JavaVendor {
    let raw = raw.trim().to_ascii_lowercase();
    if raw.is_empty() || raw == "n/a" || raw == "unknown" {
        return JavaVendor::Unknown;
    }
    if raw.contains("oracle") {
        JavaVendor::Oracle
    } else if raw.contains("adoptium") || raw.contains("adoptopenjdk") {
        JavaVendor::EclipseAdoptium
    } else if raw.contains("microsoft") {
        JavaVendor::Microsoft
    } else if raw.contains("amazon") || raw.contains("corretto") {
        JavaVendor::AmazonCorretto
    } else if raw.contains("azul") || raw.contains("zulu") {
        JavaVendor::AzulZulu
    } else if raw.contains("bellsoft") || raw.contains("liberica") {
        JavaVendor::BellsoftLiberica
    } else if raw.contains("semeru") || raw.contains("ibm") || raw.contains("openj9") {
        JavaVendor::Semeru
    } else if raw.contains("sap") {
        JavaVendor::Sap
    } else if raw.contains("alibaba") || raw.contains("dragonwell") {
        JavaVendor::Dragonwell
    } else if raw.contains("openjdk")
        || raw.contains("red hat")
        || raw.contains("fedoraproject")
        || raw.contains("debian")
        || raw.contains("ubuntu")
    {
        JavaVendor::OpenJdk
    } else {
        JavaVendor::Unknown
    }
}

/// Maps a raw architecture string (from `os.arch` or a release file) to a
/// normalized [`JavaArch`].
pub fn normalize_arch(raw: &str) -> JavaArch {
    match raw.trim().to_ascii_lowercase().as_str() {
        "amd64" | "x86_64" | "x64" | "ia64" | "x86-64" => JavaArch::X64,
        "x86" | "i386" | "i486" | "i586" | "i686" | "x86_32" | "x86-32" => JavaArch::X86,
        "aarch64" | "arm64" | "aarch64le" => JavaArch::Aarch64,
        "arm" | "arm32" | "armv7" | "armv7l" | "armv8l" => JavaArch::Arm,
        _ => JavaArch::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::JavaVendor;

    #[test]
    fn major_version_parsing() {
        assert_eq!(parse_major_version("1.8.0_392"), Some(8));
        assert_eq!(parse_major_version("17.0.9+9"), Some(17));
        assert_eq!(parse_major_version("21.0.1"), Some(21));
        assert_eq!(parse_major_version("22-ea"), Some(22));
        assert_eq!(parse_major_version("8"), Some(8));
        assert_eq!(parse_major_version(""), None);
        assert_eq!(parse_major_version("not-a-version"), None);
    }

    #[test]
    fn version_line_parsing() {
        assert_eq!(
            parse_version_line(r#"openjdk version "17.0.9+9" 2023-10-17"#),
            Some("17.0.9+9".to_owned())
        );
        assert_eq!(
            parse_version_line(r#"java version "1.8.0_392""#),
            Some("1.8.0_392".to_owned())
        );
        assert_eq!(parse_version_line("some random line"), None);
    }

    #[test]
    fn java_output_parsing() {
        let output = "Property settings:\n    java.home = /usr/lib/jvm/java-17-openjdk-amd64\n    java.runtime.version = 17.0.9+9\n    java.vendor = Eclipse Adoptium\n    os.arch = amd64\nopenjdk version \"17.0.9+9\" 2023-10-17\n";
        let info = parse_java_output(output);
        assert_eq!(info.version.as_deref(), Some("17.0.9+9"));
        assert_eq!(info.major_version, Some(17));
        assert_eq!(info.vendor.as_deref(), Some("Eclipse Adoptium"));
        assert_eq!(info.arch.as_deref(), Some("amd64"));
        assert_eq!(
            info.java_home,
            Some(PathBuf::from("/usr/lib/jvm/java-17-openjdk-amd64"))
        );
    }

    #[test]
    fn release_file_parsing() {
        let content = "JAVA_VERSION=\"17.0.9+9\"\nOS_NAME=\"Linux\"\nOS_ARCH=\"amd64\"\nIMPLEMENTOR=\"Eclipse Adoptium\"\n";
        let info = parse_release_file(content);
        assert_eq!(info.version.as_deref(), Some("17.0.9+9"));
        assert_eq!(info.major_version, Some(17));
        assert_eq!(info.vendor.as_deref(), Some("Eclipse Adoptium"));
        assert_eq!(info.arch.as_deref(), Some("amd64"));
    }

    #[test]
    fn release_file_java8() {
        let content = "JAVA_VERSION=\"1.8.0_392\"\nIMPLEMENTOR=\"Oracle Corporation\"\nOS_ARCH=\"amd64\"\n";
        let info = parse_release_file(content);
        assert_eq!(info.major_version, Some(8));
        assert_eq!(info.vendor.as_deref(), Some("Oracle Corporation"));
    }

    #[test]
    fn vendor_normalization() {
        assert_eq!(normalize_vendor("Oracle Corporation"), JavaVendor::Oracle);
        assert_eq!(normalize_vendor("Oracle America, Inc."), JavaVendor::Oracle);
        assert_eq!(normalize_vendor("Eclipse Adoptium"), JavaVendor::EclipseAdoptium);
        assert_eq!(normalize_vendor("AdoptOpenJDK"), JavaVendor::EclipseAdoptium);
        assert_eq!(normalize_vendor("Microsoft"), JavaVendor::Microsoft);
        assert_eq!(normalize_vendor("Amazon.com Inc."), JavaVendor::AmazonCorretto);
        assert_eq!(normalize_vendor("Azul Systems, Inc."), JavaVendor::AzulZulu);
        assert_eq!(normalize_vendor("BellSoft Liberica"), JavaVendor::BellsoftLiberica);
        assert_eq!(normalize_vendor("IBM Semeru Runtime Open Edition"), JavaVendor::Semeru);
        assert_eq!(normalize_vendor("SAP SE"), JavaVendor::Sap);
        assert_eq!(normalize_vendor("Alibaba Dragonwell"), JavaVendor::Dragonwell);
        assert_eq!(normalize_vendor("OpenJDK"), JavaVendor::OpenJdk);
        assert_eq!(normalize_vendor("Red Hat, Inc."), JavaVendor::OpenJdk);
        assert_eq!(normalize_vendor("N/A"), JavaVendor::Unknown);
        assert_eq!(normalize_vendor(""), JavaVendor::Unknown);
        assert_eq!(normalize_vendor("Some Weird Company"), JavaVendor::Unknown);
    }

    #[test]
    fn arch_normalization() {
        assert_eq!(normalize_arch("amd64"), JavaArch::X64);
        assert_eq!(normalize_arch("x86_64"), JavaArch::X64);
        assert_eq!(normalize_arch("x86"), JavaArch::X86);
        assert_eq!(normalize_arch("i686"), JavaArch::X86);
        assert_eq!(normalize_arch("aarch64"), JavaArch::Aarch64);
        assert_eq!(normalize_arch("arm"), JavaArch::Arm);
        assert_eq!(normalize_arch("weird"), JavaArch::Unknown);
    }
}
