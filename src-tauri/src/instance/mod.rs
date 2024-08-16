use crate::version::VersionManifest;
use crate::{Storage, DATA_LOCATION};
use serde::{Deserialize, Serialize};

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
}

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
pub struct Instance {
    pub config: InstanceConfig,
    pub installed: bool,
}

#[tauri::command(async)]
pub async fn create_instance(instance_name: String, config: InstanceConfig) -> Option<()> {
    async fn create_instance(instance_name: String, config: InstanceConfig) -> anyhow::Result<()> {
        let data_location = DATA_LOCATION.get().unwrap();
        let instance_root = data_location.get_instance_root(&instance_name);
        let config_file = instance_root.join("instance.toml");
        tokio::fs::create_dir_all(config_file.parent().ok_or(anyhow::anyhow!("Path Error"))?)
            .await?;
        tokio::fs::write(config_file, toml::to_string_pretty(&config)?).await?;
        Ok(())
    }
    create_instance(instance_name, config).await.ok()
}

#[tauri::command(async)]
pub async fn check_repeated_instance_name(instance_name: String) -> bool {
    let instance_root = DATA_LOCATION
        .get()
        .unwrap()
        .get_instance_root(&instance_name);
    let config = match get_instance_config_from_string(&instance_name).await {
        Ok(x) => x,
        Err(_) => return false,
    };
    let folder_name = match instance_root.file_name() {
        None => return true,
        Some(x) => x,
    }
    .to_string_lossy()
    .to_string();
    (config.name == instance_name) || (folder_name == instance_name)
}

#[tauri::command(async)]
pub async fn scan_instances_folder() -> Option<Vec<Instance>> {
    println!("Scanning Instances...");
    scan().await.ok()
}

async fn scan() -> anyhow::Result<Vec<Instance>> {
    let datafolder_path = DATA_LOCATION.get().unwrap();
    let instances_folder = &datafolder_path.instances;
    let mut folder_entries = tokio::fs::read_dir(instances_folder).await?;
    let mut results = Vec::new();
    while let Some(entry) = folder_entries.next_entry().await? {
        let file_type = match entry.file_type().await {
            Err(_) => continue,
            Ok(file_type) => file_type,
        };
        if !file_type.is_dir() {
            continue;
        }
        let path = entry.path();
        let folder_name = match path.file_name() {
            None => continue,
            Some(x) => x,
        }
        .to_string_lossy()
        .to_string();
        let instance_config = path.join("instance.toml");
        let metadata = match instance_config.metadata() {
            Err(_) => continue,
            Ok(result) => result,
        };
        if metadata.len() > 2000000 || !instance_config.is_file() {
            continue;
        }
        let config_content = match tokio::fs::read_to_string(instance_config).await {
            Err(_) => continue,
            Ok(content) => content,
        };
        let config = match toml::from_str::<InstanceConfig>(&config_content) {
            Ok(config) => config,
            Err(_) => continue,
        };
        let runtime = &config.runtime;
        if (runtime.mod_loader_type.is_none() && runtime.mod_loader_version.is_some())
            || runtime.mod_loader_type.is_some() && runtime.mod_loader_version.is_none()
        {
            continue;
        }
        if folder_name != config.name {
            continue;
        }
        results.push(Instance {
            config,
            installed: tokio::fs::File::open(path.join(".aml-ok")).await.is_ok(),
        })
    }
    println!("Done");
    Ok(results)
}

#[tauri::command]
pub fn set_current_instance(instance_name: String, storage: tauri::State<Storage>) {
    let mut current_instance = storage.current_instance.lock().unwrap();
    println!("Setting current instance to {}", instance_name);
    *current_instance = instance_name;
}

#[tauri::command(async)]
pub async fn get_instance_config(
    storage: tauri::State<'_, Storage>,
) -> std::result::Result<InstanceConfig, ()> {
    let instance_name = storage.current_instance.lock().unwrap().clone();
    let config_path = DATA_LOCATION
        .get()
        .unwrap()
        .get_instance_root(&instance_name)
        .join("instance.toml");
    match config_path.metadata() {
        Ok(_) => {
            let config_content = match tokio::fs::read_to_string(config_path).await {
                Err(_) => return Err(()),
                Ok(content) => content,
            };
            match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => Ok(config),
                Err(_) => Err(()),
            }
        }
        Err(_) => Err(()),
    }
}

#[tauri::command(async)]
pub async fn get_instance_config_from_string(instance_name: &str) -> Result<InstanceConfig, ()> {
    let config_path = DATA_LOCATION
        .get()
        .unwrap()
        .get_instance_root(instance_name)
        .join("instance.toml");
    match config_path.metadata() {
        Ok(_) => {
            let config_content = match tokio::fs::read_to_string(config_path).await {
                Err(_) => return Err(()),
                Ok(content) => content,
            };
            match toml::from_str::<InstanceConfig>(&config_content) {
                Ok(config) => Ok(config),
                Err(_) => Err(()),
            }
        }
        Err(_) => Err(()),
    }
}

pub async fn update_latest_instance() {
    let version_list = VersionManifest::new().await;
    if version_list.is_err() {
        return;
    };
    let version_list = version_list.unwrap();
    if !check_repeated_instance_name("Latest Release".to_string()).await {
        create_instance(
            "Latest Release".to_string(),
            InstanceConfig::new("Latest Release", &version_list.latest.release),
        )
        .await;
    };
    if !check_repeated_instance_name("Latest Snapshot".to_string()).await {
        create_instance(
            "Latest Snapshot".to_string(),
            InstanceConfig::new("Latest Snapshot", &version_list.latest.snapshot),
        )
        .await;
    };
}
