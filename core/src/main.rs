// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod config;
pub mod download;
pub mod folder;
pub mod game_data;
pub mod install;
pub mod instance;
pub mod launch;
pub mod platform;
pub mod utils;
pub mod version;

use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::instance::{get_instance_config, get_instance_config_by_name};
use crate::game_data::{scan_mod_folder, scan_saves_folder};
use crate::install::install;
use crate::install::{
    get_fabric_version_list, get_forge_version_list, get_minecraft_version_list,
    get_quilt_version_list,
};
use crate::instance::{
    check_repeated_instance_name, create_instance, scan_instances_folder, set_current_instance,
};
use crate::launch::launch;
use env_logger::fmt::style::{Color, Style};
use folder::DataLocation;
use log::{debug, error, info, Level, LevelFilter};
use once_cell::sync::OnceCell;
use platform::{OsType, PlatformInfo};
use tauri::{Listener, Manager, Window};
use tauri_plugin_http::reqwest;

/// use MAIN_WINDOW.emit() to send message to main window
static MAIN_WINDOW: OnceCell<Window> = OnceCell::new();
static APP_VERSION: OnceCell<String> = OnceCell::new();
static DATA_LOCATION: OnceCell<DataLocation> = OnceCell::new();
static PLATFORM_INFO: OnceCell<PlatformInfo> = OnceCell::new();
static HTTP_CLIENT: OnceCell<reqwest::Client> = OnceCell::new();
static APPLICATION_DATA: OnceCell<PathBuf> = OnceCell::new();
const DEFAULT_LAUNCHER_PROFILE: &[u8] = include_bytes!("./launcher_profiles.json");

pub struct Storage {
    pub current_instance: Arc<Mutex<String>>,
}
#[tokio::main]
async fn main() {
    initialize_logger();
    print_title();
    info!("Amethyst Launcher is starting up");
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    initialize_application().await;
    info!("Amethyst Launcher is open source, You can view the source code on Github: https://github.com/Conic-Sections/Amethyst-Launcher");
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            create_instance,
            get_minecraft_version_list,
            get_fabric_version_list,
            get_forge_version_list,
            get_quilt_version_list,
            check_repeated_instance_name,
            scan_instances_folder,
            // watch_instances_folder,
            set_current_instance,
            scan_mod_folder,
            scan_saves_folder,
            get_instance_config,
            get_instance_config_by_name,
            install,
            launch,
        ])
        .manage(Storage {
            current_instance: Arc::new(Mutex::new("".to_string())),
        })
        .setup(move |app| {
            MAIN_WINDOW.set(app.get_window("main").unwrap()).unwrap();
            APP_VERSION
                .set(app.package_info().version.to_string())
                .unwrap();
            info!("Main window loaded");
            app.listen_any("fontend-loaded", |_| info!("Frontend loaded"));
            Ok(())
        })
        .run(tauri::generate_context!());
    match result {
        Ok(_) => match tokio::fs::remove_dir_all(&DATA_LOCATION.get().unwrap().temp).await {
            Ok(_) => info!("Temporary files cleared"),
            Err(_) => {
                error!("Could not clear temp foler")
            }
        },
        Err(_) => {
            error!("Fatal Error while running tauri application!")
        }
    };
}

fn initialize_logger() {
    let env = env_logger::Env::default()
        .filter("LOG_LEVEL")
        .write_style("LOG_STYLE");
    env_logger::Builder::from_env(env)
        .filter_level(LevelFilter::Trace)
        .format(|buf, record| {
            let warn_style = buf.default_level_style(Level::Warn);
            let error_style = buf.default_level_style(Level::Error);
            let info_style = buf.default_level_style(Level::Info);
            let debug_style = buf.default_level_style(Level::Debug);
            let debug_text_style = Style::new().fg_color(Some(Color::Ansi(env_logger::fmt::style::AnsiColor::BrightBlack)));
            let trace_style = buf.default_level_style(Level::Trace);
            let timestamp = buf.timestamp();
            let level = record.level();
let target = record.target();
            match level {
                Level::Debug => {
                    writeln!(
                        buf,
                    "[{timestamp}] [{target}/{debug_style}DEBUG{debug_style:#}]: {debug_text_style}{}{debug_text_style:#}",
                        record.args()
                    )
                }
                Level::Info => {
                    writeln!(
                        buf,
                        "[{timestamp}] [{target}/{info_style}INFO{info_style:#}]: {}",
                        record.args()
                    )
                }
                Level::Error => {
                    writeln!(
                        buf,
                        "[{timestamp}] [{target}/{error_style}ERROR{error_style:#}]: {error_style}{}{error_style:#}",
                        record.args()
                    )
                }
                Level::Warn => {
                    writeln!(
                        buf,
                        "[{timestamp}] [{target}/{warn_style}WARN{warn_style:#}]: {}",
                        record.args()
                    )
                }
                Level::Trace => {
                    writeln!(
                        buf,
                        "[{timestamp}] [{target}/{trace_style}TRACE{trace_style:#}]: {}",
                        record.args()
                    )
                }
            }
        })
        .init();
    // let env = env_logger::Env::default()
    //     .filter("MY_LOG_LEVEL")
    //     .write_style("MY_LOG_STYLE");

    // env_logger::Builder::from_env(env)
    //     .format(|buf, record| {
    //         // We are reusing `anstyle` but there are `anstyle-*` crates to adapt it to your
    //         // preferred styling crate.
    //         let warn_style = buf.default_level_style(log::Level::Warn);
    //         let timestamp = buf.timestamp();

    //         writeln!(
    //             buf,
    //             "My formatted log ({timestamp}): {warn_style}{}{warn_style:#}",
    //             record.args()
    //         )
    //     })
    //     .init();
}

