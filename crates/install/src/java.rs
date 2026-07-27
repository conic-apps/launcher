// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use config::download::DownloadConfig;
use download::{Checksum, progress::DownloadState};
use folder::{DATA_LOCATION, MinecraftLocation};
use instance::Instance;
use log::info;
use serde::{Deserialize, Serialize};
use shared::HTTP_CLIENT;
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use version::resolve_version;

use download::{DownloadTask, DownloadTaskType};
use platform::{OsArch, OsFamily, PLATFORM_INFO};

use crate::error::*;

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
    size: u64,
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
    pub async fn new() -> Result<Self> {
        Ok(HTTP_CLIENT.get("https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json").send().await?.json().await?)
    }
}

/// Raw file metadata used in the Java runtime manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaFileRaw {
    sha1: String,
    size: u64,
    url: String,
}

/// LZMA-compressed file metadata used in the Java runtime manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaFileLZMA {
    sha1: String,
    size: u64,
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

/// Downloads and installs this Java runtime into the given install directory.
pub async fn install(
    runtime: &JavaRuntimeInfo,
    install_directory: &Path,
    progress: &DownloadState,
    config: DownloadConfig,
) -> Result<()> {
    let manifest = HTTP_CLIENT
        .get(&runtime.manifest.url)
        .send()
        .await?
        .json::<Manifest>()
        .await?;
    let downloads = generate_downloads(install_directory, &manifest.files);
    download::download_concurrent(downloads, progress, config).await?;
    info!("Creating links and setting permissions");
    #[cfg(not(windows))]
    for (path, file_info) in manifest.files {
        if let JavaFileInfo::Link { target } = file_info {
            let path = install_directory.join(path);
            if let Some(parent) = path.parent() {
                async_fs::create_dir_all(parent).await?;
            }
            let _ = async_fs::remove_file(&path).await;
            #[cfg(unix)]
            async_fs::unix::symlink(target, path).await?;
            #[cfg(windows)]
            async_fs::windows::symlink_file(target, path).await?;
            continue;
        }
        if let JavaFileInfo::File {
            executable: true, ..
        } = &file_info
        {
            let path = install_directory.join(path);
            let mut perm = async_fs::metadata(&path).await?.permissions();
            perm.set_mode(0o755);
            async_fs::set_permissions(path, perm).await?;
            continue;
        }
    }
    Ok(())
}

