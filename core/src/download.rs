// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::Duration,
};

use futures::StreamExt;
use log::warn;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Url};
use tokio::io::AsyncWriteExt;

use crate::{
    config::download::{DownloadConfig, MirrorConfig},
    HTTP_CLIENT, MAIN_WINDOW,
};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum DownloadType {
    VersionInfo,
    Assets,
    Libraries,
    MojangJava,
    Unknown,
}

struct Mirror(String, Arc<AtomicUsize>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MirrorUsage {
    libraries: HashMap<String, Arc<AtomicUsize>>,
    assets: HashMap<String, Arc<AtomicUsize>>,
}

impl MirrorUsage {
    fn new(mirror_config: MirrorConfig) -> Self {
        Self {
            libraries: mirror_config
                .libraries
                .iter()
                .map(|x| (x.to_string(), Arc::new(AtomicUsize::new(0))))
                .collect(),
            assets: mirror_config
                .assets
                .iter()
                .map(|x| (x.to_string(), Arc::new(AtomicUsize::new(0))))
                .collect(),
        }
    }
    // TODO: 设置镜像被使用的频率，实现方式：y.1.load.... 乘一个数放在cmp右面
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
pub struct Download {
    pub url: String,
    pub file: PathBuf,
    pub sha1: Option<String>,
    pub r#type: DownloadType,
}

impl Download {
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
        disabled: &[String],
    ) -> Option<(Download, Mirror)> {
        match self.r#type {
            DownloadType::Libraries => {
                let mirror = mirror_usage.get_libraries_mirror(disabled)?;
                mirror.1.fetch_add(1, Ordering::SeqCst);
                Some((
                    Download {
                        url: self
                            .url
                            .replace("https://libraries.minecraft.net", &mirror.0),
                        ..self
                    },
                    mirror,
                ))
            }
            DownloadType::Assets => {
                let mirror = mirror_usage.get_assets_mirror(disabled)?;
                mirror.1.fetch_add(1, Ordering::SeqCst);
                Some((
                    Download {
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

#[derive(Clone, Deserialize, Serialize)]
pub struct Progress {
    pub completed: usize,
    pub total: usize,
    pub step: usize,
}

impl Progress {
    fn send(completed: usize, total: usize, step: usize) {
        MAIN_WINDOW
            .emit(
                "install_progress",
                Progress {
                    completed,
                    total,
                    step,
                },
            )
            .expect("Could not send message to main window");
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ProgressError {
    pub step: usize,
}

impl ProgressError {
    fn send(step: usize) {
        MAIN_WINDOW.emit("install_error", Self { step }).unwrap();
    }
}

fn calculate_sha1_from_read<R: Read>(source: &mut R) -> String {
    let mut hasher = sha1_smol::Sha1::new();
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = source.read(&mut buffer).unwrap();
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    hasher.digest().to_string()
}

fn verify_existing_files(downloads: Vec<Download>) -> Vec<Download> {
    let finished: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let counter_sender_thread = {
        let check_files_finished = finished.clone();
        let counter = counter.clone();
        thread::spawn(move || {
            while !check_files_finished.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(500));
                Progress::send(counter.load(Ordering::SeqCst), 0, 2);
            }
        })
    };
    let filter_op = |download: &Download| {
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
        let file_hash = calculate_sha1_from_read(&mut file);
        counter.clone().fetch_add(1, Ordering::SeqCst);
        &file_hash != download.sha1.as_ref().unwrap()
    };
    let downloads: Vec<_> = downloads.into_par_iter().filter(filter_op).collect();
    finished.store(true, Ordering::SeqCst);
    counter_sender_thread.join().unwrap();
    downloads
}

pub async fn download_files(downloads: Vec<Download>, send_error: bool, config: DownloadConfig) {
    let downloads: Vec<Download> = verify_existing_files(downloads)
        .into_iter()
        .map(|x| x.classify())
        .collect();

    let counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let total = downloads.len();
    let speed_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let running_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    let speed_thread = {
        let speed_counter = speed_counter.clone();
        thread::spawn(|| speed_counter_loop(speed_counter, rx))
    };
    let error = Arc::new(AtomicBool::new(false));
    let mirror_usage = MirrorUsage::new(config.mirror);
    let mirror_usage_sender_thread = {
        let mirror_usage = mirror_usage.clone();
        thread::spawn(move || loop {
            if rx2.try_recv() == Ok("terminate") {
                break;
            }
            MAIN_WINDOW.emit("mirror_usage", &mirror_usage).unwrap();
            thread::sleep(Duration::from_millis(500));
        })
    };
    Progress::send(0, total, 3);

    futures::stream::iter(downloads)
        .map(|task| {
            download_file_future(
                &mirror_usage,
                &error,
                &speed_counter,
                &counter,
                task,
                config.max_download_speed,
                send_error,
            )
        })
        .buffer_unordered(config.max_connections)
        .for_each_concurrent(None, |_| async {
            let counter = counter.clone().load(Ordering::SeqCst);
            running_counter.fetch_sub(1, Ordering::SeqCst);
            Progress::send(counter, total, 3);
        })
        .await;
    tx.send("terminate").unwrap();
    tx2.send("terminate").unwrap();
    speed_thread.join().unwrap();
    mirror_usage_sender_thread.join().unwrap();
}

fn speed_counter_loop(counter: Arc<AtomicUsize>, rx: Receiver<&str>) {
    loop {
        let message = rx.try_recv();
        if message == Ok("terminate") {
            break;
        }
        MAIN_WINDOW
            .emit("download_speed", counter.load(Ordering::SeqCst))
            .unwrap();
        counter.store(0, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(2000));
    }
}

async fn download_file_future(
    mirror_usage: &MirrorUsage,
    error: &Arc<AtomicBool>,
    speed_counter: &Arc<AtomicUsize>,
    counter: &Arc<AtomicUsize>,
    task: Download,
    max_download_speed: usize,
    send_error: bool,
) {
    let mut disabled_mirrors = vec![];
    let mut retried = 0;
    loop {
        retried += 1;
        let speed_counter = speed_counter.clone();
        let (task, mirror) = match task
            .clone()
            .assignment_mirror(mirror_usage, &disabled_mirrors)
        {
            Some(x) => (x.0, Some(x.1)),
            None => (task.clone(), None),
        };
        let result = download_file(
            &task,
            counter,
            &speed_counter,
            max_download_speed,
            error.clone(),
        )
        .await;
        if let Some(mirror) = &mirror {
            mirror.1.fetch_sub(1, Ordering::SeqCst);
        }
        if result.is_ok() {
            break;
        }
        warn!("Downloaded failed: {}, retried: {}", &task.url, retried);
        if let Some(mirror) = mirror {
            disabled_mirrors.push(mirror.0);
        }
        if retried >= 5 {
            error.store(true, Ordering::SeqCst);
            if send_error {
                ProgressError::send(3);
            }
            break;
        }
    }
}

async fn download_file(
    task: &Download,
    counter: &Arc<AtomicUsize>,
    speed_counter: &Arc<AtomicUsize>,
    max_download_speed: usize,
    error: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let file_path = task.file.clone();
    tokio::fs::create_dir_all(file_path.parent().ok_or(anyhow::Error::msg(
        "Unknown Error in instance/mod.rs".to_string(),
    ))?)
    .await?;
    let mut response = HTTP_CLIENT.get(task.url.clone()).send().await?;
    let mut file = tokio::fs::File::create(&file_path).await?;
    while let Some(chunk) = response.chunk().await? {
        if max_download_speed > 1024 {
            while speed_counter.load(Ordering::SeqCst) > max_download_speed {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
        if error.load(Ordering::SeqCst) {
            // Return `Ok(())` because we have already sent an error to the frontend
            return Ok(());
        }
        file.write_all(&chunk).await?;
        speed_counter.fetch_add(chunk.len(), Ordering::SeqCst);
    }
    counter.fetch_add(1, Ordering::SeqCst);
    Ok(())
}
