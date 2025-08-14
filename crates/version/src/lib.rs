// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, fs::read_to_string, path::PathBuf};

use folder::MinecraftLocation;

pub mod argument;
mod checks;
mod error;
mod library;
mod model;

pub use crate::model::*;
use argument::*;
pub use error::*;
pub use library::*;

static DEFAULT_GAME_ARGS: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        "--username".to_string(),
        "${auth_player_name}".to_string(),
        "--version".to_string(),
        "${version_name}".to_string(),
        "--gameDir".to_string(),
        "${game_directory}".to_string(),
        "--assetsDir".to_string(),
        "${assets_root}".to_string(),
        "--assetIndex".to_string(),
        "${asset_index}".to_string(),
        "--uuid".to_string(),
        "${auth_uuid}".to_string(),
        "--accessToken".to_string(),
        "${auth_access_token}".to_string(),
        "--clientId".to_string(),
        "${clientid}".to_string(),
        "--xuid".to_string(),
        "${auth_xuid}".to_string(),
        "--userType".to_string(),
        "${user_type}".to_string(),
        "--versionType".to_string(),
        "${version_type}".to_string(),
        "--width".to_string(),
        "${resolution_width}".to_string(),
        "--height".to_string(),
        "${resolution_height}".to_string(),
    ]
});

static DEFAULT_JVM_ARGS: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        "\"-Djava.library.path=${natives_directory}\"".to_string(),
        "\"-Djna.tmpdir=${natives_directory}\"".to_string(),
        "\"-Dorg.lwjgl.system.SharedLibraryExtractPath=${natives_directory}\"".to_string(),
        "\"-Dio.netty.native.workdir=${natives_directory}\"".to_string(),
        "\"-Dminecraft.launcher.brand=${launcher_name}\"".to_string(),
        "\"-Dminecraft.launcher.version=${launcher_version}\"".to_string(),
        "\"-Dfile.encoding=UTF-8\"".to_string(),
        "\"-Dsun.stdout.encoding=UTF-8\"".to_string(),
        "\"-Dsun.stderr.encoding=UTF-8\"".to_string(),
        "\"-Djava.rmi.server.useCodebaseOnly=true\"".to_string(),
        "\"-XX:MaxInlineSize=420\"".to_string(),
        "\"-XX:-UseAdaptiveSizePolicy\"".to_string(),
        "\"-XX:-OmitStackTraceInFastThrow\"".to_string(),
        "\"-XX:-DontCompileHugeMethods\"".to_string(),
        "\"-Dcom.sun.jndi.rmi.object.trustURLCodebase=false\"".to_string(),
        "\"-Dcom.sun.jndi.cosnaming.object.trustURLCodebase=false\"".to_string(),
        "\"-Dlog4j2.formatMsgNoLookups=true\"".to_string(),
        "-cp".to_string(),
        "${classpath}".to_string(),
    ]
});

/// Resolved version.json
///
/// Use `new` to parse a Minecraft version json, and see the detail info of the version,
/// equivalent to `crate::core::version::Version::parse`.
#[derive(Clone, Serialize, Default)]
pub struct ResolvedVersion {
    pub id: String,
    pub game_arguments: Vec<String>,
    pub jvm_arguments: Vec<String>,
    pub main_class: Option<String>,
    pub asset_index: Option<AssetIndex>,
    pub assets: Option<String>,
    pub downloads: HashMap<String, Download>,
    pub libraries: Vec<ResolvedLibrary>,
    pub minimum_launcher_version: i32,
    pub release_time: Option<String>,
    pub time: Option<String>,
    pub version_type: Option<String>,
    pub logging: HashMap<String, Logging>,
    pub java_version: JavaVersion,

    /// The version inheritances of this whole resolved version.
    ///
    /// The first element is this version, and the last element is the root Minecraft version.
    /// The dependencies of \[\<a\>, \<b\>, \<c\>\] should be \<a\> -> \<b\> -> \<c\>, where c is a Minecraft version.
    pub inheritances: Vec<String>,

    /// All array of json file paths.
    ///
    /// It's the chain of inherits json path. The root json will be the last element of the array.
    /// The first element is the user provided version.
    pub path_chain: Vec<PathBuf>,
}

