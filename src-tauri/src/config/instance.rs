use serde::{Deserialize, Serialize};

use crate::{Storage, DATA_LOCATION};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ModLoaderType {
    Fabric,
    Forge,
    Quilt,
    Neoforge,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceRuntime {
    pub minecraft: String,
    pub mod_loader_type: Option<ModLoaderType>,
    pub mod_loader_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceConfig {
    pub name: String,
    pub runtime: InstanceRuntime,
    pub group: Option<Vec<String>>,
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
        }
    }
    pub async fn write(self, _instance_name: &str) -> anyhow::Result<()> {
        Ok(())
    }
    pub async fn get(instance_name: &str) -> anyhow::Result<Self> {
        let config_path = DATA_LOCATION
            .get()
            .unwrap()
            .get_instance_root(&instance_name)
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
}

#[tauri::command(async)]
pub async fn get_instance_config(
    storage: tauri::State<'_, Storage>,
) -> std::result::Result<InstanceConfig, ()> {
    let instance_name = storage.current_instance.lock().unwrap().clone();
    match InstanceConfig::get(&instance_name).await {
        anyhow::Result::Ok(x) => Ok(x),
        anyhow::Result::Err(_) => Err(())
    }
}

#[tauri::command(async)]
pub async fn get_instance_config_by_name(instance_name: &str) -> Result<InstanceConfig, ()> {
    match InstanceConfig::get(instance_name).await {
        anyhow::Result::Ok(x) => Ok(x),
        anyhow::Result::Err(_) => Err(())
    }
}