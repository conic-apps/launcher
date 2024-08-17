use std::{
    io::Read,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

use futures::StreamExt;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tauri_plugin_http::reqwest;
use tokio::io::AsyncWriteExt;

use crate::{HTTP_CLIENT, MAIN_WINDOW};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Download {
    pub url: String,
    pub file: PathBuf,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadProgress {
    pub completed: usize,
    pub total: usize,
    pub step: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DownloadError {
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

pub async fn download_files(downloads: Vec<Download>) {
    let main_window = MAIN_WINDOW.get().unwrap();
    main_window
        .emit(
            "install_progress",
            DownloadProgress {
                completed: 0,
                total: 0,
                step: 2,
            },
        )
        .unwrap();
    let counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
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
                return false;
            };
            let file_hash = calculate_sha1_from_read(&mut file);
            counter.clone().fetch_add(1, Ordering::SeqCst);
            main_window
                .emit(
                    "install_progress",
                    DownloadProgress {
                        completed: counter.load(Ordering::SeqCst),
                        total: 0,
                        step: 2,
                    },
                )
                .unwrap();
            &file_hash != download.sha1.as_ref().unwrap()
        })
        .collect();
    let counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let total = downloads.len();
    let client = HTTP_CLIENT.get().unwrap();
    let speed_counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let speed_counter_clone = speed_counter.clone();
    let (tx, rx) = mpsc::channel();
    let speed_thread = thread::spawn(move || {
        let counter = speed_counter_clone;
        loop {
            let message = rx.try_recv();
            if message == Ok("terminate") {
                break;
            }
            thread::sleep(Duration::from_millis(2000));
            main_window
                .emit("download_speed", counter.load(Ordering::SeqCst))
                .unwrap();
            counter.store(0, Ordering::SeqCst);
        }
    });

    futures::stream::iter(downloads)
        .map(|task| {
            let counter = counter.clone();
            let speed_counter = speed_counter.clone();
            async move {
                let mut retries = 0;
                let task = task;
                loop {
                    let speed_counter = speed_counter.clone();
                    if download_file(client, &task, &counter, &speed_counter)
                        .await
                        .is_ok()
                    {
                        break;
                    }
                    println!("Downloaded failed: {}, retrying...", &task.url);
                    if retries >= 5 {
                        MAIN_WINDOW
                            .get()
                            .unwrap()
                            .emit("install_error", DownloadError {
                                step: 3
                            })
                            .unwrap();
                        break;
                    }
                    retries += 1;
                }
            }
        })
        .buffer_unordered(100)
        .for_each_concurrent(None, |_| async {
            let counter = counter.clone().load(Ordering::SeqCst);
            // println!("Progress: {counter} / {total}");
            main_window
                .emit(
                    "install_progress",
                    DownloadProgress {
                        completed: counter,
                        total,
                        step: 3,
                    },
                )
                .unwrap();
        })
        .await;
    tx.send("terminate").unwrap();
    speed_thread.join().unwrap();
}

pub async fn download_file(
    client: &reqwest::Client,
    task: &Download,
    counter: &Arc<AtomicUsize>,
    speed_counter: &Arc<AtomicUsize>,
) -> anyhow::Result<()> {
    let file_path = task.file.clone();
    tokio::fs::create_dir_all(file_path.parent().ok_or(anyhow::Error::msg(
        "Unknown Error in instance/mod.rs".to_string(),
    ))?)
    .await?;
    let mut response = client.get(task.url.clone()).send().await?;
    let mut file = tokio::fs::File::create(&file_path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        speed_counter.fetch_add(chunk.len(), Ordering::SeqCst);
    }
    counter.fetch_add(1, Ordering::SeqCst);
    Ok(())
}
