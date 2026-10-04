// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Signs the update bundles the release pipeline produces, writing the minisign
//! signature the app's `minisign-verify` check accepts next to each bundle.
//!
//! The secret key is read from the environment, so it never reaches a command
//! line or the repository:
//!
//! ```text
//! CONIC_UPDATE_SIGNING_KEY=<base64 of the minisign secret key box>
//! CONIC_UPDATE_SIGNING_KEY_PASSWORD=<optional>
//! update-sign <bundle>...          # writes <bundle>.sig for each
//! update-sign generate             # mints a key pair and prints both halves
//! ```

use std::{
    env,
    error::Error,
    fs::File,
    path::{Path, PathBuf},
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use minisign::{KeyPair, SecretKeyBox};

fn main() {
    if let Err(error) = run() {
        eprintln!("update-sign: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("generate") {
        return generate();
    }
    let files: Vec<PathBuf> = args.into_iter().map(PathBuf::from).collect();
    if files.is_empty() {
        return Err("usage: update-sign <bundle>... | update-sign generate".into());
    }
    sign_all(&files)
}

fn generate() -> Result<(), Box<dyn Error>> {
    // An encrypted key when `CONIC_UPDATE_SIGNING_KEY_PASSWORD` is set, so the
    // box on disk (and in the CI secret) needs the password to sign at all; an
    // unencrypted one otherwise, which is only for a throwaway local run.
    let password = env::var("CONIC_UPDATE_SIGNING_KEY_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty());
    let KeyPair { pk, sk } = match password {
        Some(password) => KeyPair::generate_encrypted_keypair(Some(password))?,
        None => KeyPair::generate_unencrypted_keypair()?,
    };
    let key_line = pk
        .to_box()?
        .to_string()
        .lines()
        .find(|line| !line.starts_with("untrusted comment"))
        .ok_or("the public key box has no key line")?
        .to_string();
    let secret = STANDARD.encode(sk.to_box(None)?.to_string());
    println!("public key (embed in crates/update/src/manifest.rs PUBLIC_KEY):\n{key_line}");
    println!("\nsecret (store as CONIC_UPDATE_SIGNING_KEY, base64):\n{secret}");
    Ok(())
}

fn sign_all(files: &[PathBuf]) -> Result<(), Box<dyn Error>> {
    let key_base64 =
        env::var("CONIC_UPDATE_SIGNING_KEY").map_err(|_| "CONIC_UPDATE_SIGNING_KEY is not set")?;
    let key_text = String::from_utf8(STANDARD.decode(key_base64.trim())?)
        .map_err(|_| "CONIC_UPDATE_SIGNING_KEY is not valid base64")?;
    let secret_box = SecretKeyBox::from_string(&key_text)?;
    let password = env::var("CONIC_UPDATE_SIGNING_KEY_PASSWORD")
        .ok()
        .filter(|value| !value.is_empty());
    let secret = secret_box.into_secret_key(password)?;

    for file in files {
        let signature = minisign::sign(None, &secret, File::open(file)?, None, None)?;
        let signature_path = signature_path(file);
        std::fs::write(&signature_path, signature.into_string())?;
        println!("signed {}", signature_path.display());
    }
    Ok(())
}

fn signature_path(file: &Path) -> PathBuf {
    let mut path = file.as_os_str().to_owned();
    path.push(".sig");
    PathBuf::from(path)
}
