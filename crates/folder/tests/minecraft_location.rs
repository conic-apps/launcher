// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The data-directory layout: every path is built from the root and the
//! version/asset names the game uses, so the shape is asserted here without
//! touching the filesystem.

use std::path::Path;

use folder::{DataLocation, MinecraftLocation};

fn location() -> MinecraftLocation {
    MinecraftLocation::new("/minecraft")
}

#[test]
fn the_subfolders_hang_off_the_root() {
    let location = location();
    assert_eq!(location.root, Path::new("/minecraft"));
    assert_eq!(location.assets, Path::new("/minecraft/assets"));
    assert_eq!(location.libraries, Path::new("/minecraft/libraries"));
    assert_eq!(location.versions, Path::new("/minecraft/versions"));
}

#[test]
fn a_version_keeps_its_json_jar_and_natives_together() {
    let location = location();
    assert_eq!(
        location.get_version_root("1.20.1"),
        Path::new("/minecraft/versions/1.20.1")
    );
    assert_eq!(
        location.get_version_json("1.20.1"),
        Path::new("/minecraft/versions/1.20.1/1.20.1.json")
    );
    assert_eq!(
        location.get_version_jar("1.20.1", None),
        Path::new("/minecraft/versions/1.20.1/1.20.1.jar")
    );
    assert_eq!(
        location.get_natives_root("1.20.1"),
        Path::new("/minecraft/versions/1.20.1/conic-natives")
    );
}

#[test]
fn a_client_jar_is_the_plain_one_but_other_types_are_suffixed() {
    let location = location();
    assert_eq!(
        location.get_version_jar("1.20.1", Some("client")),
        Path::new("/minecraft/versions/1.20.1/1.20.1.jar")
    );
    assert_eq!(
        location.get_version_jar("1.20.1", Some("server")),
        Path::new("/minecraft/versions/1.20.1/1.20.1-server.jar")
    );
}

#[test]
fn libraries_assets_and_logs_are_found_under_their_roots() {
    let location = location();
    assert_eq!(
        location.get_library_by_path("com/example/demo/1.0/demo-1.0.jar"),
        Path::new("/minecraft/libraries/com/example/demo/1.0/demo-1.0.jar")
    );
    assert_eq!(
        location.get_assets_index("5"),
        Path::new("/minecraft/assets/indexes/5.json")
    );
    assert_eq!(
        location.get_log_config("1.20.1"),
        Path::new("/minecraft/versions/1.20.1/log4j2.xml")
    );
}

#[test]
fn an_instance_lives_below_the_instances_folder() {
    let data = DataLocation::new("/data");
    assert_eq!(
        data.get_instance_root("abc"),
        Path::new("/data/instances/abc")
    );
}
