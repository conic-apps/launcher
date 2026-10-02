// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Choosing and downloading a project's file.

use super::*;

/// Resolves the project's best file for the instance's runtime and downloads
/// it.
///
/// The download's progress is reported into the detail panel through `weak` and
/// `token`, sampled by [`download::progress::watch`] rather than by this
/// function.
pub(crate) async fn install(
    instance_id: &str,
    detail: &OpenDetail,
    weak: Weak<App>,
    token: Token,
) -> Result<(), String> {
    let runtime = instance::get_instance_by_id(instance_id)
        .await
        .map(|instance| instance.config.runtime);
    // `InstanceRuntime.minecraft` is a plain string; empty means unset.
    let minecraft = runtime
        .as_ref()
        .map(|runtime| runtime.minecraft.clone())
        .filter(|minecraft| !minecraft.is_empty());
    let loader = runtime
        .as_ref()
        .and_then(|runtime| runtime.mod_loader_type.as_ref())
        .map(|loader| loader.to_string().to_lowercase());

    // A modpack is not installed into the instance; it goes into the launcher's
    // own `modpacks` folder.
    let target_dir = if detail.kind == RemoteKind::Packs {
        storage::LOCATIONS.launcher.root.join("modpacks")
    } else {
        storage::LOCATIONS
            .instances
            .get_instance_root(instance_id)
            .join(detail.kind.folder())
    };

    let task = match detail.platform {
        Platform::Modrinth => {
            modrinth_install_task(detail, minecraft.as_deref(), loader.as_deref(), &target_dir)
                .await?
        }
        Platform::CurseForge => {
            curseforge_install_task(detail, minecraft.as_deref(), loader.as_deref(), &target_dir)
                .await?
        }
    };

    std::fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    let progress = download::progress::DownloadState::default();
    // The change-detecting reporter drops the 100ms ticks where nothing moved,
    // so the event loop is only woken for a real byte count.
    let reporter = shared::ChangeReporter::new({
        let weak = weak.clone();
        let token = token.clone();
        std::sync::Arc::new(move |snapshot| {
            let view = download_view(&snapshot);
            deliver(&weak, &token, move |ui| {
                view.apply(&ui.global::<ContentState>())
            });
        })
    });
    download::progress::watch(
        &progress,
        |snapshot| reporter.report(snapshot),
        download::download(&task, &progress),
    )
    .await
    .map_err(|error| error.to_string())?;

    // Re-read the mods so a later list shows the new file with its metadata.
    if detail.kind == RemoteKind::Mods {
        content::mods::remote::parse_mods(instance_id).await;
    }
    Ok(())
}

/// The download of a Modrinth project: its compatible version's primary file.
pub(crate) async fn modrinth_install_task(
    detail: &OpenDetail,
    minecraft: Option<&str>,
    loader: Option<&str>,
    target_dir: &std::path::Path,
) -> Result<download::DownloadTask, String> {
    let params = modrinth::ListProjectVersionsParams {
        loaders: (detail.kind.has_loaders() && loader.is_some())
            .then(|| serde_json::to_string(&[loader.unwrap_or_default()]).unwrap_or_default()),
        game_versions: minecraft
            .map(|minecraft| serde_json::to_string(&[minecraft]).unwrap_or_default()),
        featured: None,
        include_changelog: None,
    };
    let versions = modrinth::list_project_versions(&detail.id, &params)
        .await
        .map_err(|error| error.to_string())?;
    let versions = versions.as_array().cloned().unwrap_or_default();
    let version = pick_modrinth_version(&versions, minecraft, loader)
        .ok_or("no compatible Modrinth version")?;
    let file = version
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files
                .iter()
                .find(|file| file.get("primary").and_then(Value::as_bool) == Some(true))
                .or_else(|| files.first())
        })
        .ok_or("no downloadable file")?;
    Ok(make_task(
        file.get("url")
            .and_then(Value::as_str)
            .ok_or("no download url")?,
        &target_dir.join(
            file.get("filename")
                .and_then(Value::as_str)
                .ok_or("no file name")?,
        ),
        file.get("size").and_then(Value::as_u64),
        file.pointer("/hashes/sha512")
            .and_then(Value::as_str)
            .map(str::to_string),
        download::DownloadTaskType::ModrinthMod,
    ))
}

