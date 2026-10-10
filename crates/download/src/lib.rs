// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The mirrored downloader the installer and the launcher use.
//!
//! The app owns the tasks and cancels a download by aborting the tokio
//! `JoinHandle`, so this crate carries only the downloader itself:
//! [`download`], [`download_concurrent`] and the task model.
//!
//! Chunked downloads (an `Accept-Ranges` probe selecting a parallel slice
//! path) are deliberately disabled: they are known to misbehave, so every task
//! goes through the sequential path below.

use std::{
    io::Read,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use futures::{StreamExt, TryStreamExt};
use log::{debug, error, info, trace, warn};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use config::download::DownloadConfig;
use progress::{DownloadPhase, DownloadState};
use shared::HTTP_CLIENT;

pub mod checksum;
pub mod error;
pub(crate) mod mirror;
pub mod progress;

pub use checksum::*;
pub use error::*;
use mirror::*;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadTaskType {
    VersionInfo,
    Assets,
    Libraries,
    MojangJava,
    AuthlibInjector,
    ModrinthMod,
    CurseforgeMod,
    BeatThis,
    ConicNexus,
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
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
    info!(
        "Downloading: {} -> {}",
        download.url,
        download.file.display()
    );
    progress.reset(Ordering::SeqCst);
    progress.total_tasks.store(1, Ordering::SeqCst);
    progress.completed_tasks.store(0, Ordering::SeqCst);
    let file_path = download.file.clone();
    let url = download.url.clone();
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await?
    }
    let mut file = tokio::fs::File::create(&file_path).await?;
    let response = HTTP_CLIENT.get(&url).send().await?;
    log_response(&url, &response);
    let mut response = response.error_for_status()?;
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
        debug!(
            "No Content-Length header, using declared size: {file_size} bytes for {}",
            download.url
        );
        progress.total_bytes.store(file_size, Ordering::SeqCst);
    } else if let Some(response_length) = response_length
        && download.size_bytes.is_none()
    {
        debug!(
            "No declared size, using Content-Length header: {response_length} bytes for {}",
            download.url
        );
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
        debug!(
            "Content-Length header ({response_length} bytes) does not match declared size \
             ({file_size} bytes) for {}",
            download.url
        );
        progress.total_bytes.store(file_size, Ordering::SeqCst);
    } else if download.size_bytes.is_none() {
        // Neither side declared a size. `total_bytes` stays 0, which the progress
        // bar divides by — worth a line, because an unending progress bar is the
        // symptom and the cause is not otherwise anywhere.
        debug!(
            "No size for {}: neither a declared size nor a Content-Length header",
            download.url
        );
    }
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
        error!(
            "Checksum verification failed for {}: expected {:?}",
            url, download.checksum
        );
        return Err(Error::ChecksumMissmatch(url));
    }
    debug!("Checksum verified for {url}");
    // A `sync_all` failure is a full or read-only disk, and it arrives here with
    // every byte already written and the checksum already satisfied — so without
    // this line the task reports success over a file that is not on the disk.
    file.sync_all().await.map_err(|error| {
        error!("Could not flush {url} to {}: {error}", file_path.display());
        Error::Io(error)
    })?;
    progress.completed_bytes.store(
        progress.total_bytes.load(Ordering::SeqCst),
        Ordering::SeqCst,
    );
    progress.completed_tasks.store(1, Ordering::SeqCst);
    info!("Download finished: {url}");
    Ok(())
}

pub async fn download_concurrent(
    tasks: Vec<DownloadTask>,
    progress: &DownloadState,
    download_config: DownloadConfig,
) -> Result<()> {
    let download_tasks: Result<Vec<DownloadTask>> =
        filter_existing_and_verified_files(tasks, progress)
            .await?
            .into_iter()
            .map(|x| x.classify())
            .collect();
    let download_tasks = download_tasks?;

    info!(
        "Starting concurrent download of {} task(s), total {} byte(s)",
        download_tasks.len(),
        download_tasks
            .iter()
            .map(|x| x.size_bytes.unwrap_or_default())
            .sum::<u64>()
    );

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
        .await?;
    info!("Concurrent download finished");
    Ok(())
}

/// Drops the tasks whose file is already on disk with a matching checksum and
/// returns what is left to download.
///
/// Hashing every already-downloaded file is blocking disk and CPU work over
/// hundreds of files, so the pass runs on the blocking pool instead of inline: a
/// runtime worker cannot be parked for its whole duration, and it is the very
/// thread that has to drive the downloads which follow. `rayon` still walks the
/// files in parallel inside that task, because the pool hands us one thread and
/// hashing is exactly the work that wants one per core.
///
/// A blocking task that has started cannot be cancelled, so aborting the
/// download's `JoinHandle` during this phase detaches the pass rather than
/// stopping it. That is harmless — it only reads files and bumps a counter — but
/// a cancelled download can keep a core busy until the pass ends.
pub async fn filter_existing_and_verified_files(
    downloads: Vec<DownloadTask>,
    progress: &DownloadState,
) -> Result<Vec<DownloadTask>> {
    let completed = progress.completed_tasks.clone();
    {
        let mut task = progress
            .phase
            .lock()
            .expect("Internal error: another thread hold lock and panic");
        *task = DownloadPhase::VerifyExistingFiles;
    }
    progress.total_tasks.store(0, Ordering::SeqCst);
    let total = downloads.len();
    let downloads: Vec<DownloadTask> = tokio::task::spawn_blocking(move || {
        let filter_op = |download: &DownloadTask| {
            if std::fs::metadata(&download.file).is_err() {
                return true;
            }
            let mut file = match std::fs::File::open(&download.file) {
                Ok(file) => file,
                Err(error) => {
                    // Not "missing", so not the routine case: the file is there
                    // and cannot be read. Re-downloading it either fixes it or
                    // fails again with the real reason.
                    warn!(
                        "Could not open {} to verify it: {error}",
                        download.file.display()
                    );
                    return true;
                }
            };
            let check_result =
                verify_checksum_from_read(&mut file, &download.checksum, &download.file);
            completed.fetch_add(1, Ordering::SeqCst);
            match check_result {
                Some(x) => !x,
                // Without an expected checksum the content cannot be verified,
                // so an existing file counts as complete instead of being
                // redownloaded on every run.
                None => false,
            }
        };
        downloads.into_par_iter().filter(filter_op).collect()
    })
    .await?;
    let skipped = total - downloads.len();
    debug!(
        "Checked {total} existing file(s): {skipped} already verified, {} to download",
        downloads.len()
    );
    if skipped > 0 {
        trace!(
            "Skipped files: {skipped}, remaining tasks: {}",
            downloads.len()
        );
    }
    Ok(downloads)
}

