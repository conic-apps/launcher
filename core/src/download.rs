// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use futures::StreamExt;
use log::warn;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

use crate::{HTTP_CLIENT, MAIN_WINDOW};

#[derive(Clone, Serialize, Deserialize)]
pub struct Download {
    pub url: String,
    pub file: PathBuf,
    pub sha1: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Progress {
    pub completed: usize,
    pub total: usize,
    pub step: usize,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ProgressError {
    pub step: usize,
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

pub async fn download_files(
    downloads: Vec<Download>,
    send_progress: bool,
    send_error: bool,
    max_connections: usize,
    max_download_speed: usize,
) {
    if send_progress {
        MAIN_WINDOW
            .emit(
                "install_progress",
                Progress {
                    completed: 0,
                    total: 0,
                    step: 2,
                },
            )
            .unwrap();
    }
    let counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let check_files_finished: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let counter_sender_thread = {
        let check_files_finished = check_files_finished.clone();
        let counter = counter.clone();
        thread::spawn(move || {
            while !check_files_finished.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(500));
                if send_progress {
                    MAIN_WINDOW
                        .emit(
                            "install_progress",
                            Progress {
                                completed: counter.load(Ordering::SeqCst),
                                total: 0,
                                step: 2,
                            },
                        )
                        .unwrap();
                }
            }
        })
    };
    let downloads: Vec<_> = downloads
        .into_par_iter()
        .filter(|download| {
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
        })
        .collect();
    check_files_finished.store(true, Ordering::SeqCst);
    counter_sender_thread.join().unwrap();
    let counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let total = downloads.len();
    let speed_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let running_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    let running_counter_closure = {
        let running_counter = running_counter.clone();
        move || {
            let running_counter = running_counter;
            loop {
                let message = rx.try_recv();
                if message == Ok("terminate") {
                    break;
                }
                MAIN_WINDOW
                    .emit(
                        "running_download_task",
                        running_counter.load(Ordering::SeqCst),
                    )
                    .unwrap();
                thread::sleep(Duration::from_millis(100))
            }
        }
    };
    let running_counter_thread = thread::spawn(running_counter_closure);
    let speed_thread_closure = {
        let speed_counter = speed_counter.clone();
        move || {
            let counter = speed_counter;
            loop {
                let message = rx2.try_recv();
                if message == Ok("terminate") {
                    break;
                }
                thread::sleep(Duration::from_millis(2000));
                MAIN_WINDOW
                    .emit("download_speed", counter.load(Ordering::SeqCst))
                    .unwrap();
                counter.store(0, Ordering::SeqCst);
            }
        }
    };
    let speed_thread = thread::spawn(speed_thread_closure);
    let error = Arc::new(AtomicBool::new(false));
    MAIN_WINDOW
        .emit(
            "install_progress",
            Progress {
                completed: 0,
                total,
                step: 3,
            },
        )
        .unwrap();
    futures::stream::iter(downloads)
        .map(|task| {
            let counter = counter.clone();
            let speed_counter = speed_counter.clone();
            let error = error.clone();
            let running_counter = running_counter.clone();
            async move {
                let error = error;
                let running_counter = running_counter;
                running_counter.fetch_add(1, Ordering::SeqCst);
                if error.load(Ordering::SeqCst) {
                    return;
                }
                let mut retried = 0;
                let task = task;
                loop {
                    retried += 1;
                    let speed_counter = speed_counter.clone();
                    if download_file(
                        &task,
                        &counter,
                        &speed_counter,
                        max_download_speed,
                        error.clone(),
                    )
                    .await
                    .is_ok()
                    {
                        break;
                    }
                    warn!("Downloaded failed: {}, retried: {}", &task.url, retried);
                    if retried >= 5 {
                        error.store(true, Ordering::SeqCst);
                        if send_error {
                            MAIN_WINDOW
                                .emit("install_error", ProgressError { step: 3 })
                                .unwrap();
                        }
                        break;
                    }
                }
            }
        })
        .buffer_unordered(max_connections)
        .for_each_concurrent(None, |_| async {
            let counter = counter.clone().load(Ordering::SeqCst);
            running_counter.fetch_sub(1, Ordering::SeqCst);
            if send_progress {
                MAIN_WINDOW
                    .emit(
                        "install_progress",
                        Progress {
                            completed: counter,
                            total,
                            step: 3,
                        },
                    )
                    .unwrap();
            }
        })
        .await;
    tx.send("terminate").unwrap();
    tx2.send("terminate").unwrap();
    speed_thread.join().unwrap();
    running_counter_thread.join().unwrap();
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
