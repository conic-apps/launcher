// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    io::{Read, SeekFrom},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use async_fs::OpenOptions;
use futures::{AsyncSeekExt, AsyncWriteExt, StreamExt, TryStreamExt};
use log::warn;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use reqwest::{IntoUrl, header::ACCEPT_RANGES};
use serde::{Deserialize, Serialize};

use config::download::{DownloadConfig, MirrorConfig};
use sha2::Digest;
use shared::HTTP_CLIENT;
use task::{Progress, Step};

mod error;
pub mod task;

pub use error::*;
use url::Url;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadType {
    VersionInfo,
    Assets,
    Libraries,
    MojangJava,
    AuthlibInjector,
    Unknown,
}

struct Mirror(String, Arc<AtomicU64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MirrorUsage {
    libraries: HashMap<String, Arc<AtomicU64>>,
    assets: HashMap<String, Arc<AtomicU64>>,
}

impl MirrorUsage {
    fn new(mirror_config: &MirrorConfig) -> Self {
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

#[derive(Clone, PartialEq)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
    None,
}

#[derive(Clone)]
pub struct DownloadTask {
    pub url: String,
    pub file: PathBuf,
    pub size_bytes: Option<u64>,
    pub checksum: Checksum,
    pub r#type: DownloadType,
}

impl DownloadTask {
    pub fn classify(&self) -> Result<Self> {
        if self.r#type != DownloadType::Unknown {
            return Ok(self.clone());
        };
        let url = Url::parse(&self.url)?;
        let host = if let Some(host) = url.host_str() {
            host
        } else {
            return Ok(self.clone());
        };
        let download_type = match host {
            "resources.download.minecraft.net" => DownloadType::Assets,
            "libraries.minecraft.net" => DownloadType::Libraries,
            _ => DownloadType::Unknown,
        };
        Ok(Self {
            r#type: download_type,
            ..self.clone()
        })
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

enum Hasher {
    Sha1(sha1_smol::Sha1),
    Sha256(sha2::Sha256),
    None,
}

impl From<&Checksum> for Hasher {
    fn from(value: &Checksum) -> Self {
        match value {
            Checksum::Sha1(_) => Self::Sha1(sha1_smol::Sha1::new()),
            Checksum::Sha256(_) => Self::Sha256(sha2::Sha256::new()),
            Checksum::None => Self::None,
        }
    }
}

impl Hasher {
    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Sha1(sha1_hasher) => sha1_hasher.update(data),
            Self::Sha256(sha256_hasher) => sha256_hasher.update(data),
            Self::None => (),
        }
    }
    fn verify(self, checksum: &Checksum) -> bool {
        match (self, checksum) {
            (Self::Sha1(sha1_hasher), Checksum::Sha1(sha1_checksum)) => {
                &sha1_hasher.digest().to_string() == sha1_checksum
            }
            (Self::Sha256(sha256_hasher), Checksum::Sha256(sha256_checksum)) => {
                &format!("{:02x}", sha256_hasher.finalize()) == sha256_checksum
            }
            (Self::None, Checksum::None) => true,
            _ => false,
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

pub async fn download(download: &DownloadTask, progress: &Progress) -> Result<()> {
    progress.reset(Ordering::SeqCst);
    let file_path = download.file.clone();
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

    if let Some(len) = response.content_length() {
        progress.total.store(len, Ordering::SeqCst);
    }
    let mut file = async_fs::File::create(&file_path).await?;
    let mut hasher = Hasher::from(&download.checksum);
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        progress
            .completed
            .fetch_add(chunk.len() as u64, Ordering::SeqCst);
        speed_counter_input.fetch_add(chunk.len() as u64, Ordering::SeqCst);
    }
    if !hasher.verify(&download.checksum) {
        return Err(Error::Sha1Missmatch(url));
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

    progress.completed.store(0, Ordering::SeqCst);
    progress
        .total
        .store(download_tasks.len() as u64, Ordering::SeqCst);
    {
        let mut task = progress
            .step
            .lock()
            .expect("Internal error: another thread hold lock and panic");
        *task = Step::DownloadFiles;
    }

    futures::stream::iter(download_tasks)
        .map(Ok)
        .try_for_each_concurrent(download_config.max_connections, |task| {
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
    progress: &Progress,
) -> Vec<DownloadTask> {
    let completed = progress.completed.clone();
    {
        let mut task = progress
            .step
            .lock()
            .expect("Internal error: another thread hold lock and panic");
        *task = Step::VerifyExistingFiles;
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
    progress: &Progress,
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
    config: &DownloadConfig,
    progress: Progress,
    speed_counter_input: Arc<AtomicU64>,
) -> Result<()> {
    if let Some(length) = task.size_bytes
        && is_support_range(&task.url).await == Some(true)
    {
        return inner_chunk_download_executer(task, length, config, &progress, speed_counter_input)
            .await;
    }
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
    }
    file.sync_all().await?;
    if !hasher.verify(&task.checksum) {
        return Err(Error::Sha1Missmatch(url));
    }
    progress.completed.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

async fn is_support_range<U: IntoUrl>(url: U) -> Option<bool> {
    let response = HTTP_CLIENT.head(url).send().await.ok()?;
    let accept_ranges = response
        .headers()
        .get(ACCEPT_RANGES)
        .and_then(|x| x.to_str().ok())
        .unwrap_or("");
    Some(accept_ranges.eq_ignore_ascii_case("bytes"))
}

async fn inner_chunk_download_executer(
    task: &DownloadTask,
    length: u64,
    config: &DownloadConfig,
    progress: &Progress,
    speed_counter_input: Arc<AtomicU64>,
) -> Result<()> {
    let chunks = calculate_chunks_length(length);
    let file_path = task.file.clone();
    if let Some(parent) = file_path.parent() {
        async_fs::create_dir_all(parent).await?;
    }
    futures::stream::iter(chunks)
        .map(Ok)
        .try_for_each_concurrent(4, async |range| {
            let mut result = Ok(());
            for retried in 0..10 {
                match download_slice(
                    task.clone(),
                    config,
                    progress,
                    speed_counter_input.clone(),
                    range,
                )
                .await
                {
                    Ok(()) => return Ok(()),
                    Err(e) => result = Err(e),
                };
                println!("{:?}", result);
                println!("retried: {retried}");
            }
            result
        })
        .await?;
    progress.completed.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn calculate_chunks_length(length: u64) -> Vec<(u64, u64)> {
    if length < 4 * 1000 * 1000 {
        return vec![(0, length - 1)];
    }
    let chunk_count = if length < 30 * 1000 * 1000 {
        length / (2 * 1000 * 1000) + 1
    } else if length < 100 {
        length / (4 * 1000 * 1000) + 1
    } else {
        length / (10 * 1000 * 1000) + 1
    };
    let chunk_size = length / chunk_count;
    let mut chunks = Vec::with_capacity(chunk_count as usize);
    for i in 0..chunk_count {
        if i == chunk_count - 1 {
            chunks.push((i * chunk_size, length - 1));
        } else {
            chunks.push((i * chunk_size, (i + 1) * chunk_size - 1));
        }
    }
    chunks
}

async fn download_slice(
    task: DownloadTask,
    config: &DownloadConfig,
    progress: &Progress,
    speed_counter_input: Arc<AtomicU64>,
    range: (u64, u64),
) -> Result<()> {
    let url = task.url.clone();
    if let Some(parent) = task.file.parent() {
        async_fs::create_dir_all(parent).await?;
    }
    let mut target_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(task.file)
        .await?;
    target_file.seek(SeekFrom::Start(range.0)).await?;
    let mut response = HTTP_CLIENT
        .get(&url)
        .header("Range", format!("bytes={}-{}", range.0, range.1))
        .send()
        .await?
        .error_for_status()?;
    let mut size = 0u64;
    while let Some(chunk) = response.chunk().await? {
        while progress.speed.load(Ordering::SeqCst) > config.max_download_speed
            && config.max_download_speed > 1024
        {
            async_io::Timer::after(Duration::from_millis(100)).await;
        }
        target_file.write_all(&chunk).await?;
        speed_counter_input.fetch_add(chunk.len() as u64, Ordering::SeqCst);
        size += chunk.len() as u64;
    }
    if size != range.1 - range.0 + 1 {
        Err(Error::ChunkLengthMismatch)
    } else {
        Ok(())
    }
}
