// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use log::{info, warn};
use serde::{Deserialize, Serialize};
use shared::HTTP_CLIENT;
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
use std::{collections::HashMap, path::Path};
use task::Progress;

use download::{DownloadTask, DownloadType, download_and_check};
use platform::{OsFamily, PLATFORM_INFO};

/// Represents the availability group and progress index of a Java runtime version.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Availability {
    group: usize,
    progress: usize,
}

/// Contains metadata for downloading a Java runtime manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestDownloadInfo {
    sha1: String,
    size: usize,
    url: String,
}

/// Contains the name and release date of a Java runtime version.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Version {
    name: String,
    released: String,
}

/// Represents the Mojang-provided Java version list for all supported platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangJavaVersionList {
    gamecore: HashMap<String, Vec<JavaRuntimeInfo>>,
    linux: HashMap<String, Vec<JavaRuntimeInfo>>,
    #[serde(rename = "linux-i386")]
    linux_i386: HashMap<String, Vec<JavaRuntimeInfo>>,
    #[serde(rename = "mac-os")]
    mac_os: HashMap<String, Vec<JavaRuntimeInfo>>,
    #[serde(rename = "mac-os-arm64")]
    mac_os_arm64: HashMap<String, Vec<JavaRuntimeInfo>>,
    #[serde(rename = "windows-arm64")]
    windows_arm64: HashMap<String, Vec<JavaRuntimeInfo>>,
    #[serde(rename = "windows-x64")]
    windows_x64: HashMap<String, Vec<JavaRuntimeInfo>>,
    #[serde(rename = "windows-x86")]
    windows_x86: HashMap<String, Vec<JavaRuntimeInfo>>,
}

impl MojangJavaVersionList {
    /// Downloads and returns the full Java version list manifest from Mojang servers.
    pub async fn new() -> anyhow::Result<Self> {
        Ok(HTTP_CLIENT.get("https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json").send().await?.json().await?)
    }

    /// Returns the Java runtime list for the current platform and architecture.
    pub fn get_current_platform(self) -> Option<HashMap<String, Vec<JavaRuntimeInfo>>> {
        match PLATFORM_INFO.os_family {
            OsFamily::Linux => {
                if PLATFORM_INFO.arch == "x64" {
                    Some(self.linux)
                } else if PLATFORM_INFO.arch == "x86" {
                    Some(self.linux_i386)
                } else {
                    None
                }
            }
            OsFamily::Macos => {
                if PLATFORM_INFO.arch == "x64" {
                    Some(self.mac_os)
                } else if PLATFORM_INFO.arch == "arm64" {
                    Some(self.mac_os_arm64)
                } else {
                    None
                }
            }
            OsFamily::Windows => {
                if PLATFORM_INFO.arch == "x64" {
                    Some(self.windows_x64)
                } else if PLATFORM_INFO.arch == "x86" {
                    Some(self.windows_x86)
                } else if PLATFORM_INFO.arch == "arm64" {
                    Some(self.windows_arm64)
                } else {
                    None
                }
            }
        }
    }
}

/// Raw file metadata used in the Java runtime manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaFileRaw {
    sha1: String,
    size: usize,
    url: String,
}

/// LZMA-compressed file metadata used in the Java runtime manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaFileLZMA {
    sha1: String,
    size: usize,
    url: String,
}

/// Describes both raw and optionally compressed downloads for a Java file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaFileDownloads {
    lzma: Option<JavaFileLZMA>,
    raw: JavaFileRaw,
}

/// Enum describing the type and metadata of each Java runtime file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum JavaFileInfo {
    #[serde(rename = "file")]
    File {
        downloads: JavaFileDownloads,
        executable: bool,
    },
    #[serde(rename = "directory")]
    Directory,
    #[serde(rename = "link")]
    Link { target: String },
}

/// Represents the complete manifest structure for a Java runtime version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    files: HashMap<String, JavaFileInfo>,
}

/// Holds all data required to download and install a single Java runtime version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaRuntimeInfo {
    availability: Availability,
    manifest: ManifestDownloadInfo,
    version: Version,
}

impl JavaRuntimeInfo {
    /// Downloads and installs this Java runtime into the given install directory.
    pub(super) async fn install(self, install_directory: &Path) {
        let manifest = HTTP_CLIENT
            .get(self.manifest.url)
            .send()
            .await
            .unwrap()
            .json::<Manifest>()
            .await
            .unwrap();
        let downloads = generate_downloads(install_directory, &manifest.files);
        download_files(downloads).await.unwrap();
        info!("Creating links and setting permissions");
        #[cfg(not(windows))]
        for (path, file_info) in manifest.files {
            if let JavaFileInfo::Link { target } = file_info {
                let path = install_directory.join(path);
                async_fs::create_dir_all(path.parent().unwrap())
                    .await
                    .unwrap();
                let _ = async_fs::remove_file(&path).await;
                #[cfg(unix)]
                async_fs::unix::symlink(target, path).await.unwrap();
                #[cfg(windows)]
                async_fs::windows::symlink_file(target, path).await.unwrap();
                continue;
            }
            if let JavaFileInfo::File {
                executable: true, ..
            } = &file_info
            {
                let path = install_directory.join(path);
                let mut perm = async_fs::metadata(&path).await.unwrap().permissions();
                perm.set_mode(0o755);
                async_fs::set_permissions(path, perm).await.unwrap();
                continue;
            }
        }
    }
}

/// Installs all Java runtimes in the provided map into the target installation directory.
pub(super) async fn group_install(
    install_directory: &Path,
    java_runtimes: HashMap<String, Vec<JavaRuntimeInfo>>,
) {
    for (name, runtime_info) in java_runtimes {
        info!("Installing Java: {name}");
        if let Some(runtime_info) = runtime_info.first() {
            runtime_info
                .clone()
                .install(&install_directory.join(name))
                .await;
        }
    }
}

/// Generates a list of files to be downloaded based on the manifest.
fn generate_downloads(
    install_directory: &Path,
    files: &HashMap<String, JavaFileInfo>,
) -> Vec<DownloadTask> {
    let mut result = vec![];
    files.iter().for_each(|(path, file_info)| {
        if let JavaFileInfo::File { downloads, .. } = file_info {
            result.push(DownloadTask {
                url: downloads.raw.url.clone(),
                file: install_directory.join(path),
                sha1: Some(downloads.raw.sha1.clone()),
                r#type: DownloadType::Unknown,
            });
        }
    });
    result
}

/// Downloads all files in the given download list and verifies them.
/// TODO: REMOVE THIS, USE DOWNLOAD MODULE
async fn download_files(downloads: Vec<DownloadTask>) -> anyhow::Result<()> {
    for download in downloads {
        let mut retried = 0;
        while retried <= 5 {
            retried += 1;
            let progress = Progress::default();
            match download_and_check(&download, &progress).await {
                Ok(_) => break,
                Err(_) => warn!("Downloaded failed: {}, retried: {}", &download.url, retried),
            }
        }
    }
    Ok(())
}
