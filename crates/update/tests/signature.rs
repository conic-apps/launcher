// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The release pipeline signs with the `minisign` crate and the app verifies
//! with `minisign-verify`; this proves the two agree, in the streaming mode the
//! app uses so a large bundle is never held in memory.

use std::io::Cursor;

use minisign::{KeyPair, PublicKeyBox, SignatureBox};
use minisign_verify::{PublicKey, Signature};

#[test]
fn a_crate_signature_streams_through_the_app_verifier() {
    let KeyPair { pk, sk } = KeyPair::generate_unencrypted_keypair().expect("generate a key pair");

    let data = b"the quick brown fox jumps over the lazy dog";
    let signature_box =
        minisign::sign(None, &sk, Cursor::new(data), None, None).expect("sign the data");

    // What the app reads: the key line of the `.pub` box, and the `.sig` text.
    let pk_box = PublicKeyBox::from_string(&pk.to_box().unwrap().to_string())
        .expect("parse the public key box");
    let key_line = pk_box
        .to_string()
        .lines()
        .find(|line| !line.starts_with("untrusted comment"))
        .expect("the key line")
        .to_string();

    let public_key = PublicKey::from_base64(&key_line).expect("decode the public key");
    let signature = Signature::decode(&signature_box.into_string()).expect("decode the signature");

    let mut verifier = public_key
        .verify_stream(&signature)
        .expect("a prehashed signature can stream");
    verifier.update(data);
    verifier.finalize().expect("the signature verifies");

    // A single flipped byte must fail.
    let mut verifier = public_key.verify_stream(&signature).unwrap();
    verifier.update(b"the quick brown fox jumps over the lazy DOG");
    assert!(
        verifier.finalize().is_err(),
        "a tampered body must not verify"
    );
}

#[test]
fn the_signature_round_trips_through_a_file() {
    // Same as above, but through the `Signature::decode` path the app uses on
    // the server's `signature` field.
    let KeyPair { sk, .. } = KeyPair::generate_unencrypted_keypair().expect("generate a key pair");
    let data = b"payload";
    let signature_box = minisign::sign(None, &sk, Cursor::new(data), None, None).unwrap();
    let text = SignatureBox::from_string(&signature_box.into_string())
        .unwrap()
        .into_string();
    assert!(Signature::decode(&text).is_ok());
}
