// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use futures::{AsyncWriteExt, StreamExt, TryStreamExt};
use log::warn;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Url};

use config::download::{DownloadConfig, MirrorConfig};
use shared::{HTTP_CLIENT, MAIN_WINDOW};
use task::{Progress, Task};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadType {
    VersionInfo,
    Assets,
    Libraries,
    MojangJava,
    Unknown,
}

struct Mirror(String, Arc<AtomicU64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MirrorUsage {
    libraries: HashMap<String, Arc<AtomicU64>>,
    assets: HashMap<String, Arc<AtomicU64>>,
}

impl MirrorUsage {
    fn new(mirror_config: MirrorConfig) -> Self {
        Self {
            libraries: mirror_config
                .libraries
                .iter()
                .map(|x| (x.to_string(), Arc::new(AtomicU64::new(0))))
                .collect(),
            assets: mirror_config
                .assets
                .iter()
                .map(|x| (x.to_string(), Arc::new(AtomicU64::new(0))))
                .collect(),
        }
    }
    /// Get a fewest connections libraries mirror
    fn get_libraries_mirror(&self, disabled: &[String]) -> Option<Mirror> {
        let (k, v) = self
            .libraries
            .iter()
            .filter(|x| !disabled.iter().any(|y| x.0 == y))
            .min_by(|x, y| x.1.load(Ordering::SeqCst).cmp(&y.1.load(Ordering::SeqCst)))?;
        Some(Mirror(k.clone(), v.clone()))
    }
    /// Get a fewest connections assets mirror
    fn get_assets_mirror(&self, disabled: &[String]) -> Option<Mirror> {
        let (k, v) = self
            .assets
            .iter()
            .filter(|x| !disabled.iter().any(|y| x.0 == y))
            .min_by(|x, y| x.1.load(Ordering::SeqCst).cmp(&y.1.load(Ordering::SeqCst)))?;
        Some(Mirror(k.clone(), v.clone()))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub url: String,
    pub file: PathBuf,
    pub sha1: Option<String>,
    pub r#type: DownloadType,
}

impl DownloadTask {
    pub fn classify(self) -> Self {
        if self.r#type != DownloadType::Unknown {
            return self.clone();
        };
        let url = Url::parse(&self.url).unwrap();
        let host = if let Some(host) = url.host_str() {
            host
        } else {
            return self.clone();
        };
        let download_type = match host {
            "resources.download.minecraft.net" => DownloadType::Assets,
            "libraries.minecraft.net" => DownloadType::Libraries,
            _ => DownloadType::Unknown,
        };
        Self {
            r#type: download_type,
            ..self
        }
    }

    fn assignment_mirror(
        self,
        mirror_usage: &MirrorUsage,
        disabled_mirrors: &[String],
    ) -> Option<(DownloadTask, Mirror)> {
        match self.r#type {
            DownloadType::Libraries => {
                let mirror = mirror_usage.get_libraries_mirror(disabled_mirrors)?;
                mirror.1.fetch_add(1, Ordering::SeqCst);
                Some((
                    DownloadTask {
                        url: self
                            .url
                            .replace("https://libraries.minecraft.net", &mirror.0),
                        ..self
                    },
                    mirror,
                ))
            }
            DownloadType::Assets => {
                let mirror = mirror_usage.get_assets_mirror(disabled_mirrors)?;
                mirror.1.fetch_add(1, Ordering::SeqCst);
                Some((
                    DownloadTask {
                        url: self
                            .url
                            .replace("https://resources.download.minecraft.net", &mirror.0),
                        ..self
                    },
                    mirror,
                ))
            }
            _ => None,
        }
    }
}

pub async fn download(download: &DownloadTask, progress: Progress) -> anyhow::Result<()> {
    progress.reset(Ordering::SeqCst);
    let file_path = download.file.clone();
    if let Some(parent) = file_path.parent() {
        async_fs::create_dir_all(parent).await?
    }
    let mut response = HTTP_CLIENT.get(download.url.clone()).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Download failed"));
    }
    if let Some(len) = response.content_length() {
        progress.total.store(len, Ordering::SeqCst);
    }
    let mut file = async_fs::File::create(&file_path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        progress
            .completed
            .fetch_add(chunk.len() as u64, Ordering::SeqCst);
    }
    file.sync_all().await?;
    progress
        .completed
        .store(progress.total.load(Ordering::SeqCst), Ordering::SeqCst);
    Ok(())
}

