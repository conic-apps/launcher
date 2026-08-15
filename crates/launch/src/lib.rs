// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::BufRead,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use account::Account;
use arguments::generate_command_arguments;
use complete::complete_files;
use config::Config;
use download::progress::DownloadState;
use folder::{DATA_LOCATION, MinecraftLocation};
use instance::Instance;
use java_runtime::JavaArch;
use log::{debug, error, info, warn};
use options::LaunchOptions;
use platform::{OsArch, OsFamily, PLATFORM_INFO};
use serde::Serialize;
use statistics::{StatisticsProfile, log_launch};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use tauri::{
    Manager, Runtime, State, command,
    ipc::Channel,
    plugin::{Builder, TauriPlugin},
};
use uuid::Uuid;
use version::{ResolvedVersion, Version, resolve_version};

mod arguments;
mod complete;
pub mod error;
mod options;

use error::*;

#[derive(Clone, Default)]
struct PluginState {
    task: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("launch")
        .invoke_handler(tauri::generate_handler![
            cmd_spawn_launch_task,
            cmd_cancel_launch_task
        ])
        .setup(|app, _| {
            app.manage(PluginState::default());
            Ok(())
        })
        .build()
}

#[derive(Clone, Serialize, PartialEq)]
#[serde(tag = "job", content = "progress")]
pub enum LaunchEvent {
    Prepare,
    RefreshAccount,
    InstallAuthlibInjector(DownloadState),
    CompleteFiles(DownloadState),
    GenerateScriptlet,
    WaitForLaunch,
    LogSettingUser,
    LogLwjglVersion,
    LogOpenALLoaded,
    LogTextureLoaded,
}