/// The download of a CurseForge mod: its first available file, through the
/// separate download-url endpoint for the files whose `downloadUrl` is null.
pub(crate) async fn curseforge_install_task(
    detail: &OpenDetail,
    minecraft: Option<&str>,
    loader: Option<&str>,
    target_dir: &std::path::Path,
) -> Result<download::DownloadTask, String> {
    let mod_id: i64 = detail
        .id
        .parse()
        .map_err(|_| "not a CurseForge mod id".to_string())?;
    let mut params = json!({});
    let object = params.as_object_mut().expect("just built");
    if let Some(minecraft) = minecraft {
        object.insert("gameVersion".into(), json!(minecraft));
    }
    if detail.kind.has_loaders()
        && let Some(loader) = loader.and_then(curseforge_loader_type)
    {
        object.insert("modLoaderType".into(), json!(loader));
    }
    let response = curseforge::get_mod_files(mod_id, &params)
        .await
        .map_err(|error| error.to_string())?;
    let files = response
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let file = files
        .iter()
        .find(|file| file.get("isAvailable").and_then(Value::as_bool) == Some(true))
        .or_else(|| files.first())
        .ok_or("no compatible CurseForge file")?;
    let file_id = file.get("id").and_then(Value::as_i64).ok_or("no file id")?;
    let url = match file.get("downloadUrl").and_then(Value::as_str) {
        Some(url) if !url.is_empty() => url.to_string(),
        // `downloadUrl` is null for some files and the API has a separate
        // endpoint for those.
        _ => curseforge::get_mod_file_download_url(mod_id, file_id)
            .await
            .map_err(|error| error.to_string())?
            .get("data")
            .and_then(Value::as_str)
            .ok_or("no download url")?
            .to_string(),
    };
    let sha1 = file
        .get("hashes")
        .and_then(Value::as_array)
        .and_then(|hashes| {
            hashes
                .iter()
                .find(|hash| hash.get("algo").and_then(Value::as_i64) == Some(1))
        })
        .and_then(|hash| hash.get("value").and_then(Value::as_str))
        .map(str::to_string);
    Ok(make_task(
        &url,
        &target_dir.join(
            file.get("fileName")
                .and_then(Value::as_str)
                .ok_or("no file name")?,
        ),
        file.get("fileLength").and_then(Value::as_u64),
        sha1,
        download::DownloadTaskType::CurseforgeMod,
    ))
}

pub(crate) fn make_task(
    url: &str,
    file: &std::path::Path,
    size_bytes: Option<u64>,
    sha: Option<String>,
    task_type: download::DownloadTaskType,
) -> download::DownloadTask {
    download::DownloadTask {
        url: url.to_string(),
        file: file.to_path_buf(),
        size_bytes,
        // A Modrinth file carries a SHA-512 and a CurseForge one a SHA-1; the
        // two lengths tell them apart.
        checksum: match sha {
            Some(sha) if sha.len() == 40 => download::Checksum::Sha1(sha),
            Some(sha) => download::Checksum::Sha512(sha),
            None => download::Checksum::None,
        },
        task_type,
    }
}

/// The first version that has a file and matches the instance's loader and
/// Minecraft version, falling back to the first that has a file at all. An empty
/// loader/version list on the version means "no constraint".
pub(crate) fn pick_modrinth_version<'a>(
    versions: &'a [Value],
    minecraft: Option<&str>,
    loader: Option<&str>,
) -> Option<&'a Value> {
    let has_file = |version: &Value| {
        version
            .get("files")
            .and_then(Value::as_array)
            .is_some_and(|files| !files.is_empty())
    };
    let matches = |version: &Value| {
        let list_ok = |key: &str, wanted: Option<&str>| match wanted {
            None => true,
            Some(wanted) => match version.get(key).and_then(Value::as_array) {
                Some(list) if !list.is_empty() => {
                    list.iter().any(|item| item.as_str() == Some(wanted))
                }
                _ => true,
            },
        };
        has_file(version) && list_ok("game_versions", minecraft) && list_ok("loaders", loader)
    };
    versions
        .iter()
        .find(|version| matches(version))
        .or_else(|| versions.iter().find(|version| has_file(version)))
}
