// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The storage layout: the Minecraft folder shape, the instance folder, and how
//! the three roots are derived from a root and its overrides.

use std::path::{Path, PathBuf};

use storage::{LocationOverrides, Locations, MinecraftLocation};

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
    assert_eq!(location.runtime, Path::new("/minecraft/runtime"));
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
fn a_default_layout_hangs_minecraft_and_instances_off_the_root() {
    let locations = Locations::from_overrides(PathBuf::from("/root"), LocationOverrides::default());
    assert_eq!(locations.launcher.root, Path::new("/root"));
    assert_eq!(locations.minecraft.root, Path::new("/root/minecraft"));
    assert_eq!(locations.instances.root, Path::new("/root/instances"));
}

#[test]
fn an_override_replaces_only_its_own_location() {
    let overrides = LocationOverrides {
        initialized: true,
        launcher: None,
        minecraft: Some(PathBuf::from("/elsewhere/mc")),
        instances: None,
    };
    let locations = Locations::from_overrides(PathBuf::from("/root"), overrides);
    assert_eq!(locations.launcher.root, Path::new("/root"));
    assert_eq!(locations.minecraft.root, Path::new("/elsewhere/mc"));
    assert_eq!(locations.instances.root, Path::new("/root/instances"));
}

#[test]
fn an_instance_lives_below_the_instances_folder() {
    let locations = Locations::from_overrides(PathBuf::from("/root"), LocationOverrides::default());
    assert_eq!(
        locations.instances.get_instance_root("abc"),
        Path::new("/root/instances/abc")
    );
}
