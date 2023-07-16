#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod folder;
pub mod instance;

use instance::{
    check_repeated_instance_name, create_instance, get_fabric_version_list, get_forge_version_list,
    get_minecraft_version_list, get_quilt_version_list,
};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            create_instance,
            get_minecraft_version_list,
            get_fabric_version_list,
            get_forge_version_list,
            get_quilt_version_list,
            check_repeated_instance_name
        ])
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
