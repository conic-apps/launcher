// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Turning a resolved pack into an instance and installing it.
//!
//! The game and loader come from the `install` crate, the pack's own files from
//! `download`, and the overrides from the archive or folder the pack was
//! dropped as. Progress is reported through a [`shared::Sink`] of [`Progress`]
//! so the frontend can drive its install screen without this crate knowing
//! about it.

use std::sync::Arc;

use config::Config;
use download::progress::{DownloadSnapshot, DownloadState};
use install as install_crate;
use instance::{InstanceConfig, InstanceLaunchConfig, InstanceRuntime, SortBy};
use storage::LOCATIONS;

use crate::Plan;
use crate::error::{Error, Result};
use crate::overrides::extract_all;

/// What an install reports.
pub enum Progress {
    /// Minecraft, Java and the loader, as the `install` crate reports them.
    Install(install_crate::InstallProgress),
    /// The pack's own files being downloaded.
    Download(DownloadSnapshot),
}

/// The sink an install reports through.
pub type ProgressSink = shared::Sink<Progress>;

/// Creates the instance for a resolved pack.
///
/// The instance is written before the install starts so the frontend can show
/// its name and runtime while the download runs. A cancelled install therefore
/// leaves an uninstalled instance behind, exactly like cancelling a first launch
/// does.
pub async fn create_instance(plan: &Plan, name: &str) -> Result<String> {
    let name = name.trim();
    let name = if name.is_empty() {
        plan.name.as_str()
    } else {
        name
    };
    let id = unique_id(name).await?;
    let instance_config = InstanceConfig {
        name: name.to_string(),
        runtime: InstanceRuntime {
            minecraft: plan.minecraft.clone(),
            mod_loader_type: plan.loader_type.clone(),
            mod_loader_version: plan.loader_version.clone(),
        },
        launch_config: InstanceLaunchConfig {
            enable_instance_specific_settings: false,
            ..Default::default()
        },
        ..Default::default()
    };
    if let Err(error) = instance::create_instance(instance_config, Some(&id)).await {
        let _ = instance::delete_instance(&id).await;
        return Err(Error::Instance(format!(
            "could not create the instance: {error}"
        )));
    }
    Ok(id)
}

/// Installs a created instance's game, loader, files and overrides.
pub async fn install(id: String, plan: Plan, config: Config, sink: ProgressSink) -> Result<()> {
    let Some(instance) = instance::get_instance_by_id(&id).await else {
        return Err(Error::MissingInstance);
    };
    for file in &plan.unavailable {
        log::info!("skipping modpack file {}: {}", file.file_name, file.reason);
    }
    log::info!(
        "installing Minecraft {} for instance '{id}'",
        instance.config.runtime.minecraft
    );
    // Once `install::install` finishes it writes `.install.lock`. Anything that
    // fails or is cancelled after that must take the lock back, or the launch
    // page would skip the repair pass on a half-filled instance.
    let mut lock = InstallLockGuard {
        id: id.clone(),
        armed: true,
    };

    let download_config = config.download.clone();
    let install_out = sink.clone();
    let install_sink: install_crate::InstallSink =
        Arc::new(move |progress| install_out(Progress::Install(progress)));
    install_crate::install(config, instance, install_sink).await?;

    let mods_dir = LOCATIONS.instances.get_instance_root(&id).join("mods");
    let downloads: Vec<download::DownloadTask> = plan
        .files
        .iter()
        .map(|file| download::DownloadTask {
            url: file.url.clone(),
            file: mods_dir.join(&file.file_name),
            size_bytes: file.size_bytes,
            checksum: file
                .sha1
                .clone()
                .map(download::Checksum::Sha1)
                .unwrap_or(download::Checksum::None),
            task_type: download::DownloadTaskType::CurseforgeMod,
        })
        .collect();
    if !downloads.is_empty() {
        log::info!("downloading {} modpack files", downloads.len());
        let state = DownloadState::default();
        let download_out = sink.clone();
        download::progress::watch(
            &state,
            move |snapshot| download_out(Progress::Download(snapshot)),
            download::download_concurrent(downloads, &state, download_config),
        )
        .await?;
    }

    extract_all(&plan.overrides, &LOCATIONS.instances.get_instance_root(&id)).await?;

    lock.disarm();
    Ok(())
}

/// The directory name an instance called `name` can be stored under.
fn sanitize_id(name: &str) -> String {
    const RESERVED: [&str; 22] = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
        "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];

    let cleaned: String = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    let cleaned: String = cleaned.chars().take(80).collect();
    let cleaned = cleaned.trim_end_matches(['.', ' ']).trim();
    if cleaned.is_empty() {
        return "Instance".to_string();
    }
    let stem = cleaned
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if RESERVED.contains(&stem.as_str()) {
        format!("_{cleaned}")
    } else {
        cleaned.to_string()
    }
}

/// A [`sanitize_id`] result that no existing instance already uses.
async fn unique_id(name: &str) -> Result<String> {
    let base = sanitize_id(name);
    let instances = instance::list_instances(SortBy::Name)
        .await
        .map_err(|error| Error::Instance(format!("could not list instances: {error}")))?;
    let mut suffix = 0;
    loop {
        let candidate = if suffix == 0 {
            base.clone()
        } else {
            format!("{base} {suffix}")
        };
        if !instances.iter().any(|instance| instance.id == candidate)
            && !LOCATIONS.instances.get_instance_root(&candidate).exists()
        {
            return Ok(candidate);
        }
        suffix += 1;
    }
}

/// Removes `.install.lock` when an install is dropped before it finishes.
struct InstallLockGuard {
    id: String,
    armed: bool,
}

impl InstallLockGuard {
    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for InstallLockGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let id = self.id.clone();
        tokio::spawn(async move {
            let _ = instance::remove_install_lock(&id).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_id;

    #[test]
    fn sanitizes_instance_ids() {
        assert_eq!(sanitize_id("DUEL OF THE END"), "DUEL OF THE END");
        assert_eq!(sanitize_id("a/b:c"), "a_b_c");
        assert_eq!(sanitize_id("con"), "_con");
        assert_eq!(sanitize_id("   "), "Instance");
    }
}
