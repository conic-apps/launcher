use aml_core::{
    core::version::VersionManifest,
    install::{
        fabric::LoaderArtifactList,
        forge::version_list::ForgeVersionList,
        quilt::{version_list::get_quilt_version_list_from_mcversion, QuiltVersion},
    },
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::folder::DataLocation;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceConfig {
    pub name: String,
    pub runtime: InstanceRuntime
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstanceRuntime {
    pub minecraft: String,
    pub fabric: String,
    pub forge: String,
    pub quilt: String,
    pub optifine: String,
}

#[tauri::command(async)]
pub async fn get_minecraft_version_list() -> Option<VersionManifest> {
    VersionManifest::new().await.ok()
}

#[tauri::command(async)]
pub async fn get_fabric_version_list(mcversion: String) -> Option<LoaderArtifactList> {
    LoaderArtifactList::from_mcversion(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_forge_version_list(mcversion: String) -> Option<ForgeVersionList> {
    ForgeVersionList::from_mcversion(&mcversion).await.ok()
}

#[tauri::command(async)]
pub async fn get_quilt_version_list(mcversion: String) -> Option<Vec<QuiltVersion>> {
    get_quilt_version_list_from_mcversion(None, &mcversion)
        .await
        .ok()
}

#[tauri::command(async)]
pub async fn create_instance(
    instance_name: String,
    config: InstanceConfig,
    datafolder_path: String,
) -> Option<()> {
    async fn create_instance(
        instance_name: String,
        config: InstanceConfig,
        datafolder_path: String,
    ) -> Result<()> {
        let data_location = DataLocation::new(&datafolder_path);
        let instance_root = data_location.get_instance_root(&instance_name);
        let config_file = instance_root.join("instance.toml");
        fs::create_dir_all(config_file.parent().ok_or(anyhow::anyhow!("Path Error"))?).await?;
        fs::write(config_file, toml::to_string_pretty(&config)?).await?;
        Ok(())
    }
    create_instance(instance_name, config, datafolder_path).await.ok()
}

#[tauri::command(async)]
pub async fn check_repeated_instance_name(instance_name: String, datafolder_path: String) -> bool {
    let config_path = DataLocation::new(&datafolder_path).get_instance_root(instance_name).join("instance.toml");
    match config_path.metadata() {
        Ok(_) => true,
        Err(_) => false
    }
}