use std::{fmt, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Storage, DATA_LOCATION};

use super::launch::{ProcessPriority, Server, GC};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ModLoaderType {
    Fabric,
    Forge,
    Quilt,
    Neoforge,
}

impl fmt::Display for ModLoaderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fabric => {
                write!(f, "Fabric")
            }
            Self::Quilt => {
                write!(f, "Quilt")
            }
            Self::Forge => {
                write!(f, "Forge")
            }
            Self::Neoforge => {
                write!(f, "Neoforged")
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceRuntime {
    pub minecraft: String,
    pub mod_loader_type: Option<ModLoaderType>,
    pub mod_loader_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct InstanceLaunchConfig {
    /// Min memory, this will add a jvm flag -XMS to the command result
    pub(crate) min_memory: Option<u32>,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub(crate) max_memory: Option<u32>,
    pub(crate) server: Option<Server>,
    /// window width
    pub(crate) width: Option<u32>,

    /// window height
    pub(crate) height: Option<u32>,

    pub(crate) fullscreen: Option<bool>,

    /// User custom additional java virtual machine command line arguments.
    pub(crate) extra_jvm_args: Option<Vec<String>>,

    /// User custom additional minecraft command line arguments.
    pub(crate) extra_mc_args: Option<Vec<String>>,

    pub(crate) is_demo: Option<bool>,
    /// Game process priority, invalid on windows
    pub(crate) process_priority: Option<ProcessPriority>,

    /// Add `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub(crate) ignore_invalid_minecraft_certificates: Option<bool>,

    /// Add `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub(crate) ignore_patch_discrepancies: Option<bool>,

    /// Add extra classpath
    pub(crate) extra_class_paths: Option<Vec<String>>,

    pub(crate) gc: Option<GC>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceConfig {
    pub name: String,
    pub runtime: InstanceRuntime,
    pub group: Option<Vec<String>>,
    pub launch_config: InstanceLaunchConfig,
}

impl InstanceConfig {
    pub fn new(instance_name: &str, minecraft_version: &str) -> Self {
        Self {
            name: instance_name.to_string(),
            runtime: InstanceRuntime {
                minecraft: minecraft_version.to_string(),
                mod_loader_type: None,
                mod_loader_version: None,
            },
            group: None,
            launch_config: InstanceLaunchConfig::default(),
        }
    }
    pub async fn write(self, _instance_name: &str) -> anyhow::Result<()> {
        Ok(())
    }
    pub async fn get(instance_name: &str) -> anyhow::Result<Self> {
        let config_path = DATA_LOCATION
            .get()
            .unwrap()
            .get_instance_root(instance_name)
            .join("instance.toml");
        if config_path.metadata().is_err() {
            return anyhow::Result::Err(anyhow::Error::msg("message"));
        }
        let config_content = tokio::fs::read_to_string(config_path).await?;
        Ok(toml::from_str::<InstanceConfig>(&config_content)?)
    }
    pub async fn remove(_instance_name: &str) -> anyhow::Result<()> {
        Ok(())
    }
    pub fn get_version_id(&self) -> String {
        match self.runtime.mod_loader_type.as_ref() {
            Some(mod_loader_type) => match mod_loader_type {
                ModLoaderType::Fabric => {
                    format!(
                        "fabric-loader-{}-{}",
                        self.runtime.mod_loader_version.as_ref().unwrap(),
                        self.runtime.minecraft
                    )
                }
                ModLoaderType::Quilt => {
                    format!(
                        "quilt-loader-{}-{}",
                        self.runtime.mod_loader_version.as_ref().unwrap(),
                        self.runtime.minecraft
                    )
                }
                ModLoaderType::Forge => {
                    format!(
                        "{}-forge-{}",
                        self.runtime.minecraft,
                        self.runtime.mod_loader_version.as_ref().unwrap()
                    )
                }
                ModLoaderType::Neoforge => {
                    format!(
                        "neoforge-{}",
                        self.runtime.mod_loader_version.as_ref().unwrap()
                    )
                }
            },
            None => self.runtime.minecraft.to_string(),
        }
    }
    pub fn get_instance_root(&self) -> PathBuf {
        DATA_LOCATION.get().unwrap().instances.join(&self.name)
    }
}

#[tauri::command(async)]
pub async fn get_instance_config(
    storage: tauri::State<'_, Storage>,
) -> std::result::Result<InstanceConfig, ()> {
    let instance_name = storage.current_instance.lock().unwrap().clone();
    match InstanceConfig::get(&instance_name).await {
        anyhow::Result::Ok(x) => Ok(x),
        anyhow::Result::Err(_) => Err(()),
    }
}

#[tauri::command(async)]
pub async fn get_instance_config_by_name(instance_name: &str) -> Result<InstanceConfig, ()> {
    match InstanceConfig::get(instance_name).await {
        anyhow::Result::Ok(x) => Ok(x),
        anyhow::Result::Err(_) => Err(()),
    }
}