async fn initialize_application() {
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    PLATFORM_INFO.set(PlatformInfo::new().await).unwrap();
    initialize_application_data().await;
    DATA_LOCATION
        .set(DataLocation::new(APPLICATION_DATA.get().unwrap()))
        .unwrap();
    HTTP_CLIENT
        .set(
            reqwest::ClientBuilder::new()
                .pool_idle_timeout(Duration::from_secs(30))
                .pool_max_idle_per_host(100)
                .build()
                .unwrap(),
        )
        .unwrap();
    let launch_profiles_path = DATA_LOCATION
        .get()
        .unwrap()
        .root
        .join("launcher_profiles.json");
    let _ = tokio::fs::remove_file(&launch_profiles_path).await;
    tokio::fs::write(&launch_profiles_path, DEFAULT_LAUNCHER_PROFILE)
        .await
        .expect("C");
    instance::update_latest_instance().await;
}

async fn initialize_application_data() {
    let platform_info = PLATFORM_INFO.get().unwrap();
    match platform_info.os_type {
        OsType::Linux => info!("The program is running on Linux {}", platform_info.version),
        OsType::Osx => info!("The program is running on macOS {}", platform_info.version),
        OsType::Windows => info!(
            "The program is running on fucking Windows {}",
            platform_info.version
        ),
    }
    match platform_info.os_type {
        OsType::Windows => {
            APPLICATION_DATA
                .set(
                    PathBuf::from(
                        std::env::var("APP_DATA").expect("Could not found APP_DATA directory"),
                    )
                    .join("aml"),
                )
                .unwrap();
        }
        OsType::Linux => {
            APPLICATION_DATA
                .set(
                    PathBuf::from(std::env::var("HOME").expect("Could not found home"))
                        .join(".aml"),
                )
                .unwrap();
        }
        OsType::Osx => {
            APPLICATION_DATA
                .set(PathBuf::from("/Users/").join("aml"))
                .unwrap();
        }
    }
    info!(
        "Application data path: {}",
        APPLICATION_DATA.get().unwrap().to_string_lossy()
    );
}

fn print_title() {
    debug!("  █████╗ ███╗   ███╗███████╗████████╗██╗  ██╗██╗   ██╗███████╗████████╗ ");
    debug!(" ██╔══██╗████╗ ████║██╔════╝╚══██╔══╝██║  ██║╚██╗ ██╔╝██╔════╝╚══██╔══╝ ");
    debug!(" ███████║██╔████╔██║█████╗     ██║   ███████║ ╚████╔╝ ███████╗   ██║    ");
    debug!(" ██╔══██║██║╚██╔╝██║██╔══╝     ██║   ██╔══██║  ╚██╔╝  ╚════██║   ██║    ");
    debug!(" ██║  ██║██║ ╚═╝ ██║███████╗   ██║   ██║  ██║   ██║   ███████║   ██║    ");
    debug!(" ╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝   ╚═╝   ╚═╝  ╚═╝   ╚═╝   ╚══════╝   ╚═╝    ");
    debug!("");
    debug!(" ██╗      █████╗ ██╗   ██╗███╗   ██╗ ██████╗██╗  ██╗███████╗██████╗     ");
    debug!(" ██║     ██╔══██╗██║   ██║████╗  ██║██╔════╝██║  ██║██╔════╝██╔══██╗    ");
    debug!(" ██║     ███████║██║   ██║██╔██╗ ██║██║     ███████║█████╗  ██████╔╝    ");
    debug!(" ██║     ██╔══██║██║   ██║██║╚██╗██║██║     ██╔══██║██╔══╝  ██╔══██╗    ");
    debug!(" ███████╗██║  ██║╚██████╔╝██║ ╚████║╚██████╗██║  ██║███████╗██║  ██║    ");
    debug!(" ╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝    ");
}