fn verify_checksum_from_read<R: Read>(
    source: &mut R,
    checksum: &Checksum,
    path: &std::path::Path,
) -> Option<bool> {
    if checksum == &Checksum::None {
        return None;
    }
    let mut hasher = Hasher::from(checksum);
    let mut buffer = [0; 1024];
    loop {
        // A read failure returns `None` (below), and the caller takes `None` to
        // mean "no checksum" — that is, *verified* — so a truncated or
        // partially-readable file would be accepted as complete and never
        // re-fetched. `debug`, not `warn`: this only records why no verdict was
        // reached.
        let bytes_read = match source.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) => {
                debug!(
                    "Could not read {} to verify its checksum: {error}",
                    path.display()
                );
                return None;
            }
        };
        hasher.update(&buffer[..bytes_read]);
    }
    let valid = hasher.verify(checksum);
    trace!("Verified existing file checksum: valid={valid}, expected={checksum:?}");
    Some(valid)
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

/// Records what the server answered before the body is consumed.
///
/// `error_for_status` collapses every non-2xx into one message and drops the two
/// that mean something specific to a downloader: a 429 carries a `Retry-After` the
/// caller should honour, and a 404 from a mirror is worth distinguishing from a
/// 500 on the same host. Neither survives into `reqwest::Error`, and this crate's
/// retry loop rotates mirrors on any failure — so without the status there is no
/// way to tell a bad mirror from a bad URL.
///
/// Success is `debug`, not `warn`: a release log should not carry one line per
/// successful fetch, and the retry loop's own `Download succeeded` covers it.
fn log_response(url: &str, response: &reqwest::Response) {
    let status = response.status();
    if status.is_success() {
        debug!(
            "The server answered {status} for {url} ({} bytes declared)",
            response.content_length().unwrap_or(0)
        );
        return;
    }
    match response.headers().get(reqwest::header::RETRY_AFTER) {
        Some(retry_after) => warn!(
            "The server answered {status} for {url} and asked to retry after {}",
            retry_after
                .to_str()
                .unwrap_or("<an unreadable Retry-After>")
        ),
        None => warn!("The server answered {status} for {url}"),
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
            Some(x) => {
                trace!("Assigned mirror {} for {}", x.1.0, x.0.url);
                (x.0, Some(x.1))
            }
            None => (task.clone(), None),
        };
        debug!("Download attempt {retried}: {}", task.url);
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
        // The error itself is the point of the line. Without it a failed download
        // reads as "it failed, five times", and a 404, a mirror timeout and a
        // disk-full are indistinguishable — the first two are worth retrying on a
        // different mirror, the third is not retryable at all.
        warn!("Download failed: {}: {error}, retried: {retried}", task.url);
        if let Some(mirror) = mirror {
            debug!(
                "Disabling mirror {} for the rest of this download",
                mirror.0
            );
            disabled_mirrors.push(mirror.0);
        }
        if retried >= 5 {
            return Err(error);
        }
    }
    info!("Download succeeded: {}", task.url);
    Ok(())
}

async fn inner_download_executer(
    task: &DownloadTask,
    config: &DownloadConfig,
    progress: DownloadState,
    speed_counter_input: Arc<AtomicU64>,
) -> Result<()> {
    debug!("Using sequential download for {}", task.url);
    let file_path = task.file.clone();
    let url = task.url.clone();
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let response = HTTP_CLIENT.get(&url).send().await?;
    log_response(&url, &response);
    let mut response = response.error_for_status()?;
    let mut file = tokio::fs::File::create(&file_path).await?;
    let mut hasher = Hasher::from(&task.checksum);
    let mut throttled = false;
    while let Some(chunk) = response.chunk().await? {
        if progress.speed.load(Ordering::SeqCst) > config.max_download_speed
            && config.max_download_speed > 1024
        {
            // Logged once per transfer, not per chunk: the throttle itself is
            // unobservable from the progress bar, so a download that looks stalled
            // is otherwise indistinguishable from a hung one.
            if !throttled {
                debug!(
                    "Throttling {url} to {} bytes per second",
                    config.max_download_speed
                );
                throttled = true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        speed_counter_input.fetch_add(chunk.len() as u64, Ordering::SeqCst);
        progress
            .completed_bytes
            .fetch_add(chunk.len() as u64, Ordering::SeqCst);
    }
    file.sync_all().await.map_err(|error| {
        error!("Could not flush {url} to {}: {error}", file_path.display());
        Error::Io(error)
    })?;
    if !hasher.verify(&task.checksum) {
        error!(
            "Checksum verification failed for {}: expected {:?}",
            url, task.checksum
        );
        return Err(Error::ChecksumMissmatch(url));
    }
    debug!("Checksum verified for {url}");
    progress.completed_tasks.fetch_add(1, Ordering::SeqCst);
    Ok(())
}