impl ResolvedVersion {
    fn join_jvm_arguments(
        &mut self,
        arguments: &Option<Arguments>,
        enabled_features: &[String],
    ) -> &mut Self {
        if self.minimum_launcher_version < 21 {
            self.jvm_arguments = DEFAULT_JVM_ARGS.clone();
            return self;
        }
        if let Some(arguments) = arguments
            && let Some(raw_jvm_arguments) = &arguments.jvm
        {
            let resolved_arguments = resolve_arguments(raw_jvm_arguments, enabled_features);
            self.jvm_arguments.extend(resolved_arguments);
        }
        self
    }
    fn join_game_arguments(
        &mut self,
        arguments: Option<Arguments>,
        enabled_features: &[String],
    ) -> &mut Self {
        if self.minimum_launcher_version < 21 {
            self.game_arguments = DEFAULT_GAME_ARGS.clone();
            return self;
        }
        if let Some(arguments) = arguments
            && let Some(raw_game_arguments) = arguments.game
        {
            let resolved_arguments = resolve_arguments(&raw_game_arguments, enabled_features);
            self.game_arguments.extend(resolved_arguments);
        }
        self
    }
    fn join_id(&mut self, id: String) -> &mut Self {
        if !id.is_empty() {
            self.id = id
        }
        self
    }
    fn join_minimum_launcher_version(&mut self, version: Option<i32>) -> &mut Self {
        self.minimum_launcher_version =
            std::cmp::max(version.unwrap_or(0), self.minimum_launcher_version);
        self
    }
    fn join_release_time(&mut self, release_time: Option<String>) -> &mut Self {
        if release_time.is_some() {
            self.time = release_time
        }
        self
    }
    fn join_time(&mut self, time: Option<String>) -> &mut Self {
        if time.is_some() {
            self.time = time
        }
        self
    }
    fn join_logging(&mut self, logging: Option<HashMap<String, Logging>>) -> &mut Self {
        if let Some(logging) = logging {
            if !logging.is_empty() {
                self.logging = logging
            } else {
                self.logging = logging.clone()
            }
        };
        self
    }
    fn join_assets(&mut self, assets: Option<String>) -> &mut Self {
        if assets.is_some() {
            self.assets = assets
        }
        self
    }
    fn join_version_type(&mut self, version_type: Option<String>) -> &mut Self {
        if version_type.is_some() {
            self.version_type = version_type
        }
        self
    }
    fn join_main_class(&mut self, main_class: Option<String>) -> &mut Self {
        if main_class.is_some() {
            self.main_class = main_class
        }
        self
    }
    fn join_java_version(&mut self, java_version: Option<JavaVersion>) -> &mut Self {
        if let Some(java_version) = java_version {
            self.java_version = java_version
        }
        self
    }
    fn join_asset_index(&mut self, asset_index: Option<AssetIndex>) -> &mut Self {
        if asset_index.is_some() {
            self.asset_index = asset_index
        }
        self
    }
    fn join_downloads(&mut self, downloads: Option<HashMap<String, Download>>) -> &mut Self {
        if let Some(downloads) = downloads {
            self.downloads.extend(downloads)
        }
        self
    }
    fn join_libraries(&mut self, libraries: Option<Vec<Value>>) -> Result<&mut Self> {
        if let Some(libraries) = libraries {
            let resolved = resolve_libraries(libraries)?;
            self.libraries.splice(0..0, resolved);
        }
        Ok(self)
    }
}

/// parse a Minecraft version json
///
/// If you are not use this to launch the game, you can set `enabled_features` to `&vec![]`
pub async fn resolve_version(
    version: &Version,
    minecraft: &MinecraftLocation,
    enabled_features: &[String],
) -> Result<ResolvedVersion> {
    let mut inherits_from = version.inherits_from.clone();
    let versions_folder = &minecraft.versions;
    let mut versions = Vec::new();
    let mut resolved_version = ResolvedVersion::default();
    versions.push(version.clone());
    while let Some(inherits_from_unwrap) = inherits_from {
        resolved_version
            .inheritances
            .push(inherits_from_unwrap.clone());

        let path = versions_folder
            .join(inherits_from_unwrap.clone())
            .join(format!("{}.json", inherits_from_unwrap.clone()));
        resolved_version.path_chain.push(path.clone());
        let version_json = read_to_string(path)?;
        let version_json: Version = serde_json::from_str(&version_json)?;

        versions.push(version_json.clone());
        inherits_from = version_json.inherits_from;
    }

    while let Some(version) = versions.pop() {
        resolved_version
            .join_id(version.id)
            .join_minimum_launcher_version(version.minimum_launcher_version)
            .join_release_time(version.release_time)
            .join_time(version.time)
            .join_logging(version.logging)
            .join_assets(version.assets)
            .join_version_type(version.r#type)
            .join_main_class(version.main_class)
            .join_java_version(version.java_version)
            .join_asset_index(version.asset_index)
            .join_downloads(version.downloads)
            .join_jvm_arguments(&version.arguments, enabled_features)
            .join_game_arguments(version.arguments, enabled_features)
            .join_libraries(version.libraries)?;
    }
    if resolved_version.main_class.is_none()
        || resolved_version.asset_index.is_none()
        || resolved_version.downloads.is_empty()
        || resolved_version.libraries.is_empty()
    {
        return Err(Error::InvalidVersionJson);
    }
    Ok(resolved_version)
}
