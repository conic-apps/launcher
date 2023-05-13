#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{sync::atomic::Ordering, thread, time::Duration};

// use mgl_core::{installer::vanilla, utils::folder::MinecraftLocation};

fn main() {
    tauri::Builder::default()
        // .invoke_handler(tauri::generate_handler![get_user_config])
        .run(tauri::generate_context!())
        .expect("error while running magical launcher!");
}

// #[tauri::command]
// fn get_user_config() -> String {
// return "".to_string();
// if !fs::metadata("/home/brokendeer/桌面/mc2/magical-launcher/test/test.json").is_ok() {
//     fs::File::create("/home/brokendeer/桌面/mc2/magical-launcher/test/test.json").unwrap();
//     return "".to_string();
// }
// fs::read_to_string("/home/brokendeer/桌面/mc2/magical-launcher/test/test.json").unwrap()
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
