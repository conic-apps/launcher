// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fs::read_to_string, path::PathBuf, str::FromStr};
use tauri_plugin_http::reqwest;

use crate::folder::MinecraftLocation;

use crate::platform::PlatformInfo;

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
        // "\"-Djna.tmpdir=${natives_directory}\"".to_string(),
        // "\"-Dorg.lwjgl.system.SharedLibraryExtractPath=${natives_directory}\"".to_string(),
        // "\"-Dio.netty.native.workdir=${natives_directory}\"".to_string(),
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LatestVersion {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub id: String,
    pub r#type: String,
    pub url: String,
    pub time: String,
    pub release_time: String,
    pub sha1: String,
    pub compliance_level: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VersionManifest {
    pub latest: LatestVersion,
    pub versions: Vec<VersionInfo>,
}

impl VersionManifest {
    pub async fn new() -> Result<VersionManifest> {
        let response =
            reqwest::get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json").await?;
        Ok(response.json::<VersionManifest>().await?)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Download {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    // pub sha1: String,
    pub size: u64,
    pub url: String,
    pub id: String,
    pub total_size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AssetIndexObjectInfo {
    pub hash: String,
    pub size: u32,
}

// #[derive(Debug, Clone, Deserialize, PartialEq)]
pub type AssetIndexObject = HashMap<String, AssetIndexObjectInfo>;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LibraryDownload {
    pub sha1: Option<String>,
    pub size: Option<u64>,
    pub url: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LoggingFile {
    pub size: u64,
    pub url: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct NormalLibrary {
    pub name: String,
    pub downloads: HashMap<String, LibraryDownload>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Rule {
    pub action: String,
    pub os: Option<Platform>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Extract {
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct NativeLibrary {
    pub name: String,
    pub downloads: HashMap<String, LibraryDownload>,
    pub classifiers: HashMap<String, LibraryDownload>,
    pub rules: Vec<Rule>,
    pub extract: Extract,
    pub natives: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PlatformSpecificLibrary {
    pub name: String,
    pub downloads: HashMap<String, LibraryDownload>,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LegacyLibrary {
    pub name: String,
    pub url: Option<String>,
    pub clientreq: Option<bool>,
    pub serverreq: Option<bool>,
    pub checksums: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum Library {
    Normal(NormalLibrary),
    Native(NativeLibrary),
    PlatformSpecific(PlatformSpecificLibrary),
    Legacy(LegacyLibrary),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum LaunchArgument {
    String(String),
    Object(serde_json::map::Map<String, Value>),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Platform {
    pub name: String,
    pub version: Option<String>,
    // Add other platform properties if needed
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Arguments {
    pub game: Option<Vec<Value>>,
    pub jvm: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Logging {
    pub file: LoggingFileDownload,
    pub argument: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LoggingFileDownload {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub component: String,
    pub major_version: i32,
}

/// Minecraft Version
///
/// It used to compare the version of the game
#[derive(Debug, Clone, Serialize)]
pub enum MinecraftVersion {
    Release(u8, u8, Option<u8>),
    Snapshot(u8, u8, String),
    Unknown(String),
}

impl FromStr for MinecraftVersion {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parse_version(s)
    }
}

fn parse_version(s: &str) -> Result<MinecraftVersion> {
    if s.contains(".") {
        let split = s.split(".").collect::<Vec<&str>>();
        Ok(MinecraftVersion::Release(
            #[allow(clippy::get_first)]
            split.get(0).ok_or(anyhow::anyhow!(""))?.parse()?,
            split.get(1).ok_or(anyhow::anyhow!(""))?.parse()?,
            match split.get(2) {
                Some(x) => Some(x.parse()?),
                None => None,
            },
        ))
    } else if s.contains("w") {
        let split = s.split("w").collect::<Vec<&str>>();
        let minor_version = split.get(1).ok_or(anyhow::anyhow!(""))?;
        Ok(MinecraftVersion::Snapshot(
            split.first().ok_or(anyhow::anyhow!(""))?.parse()?,
            (minor_version[..2]).parse()?,
            (minor_version[2..]).to_string(),
        ))
    } else {
        Ok(MinecraftVersion::Unknown(s.to_string()))
    }
}

/// Resolved version.json
///
/// Use `new` to parse a Minecraft version json, and see the detail info of the version,
/// equivalent to `crate::core::version::Version::parse`.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedVersion {
    /// The id of the version, should be identical to the version folder.
    pub id: String,
    pub arguments: ResolvedArguments,

    /// The main class full qualified name.
    pub main_class: Option<String>,
    pub asset_index: Option<AssetIndex>,

    /// The asset index id of this version. Should be something like `1.14`, `1.12`.
    pub assets: Option<String>,
    pub downloads: HashMap<String, Download>,
    pub libraries: Vec<ResolvedLibrary>,
    pub minimum_launcher_version: i32,
    pub release_time: Option<String>,
    pub time: Option<String>,
    pub version_type: Option<String>,
    pub logging: HashMap<String, Logging>,

    /// Recommended java version.
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

impl Default for ResolvedVersion {
    // TODO: use None, dont use empty string
    fn default() -> Self {
        Self {
            id: String::new(),
            arguments: ResolvedArguments::default(),
            main_class: None,
            asset_index: None,
            assets: None,
            downloads: HashMap::new(),
            libraries: Vec::new(),
            minimum_launcher_version: 0,
            release_time: None,
            time: None,
            version_type: None,
            logging: HashMap::new(),
            java_version: JavaVersion {
                component: "jre-legacy".to_string(),
                major_version: 8,
            },
            inheritances: Vec::new(),
            path_chain: Vec::new(),
        }
    }
}

impl ResolvedVersion {
    fn id(&mut self, id: String) -> &mut Self {
        if !id.is_empty() {
            self.id = id
        }
        self
    }
    fn minimum_launcher_version(&mut self, version: Option<i32>) -> &mut Self {
        self.minimum_launcher_version =
            std::cmp::max(version.unwrap_or(0), self.minimum_launcher_version);
        self
    }
    fn release_time(&mut self, release_time: Option<String>) -> &mut Self {
        if release_time.is_some() {
            self.time = release_time
        }
        self
    }
    fn time(&mut self, time: Option<String>) -> &mut Self {
        if time.is_some() {
            self.time = time
        }
        self
    }
    fn logging(&mut self, logging: Option<HashMap<String, Logging>>) -> &mut Self {
        if let Some(logging) = logging {
            if !logging.is_empty() {
                self.logging = logging
            } else {
                self.logging = logging.clone()
            }
        };
        self
    }
    fn assets(&mut self, assets: Option<String>) -> &mut Self {
        if assets.is_some() {
            self.assets = assets
        }
        self
    }
    fn version_type(&mut self, version_type: Option<String>) -> &mut Self {
        if version_type.is_some() {
            self.version_type = version_type
        }
        self
    }
    fn main_class(&mut self, main_class: Option<String>) -> &mut Self {
        if main_class.is_some() {
            self.main_class = main_class
        }
        self
    }
    fn java_version(&mut self, java_version: Option<JavaVersion>) -> &mut Self {
        if let Some(java_version) = java_version {
            self.java_version = java_version
        }
        self
    }
    fn asset_index(&mut self, asset_index: Option<AssetIndex>) -> &mut Self {
        if asset_index.is_some() {
            self.asset_index = asset_index
        }
        self
    }
    fn downloads(&mut self, downloads: Option<HashMap<String, Download>>) -> &mut Self {
        if let Some(downloads) = downloads {
            self.downloads.extend(downloads)
        }
        self
    }
}

/// The raw json format provided by Minecraft.
///
/// Use `parse` to parse a Minecraft version json, and see the detail info of the version.
///
/// With `ResolvedVersion`, you can use the resolved version to launch the game.
///
/// ### Example
///
/// usage 1:
///
/// ```rust
/// use aml_core::core::version::Version;
///
/// async fn fn_name() {
///     let version = reqwest::get("https://piston-meta.mojang.com/v1/packages/715ccf3330885e75b205124f09f8712542cbe7e0/1.20.1.json")
///         .await
///         .unwrap()
///         .json::<Version>()
///         .await
///         .unwrap();
///     println!("{:#?}", version);
/// }
/// ```
///
/// usage 2:
///
/// ```rust
/// use std::str::FromStr;
/// use aml_core::core::version::Version;
///
/// async fn fn_name() {
///     let response = reqwest::get("https://piston-meta.mojang.com/v1/packages/715ccf3330885e75b205124f09f8712542cbe7e0/1.20.1.json")
///         .await
///         .unwrap()
///         .text()
///         .await
///         .unwrap();
///     let version = Version::from_str(&response).unwrap();
///     println!("{:#?}", version);
/// }
/// ```
///
/// usage 3:
///
/// ```rust
/// use aml_core::core::version::Version;
/// use aml_core::core::folder::MinecraftLocation;
/// use aml_core::core::PlatformInfo;
///
/// async fn fn_name(version: Version) {
///     let platform = PlatformInfo::new().await;
///     let resolved_version = version.parse(&MinecraftLocation::new("test"), &platform).await;
///     println!("{:#?}", resolved_version);
/// }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub id: String,
    pub time: Option<String>,
    pub r#type: Option<String>,
    pub release_time: Option<String>,
    pub inherits_from: Option<String>,
    pub minimum_launcher_version: Option<i32>,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    pub main_class: Option<String>,
    pub libraries: Option<Vec<Value>>,
    pub jar: Option<String>,
    pub asset_index: Option<AssetIndex>,
    pub assets: Option<String>,
    pub downloads: Option<HashMap<String, Download>>,
    pub client: Option<String>,
    pub server: Option<String>,
    pub logging: Option<HashMap<String, Logging>>,
    pub java_version: Option<JavaVersion>,
    pub client_version: Option<String>,
}

impl FromStr for Version {
    type Err = serde_json::Error;
    fn from_str(raw: &str) -> Result<Version, serde_json::Error> {
        serde_json::from_str(raw)
    }
}

impl Version {
    pub fn from_value(raw: Value) -> Result<Version, serde_json::Error> {
        serde_json::from_value(raw)
    }

    pub fn from_versions_folder(
        minecraft: &MinecraftLocation,
        version_name: &str,
    ) -> Result<Version, std::io::Error> {
        let path = minecraft
            .versions
            .join(version_name)
            .join(format!("{}.json", version_name));

        let raw = read_to_string(path)?;
        let version: Version = serde_json::from_str((raw).as_ref())?;
        Ok(version)
    }

    /// parse a Minecraft version json
    pub async fn parse(
        &self,
        minecraft: &MinecraftLocation,
        platform: &PlatformInfo,
    ) -> Result<ResolvedVersion> {
        let mut inherits_from = self.inherits_from.clone();
        let versions_folder = &minecraft.versions;
        let mut versions = Vec::new();
        let mut resolved_version = ResolvedVersion::default();
        versions.push(self.clone());
        while let Some(inherits_from_unwrap) = inherits_from {
            resolved_version
                .inheritances
                .push(inherits_from_unwrap.clone());

            let path = versions_folder
                .join(inherits_from_unwrap.clone())
                .join(format!("{}.json", inherits_from_unwrap.clone()));
            resolved_version.path_chain.push(path.clone());
            let version_json = read_to_string(path)?;
            let version_json: Version = serde_json::from_str((version_json).as_ref())?;

            versions.push(version_json.clone());
            inherits_from = version_json.inherits_from;
        }

        let mut libraries_raw = Vec::new();

        while let Some(version) = versions.pop() {
            resolved_version
                .id(version.id)
                .minimum_launcher_version(version.minimum_launcher_version)
                .release_time(version.release_time)
                .time(version.time)
                .logging(version.logging)
                .assets(version.assets)
                .version_type(version.r#type)
                .main_class(version.main_class)
                .java_version(version.java_version)
                .asset_index(version.asset_index)
                .downloads(version.downloads);

            if let Some(libraries) = version.libraries {
                libraries_raw.splice(0..0, libraries);
            }
        }
        resolved_version.libraries = resolve_libraries(libraries_raw, platform).await;
        if resolved_version.main_class.is_none()
            || resolved_version.asset_index.is_none()
            || resolved_version.downloads.is_empty()
            || resolved_version.libraries.is_empty()
        {
            return Err(anyhow::anyhow!("Bad Version JSON"));
        }
        Ok(resolved_version)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedArguments {
    pub game: Vec<String>,
    pub jvm: Vec<String>,
}

impl Default for ResolvedArguments {
    fn default() -> Self {
        ResolvedArguments {
            game: DEFAULT_GAME_ARGS.clone(),
            jvm: DEFAULT_JVM_ARGS.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedLibrary {
    pub download_info: LibraryDownload,
    pub is_native_library: bool,
}

async fn resolve_libraries(libraries: Vec<Value>, platform: &PlatformInfo) -> Vec<ResolvedLibrary> {
    let mut result = Vec::new();
    for library in libraries {
        let rules = library["rules"].as_array();
        // check rules
        if let Some(rules) = rules {
            if !check_allowed(rules.clone(), platform) {
                continue;
            }
        }
        // resolve native lib
        let classifiers = library["downloads"]["classifiers"].as_object();
        let natives = library["natives"].as_object();
        if classifiers.is_some() && natives.is_some() {
            let classifiers = classifiers.unwrap();
            let natives = natives.unwrap();
            let classifier_key = match natives[&platform.name].as_str() {
                None => continue,
                Some(x) => x,
            };
            let classifier = match classifiers[classifier_key].as_object() {
                None => continue,
                Some(x) => x,
            };
            let url = match classifier["url"].as_str() {
                Some(url) => url.to_string(),
                None => continue,
            };
            let path = match classifier["path"].as_str() {
                Some(path) => path.to_string(),
                None => continue,
            };
            result.push(ResolvedLibrary {
                download_info: LibraryDownload {
                    sha1: classifier["sha1"].as_str().map(|sha1| sha1.to_string()),
                    size: classifier["size"].as_u64(),
                    url,
                    path,
                },
                is_native_library: true,
            });
        }
        // resolve common lib
        if library["downloads"]["artifact"].is_object() {
            result.push(ResolvedLibrary {
                download_info: serde_json::from_value(library["downloads"]["artifact"].clone())
                    .unwrap(),
                is_native_library: false,
            });
            continue;
        }
        // resolve mod loader
        let name = match library["name"].as_str() {
            None => continue,
            Some(x) => x,
        };
        let name: Vec<&str> = name.split(":").collect();
        if name.len() != 3 {
            continue;
        }
        #[allow(clippy::get_first)]
        let package = name.get(0).unwrap().replace(".", "/");
        let version = name.get(2).unwrap();
        let name = name.get(1).unwrap();

        let url = if let Some(url) = library["url"].as_str() {
            url
        } else {
            "https://libraries.minecraft.net/"
        };
        let path = format!("{package}/{name}/{version}/{name}-{version}.jar");
        result.push(ResolvedLibrary {
            download_info: LibraryDownload {
                sha1: None,
                size: None,
                url: format!("{url}{path}"),
                path,
            },
            is_native_library: false,
        });
    }
    result
}

/// Check if all the rules in Rule[] are acceptable in certain OS platform and features.
fn check_allowed(rules: Vec<Value>, platform: &PlatformInfo) -> bool {
    // by default it's allowed
    if rules.is_empty() {
        return true;
    }
    // else it's disallow by default
    let mut allow = false;
    for rule in rules {
        let action = rule["action"].as_str().unwrap() == "allow";
        let os = rule["os"].clone();
        if !os.is_object() {
            allow = action;
            continue;
        }
        if !os["name"].is_string() {
            allow = action;
            continue;
        }
        if platform.name != os["name"].as_str().unwrap() {
            continue;
        }
        if os["features"].is_object() {
            return false;
        }
        if !os["version"].is_string() {
            allow = action;
            continue;
        }
        let version = os["version"].as_str().unwrap();
        if Regex::is_match(
            &Regex::new(version).unwrap(),
            (platform.version.to_string()).as_ref(),
        ) {
            allow = action;
        }
        // todo: check `features`
    }
    allow
}

pub struct LibraryInfo {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub is_snapshot: bool,

    /// The file extension. Default is `jar`. Some files in forge are `zip`.
    pub r#type: String,

    /// The classifier. Normally, this is empty. For forge, it can be like `universal`, `installer`.
    pub classifier: String,

    /// The maven path.
    pub path: String,

    /// The original maven name of this library
    pub name: String,
}

#[allow(clippy::get_first)]
impl LibraryInfo {
    /// Get the base info of the library from its name
    /// * `lib` - The name of library of the library itself
    pub fn from_value(lib: &Value) -> Self {
        let name = lib["name"].as_str().unwrap().to_string();
        let split_name = name.split("@").collect::<Vec<&str>>();
        let body = split_name.get(0).unwrap().split(":").collect::<Vec<&str>>();
        let r#type = split_name.get(1).unwrap_or(&"jar").to_string();
        let group_id = body.get(0).unwrap().to_string();
        let artifact_id = body.get(1).unwrap().to_string();
        let version = body.get(2).unwrap().to_string();
        let is_snapshot = version.ends_with("SNAPSHOT");
        let group_path = group_id.replace(".", "/");
        let base = format!("{group_path}/{artifact_id}/{version}/{artifact_id}-{version}");
        let classifier = match body.get(3) {
            Some(classifier) => format!("{base}-{classifier}"),
            None => "".to_string(),
        };
        let path = format!("{base}.{type}");
        Self {
            group_id,
            artifact_id,
            version,
            is_snapshot,
            r#type,
            classifier,
            path,
            name,
        }
    }
}
