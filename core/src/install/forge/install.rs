// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{io::BufRead, path::PathBuf, process::Stdio};

use log::{debug, error, info};
use tokio::io::AsyncWriteExt;

use crate::{platform::DELIMITER, DATA_LOCATION, HTTP_CLIENT};

/// Forge Install Bootstrapper - By bangbang93
/// [Github Repo](https://github.com/bangbang93/forge-install-bootstrapper)
const FORGE_INSTALL_BOOTSTRAPPER: &[u8] = include_bytes!("./forge-install-bootstrapper.jar");

pub async fn install(
    install_dir: &PathBuf,
    forge_version: &str,
    mcversion: &str,
) -> anyhow::Result<()> {
    let splited_forge_version: Vec<_> = forge_version.split(".").collect();
    let bootstrapper = if splited_forge_version
        .first()
        .ok_or(anyhow::Error::msg("Error forge version"))?
        .parse::<usize>()?
        < 25
    {
        info!("Not using bootstrapper");
        None
    } else {
        info!("Using bootstrapper");
        Some(FORGE_INSTALL_BOOTSTRAPPER)
    };
    info!("Start downloading the installer");
    let installer_path = download_installer(mcversion, forge_version).await?;
    info!("Saving bootstrapper");
    let bootstrapper_path = DATA_LOCATION.temp.join("forge-install-bootstrapper.jar");
    if let Some(bootstrapper) = bootstrapper {
        tokio::fs::write(&bootstrapper_path, bootstrapper).await?;
    }
    let java = DATA_LOCATION.default_jre.clone();
    info!("Starting installer");
    let mut command = match bootstrapper {
        Some(_) => std::process::Command::new(java)
            .arg("-cp")
            .arg(format!(
                "{}{}{}",
                bootstrapper_path.to_str().unwrap(),
                DELIMITER,
                installer_path.to_str().unwrap()
            ))
            .arg("com.bangbang93.ForgeInstaller")
            .arg(install_dir)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
        None => std::process::Command::new(java)
            .arg("-jar")
            .arg(&installer_path)
            .arg("--installClient")
            .arg(install_dir)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    };
    let out = command.stdout.take().unwrap();
    let mut out = std::io::BufReader::new(out);
    let mut buf = String::new();
    let mut success = false;
    while out.read_line(&mut buf).is_ok() {
        if let Ok(Some(_)) = command.try_wait() {
            break;
        }
        if buf.ends_with("\ntrue\n") {
            success = true;
            info!("Successfully ran the fucking forge installer")
        } else {
            let lines: Vec<_> = buf.split("\n").collect();
            if let Some(last) = lines.get(lines.len() - 2) {
                debug!("{}", last);
            }
        }
    }
    let output = command.wait_with_output().unwrap();
    if (!success && bootstrapper.is_some()) || !output.status.success() {
        tokio::fs::remove_file(installer_path).await?;
        error!("Failed to ran forge installer");
        return Err(anyhow::Error::msg("Failed to ran forge installer"));
    }
    tokio::fs::remove_file(installer_path).await?;
    Ok(())
}

async fn download_installer(mcversion: &str, forge_version: &str) -> anyhow::Result<PathBuf> {
    let installer_url  = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{mcversion}-{forge_version}/forge-{mcversion}-{forge_version}-installer.jar");
    info!("The installer url is: {installer_url}");
    let installer_path = DATA_LOCATION.temp.join("forge-installer.jar");
    tokio::fs::create_dir_all(
        installer_path
            .parent()
            .ok_or(anyhow::Error::msg("Unknown Error"))?,
    )
    .await?;
    let mut file = tokio::fs::File::create(&installer_path).await?;
    let response = HTTP_CLIENT.get(installer_url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::Error::msg("Forge website return 404"));
    }
    let src = response.bytes().await?;
    file.write_all(&src).await?;
    Ok(installer_path)
}
