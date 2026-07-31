// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::Deserialize;
use sha2::Digest;

#[derive(Clone, PartialEq, Deserialize)]
pub enum Checksum {
    Sha1(String),
    Sha256(String),
    Sha512(String),
    None,
}

pub(crate) enum Hasher {
    Sha1(sha1_smol::Sha1),
    Sha256(sha2::Sha256),
    Sha512(sha2::Sha512),
    None,
}

impl From<&Checksum> for Hasher {
    fn from(value: &Checksum) -> Self {
        match value {
            Checksum::Sha1(_) => Self::Sha1(sha1_smol::Sha1::new()),
            Checksum::Sha256(_) => Self::Sha256(sha2::Sha256::new()),
            Checksum::Sha512(_) => Self::Sha512(sha2::Sha512::new()),
            Checksum::None => Self::None,
        }
    }
}

impl Hasher {
    pub(crate) fn update(&mut self, data: &[u8]) {
        match self {
            Self::Sha1(sha1_hasher) => sha1_hasher.update(data),
            Self::Sha256(sha256_hasher) => sha256_hasher.update(data),
            Self::Sha512(sha512_hasher) => sha512_hasher.update(data),
            Self::None => (),
        }
    }
    pub(crate) fn verify(self, checksum: &Checksum) -> bool {
        match (self, checksum) {
            (Self::Sha1(sha1_hasher), Checksum::Sha1(sha1_checksum)) => {
                &sha1_hasher.digest().to_string() == sha1_checksum
            }
            (Self::Sha256(sha256_hasher), Checksum::Sha256(sha256_checksum)) => {
                &format!("{:02x}", sha256_hasher.finalize()) == sha256_checksum
            }
            (Self::Sha512(sha512_hasher), Checksum::Sha512(sha512_checksum)) => {
                &format!("{:02x}", sha512_hasher.finalize()) == sha512_checksum
            }
            (Self::None, Checksum::None) => true,
            _ => false,
        }
    }
}