/// Installs all Java runtimes in the provided map into the target installation directory.
pub async fn group_install(
    install_directory: &Path,
    java_runtimes: HashMap<String, Vec<JavaRuntimeInfo>>,
    progress: &DownloadState,
    config: DownloadConfig,
) -> Result<()> {
    for (name, runtime_info) in java_runtimes {
        info!("Installing Java: {name}");
        if let Some(runtime_info) = runtime_info.first() {
            install(
                runtime_info,
                &install_directory.join(name),
                progress,
                config.clone(),
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn install_for_instance(
    instance: &Instance,
    progress: &DownloadState,
    config: DownloadConfig,
) -> Result<()> {
    let minecraft_location = MinecraftLocation::new(&DATA_LOCATION.root);
    let version_json_path = minecraft_location.get_version_json(
        instance
            .get_version_id()
            .map_err(|_| Error::InvalidInstanceConfig)?,
    );
    let unresolved_version = serde_json::from_str::<version::Version>(
        &async_fs::read_to_string(version_json_path).await?,
    )?;
    let resolved_version = resolve_version(&unresolved_version, &minecraft_location, &[]).await?;
    let java_version_list = MojangJavaVersionList::new().await?;

    let java_runtime_info = match PLATFORM_INFO.os_family {
        OsFamily::Windows => match PLATFORM_INFO.arch {
            OsArch::X64 => java_version_list
                .windows_x64
                .get(&resolved_version.java_version.component)
                .ok_or(Error::NoSupportedJavaRuntime)?
                .first()
                .ok_or(Error::NoSupportedJavaRuntime)?,
            OsArch::X86 => java_version_list
                .windows_x86
                .get(&resolved_version.java_version.component)
                .ok_or(Error::NoSupportedJavaRuntime)?
                .first()
                .ok_or(Error::NoSupportedJavaRuntime)?,
            OsArch::Aarch64 => java_version_list
                .windows_arm64
                .get(&resolved_version.java_version.component)
                .ok_or(Error::NoSupportedJavaRuntime)?
                .first()
                .unwrap_or(
                    java_version_list
                        .windows_x64
                        .get(&resolved_version.java_version.component)
                        .ok_or(Error::NoSupportedJavaRuntime)?
                        .first()
                        .ok_or(Error::NoSupportedJavaRuntime)?,
                ),
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
        OsFamily::Linux => match PLATFORM_INFO.arch {
            OsArch::X64 => java_version_list
                .linux
                .get(&resolved_version.java_version.component)
                .ok_or(Error::NoSupportedJavaRuntime)?
                .first()
                .ok_or(Error::NoSupportedJavaRuntime)?,
            OsArch::X86 => java_version_list
                .linux_i386
                .get(&resolved_version.java_version.component)
                .ok_or(Error::NoSupportedJavaRuntime)?
                .first()
                .ok_or(Error::NoSupportedJavaRuntime)?,
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
        OsFamily::Macos => match PLATFORM_INFO.arch {
            OsArch::X64 => java_version_list
                .mac_os
                .get(&resolved_version.java_version.component)
                .ok_or(Error::NoSupportedJavaRuntime)?
                .first()
                .ok_or(Error::NoSupportedJavaRuntime)?,
            OsArch::Aarch64 => java_version_list
                .mac_os_arm64
                .get(&resolved_version.java_version.component)
                .ok_or(Error::NoSupportedJavaRuntime)?
                .first()
                .unwrap_or(
                    java_version_list
                        .mac_os
                        .get(&resolved_version.java_version.component)
                        .ok_or(Error::NoSupportedJavaRuntime)?
                        .first()
                        .ok_or(Error::NoSupportedJavaRuntime)?,
                ),
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
    };
    install(
        java_runtime_info,
        &get_installation_directory(&resolved_version.java_version.component)?,
        progress,
        config,
    )
    .await
}

pub fn get_installation_directory(java_component: &str) -> Result<PathBuf> {
    let root = &DATA_LOCATION.runtime;
    let platform_folder_name = match PLATFORM_INFO.os_family {
        OsFamily::Windows => match PLATFORM_INFO.arch {
            OsArch::X64 => "windows_x64",
            OsArch::X86 => "windows_x86",
            OsArch::Aarch64 => "windows_arm64",
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
        OsFamily::Linux => match PLATFORM_INFO.arch {
            OsArch::X64 => "linux_amd64",
            OsArch::X86 => "linux_i386",
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
        OsFamily::Macos => match PLATFORM_INFO.arch {
            OsArch::X64 => "macos_x64",
            OsArch::Aarch64 => "macos_arm64",
            _ => return Err(Error::NoSupportedJavaRuntime),
        },
    };
    Ok(root.join(platform_folder_name).join(java_component))
}

pub fn get_executable_path(java_component: &str) -> Result<PathBuf> {
    let installation_directory = get_installation_directory(java_component)?;
    match PLATFORM_INFO.os_family {
        OsFamily::Linux => Ok(installation_directory.join("bin").join("java")),
        OsFamily::Macos => Ok(installation_directory
            .join("jre.bundle")
            .join("Contents")
            .join("Home")
            .join("bin")
            .join("java")),
        OsFamily::Windows => {
            if java_component == "minecraft-java-exe" {
                Ok(installation_directory.join("MinecraftJava.exe"))
            } else {
                Ok(installation_directory.join("bin").join("javaw.exe"))
            }
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
                checksum: Checksum::Sha1(downloads.raw.sha1.clone()),
                size_bytes: Some(downloads.raw.size),
                task_type: DownloadTaskType::Unknown,
            });
        }
    });
    result
}