pub async fn download_and_check(
    download: &DownloadTask,
    progress: &Progress,
) -> anyhow::Result<()> {
    progress.reset(Ordering::SeqCst);
    let file_path = download.file.clone();
    if let Some(parent) = file_path.parent() {
        async_fs::create_dir_all(parent).await?
    }
    let mut response = HTTP_CLIENT.get(download.url.clone()).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Download failed"));
    }
    // TODO: Speed
    if let Some(len) = response.content_length() {
        progress.total.store(len, Ordering::SeqCst);
    }
    let mut file = async_fs::File::create(&file_path).await?;
    let mut hasher = sha1_smol::Sha1::new();
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        progress
            .completed
            .fetch_add(chunk.len() as u64, Ordering::SeqCst);
    }
    if let Some(sha1) = download.sha1.as_ref()
        && &hasher.digest().to_string() != sha1
    {
        return Err(anyhow::Error::msg("SHA1 check failed".to_string()));
    }
    file.sync_all().await?;
    progress
        .completed
        .store(progress.total.load(Ordering::SeqCst), Ordering::SeqCst);
    Ok(())
}

pub async fn download_concurrent(
    tasks: Vec<DownloadTask>,
    progress: &Progress,
    download_config: DownloadConfig,
) -> anyhow::Result<()> {
    inner_download_concurrent(tasks, progress, download_config, false).await
}

pub async fn download_concurrent_and_check(
    tasks: Vec<DownloadTask>,
    progress: &Progress,
    download_config: DownloadConfig,
) -> anyhow::Result<()> {
    inner_download_concurrent(tasks, progress, download_config, true).await
}

async fn inner_download_concurrent(
    tasks: Vec<DownloadTask>,
    progress: &Progress,
    download_config: DownloadConfig,
    verify_checksum: bool,
) -> anyhow::Result<()> {
    let download_tasks: Vec<DownloadTask> = filter_existing_and_verified_files(tasks, progress)
        .into_iter()
        .map(|x| x.classify())
        .collect();

    let is_finished = Arc::new(AtomicBool::new(false));
    let speed_thread = {
        let speed_counter = progress.speed.clone();
        let is_finished_cloned = is_finished.clone();
        thread::spawn(move || speed_counter_loop(speed_counter, is_finished_cloned))
    };

    let mirror_usage = MirrorUsage::new(download_config.mirror);
    let mirror_usage_sender_thread = {
        let mirror_usage_cloned = mirror_usage.clone();
        let is_finished_cloned = is_finished.clone();
        thread::spawn(move || mirror_usage_sender_loop(mirror_usage_cloned, is_finished_cloned))
    };

    progress.completed.store(0, Ordering::SeqCst);
    progress
        .total
        .store(download_tasks.len() as u64, Ordering::SeqCst);
    {
        #[allow(clippy::unwrap_used)]
        let mut task = progress.task.lock().unwrap();
        *task = Task::DownloadFiles;
    }

    let result = futures::stream::iter(download_tasks)
        .map(|task| {
            inner_download_future(
                task,
                download_config.max_download_speed,
                &mirror_usage,
                progress,
                verify_checksum,
            )
        })
        .buffer_unordered(download_config.max_connections)
        .try_for_each_concurrent(None, |_| async { Ok(()) })
        .await;
    is_finished.store(true, Ordering::SeqCst);
    let _ = speed_thread.join();
    let _ = mirror_usage_sender_thread.join();
    result
}

