use aml_core::core::version::VersionManifest;
use anyhow::Result;
use tauri::Error;

#[tauri::command(async)]
pub async fn get_version_list() -> Option<VersionManifest> {
    VersionManifest::new().await.ok()
}

#[tauri::command]
pub fn add_instance(instance_name: &str) {
    println!("{instance_name}")
}
