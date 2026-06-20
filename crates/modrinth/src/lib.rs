// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

pub mod error;

use std::{
    collections::HashMap,
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use download::{Checksum, DownloadTask, DownloadTaskType, task::Progress};
use error::*;
use folder::DATA_LOCATION;
use futures::stream::{AbortHandle, Abortable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::HTTP_CLIENT;
use tauri::{
    Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};
use url::Url;
use uuid::Uuid;

struct PluginState {
    abort_handles: Arc<Mutex<HashMap<Uuid, AbortHandle>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("modrinth")
        .invoke_handler(tauri::generate_handler![
            cmd_search_projects,
            cmd_get_project,
            cmd_get_multiple_projects,
            cmd_get_all_dependencies,
            cmd_list_project_versions,
            cmd_spawn_download_mod_task,
            cmd_cancel_download_task,
        ])
        .build()
}

#[command]
async fn cmd_search_projects(params: SearchParameters) -> Result<Value> {
    search_projects(&params).await
}

#[command]
async fn cmd_get_project(id_or_slug: &str) -> Result<Value> {
    get_project(id_or_slug).await
}

#[command]
async fn cmd_get_multiple_projects(ids: Vec<&str>) -> Result<Value> {
    get_multiple_projects(&ids).await
}

#[command]
async fn cmd_get_all_dependencies(id: &str) -> Result<Value> {
    get_all_dependencies(id).await
}

#[command]
async fn cmd_list_project_versions(
    id_or_slug: &str,
    params: ListProjectVersionsParams,
) -> Result<Value> {
    list_project_versions(id_or_slug, &params).await
}

#[derive(Serialize, Clone)]
pub struct DownloadEvent {
    pub task_id: Uuid,
    pub progress: Progress,
}

#[command]
async fn cmd_spawn_download_mod_task(
    state: State<'_, PluginState>,
    mod_file: ModFile,
    task_id: Uuid,
    instance_id: Uuid,
    channel: Channel<DownloadEvent>,
) -> Result<()> {
    let progress = Progress::default();
    let task_status = Arc::new(Mutex::new(DownloadEvent { task_id, progress }));
    let (handle, reg) = AbortHandle::new_pair();
    let future = Abortable::new(
        download_mod(mod_file, task_status.clone(), &instance_id),
        reg,
    );
    {
        let mut running_tasks = state.abort_handles.lock().expect("Internal error");
        (*running_tasks).insert(task_id, handle);
    }
    let finished = Arc::new(AtomicBool::new(false));
    let event_sender_thread = {
        let status_cloned = task_status.clone();
        let finished = finished.clone();
        thread::spawn(move || {
            while !finished.load(Ordering::SeqCst) {
                let _ = channel.send(status_cloned.lock().expect("Internal error").clone());
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };
    let result = match future.await {
        Ok(result) => result,
        Err(e) => Err(Error::Aborted(e)),
    };
    finished.store(true, Ordering::SeqCst);
    let _ = event_sender_thread.join();
    result
}

#[command]
fn cmd_cancel_download_task(state: State<'_, PluginState>, task_id: Uuid) {
    let mut running_tasks = state.abort_handles.lock().expect("Internal error");
    if let Some(handle) = running_tasks.get(&task_id) {
        handle.abort();
    }
    (*running_tasks).remove(&task_id);
}

#[derive(Serialize, Deserialize)]
pub struct SearchParameters {
    query: Option<String>,
    facets: Option<String>,
    index: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

pub async fn search_projects(params: &SearchParameters) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get("https://api.modrinth.com/v2/search")
        .query(params)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_project(id_or_slug: &str) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get(format!("https://api.modrinth.com/v2/project/{id_or_slug}"))
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_multiple_projects(ids: &[&str]) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get("https://api.modrinth.com/v2/projects")
        .query(ids)
        .send()
        .await?
        .json()
        .await?)
}

pub async fn get_all_dependencies(id: &str) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get(format!(
            "https://api.modrinth.com/v2/project/{id}/dependencies"
        ))
        .send()
        .await?
        .json()
        .await?)
}

#[derive(Serialize, Deserialize)]
pub struct ListProjectVersionsParams {
    loaders: Option<String>,
    game_versions: Option<String>,
    featured: Option<String>,
    include_changelog: Option<String>,
}

pub async fn list_project_versions(
    id_or_slug: &str,
    params: &ListProjectVersionsParams,
) -> Result<Value> {
    Ok(HTTP_CLIENT
        .get(format!(
            "https://api.modrinth.com/v2/project/{id_or_slug}/version"
        ))
        .query(params)
        .send()
        .await?
        .json()
        .await?)
}

#[derive(Deserialize)]
pub struct ModFile {
    pub url: String,
    pub file_name: String,
    pub sha512: String,
    pub size_bytes: u64,
}

pub async fn download_mod(
    mod_file: ModFile,
    status: Arc<Mutex<DownloadEvent>>,
    instance_id: &Uuid,
) -> Result<()> {
    let url_parsed = Url::from_str(&mod_file.url).unwrap();
    if Some("cdn.modrinth.com") == url_parsed.host_str() || url_parsed.host_str().is_none() {
        panic!("xx")
    }
    let mods_folder_path = DATA_LOCATION.get_instance_root(instance_id).join("mods");
    let download_task = DownloadTask {
        url: mod_file.url,
        file: mods_folder_path.join(mod_file.file_name),
        size_bytes: Some(mod_file.size_bytes),
        checksum: Checksum::Sha512(mod_file.sha512),
        task_type: DownloadTaskType::ModrinthMod,
    };
    let progress = status.lock().expect("Internal error").progress.clone();
    download::download(&download_task, &progress).await?;
    Ok(())
}
