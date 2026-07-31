// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use futures::{AsyncWriteExt, StreamExt, TryStreamExt};
use log::warn;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use config::download::DownloadConfig;
use progress::{DownloadPhase, DownloadState};
use shared::HTTP_CLIENT;

pub mod checksum;
pub mod error;
pub(crate) mod mirror;
pub mod progress;
// pub mod state;

pub use checksum::*;
pub use error::*;
use mirror::*;
use tauri::{
    Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};
use url::Url;
use uuid::Uuid;

#[derive(Clone, Default)]
struct PluginState {
    task: Arc<Mutex<HashMap<Uuid, tokio::task::AbortHandle>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("download")
        .invoke_handler(tauri::generate_handler![
            cmd_spawn_download_task,
            cmd_cancel_download_task
        ])
        .build()
}

#[command]
async fn cmd_spawn_download_task(
    state: State<'_, PluginState>,
    download_task: DownloadTask,
    task_id: Uuid,
    channel: Channel<DownloadState>,
) -> Result<()> {
    let task_status = DownloadState::default();
    let finished = Arc::new(AtomicBool::new(false));
    let handle = tokio::spawn({
        let task_status_cloned = task_status.clone();
        let finished = finished.clone();
        async move {
            let result = download(&download_task, &task_status_cloned).await;
            finished.store(true, Ordering::SeqCst);
            result
        }
    });
    {
        let mut current_task = state.task.lock().expect("Internal error");
        (*current_task).insert(task_id, handle.abort_handle());
    }
    let event_sender_thread = {
        let status_cloned = task_status.clone();
        let finished = finished.clone();
        thread::spawn(move || {
            while !finished.load(Ordering::SeqCst) {
                let _ = channel.send(status_cloned.clone());
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };
    let result = match handle.await {
        Ok(result) => result,
        Err(error) => {
            warn!("Installation cancelled");
            Err(Error::Aborted(error))
        }
    };
    let _ = event_sender_thread.join();
    {
        let mut current_task = state.task.lock().expect("Internal error");
        (*current_task).remove(&task_id);
    }
    result
}

#[command]
fn cmd_cancel_download_task(state: State<'_, PluginState>, task_id: Uuid) {
    let mut current_task = state.task.lock().expect("Internal error");
    if let Some(handle) = current_task.get(&task_id) {
        handle.abort();
        warn!("Cancelling installation!");
    }
    (*current_task).remove(&task_id);
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadTaskType {
    VersionInfo,
    Assets,
    Libraries,
    MojangJava,
    AuthlibInjector,
    ModrinthMod,
    CurseforgeMod,
    BeatThis,
    Unknown,
}

#[derive(Clone, Deserialize)]
pub struct DownloadTask {
    pub url: String,
    pub file: PathBuf,
    pub size_bytes: Option<u64>,
    pub checksum: Checksum,
    pub task_type: DownloadTaskType,
}

impl DownloadTask {
    fn classify(&self) -> Result<Self> {
        if self.task_type != DownloadTaskType::Unknown {
            return Ok(self.clone());
        };
        let url = Url::parse(&self.url)?;
        let host = if let Some(host) = url.host_str() {
            host
        } else {
            return Ok(self.clone());
        };
        let download_type = match host {
            "resources.download.minecraft.net" => DownloadTaskType::Assets,
            "libraries.minecraft.net" => DownloadTaskType::Libraries,
            "cdn.modrinth.com" => DownloadTaskType::ModrinthMod,
            _ => DownloadTaskType::Unknown,
        };
        Ok(Self {
            task_type: download_type,
            ..self.clone()
        })
    }

    fn assignment_mirror(
        self,
        mirror_usage: &MirrorUsage,
        disabled_mirrors: &[String],
    ) -> Option<(DownloadTask, Mirror)> {
        match self.task_type {
            DownloadTaskType::Libraries => {
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
            DownloadTaskType::Assets => {
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

struct ScopedThread {
    is_aborted: Arc<AtomicBool>,
}

impl ScopedThread {
    fn new<F>(f: F) -> Self
    where
        F: FnOnce(Arc<AtomicBool>) + Send + 'static,
    {
        let is_aborted = Arc::new(AtomicBool::new(false));
        let is_aborted_cloned = Arc::new(AtomicBool::new(false));
        thread::spawn(move || f(is_aborted_cloned));
        Self { is_aborted }
    }
}

impl Drop for ScopedThread {
    fn drop(&mut self) {
        self.is_aborted.store(true, Ordering::SeqCst);
    }
}

pub async fn download(download: &DownloadTask, progress: &DownloadState) -> Result<()> {
    progress.reset(Ordering::SeqCst);
    progress.total_tasks.store(1, Ordering::SeqCst);
    progress.completed_tasks.store(0, Ordering::SeqCst);
    let file_path = download.file.clone();
    let mut file = async_fs::File::create(&file_path).await?;
    let url = download.url.clone();
    if let Some(parent) = file_path.parent() {
        async_fs::create_dir_all(parent).await?
    }
    let mut response = HTTP_CLIENT.get(&url).send().await?.error_for_status()?;
    let speed_counter_input = Arc::new(AtomicU64::new(0));
    let _speed_thread = {
        let speed_counter_input = speed_counter_input.clone();
        let speed_counter_output = progress.speed.clone();
        ScopedThread::new(move |is_finished| {
            speed_counter_loop(speed_counter_input, speed_counter_output, is_finished)
        })
    };
    let response_length = response.content_length();
    if let Some(file_size) = download.size_bytes
        && response_length.is_none()
    {
        progress.total_bytes.store(file_size, Ordering::SeqCst);
    } else if let Some(response_length) = response_length
        && download.size_bytes.is_none()
    {
        progress
            .total_bytes
            .store(response_length, Ordering::SeqCst);
    } else if let Some(response_length) = response_length
        && let Some(file_size) = download.size_bytes
        && response_length == file_size
    {
        progress.total_bytes.store(file_size, Ordering::SeqCst);
    } else if let Some(response_length) = response_length
        && let Some(file_size) = download.size_bytes
        && response_length != file_size
    {
        progress.total_bytes.store(file_size, Ordering::SeqCst);
    };
    let mut hasher = Hasher::from(&download.checksum);
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        progress
            .completed_bytes
            .fetch_add(chunk.len() as u64, Ordering::SeqCst);
        speed_counter_input.fetch_add(chunk.len() as u64, Ordering::SeqCst);
    }
    if !hasher.verify(&download.checksum) {
        return Err(Error::ChecksumMissmatch(url));
    }
    file.sync_all().await?;
    progress.completed_bytes.store(
        progress.total_bytes.load(Ordering::SeqCst),
        Ordering::SeqCst,
    );
    progress.completed_tasks.store(1, Ordering::SeqCst);
    Ok(())
}

pub async fn download_concurrent(
    tasks: Vec<DownloadTask>,
    progress: &DownloadState,
    download_config: DownloadConfig,
) -> Result<()> {
    let download_tasks: Result<Vec<DownloadTask>> =
        filter_existing_and_verified_files(tasks, progress)
            .into_iter()
            .map(|x| x.classify())
            .collect();
    let download_tasks = download_tasks?;

    let speed_counter_input = Arc::new(AtomicU64::new(0));
    let _speed_thread = {
        let speed_counter_input = speed_counter_input.clone();
        let speed_counter_output = progress.speed.clone();
        ScopedThread::new(move |is_finished| {
            speed_counter_loop(speed_counter_input, speed_counter_output, is_finished)
        })
    };

    let mirror_usage = MirrorUsage::new(&download_config.mirror);

    progress.completed_tasks.store(0, Ordering::SeqCst);
    progress
        .total_tasks
        .store(download_tasks.len() as u64, Ordering::SeqCst);
    progress.completed_bytes.store(0, Ordering::SeqCst);
    progress.total_bytes.store(
        download_tasks
            .iter()
            .map(|x| x.size_bytes.unwrap_or_default())
            .sum(),
        Ordering::SeqCst,
    );
    {
        let mut task = progress
            .phase
            .lock()
            .expect("Internal error: another thread hold lock and panic");
        *task = DownloadPhase::DownloadFiles;
    }

    futures::stream::iter(download_tasks)
        .map(Ok)
        .try_for_each_concurrent(8, |task| {
            inner_download_future(
                task,
                &download_config,
                &mirror_usage,
                progress,
                speed_counter_input.clone(),
            )
        })
        .await
}

pub fn filter_existing_and_verified_files(
    downloads: Vec<DownloadTask>,
    progress: &DownloadState,
) -> Vec<DownloadTask> {
    let completed = progress.completed_tasks.clone();
    {
        let mut task = progress
            .phase
            .lock()
            .expect("Internal error: another thread hold lock and panic");
        *task = DownloadPhase::VerifyExistingFiles;
    }
    progress.total_tasks.store(0, Ordering::SeqCst);
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
        let check_result = verify_checksum_from_read(&mut file, &download.checksum);
        completed.fetch_add(1, Ordering::SeqCst);
        match check_result {
            Some(x) => !x,
            None => true,
        }
    };
    let downloads: Vec<_> = downloads.into_par_iter().filter(filter_op).collect();
    downloads
}

fn verify_checksum_from_read<R: Read>(source: &mut R, checksum: &Checksum) -> Option<bool> {
    if checksum == &Checksum::None {
        return None;
    }
    let mut hasher = Hasher::from(checksum);
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = source.read(&mut buffer).ok()?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Some(hasher.verify(checksum))
}

fn speed_counter_loop(input: Arc<AtomicU64>, output: Arc<AtomicU64>, finished: Arc<AtomicBool>) {
    let mut buffer = Vec::with_capacity(20);
    while finished.load(Ordering::SeqCst) {
        buffer.push(input.swap(0, Ordering::SeqCst));
        while buffer.len() > 20 {
            buffer.remove(0);
        }
        output.store(buffer.iter().sum(), Ordering::SeqCst);
        thread::sleep(Duration::from_millis(2000));
    }
}

async fn inner_download_future(
    task: DownloadTask,
    config: &DownloadConfig,
    mirror_usage: &MirrorUsage,
    progress: &DownloadState,
    speed_counter_input: Arc<AtomicU64>,
) -> Result<()> {
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
            inner_download_executer(&task, config, progress.clone(), speed_counter_input.clone())
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
        warn!("Downloaded failed: {}, retried: {}", task.url, retried);
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
    config: &DownloadConfig,
    progress: DownloadState,
    speed_counter_input: Arc<AtomicU64>,
) -> Result<()> {
    // if let Some(length) = task.size_bytes
    //     && is_support_range(&task.url).await == Some(true)
    // {
    //     return inner_chunk_download_executer(task, length, config, &progress, speed_counter_input)
    //         .await;
    // }
    let file_path = task.file.clone();
    let url = task.url.clone();
    if let Some(parent) = file_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }
    let mut response = HTTP_CLIENT.get(&url).send().await?.error_for_status()?;
    let mut file = async_fs::File::create(&file_path).await?;
    let mut hasher = Hasher::from(&task.checksum);
    while let Some(chunk) = response.chunk().await? {
        while progress.speed.load(Ordering::SeqCst) > config.max_download_speed
            && config.max_download_speed > 1024
        {
            async_io::Timer::after(Duration::from_millis(100)).await;
        }
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        speed_counter_input.fetch_add(chunk.len() as u64, Ordering::SeqCst);
        progress
            .completed_bytes
            .fetch_add(chunk.len() as u64, Ordering::SeqCst);
    }
    file.sync_all().await?;
    if !hasher.verify(&task.checksum) {
        return Err(Error::ChecksumMissmatch(url));
    }
    progress.completed_tasks.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

// async fn is_support_range<U: IntoUrl>(url: U) -> Option<bool> {
//     let response = HTTP_CLIENT.head(url).send().await.ok()?;
//     let accept_ranges = response
//         .headers()
//         .get(ACCEPT_RANGES)
//         .and_then(|x| x.to_str().ok())
//         .unwrap_or("");
//     Some(accept_ranges.eq_ignore_ascii_case("bytes"))
// }

// async fn inner_chunk_download_executer(
//     task: &DownloadTask,
//     length: u64,
//     config: &DownloadConfig,
//     progress: &DownloadState,
//     speed_counter_input: Arc<AtomicU64>,
// ) -> Result<()> {
//     let chunks = calculate_chunks_length(length);
//     let file_path = task.file.clone();
//     if let Some(parent) = file_path.parent() {
//         async_fs::create_dir_all(parent).await?;
//     }
//     futures::stream::iter(chunks)
//         .map(Ok)
//         .try_for_each_concurrent(4, async |range| {
//             let mut result = Ok(());
//             for retried in 0..10 {
//                 match download_slice(
//                     task.clone(),
//                     config,
//                     progress,
//                     speed_counter_input.clone(),
//                     range,
//                 )
//                 .await
//                 {
//                     Ok(()) => return Ok(()),
//                     Err(e) => result = Err(e),
//                 };
//                 warn!("{:?}", result);
//                 warn!("retried: {retried}");
//             }
//             result
//         })
//         .await?;
//     progress.completed.fetch_add(1, Ordering::SeqCst);
//     Ok(())
// }

// fn calculate_chunks_length(length: u64) -> Vec<(u64, u64)> {
//     if length < 4 * 1000 * 1000 {
//         return vec![(0, length - 1)];
//     }
//     let chunk_count = if length < 30 * 1000 * 1000 {
//         length / (2 * 1000 * 1000) + 1
//     } else if length < 100 {
//         length / (4 * 1000 * 1000) + 1
//     } else {
//         length / (10 * 1000 * 1000) + 1
//     };
//     let chunk_size = length / chunk_count;
//     let mut chunks = Vec::with_capacity(chunk_count as usize);
//     for i in 0..chunk_count {
//         if i == chunk_count - 1 {
//             chunks.push((i * chunk_size, length - 1));
//         } else {
//             chunks.push((i * chunk_size, (i + 1) * chunk_size - 1));
//         }
//     }
//     chunks
// }

// async fn download_slice(
//     task: DownloadTask,
//     config: &DownloadConfig,
//     progress: &DownloadState,
//     speed_counter_input: Arc<AtomicU64>,
//     range: (u64, u64),
// ) -> Result<()> {
//     let url = task.url.clone();
//     if let Some(parent) = task.file.parent() {
//         async_fs::create_dir_all(parent).await?;
//     }
//     let mut target_file = OpenOptions::new()
//         .write(true)
//         .create(true)
//         .truncate(true)
//         .open(task.file)
//         .await?;
//     target_file.seek(SeekFrom::Start(range.0)).await?;
//     let mut response = HTTP_CLIENT
//         .get(&url)
//         .header("Range", format!("bytes={}-{}", range.0, range.1))
//         .send()
//         .await?
//         .error_for_status()?;
//     let mut size = 0u64;
//     while let Some(chunk) = response.chunk().await? {
//         while progress.speed.load(Ordering::SeqCst) > config.max_download_speed
//             && config.max_download_speed > 1024
//         {
//             async_io::Timer::after(Duration::from_millis(100)).await;
//         }
//         target_file.write_all(&chunk).await?;
//         speed_counter_input.fetch_add(chunk.len() as u64, Ordering::SeqCst);
//         size += chunk.len() as u64;
//     }
//     if size != range.1 - range.0 + 1 {
//         Err(Error::ChunkLengthMismatch)
//     } else {
//         Ok(())
//     }
// }
