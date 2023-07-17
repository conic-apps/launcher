#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod folder;
pub mod instance;

use std::sync::{Arc, Mutex};

use crate::instance::{
    check_repeated_instance_name, create_instance, get_fabric_version_list, get_forge_version_list,
    get_minecraft_version_list, get_quilt_version_list, scan_instances_folder, scan_mod_folder,
    scan_saves_folder, set_active_instance, watch_instances_folder,
};
use folder::DataLocation;
use once_cell::sync::OnceCell;
use tauri::{Manager, Window};

/// use MAIN_WINDOW.emit() to send message to main window
static MAIN_WINDOW: OnceCell<Window> = OnceCell::new();
static DATA_LOCATION: OnceCell<DataLocation> = OnceCell::new();

pub struct Storage {
    pub active_instance: Arc<Mutex<String>>,
}

fn main() {
    DATA_LOCATION.set(DataLocation::new("test")).unwrap();
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
            scan_saves_folder
        ])
        .setup(|app| {
            MAIN_WINDOW.set(app.get_window("main").unwrap()).unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running amethyst launcher!");
}

// #[tauri::command]
// fn get_user_config() -> String {
// return "".to_string();
// if !fs::metadata("/home/brokendeer/桌面/mc2/amethyst-launcher/test/test.json").is_ok() {
//     fs::File::create("/home/brokendeer/桌面/mc2/amethyst-launcher/test/test.json").unwrap();
//     return "".to_string();
// }
// fs::read_to_string("/home/brokendeer/桌面/mc2/amethyst-launcher/test/test.json").unwrap()
// }

// #[tokio::test]
// async fn install() {
//     let task = mgl_core::installer::vanilla::Task::new("installer");
//     let task_clone = task.clone();
//     thread::spawn(move || loop {
//         let progress = task_clone.progress.load(Ordering::SeqCst);
//         let total = task_clone.total.load(Ordering::SeqCst);
//         let percentage = if total == 0 {
//             0.0
//         } else {
//             progress as f64 / total as f64 * 100.0
//         };
//         println!("{}% {}/{}", format!("{:.2}", percentage), progress, total);
//         thread::sleep(Duration::from_micros(50000));
//     });
//     vanilla::install("1.19.4", MinecraftLocation::new("test"), task).await;
// }
