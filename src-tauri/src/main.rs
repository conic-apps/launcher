#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod folder;
pub mod instance;

use std::sync::{Arc, Mutex};

use crate::instance::{
    check_repeated_instance_name, create_instance, get_fabric_version_list, get_forge_version_list,
    get_instance_config, get_instance_config_from_string, get_minecraft_version_list,
    get_quilt_version_list, install_command, scan_instances_folder, scan_mod_folder,
    scan_saves_folder, set_active_instance, watch_instances_folder,
};
use aml_core::core::PlatformInfo;
use folder::DataLocation;
use once_cell::sync::OnceCell;
use tauri::{Manager, Window};

/// use MAIN_WINDOW.emit() to send message to main window
static MAIN_WINDOW: OnceCell<Window> = OnceCell::new();
static DATA_LOCATION: OnceCell<DataLocation> = OnceCell::new();
static PLATFORM_INFO: OnceCell<PlatformInfo> = OnceCell::new();
static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();

pub struct Storage {
    pub active_instance: Arc<Mutex<String>>,
}

#[tokio::main]
async fn main() {
    DATA_LOCATION.set(DataLocation::new("test")).unwrap();
    PLATFORM_INFO.set(PlatformInfo::new().await).unwrap();
    HTTP_CLIENT
        .set(reqwest::ClientBuilder::new().build().unwrap())
        .unwrap();
    tauri::Builder::default()
        .manage(Storage {
            active_instance: Arc::new(Mutex::new("".to_string())),
        })
        .invoke_handler(tauri::generate_handler![
            create_instance,
            get_minecraft_version_list,
            get_fabric_version_list,
            get_forge_version_list,
            get_quilt_version_list,
            check_repeated_instance_name,
            scan_instances_folder,
            watch_instances_folder,
            set_active_instance,
            scan_mod_folder,
            scan_saves_folder,
            get_instance_config,
            get_instance_config_from_string,
            install_command,
        ])
        .setup(|app| {
            MAIN_WINDOW.set(app.get_window("main").unwrap()).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running amethyst launcher!");
}
