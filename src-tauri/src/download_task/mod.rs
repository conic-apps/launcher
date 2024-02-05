use futures::{stream, StreamExt};
use once_cell::sync::Lazy;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::io::AsyncWriteExt;

use aml_core::{core::Download, utils::sha1::calculate_sha1_from_read};
use serde::{Deserialize, Serialize};

/// To save download task progress
#[derive(Debug, Clone)]
pub struct DownloadTasks {
    pub completed: Option<usize>,
    pub total: Option<usize>,
    // pub progress_unit: ProgressUnit,
    pub statue: State,
    pub download_info: Vec<Download>,
    pub instance_id: String,
}

// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
// pub enum ProgressUnit {
//     Mib,
//     File,
// }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum State {
    Paused,
    Active,
    Success,
    Failed,
    Canceled,
    Waiting,
}

impl DownloadTasks {
    pub fn new(instance_id: String, download_info: Vec<Download>) -> Self {
        Self {
            completed: None,
            total: None,
            statue: State::Waiting,
            download_info,
            instance_id,
        }
    }

    pub async fn start(&mut self, verify_exists: bool) -> anyhow::Result<()> {
        self.statue = State::Active;
        let download_tasks = filtrate_download_tasks(&self.download_info, verify_exists);
        self.total = Some(download_tasks.len());
        let counter: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
        let stream = stream::iter(download_tasks)
            .map(|download_task| {
                let counter = Arc::clone(&counter);
                async move {
                    let result = download(&download_task).await;
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    result
                }
            })
            .buffer_unordered(16);
        stream
            .for_each_concurrent(1, |_| async {
                Some(Arc::clone(&counter).load(Ordering::SeqCst));
            })
            .await;
        if counter.load(Ordering::SeqCst) == self.total.unwrap() {
            // success
        } else {
            // failed
        }
        Ok(())
    }
}

fn verify_file(download_task: &Download) -> bool {
    let mut file = match std::fs::File::open(&download_task.file) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let file_sha1 = calculate_sha1_from_read(&mut file);
    let sha1 = match &download_task.sha1 {
        Some(sha1) => sha1,
        None => return false,
    };
    &file_sha1 == sha1
}

fn filtrate_download_tasks(download_tasks: &Vec<Download>, verify_exists: bool) -> Vec<&Download> {
    download_tasks
        .into_iter()
        .filter(|download_task| {
            match std::fs::metadata(&download_task.file) {
                Err(_) => return true,
                Ok(_) => (),
            }
            if !verify_exists {
                return false;
            };
            verify_file(&download_task);
            true
        })
        .collect()
}

pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::new());

async fn download(download: &Download) -> anyhow::Result<reqwest::Response> {
    let file_path = PathBuf::from(&download.file);
    let direction = file_path.parent().unwrap();
    if !direction.exists() {
        tokio::fs::create_dir_all(&direction).await?
    }
    let mut response = HTTP_CLIENT.get(&download.url).send().await?;
    let mut file = tokio::fs::File::create(&download.file).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
    }
    Ok(response)
}
