// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use log::trace;
use serde::Deserialize;
use sha2::Digest;

#[derive(Debug, Clone, PartialEq, Deserialize)]
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
        let result = match (self, checksum) {
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
        };
        trace!("Checksum verification result: {result}, expected={checksum:?}");
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SHA digests of the bytes `abc`.
    const SHA1_ABC: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";
    const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const SHA512_ABC: &str = "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f";

    fn verify(checksum: &Checksum, data: &[u8]) -> bool {
        let mut hasher = Hasher::from(checksum);
        hasher.update(data);
        hasher.verify(checksum)
    }

    #[test]
    fn a_matching_sha1_passes() {
        assert!(verify(&Checksum::Sha1(SHA1_ABC.to_string()), b"abc"));
    }

    #[test]
    fn a_non_matching_sha1_fails() {
        assert!(!verify(&Checksum::Sha1("deadbeef".to_string()), b"abc"));
    }

    #[test]
    fn a_matching_sha256_passes() {
        assert!(verify(&Checksum::Sha256(SHA256_ABC.to_string()), b"abc"));
        assert!(!verify(&Checksum::Sha256(SHA256_ABC.to_string()), b"abcd"));
    }

    #[test]
    fn a_matching_sha512_passes() {
        assert!(verify(&Checksum::Sha512(SHA512_ABC.to_string()), b"abc"));
    }

    #[test]
    fn no_checksum_accepts_anything() {
        assert!(verify(&Checksum::None, b"anything at all"));
    }

    #[test]
    fn a_digest_of_another_algorithm_does_not_match() {
        // A SHA-256 hasher checked against a SHA-1 digest's shape.
        assert!(!verify(&Checksum::Sha256(SHA1_ABC.to_string()), b"abc"));
    }
}
