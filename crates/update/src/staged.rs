// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Downloading a bundle to the staging directory and remembering it for the
//! next launch.
//!
//! The download is verified as it streams: the signature is checked chunk by
//! chunk, so an 80 MiB AppImage is never held in memory, and the whole file is
//! discarded if it fails. A verified bundle is recorded in `pending.json`,
//! which is what the exit-time swap and the next launch read.

use std::path::{Path, PathBuf};

use minisign_verify::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::{Error, Result};
use crate::install::InstallKind;
use crate::manifest::{PUBLIC_KEY, UpdateInfo};

/// Progress of one bundle download. `total` is the declared size or the
/// `Content-Length`, whichever is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownloadProgress {
    pub received: u64,
    pub total: Option<u64>,
}

/// A verified bundle waiting to be applied.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Staged {
    pub version: String,
    pub kind: InstallKind,
    pub artifact: PathBuf,
    pub sha256: String,
}

/// The directory downloaded bundles (and `pending.json`) live in.
pub fn updates_dir() -> PathBuf {
    storage::LOCATIONS.launcher.root.join("updates")
}

fn pending_path() -> PathBuf {
    updates_dir().join("pending.json")
}

/// The staged bundle from a previous run, if one is waiting.
pub fn read_pending() -> Result<Option<Staged>> {
    let path = pending_path();
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Removes the pending record (the bundle itself is left for cleanup).
pub fn clear_pending() -> Result<()> {
    match std::fs::remove_file(pending_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn write_pending(staged: &Staged) -> Result<()> {
    std::fs::create_dir_all(updates_dir())?;
    let bytes = serde_json::to_vec_pretty(staged)?;
    std::fs::write(pending_path(), bytes)?;
    Ok(())
}

/// The last path segment of a URL, percent-decoded into a file name.
pub fn artifact_name(url: &str) -> Option<String> {
    let url = url::Url::parse(url).ok()?;
    let last = url.path_segments()?.next_back()?.to_string();
    if last.is_empty() {
        return None;
    }
    Some(last)
}

/// Streams `info.url` into the staging directory, verifying its signature and
/// (when the server sent one) its checksum, then records it as pending.
pub async fn download_and_stage(
    info: &UpdateInfo,
    kind: InstallKind,
    sink: shared::Sink<DownloadProgress>,
) -> Result<Staged> {
    let name = artifact_name(&info.url)
        .ok_or_else(|| Error::Unsupported("the update URL has no file name to stage under"))?;

    let public_key = PublicKey::from_base64(PUBLIC_KEY).map_err(|error| {
        log::error!("the embedded update public key is unreadable: {error}");
        Error::Signature
    })?;
    let signature = Signature::decode(&info.signature).map_err(|error| {
        log::error!("the update signature is unreadable: {error}");
        Error::Signature
    })?;
    let mut verifier = public_key
        .verify_stream(&signature)
        .map_err(|_| Error::Signature)?;

    let mut response = shared::HTTP_CLIENT
        .get(&info.url)
        .send()
        .await?
        .error_for_status()?;
    let total = info.size.or_else(|| response.content_length());

    let dir = updates_dir();
    std::fs::create_dir_all(&dir)?;
    let part = dir.join(format!("{name}.part"));
    let final_path = dir.join(&name);

    let mut file = tokio::fs::File::create(&part).await?;
    let mut hasher = Sha256::new();
    let mut received = 0u64;
    let mut last_report = 0u64;
    sink(DownloadProgress { received, total });

    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        verifier.update(&chunk);
        received += chunk.len() as u64;
        if received - last_report >= 256 * 1024 {
            last_report = received;
            sink(DownloadProgress { received, total });
        }
    }
    file.sync_all().await?;

    // Signature first: a bundle that fails it must never be renamed into place.
    if verifier.finalize().is_err() {
        let _ = std::fs::remove_file(&part);
        return Err(Error::Signature);
    }

    let digest = hex(&hasher.finalize());
    if let Some(expected) = &info.sha256
        && !expected.eq_ignore_ascii_case(&digest)
    {
        log::error!("update checksum mismatch: expected {expected}, got {digest}");
        let _ = std::fs::remove_file(&part);
        return Err(Error::Checksum);
    }

    tokio::fs::rename(&part, &final_path).await?;
    sink(DownloadProgress {
        received,
        total: Some(received),
    });

    let staged = Staged {
        version: info.version.clone(),
        kind,
        artifact: final_path,
        sha256: digest,
    };
    write_pending(&staged)?;
    log::info!(
        "update {} downloaded and staged at {}",
        staged.version,
        staged.artifact.display()
    );
    Ok(staged)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Extracts a `.tar.gz` archive into `dest`, creating `dest` first.
pub fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let file = std::fs::File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_url_file_name_is_extracted() {
        assert_eq!(
            artifact_name("https://dl.example/app/conic-launcher_1.0.0_aarch64.app.tar.gz"),
            Some("conic-launcher_1.0.0_aarch64.app.tar.gz".to_string())
        );
        // Asset names are restricted to URL-safe characters, so the encoded
        // segment is used as the file name as-is.
        assert_eq!(
            artifact_name("https://dl.example/app/conic-launcher_1.0.0_amd64.AppImage"),
            Some("conic-launcher_1.0.0_amd64.AppImage".to_string())
        );
        assert_eq!(artifact_name("https://dl.example/"), None);
    }

    #[test]
    fn hex_encodes_a_digest() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
    }
}