#[command]
async fn cmd_spawn_launch_task(
    state: State<'_, PluginState>,
    config: Config,
    instance: Instance,
    channel: Channel<LaunchEvent>,
) -> Result<u32> {
    if state.task.lock().expect("Internal error").is_some() {
        return Err(Error::AnothorInstanceLaunching);
    }
    let task_status = Arc::new(Mutex::new(LaunchEvent::Prepare));
    let finished = Arc::new(AtomicBool::new(false));
    let handle = tokio::spawn({
        let task_status_cloned = task_status.clone();
        let finished = finished.clone();
        async move {
            let result = launch(config, instance, task_status_cloned).await;
            finished.store(true, Ordering::SeqCst);
            result
        }
    });
    {
        let mut current_task = state.task.lock().expect("Internal error");
        *current_task = Some(handle.abort_handle());
    }
    let event_sender_thread = {
        let status_cloned = task_status.clone();
        let finished = finished.clone();
        thread::spawn(move || {
            while !finished.load(Ordering::SeqCst) {
                let _ = channel.send(status_cloned.lock().expect("Internal error").clone());
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };
    let result = match handle.await {
        Ok(result) => result,
        Err(e) => {
            warn!("Launch cancelled");
            Err(Error::Aborted(e))
        }
    };
    let _ = event_sender_thread.join();
    {
        let mut current_task = state.task.lock().expect("Internal error");
        *current_task = None;
    }
    result
}

#[command]
async fn cmd_cancel_launch_task(state: State<'_, PluginState>) -> Result<()> {
    let mut current_task = state.task.lock().expect("Internal error");
    if let Some(handle) = current_task.clone() {
        handle.abort();
        warn!("Cancelling launch!");
    }
    *current_task = None;
    Ok(())
}

/// Represents a log message associated with a specific instance.
#[derive(Clone, Serialize)]
pub struct Log {
    /// The UUID of the instance this log belongs to.
    #[serde(rename = "instanceName")]
    pub instance_id: Uuid,

    /// The content of the log message.
    pub content: String,
}

/// Launches a Minecraft instance asynchronously via the Tauri command system.
///
/// # Arguments
/// * `storage` - Application state that holds shared configuration and data.
/// * `instance` - The Minecraft instance to launch.
///
/// # Returns
/// * `Ok(())` - If the instance was successfully launched.
/// * `Err(())` - If there was an error during launch (e.g., account not found).
///
/// # Side Effects
/// * Refreshes the selected account if configured to do so.
/// * Optionally checks files before launch.
/// * Spawns the Minecraft process and generates launch script.
pub async fn launch(
    config: Config,
    instance: Instance,
    status: Arc<Mutex<LaunchEvent>>,
) -> Result<u32> {
    info!(
        "Starting Minecraft client, instance: {}",
        instance.config.name
    );
    print_instance_info(&instance);
    let minecraft_location = MinecraftLocation::new(&DATA_LOCATION.root);

    if config.launch.skip_check_files {
        info!("File checking disabled by user")
    } else {
        let progress = DownloadState::default();
        {
            let mut status = status.lock().expect("Internal error");
            *status = LaunchEvent::CompleteFiles(progress.clone());
        }
        complete_files(&instance, &minecraft_location, progress, &config.download).await?;
    }

    info!("Generating startup parameters");
    let version_json_path = minecraft_location.get_version_json(instance.get_version_id()?);
    let raw_version_json = async_fs::read_to_string(version_json_path).await?;
    let resolved_version = resolve_version(
        &Version::from_str(&raw_version_json)?,
        &minecraft_location,
        &[],
    )
    .await?;
    let resolved_java = resolve_java_executable(&config, &instance, &resolved_version).await?;
    {
        let mut status = status.lock().expect("Internal error");
        *status = LaunchEvent::GenerateScriptlet;
    }
    let launch_options = LaunchOptions::new(&config, &instance, resolved_java.arch)?;
    if let Account::Yggdrasil(_) = launch_options.selected_account {
        let progress = DownloadState::default();
        install::authlib_injector::ensure_latest(&progress).await?;
    }
    let command_arguments = generate_command_arguments(
        &minecraft_location,
        &instance,
        &launch_options,
        &resolved_version,
    )
    .await?;

    let result = spawn_minecraft_process(
        command_arguments,
        launch_options,
        instance,
        resolved_java.path,
        status,
    )
    .await;
    if let Err(e) = &result {
        error!("Failed to spawn Minecraft process: {e}");
    }
    result
}

fn print_instance_info(instance: &Instance) {
    info!("------------- Instance runtime config -------------");
    info!("-> Minecraft: {}", instance.config.runtime.minecraft);
    match &instance.config.runtime.mod_loader_type {
        Some(x) => info!("-> Mod loader: {x}"),
        None => info!("-> Mod loader: none"),
    };
    match &instance.config.runtime.mod_loader_version {
        Some(x) => info!("-> Mod loader version: {x}"),
        None => info!("-> Mod loader version: none"),
    };
}

/// A resolved Java runtime: the executable path together with its architecture.
struct ResolvedJava {
    path: PathBuf,
    arch: JavaArch,
}

/// Resolves the Java executable used to launch the game.
///
/// The resolution order is:
/// 1. An instance-specific `java_path`, when configured by the user;
/// 2. When `config.prefer_mojang_java` is enabled, the Mojang-provided runtime
///    installed under the launcher runtime directory;
/// 3. A system-installed Java runtime matching the required major version,
///    excluding launcher-managed and user-disabled runtimes.
///
/// The returned [`ResolvedJava::arch`] carries the bitness of the runtime:
/// Mojang-provided Java always matches the OS architecture, while scanned
/// system runtimes report their own parsed architecture. The instance-specific
/// path is reported as [`JavaArch::Unknown`] since it is not parsed again.
///
/// Returns [`Error::NoSuitableJavaRuntime`] when no usable runtime is found.
async fn resolve_java_executable(
    config: &Config,
    instance: &Instance,
    resolved_version: &ResolvedVersion,
) -> Result<ResolvedJava> {
    if let Some(java_path) = &instance.config.launch_config.java_path {
        info!("Using instance-specific Java: {java_path}");
        return Ok(ResolvedJava {
            path: PathBuf::from(java_path),
            arch: JavaArch::Unknown,
        });
    }

    if config.prefer_mojang_java
        && let Ok(mojang_path) =
            install::java::get_executable_path(&resolved_version.java_version.component)
    {
        if mojang_path.is_file() {
            info!("Using Mojang-provided Java: {}", mojang_path.display());
            return Ok(ResolvedJava {
                path: mojang_path,
                arch: mojang_java_arch(),
            });
        }
        info!(
            "Mojang-provided Java not found at {}, falling back to system Java",
            mojang_path.display()
        );
    }

    let required_major_version = resolved_version.java_version.major_version;
    let runtimes = tokio::task::spawn_blocking(java_runtime::scan_java_runtimes)
        .await
        .map_err(|_| Error::Other)??;
    let system_java = runtimes.into_iter().find(|runtime| {
        runtime.major_version == required_major_version as u32
            && runtime.is_valid
            && !runtime.is_managed
            && !config
                .disabled_java_runtime
                .iter()
                .any(|disabled| runtime.path == Path::new(disabled))
    });
    if let Some(runtime) = system_java {
        info!("Using system Java: {}", runtime.path.display());
        return Ok(ResolvedJava {
            path: runtime.path,
            arch: runtime.arch,
        });
    }
    Err(Error::NoSuitableJavaRuntime)
}

/// Maps the current OS architecture to the architecture of the Mojang-provided
/// Java runtime, which is always built for the host platform.
fn mojang_java_arch() -> JavaArch {
    match PLATFORM_INFO.arch {
        OsArch::X64 => JavaArch::X64,
        OsArch::X86 => JavaArch::X86,
        OsArch::Aarch64 => JavaArch::Aarch64,
        OsArch::Arm => JavaArch::Arm,
        _ => JavaArch::Unknown,
    }
}

/// Spawns the Minecraft process by generating and executing a launch script,
/// customized per operating system and instance configuration.
///
/// # Arguments
/// * `command_arguments` - A list of parsed arguments.
/// * `minecraft_location` - Path to the Minecraft game files.
/// * `launch_options` - Launch customization options (pre/post-execution hooks, wrappers, etc.).
/// * `version_id` - The Minecraft version to launch.
/// * `instance` - The instance metadata and configuration.
///
/// # Behavior
/// * Creates a platform-specific shell script/batch file for launching the game.
/// * Runs the generated script using a subprocess.
/// * Streams stdout to detect key launch indicators and forward logs to the frontend.
/// * Emits `launch_success` event once LWJGL is detected.
/// * Handles cleanup of native libraries after game launch completes.
async fn spawn_minecraft_process(
    command_arguments: Vec<String>,
    launch_options: LaunchOptions,
    instance: Instance,
    java_path: PathBuf,
    status: Arc<Mutex<LaunchEvent>>,
) -> Result<u32> {
    // TODO: 要求 Java 使用高性能显卡
    let instance_root = DATA_LOCATION.get_instance_root(&instance.id);
    let mut commands = String::new();
    if PLATFORM_INFO.os_family != OsFamily::Windows {
        commands.push_str("#!/bin/sh\n\n");
    }
    let comment_prefix = if PLATFORM_INFO.os_family == OsFamily::Windows {
        "::"
    } else {
        "#"
    };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Internal error")
        .as_millis();
    commands.push_str(&format!(
        "{comment_prefix} This file is automatically generated by Conic Launcher.\n"
    ));
    commands.push_str(&format!(
        "{comment_prefix} Conic Launcher has created this file at {timestamp}.\n"
    ));
    commands.push_str(&format!(
        "{comment_prefix} NOTE: Don't use this file to launch game.\n\n"
    ));
    commands.push_str(&format!("cd \"{}\"\n", instance_root.to_string_lossy()));
    commands.push_str(&format!("{}\n", launch_options.execute_before_launch));
    if !launch_options.wrap_command.trim().is_empty() {
        commands.push_str(&format!("{} ", launch_options.wrap_command));
    }
    // todo(after java exec): add -Dfile.encoding=encoding.name() and other
    let mut launch_command = match PLATFORM_INFO.os_family {
        OsFamily::Windows => String::new(),
        _ => "exec ".to_string(),
    };
    launch_command.push_str(&format!("\"{}\"", java_path.to_string_lossy()));
    for arg in command_arguments.clone() {
        let arg = if arg.contains(" ") {
            format!("\"{arg}\"")
        } else {
            arg
        };
        launch_command.push_str(&format!(" {arg}"));
    }
    commands.push_str(&launch_command);
    let script_path = match PLATFORM_INFO.os_family {
        OsFamily::Windows => instance_root.join(".cache").join("conic-launch.bat"),
        _ => instance_root.join(".cache").join("conic-launch.sh"),
    };
    if let Some(script_path_parent) = script_path.parent() {
        std::fs::create_dir_all(script_path_parent)?;
    }
    std::fs::write(&script_path, commands)?;
    info!("The startup script is written to {}", script_path.display());
    let mut minecraft_process = match PLATFORM_INFO.os_family {
        OsFamily::Windows => std::process::Command::new(script_path),
        _ => {
            info!("Running chmod +x {}", script_path.display());
            let mut chmod = Command::new("chmod");
            chmod.args(["+x", script_path.to_string_lossy().to_string().as_ref()]);
            chmod.status()?;
            let mut command = std::process::Command::new("bash");
            command.arg(script_path);
            command
        }
    }
    .stdout(Stdio::piped())
    .spawn()?;
    {
        let mut status = status.lock().expect("Internal error");
        *status = LaunchEvent::WaitForLaunch;
    }
    info!("Spawning minecraft process");
    let out = minecraft_process
        .stdout
        .take()
        .ok_or(Error::TakeMinecraftStdoutFailed)?;
    let mut out = std::io::BufReader::new(out);
    let pid = minecraft_process.id();
    let status_cloned = status.clone();
    let mut buf = String::new();
    thread::spawn(move || {
        loop {
            buf.clear();
            let size = match out.read_line(&mut buf) {
                Ok(size) => size,
                Err(_) => break,
            };
            if size == 0 {
                break;
            }
            let line = buf.trim();
            debug!("[{pid}] {line}");
            if line.contains("Setting user:") {
                let mut status = status.lock().expect("Internal error");
                *status = LaunchEvent::LogSettingUser;
            }
            if line.to_lowercase().contains("lwjgl version") {
                info!("Found LWJGL version, the game seems to have started successfully.");
                let mut status = status.lock().expect("Internal error");
                *status = LaunchEvent::LogLwjglVersion;
            }
            if line.contains("OpenAL initialized") {
                let mut status = status.lock().expect("Internal error");
                *status = LaunchEvent::LogOpenALLoaded;
            }
            if line.contains("Created") {
                let mut status = status.lock().expect("Internal error");
                *status = LaunchEvent::LogTextureLoaded;
            }
        }

        let output = match minecraft_process.wait_with_output() {
            Ok(output) => output,
            Err(_) => {
                error!("Could not get Minecrafr exit code");
                return;
            }
        };
        if !output.status.success() {
            // TODO: log analysis and remove libraries lock file
            // NOTE: Should use tauri global event here
            // WARN: When failed, frontend should stop all "launching" animation
            error!("Minecraft exits with error code {}", output.status);
        } else {
            info!("Minecraft exits with error code {}", output.status);
        }
    });
    let start = Instant::now();
    while start.elapsed().as_secs() < 30
        && *status_cloned.lock().expect("Internal error") != LaunchEvent::LogTextureLoaded
    {
        async_io::Timer::after(Duration::from_secs(1)).await;
    }
    match PLATFORM_INFO.os_family {
        OsFamily::Windows => {
            #[cfg(target_os = "windows")]
            let _ = Command::new("cmd")
                .args(["/C", &launch_options.execute_after_launch])
                .creation_flags(0x08000000)
                .spawn();
        }
        _ => {
            let _ = Command::new("sh")
                .args(["-c", &launch_options.execute_after_launch])
                .spawn();
        }
    }
    let statistics_profile = match launch_options.selected_account {
        Account::Microsoft(account) => StatisticsProfile::Microsoft(account.profile.uuid),
        Account::Offline(account) => StatisticsProfile::Offline(account.uuid),
        Account::Yggdrasil(account) => StatisticsProfile::Yggdrasil(account.identifier),
    };
    log_launch(statistics_profile, instance.id).await.unwrap();
    Ok(pid)
}
