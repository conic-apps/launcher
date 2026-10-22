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

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use config::Config;
use download::progress::{DownloadSnapshot, DownloadState};
use install as install_crate;
use instance::{InstanceConfig, InstanceLaunchConfig, InstanceRuntime};
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
    // The id is chosen first so a failed write can remove the directory again.
    let id = instance::unique_id(name)
        .await
        .map_err(|error| Error::Instance(format!("could not choose an instance id: {error}")))?;
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
pub async fn install(
    id: String,
    plan: Plan,
    config: Config,
    sink: shared::Sink<Progress>,
) -> Result<()> {
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

    let instance_root = LOCATIONS.instances.get_instance_root(&id);
    let downloads: Vec<download::DownloadTask> = plan
        .files
        .iter()
        .map(|file| {
            Ok(download::DownloadTask {
                url: file.url.clone(),
                file: target_path(&instance_root, &file.path)?,
                size_bytes: file.size_bytes,
                checksum: file
                    .sha1
                    .clone()
                    .map(download::Checksum::Sha1)
                    .unwrap_or(download::Checksum::None),
                // The downloader classifies the source from the URL; a
                // modpack format must not pick the task type.
                task_type: download::DownloadTaskType::Unknown,
            })
        })
        .collect::<Result<Vec<_>>>()?;
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

    extract_all(&plan.overrides, &instance_root).await?;

    lock.disarm();
    Ok(())
}

/// Resolves a pack-relative path under the instance root.
///
/// A format supplies the path, so it is untrusted: only normal components are
/// allowed, or a malicious pack could write outside the instance.
fn target_path(instance_root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut target = instance_root.to_path_buf();
    let mut has_component = false;
    for component in relative.components() {
        match component {
            Component::Normal(part) => {
                target.push(part);
                has_component = true;
            }
            _ => return Err(Error::UnsafePath(relative.display().to_string())),
        }
    }
    if !has_component {
        return Err(Error::UnsafePath(relative.display().to_string()));
    }
    Ok(target)
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
    use super::target_path;
    use std::path::Path;

    #[test]
    fn a_pack_relative_path_stays_under_the_instance_root() {
        let root = Path::new("instances/test");
        assert_eq!(
            target_path(root, Path::new("mods/a.jar")).unwrap(),
            root.join("mods/a.jar")
        );
    }

    #[test]
    fn a_path_that_escapes_the_instance_root_is_rejected() {
        let root = Path::new("instances/test");
        assert!(target_path(root, Path::new("../a.jar")).is_err());
        assert!(target_path(root, Path::new("mods/../../a.jar")).is_err());
        assert!(target_path(root, Path::new("")).is_err());
    }
}
