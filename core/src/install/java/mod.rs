// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use log::{info, warn};
use serde::{Deserialize, Serialize};
#[cfg(not(windows))]
use std::os::unix::fs::PermissionsExt;
use std::{collections::HashMap, io::Read, path::Path};
use tokio::io::AsyncWriteExt;

use crate::{
    download::{Download, DownloadType},
    platform::OsFamily,
    HTTP_CLIENT, PLATFORM_INFO,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Availability {
    group: usize,
    progress: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestDownloadInfo {
    sha1: String,
    size: usize,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Version {
    name: String,
    released: String,
}

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
    pub async fn new() -> anyhow::Result<Self> {
        Ok(HTTP_CLIENT.get("https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json").send().await?.json().await?)
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaFileRaw {
    sha1: String,
    size: usize,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaFileLZMA {
    sha1: String,
    size: usize,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JavaFileDownloads {
    lzma: Option<JavaFileLZMA>,
    raw: JavaFileRaw,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    files: HashMap<String, JavaFileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaRuntimeInfo {
    availability: Availability,
    manifest: ManifestDownloadInfo,
    version: Version,
}

impl JavaRuntimeInfo {
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
        for (path, file_info) in manifest.files {
            #[cfg(not(windows))]
            if let JavaFileInfo::Link { target } = file_info {
                let path = install_directory.join(path);
                tokio::fs::create_dir_all(path.parent().unwrap())
                    .await
                    .unwrap();
                let _ = tokio::fs::remove_file(&path).await;
                tokio::fs::symlink(target, path).await.unwrap();
                continue;
            }
            #[cfg(not(windows))]
            if let JavaFileInfo::File {
                executable: true, ..
            } = &file_info
            {
                let path = install_directory.join(path);
                let mut perm = tokio::fs::metadata(&path).await.unwrap().permissions();
                perm.set_mode(0o755);
                tokio::fs::set_permissions(path, perm).await.unwrap();
                continue;
            }
        }
    }
}

pub(super) async fn group_install(
    install_directory: &Path,
    java_runtimes: HashMap<String, Vec<JavaRuntimeInfo>>,
) {
    for (name, runtime_info) in java_runtimes {
        info!("Installing Java: {}", name);
        if let Some(runtime_info) = runtime_info.first() {
            runtime_info
                .clone()
                .install(&install_directory.join(name))
                .await;
        }
    }
}

fn generate_downloads(
    install_directory: &Path,
    files: &HashMap<String, JavaFileInfo>,
) -> Vec<Download> {
    let mut result = vec![];
    files.iter().for_each(|(path, file_info)| {
        if let JavaFileInfo::File { downloads, .. } = file_info {
            result.push(Download {
                url: downloads.raw.url.clone(),
                file: install_directory.join(path),
                sha1: Some(downloads.raw.sha1.clone()),
                r#type: DownloadType::Unknown,
            });
        }
    });
    result
}

async fn download_files(downloads: Vec<Download>) -> anyhow::Result<()> {
    for download in downloads {
        let mut retried = 0;
        while retried <= 5 {
            retried += 1;
            match download_and_check(&download).await {
                Ok(_) => break,
                Err(_) => warn!("Downloaded failed: {}, retried: {}", &download.url, retried),
            }
        }
    }
    Ok(())
}
async fn download_and_check(download: &Download) -> anyhow::Result<()> {
    let file_path = download.file.clone();
    info!("Downloading {}", download.url);
    tokio::fs::create_dir_all(file_path.parent().ok_or(anyhow::Error::msg(
        "Unknown Error in instance/mod.rs".to_string(),
    ))?)
    .await?;
    let mut response = HTTP_CLIENT.get(download.url.clone()).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Downloaded failed"));
    }
    let mut file = tokio::fs::File::create(&file_path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    file.sync_all().await?;
    drop(file);
    let mut file = std::fs::File::open(&file_path).unwrap();
    info!("Verifying {}", file_path.display());
    if let Some(sha1) = download.sha1.clone() {
        if calculate_sha1_from_read(&mut file) != sha1 {
            return Err(anyhow::Error::msg("sha1 check failed".to_string()));
        }
    }
    Ok(())
}
fn calculate_sha1_from_read<R: Read>(source: &mut R) -> String {
    let mut hasher = sha1_smol::Sha1::new();
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = source.read(&mut buffer).unwrap();
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    hasher.digest().to_string()
}