pub fn filter_existing_and_verified_files(
    downloads: Vec<DownloadTask>,
    progress: &Progress,
) -> Vec<DownloadTask> {
    let completed = progress.completed.clone();
    {
        #[allow(clippy::unwrap_used)]
        let mut task = progress.task.lock().unwrap();
        *task = Task::VerifyExistingFiles;
    }
    progress.total.store(0, Ordering::SeqCst);
    let filter_op = |download: &DownloadTask| {
        if std::fs::metadata(&download.file).is_err() {
            return true;
        }
        let mut file = match std::fs::File::open(&download.file) {
            Ok(file) => file,
            Err(_) => {
                return true;
            }
        };
        if download.sha1.is_none() {
            return true;
        };
        let file_hash = match calculate_sha1_from_read(&mut file) {
            Ok(x) => x,
            Err(_) => return true,
        };
        completed.fetch_add(1, Ordering::SeqCst);
        &file_hash != download.sha1.as_ref().unwrap()
    };
    let downloads: Vec<_> = downloads.into_par_iter().filter(filter_op).collect();
    downloads
}

fn calculate_sha1_from_read<R: Read>(source: &mut R) -> anyhow::Result<String> {
    let mut hasher = sha1_smol::Sha1::new();
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = source.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(hasher.digest().to_string())
}

fn speed_counter_loop(counter: Arc<AtomicU64>, finished: Arc<AtomicBool>) {
    while finished.load(Ordering::SeqCst) {
        MAIN_WINDOW
            .emit("download_speed", counter.load(Ordering::SeqCst))
            .unwrap();
        counter.store(0, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(2000));
    }
}

fn mirror_usage_sender_loop(mirror_usage: MirrorUsage, finished: Arc<AtomicBool>) {
    while finished.load(Ordering::SeqCst) {
        MAIN_WINDOW.emit("mirror_usage", &mirror_usage).unwrap();
        thread::sleep(Duration::from_millis(500));
    }
}

async fn inner_download_future(
    task: DownloadTask,
    max_download_speed: u64,
    mirror_usage: &MirrorUsage,
    progress: &Progress,
    verify_checksum: bool,
) -> anyhow::Result<()> {
    let mut disabled_mirrors = vec![];
    let mut retried = 0;
    loop {
        retried += 1;
        let (task, mirror) = match task
            .clone()
            .assignment_mirror(mirror_usage, &disabled_mirrors)
        {
            Some(x) => (x.0, Some(x.1)),
            None => (task.clone(), None),
        };
        let result =
            inner_download_executer(&task, max_download_speed, progress.clone(), verify_checksum)
                .await;
        if let Some(mirror) = &mirror {
            mirror.1.fetch_sub(1, Ordering::SeqCst);
        }
        if result.is_ok() {
            break;
        }
        let error = match result {
            Ok(_) => break,
            Err(x) => x,
        };
        warn!("Downloaded failed: {}, retried: {}", &task.url, retried);
        if let Some(mirror) = mirror {
            disabled_mirrors.push(mirror.0);
        }
        if retried >= 5 {
            return Err(error);
        }
    }
    Ok(())
}

async fn inner_download_executer(
    task: &DownloadTask,
    max_download_speed: u64,
    progress: Progress,
    verify_checksum: bool,
) -> anyhow::Result<()> {
    let file_path = task.file.clone();
    if let Some(parent) = file_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }
    let mut response = HTTP_CLIENT.get(task.url.clone()).send().await?;
    let mut file = async_fs::File::create(&file_path).await?;
    let mut hasher = sha1_smol::Sha1::new();
    while let Some(chunk) = response.chunk().await? {
        while progress.speed.load(Ordering::SeqCst) > max_download_speed
            && max_download_speed > 1024
        {
            async_io::Timer::after(Duration::from_millis(100)).await;
        }
        file.write_all(&chunk).await?;
        if verify_checksum {
            hasher.update(&chunk);
        }
        progress
            .speed
            .fetch_add(chunk.len() as u64, Ordering::SeqCst);
    }
    file.sync_all().await?;
    if let Some(sha1) = task.sha1.clone()
        && verify_checksum
        && hasher.digest().to_string() != sha1
    {
        return Err(anyhow::anyhow!("sha1 check failed"));
    }
    progress.completed.fetch_add(1, Ordering::SeqCst);
    Ok(())
}
