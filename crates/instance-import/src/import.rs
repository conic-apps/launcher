// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Creating the instance and filling it. Progress is reported through a
//! [`shared::Sink`] of [`Progress`].

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use config::Config;
use download::progress::{DownloadSnapshot, DownloadState};
use install as install_crate;
use instance::{InstanceConfig, InstanceRuntime};
use storage::LOCATIONS;

use crate::ImportPlan;
use crate::error::{Error, Result};

/// What an import reports.
pub enum Progress {
    /// The instance exists; the import goes on filling it.
    InstanceCreated(String),
    /// Minecraft, Java and the loader of the new instance, as the `install`
    /// crate reports them.
    Game(install_crate::InstallProgress),
    Download(DownloadSnapshot),
    /// Copying a source tree into the instance, in bytes.
    Copy {
        done: u64,
        total: u64,
    },
    Extracting,
}

/// Creates the instance for `plan` and fills it.
///
/// The instance is written before the game install starts so the frontend can
/// show its name and runtime while the download runs, and so the caller learns
/// the id through [`Progress::InstanceCreated`] right then. A cancelled import
/// therefore leaves an uninstalled instance behind, exactly like cancelling a
/// first launch does.
pub async fn import(
    plan: ImportPlan,
    name: &str,
    config: Config,
    sink: shared::Sink<Progress>,
) -> Result<()> {
    for file in &plan.unavailable {
        log::info!("skipping import file {}: {}", file.file_name, file.reason);
    }
    let id = create_instance(&plan, name).await?;
    sink(Progress::InstanceCreated(id.clone()));
    let Some(instance) = instance::get_instance_by_id(&id).await else {
        return Err(Error::MissingInstance);
    };
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
        Arc::new(move |progress| install_out(Progress::Game(progress)));
    let language_config = config.clone();
    let language_instance = instance.clone();
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
                checksum: file.checksum.clone(),
                // The downloader classifies the source from the URL; an import
                // format must not pick the task type.
                task_type: download::DownloadTaskType::Unknown,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if !downloads.is_empty() {
        log::info!("downloading {} import files", downloads.len());
        let state = DownloadState::default();
        let download_out = sink.clone();
        download::progress::watch(
            &state,
            move |snapshot| download_out(Progress::Download(snapshot)),
            download::download_concurrent(downloads, &state, download_config),
        )
        .await?;
    }

    if !plan.trees.is_empty() {
        if plan.trees.iter().any(|tree| tree.is_archive()) {
            sink(Progress::Extracting);
        }
        let copy_out = sink.clone();
        let on_copy: crate::files::CopySink = Arc::new(move |snapshot| {
            copy_out(Progress::Copy {
                done: snapshot.done,
                total: snapshot.total,
            })
        });
        crate::files::write_all(plan.trees, instance_root.clone(), on_copy).await?;
    }

    // The copied files may carry their own `options.txt`, which wins over the
    // install-time language write; applying it again here keeps the launcher's
    // language without dropping the source's other options.
    install_crate::configure_instance_language(&language_config, &language_instance, true).await;

    lock.disarm();
    Ok(())
}

/// Creates the instance for a resolved plan.
async fn create_instance(plan: &ImportPlan, name: &str) -> Result<String> {
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
    let mut instance_config = InstanceConfig {
        name: name.to_string(),
        runtime: InstanceRuntime {
            minecraft: plan.minecraft.clone(),
            mod_loader_type: plan.loader_type.clone(),
            mod_loader_version: plan.loader_version.clone(),
        },
        ..Default::default()
    };
    plan.settings.apply(&mut instance_config);
    if let Err(error) = instance::create_instance(instance_config, Some(&id)).await {
        let _ = instance::delete_instance(&id).await;
        return Err(Error::Instance(format!(
            "could not create the instance: {error}"
        )));
    }
    Ok(id)
}

/// Resolves a source-relative path under the instance root.
///
/// An import supplies the path, so it is untrusted: only normal components are
/// allowed, or a malicious source could write outside the instance.
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

/// Removes `.install.lock` when an import is dropped before it finishes.
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
    fn a_source_relative_path_stays_under_the_instance_root() {
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
