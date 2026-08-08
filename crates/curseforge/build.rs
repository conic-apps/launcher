// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

fn main() {
    println!("cargo:rerun-if-env-changed=CURSEFORGE_API_KEY");
    let api_key = match std::env::var("CURSEFORGE_API_KEY") {
        Ok(api_key) => api_key,
        Err(_) => {
            println!(
                "cargo:warning=CURSEFORGE_API_KEY is not set; the official CurseForge API will be unauthenticated"
            );
            String::new()
        }
    };
    println!("cargo:rustc-env=CURSEFORGE_API_KEY={api_key}");
}
