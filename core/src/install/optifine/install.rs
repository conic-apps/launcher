// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{ffi::OsStr, fmt::Display, path::Path};

use anyhow::Result;
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    core::folder::MinecraftLocation,
    utils::download::{download, Download},
};
use crate::core::DELIMITER;

use super::{InstallOptifineOptions, DEFAULT_META_URL};

const OPTIFINE_INSTALL_HELPER: &[u8] = include_bytes!("../../../vendor/optifine-installer.jar");

/// Download forge installer
pub async fn download_optifine_installer<P, D>(
    minecraft_version: &str,
    optifine_type: &str,
    optifine_patch: &str,
    dest_path: P,
    remote: Option<D>,
) -> Result<()>
    where
        P: AsRef<Path> + AsRef<OsStr>,
        D: Display,
{
    let url = match remote {
        None => format!("{DEFAULT_META_URL}/{minecraft_version}/{optifine_type}/{optifine_patch}"),
        Some(remote) => format!("{remote}/{minecraft_version}/{optifine_type}/{optifine_patch}"),
    };
    download(Download {
        url,
        file: dest_path,
        sha1: None,
    })
        .await?;

    Ok(())
}

/// Install optifine
///
/// referenced from [Sharp Craft Launcher](https://github.com/Steve-xmh/scl/blob/main/scl-core/src/download/optifine.rs)
///
/// #### Note:
///
/// if you need to install as mod, use download_optifine_install function
pub async fn install_optifine(
    minecraft: MinecraftLocation,
    version_name: &str,
    minecraft_version: &str,
    optifine_type: &str,
    optifine_patch: &str,
    java_executable_path: &str,
    options: Option<InstallOptifineOptions>,
) -> Result<()> {
    let options = match options {
        None => InstallOptifineOptions {
            use_forge_tweaker: None,
            inherits_from: None,
            version_id: None,
            remote: None,
        },
        Some(options) => options,
    };
    let full_path = minecraft.get_library_by_path(format!("net/optifine/{minecraft_version}-{optifine_type}-{optifine_patch}/Optifine-{minecraft_version}-{optifine_type}-{optifine_patch}.jar"));
    let full_path = full_path.to_str().unwrap();

    download_optifine_installer(
        minecraft_version,
        optifine_type,
        optifine_patch,
        full_path,
        options.remote,
    )
        .await?;

    let installer_path = minecraft
        .get_library_by_path("net/stevexmh/optifine-installer/0.0.0/optifine-installer.jar");
    let installer_path = installer_path.to_str().unwrap();

    fs::create_dir_all(Path::new(&installer_path).parent().unwrap())
        .await
        ?;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(installer_path)
        .await
        ?;
    file.write_all(OPTIFINE_INSTALL_HELPER).await?;
    file.flush().await?;
    file.sync_all().await?;

    // #[cfg(not(windows))]
    let mut command = tokio::process::Command::new(java_executable_path);

    // // #[cfg(windows)]
    // let mut command = {
    //     use tokio::process::windows::CommandExt;
    //     let mut command = tokio::process::Command::new(java_executable_path);
    //     command.creation_flags(0x08000000);
    //     command
    // };

    command.args(&[
        "-cp",
        &format!("{installer_path}{}{full_path}", DELIMITER),
        "net.stevexmh.OptifineInstaller",
        minecraft.root.to_str().unwrap(),
        version_name,
    ]);

    command.status().await?;

    Ok(())
}
