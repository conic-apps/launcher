// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Writes the three callback pages out as files, so they can be opened in a
//! browser and looked at.
//!
//! `page.html` is the page; this is what puts a *filled-in* one on disk. Run
//! `cargo run -p slint-authcode --example preview` and open the three files it
//! prints, or pass a directory to put them somewhere of your choosing:
//! `cargo run -p slint-authcode --example preview -- ~/Desktop`.
//!
//! The palette is Mocha and the sentences are the English ones, so what comes
//! out is what a default install shows. The colours are the same tokens the app
//! passes in (`Theme.crust`, `Theme.dialog-background`, …) — see
//! `app/src/account_add.rs` for the live version.

use authcode::{Messages, Palette, Rgba, Screen};

fn main() {
    let palette = Palette {
        window: Rgba::new(0x11, 0x11, 0x1b, 1.0),      // Theme.crust
        card: Rgba::new(0x1e, 0x1e, 0x2e, 1.0),        // Theme.dialog-background
        card_border: Rgba::new(0x38, 0x3b, 0x41, 1.0), // Theme.dialog-border
        title: Rgba::new(0xcd, 0xd6, 0xf4, 0.9),       // default-text-color @ 0.9
        body: Rgba::new(0xa6, 0xad, 0xc8, 0.8),        // subtext0 @ 0.8
        success: Rgba::new(0xa6, 0xe3, 0xa1, 1.0),     // Theme.green
        danger: Rgba::new(0xf3, 0x8b, 0xa8, 1.0),      // Theme.red
        dark: true,
        font_family: "'Nunito', 'Comfortaa', system-ui, -apple-system, 'Segoe UI', Roboto, \
                     'PingFang SC', 'Microsoft YaHei UI', sans-serif"
            .to_string(),
    };
    let messages = Messages {
        waiting_title: "Conic Launcher".to_string(),
        waiting_body: "Finish signing in in your browser — this page will change when it is \
                       done."
            .to_string(),
        success_title: "Signed in".to_string(),
        success_body: "You can close this tab and go back to Conic Launcher.".to_string(),
        failure_title: "Sign-in failed".to_string(),
        failure_body: "Conic Launcher did not receive a valid sign-in. Close this tab and try \
                       again."
            .to_string(),
        language_tag: "en-US".to_string(),
    };

    // An argument is where to put them, and without one they go to a temporary
    // directory — the example is a thing to look at, not a build step, so it
    // should not leave anything behind in the source tree by default.
    let directory = match std::env::args().nth(1) {
        Some(argument) => std::path::PathBuf::from(argument),
        None => std::env::temp_dir().join("conic-launcher-authcode-preview"),
    };
    std::fs::create_dir_all(&directory).expect("failed to create the preview directory");
    for (screen, name) in [
        (Screen::Waiting, "waiting"),
        (Screen::Success, "success"),
        (Screen::Failure, "failure"),
    ] {
        let path = directory.join(format!("{name}.html"));
        let page = authcode::page::render(screen, &palette, &messages);
        std::fs::write(&path, &page).expect("failed to write the page");
        println!("{}", path.display());
    }
}
