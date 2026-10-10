// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch pipeline: turning an installed instance into a running Minecraft
//! process.
//!
//! [`launch`] runs the whole pipeline — file completion, version resolution,
//! Java selection, authlib-injector setup, the argument list and the generated
//! launch script with its stdout log watcher — and reports progress through the
//! [`LaunchSink`] the caller hands in. The crate samples its own download
//! counters; the caller never sees one.
//!
//! The process outlives the call: [`launch`] returns once the game is up, and
//! the process is registered in [`running`]. The app can then list and
//! terminate running sessions, and each process's stdout watcher cleans the
//! entry up when it exits. The app still cancels a *launch in progress* by
//! aborting the task that awaits [`launch`]; stopping an already-running game
//! goes through [`running::terminate`].

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use log::{debug, error, info};
use serde::Serialize;
use uuid::Uuid;

use arguments::generate_command_arguments;
use complete::complete_files;
use options::LaunchOptions;

use account::Account;
use config::Config;
use download::progress::{DownloadSnapshot, DownloadState};
use instance::Instance;
use java_discovery::ResolveJavaOptions;
use platform::{OsFamily, PLATFORM_INFO, strip_unc_prefix};
use shared::{ChangeReporter, Sink};
use statistics::{StatisticsProfile, log_launch};
use storage::LOCATIONS;
use version::{Version, resolve_version};

mod arguments;
mod complete;
pub mod crash;
pub mod error;
mod options;
pub mod running;

pub use error::*;

/// What a launch reports, in the order it reports it.
///
/// Each download variant holds a plain [`DownloadSnapshot`] rather than a
/// [`DownloadState`], so an event can cross a thread and compare by value.
#[derive(Debug, Clone, PartialEq)]
pub enum LaunchProgress {
    Prepare,
    InstallAuthlibInjector(DownloadSnapshot),
    CompleteFiles(DownloadSnapshot),
    GenerateScriptlet,
    WaitForLaunch,
    LogSettingUser,
    LogLwjglVersion,
    LogOpenALLoaded,
    LogTextureLoaded,
}

/// The output port [`launch`] reports through.
pub type LaunchSink = Sink<LaunchProgress>;

#[derive(Clone, Serialize)]
pub struct Log {
    #[serde(rename = "instanceName")]
    pub instance_id: Uuid,

    pub content: String,
}

/// Launches a Minecraft instance, returning the spawned process's PID.
///
/// The caller aborts the task to cancel a launch in progress.
pub async fn launch(config: Config, instance: Instance, sink: LaunchSink) -> Result<u32> {
    let reporter = Arc::new(ChangeReporter::new(sink));
    reporter.report(LaunchProgress::Prepare);
    info!(
        "Starting Minecraft client, instance: {}",
        instance.config.name
    );
    print_instance_info(&instance);
    let minecraft_location = LOCATIONS.minecraft.clone();

    if instance
        .config
        .launch_config
        .skip_check_files
        .unwrap_or(config.launch.skip_check_files)
    {
        info!("File checking disabled by user")
    } else {
        let progress = DownloadState::default();
        reporter.report(LaunchProgress::CompleteFiles(progress.snapshot()));
        download::progress::watch(
            &progress,
            |snapshot| reporter.report(LaunchProgress::CompleteFiles(snapshot)),
            complete_files(
                &instance,
                &minecraft_location,
                progress.clone(),
                config.prefer_mojang_java,
                &config.download,
            ),
        )
        .await?;
    }

    info!("Generating startup parameters");
    // The id becomes the resolved one here, after the whole `inheritsFrom`
    // chain has been walked, so log that rather than only the requested id.
    let requested_version_id = instance.get_version_id()?;
    info!(
        "Resolving {requested_version_id} from {}",
        minecraft_location.root.display()
    );
    let version_json_path = minecraft_location.get_version_json(&requested_version_id);
    let raw_version_json = tokio::fs::read_to_string(&version_json_path).await?;
    let resolved_version = resolve_version(
        &Version::from_str(&raw_version_json)?,
        &minecraft_location,
        &[],
    )?;
    info!(
        "Resolved {} to {} (main class {:?})",
        requested_version_id, resolved_version.id, resolved_version.main_class
    );
    info!(
        "The game wants Java {} (component {})",
        resolved_version.java_version.major_version, resolved_version.java_version.component
    );
    let resolved_java = java_discovery::resolve_java_executable(&ResolveJavaOptions {
        instance_java_path: instance.config.launch_config.java_path.clone(),
        prefer_mojang_java: config.prefer_mojang_java,
        disabled_java_runtimes: config.disabled_java_runtime.clone(),
        required_major_version: resolved_version.java_version.major_version as u32,
        mojang_component: resolved_version.java_version.component.clone(),
    })
    .await?;
    reporter.report(LaunchProgress::GenerateScriptlet);
    let launch_options = LaunchOptions::new(&config, &instance, resolved_java.arch)?;
    if let Account::Yggdrasil(account) = &launch_options.selected_account {
        // The module this calls logs nothing, so a failed injector install would
        // otherwise be silent.
        log::info!(
            "Ensuring the authlib-injector is current for the Yggdrasil account {}",
            account.identifier
        );
        let progress = DownloadState::default();
        reporter.report(LaunchProgress::InstallAuthlibInjector(progress.snapshot()));
        download::progress::watch(
            &progress,
            |snapshot| reporter.report(LaunchProgress::InstallAuthlibInjector(snapshot)),
            install::authlib_injector::ensure_latest(&progress),
        )
        .await?;
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
        reporter,
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

/// Quotes one argument for the shell the generated launch script runs under.
///
/// The Unix script is `sh`/`bash`, so POSIX single-quoting is used: the value
/// is wrapped in `'…'` and an inner `'` is closed and reopened around
/// (`'\''`). That leaves every byte literal — spaces, `$`, backticks, double
/// quotes — which is what keeps a data path with spaces (the macOS default,
/// `…/Application Support/…`) in one piece. A Windows batch file understands
/// only `"…"`, and an argument with no space or tab is left bare; the launch
/// arguments never contain a `"`.
fn quote_shell_arg(argument: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        if argument.contains(' ') || argument.contains('\t') {
            format!("\"{argument}\"")
        } else {
            argument.to_string()
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        format!("'{}'", argument.replace('\'', "'\\''"))
    }
}

/// Builds the `java …` command line for the launch script.
///
/// Every argument is shell-quoted; an empty one is dropped, because once quoting
/// wraps each argument an empty `extra_jvm_args` becomes an explicit empty word
/// and `java` reads the next word as the main class.
fn build_launch_command(java_path: &Path, command_arguments: &[String]) -> String {
    let mut command = match PLATFORM_INFO.os_family {
        OsFamily::Windows => String::new(),
        _ => "exec ".to_string(),
    };
    command.push_str(&quote_shell_arg(
        &strip_unc_prefix(java_path.to_path_buf()).to_string_lossy(),
    ));
    for argument in command_arguments {
        if argument.is_empty() {
            continue;
        }
        command.push(' ');
        command.push_str(&quote_shell_arg(argument));
    }
    command
}

/// Writes and runs the platform-specific launch script, streaming the game's
/// stdout to latch the start markers reported through `reporter`.
async fn spawn_minecraft_process(
    command_arguments: Vec<String>,
    launch_options: LaunchOptions,
    instance: Instance,
    java_path: PathBuf,
    reporter: Arc<ChangeReporter<LaunchProgress>>,
) -> Result<u32> {
    // TODO: ask Java to use the high-performance GPU.
    let instance_root = LOCATIONS.instances.get_instance_root(&instance.id);
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
    commands.push_str(&format!(
        "cd {}\n",
        quote_shell_arg(&instance_root.to_string_lossy())
    ));
    commands.push_str(&format!("{}\n", launch_options.execute_before_launch));
    if !launch_options.wrap_command.trim().is_empty() {
        commands.push_str(&format!("{} ", launch_options.wrap_command));
    }
    // todo(after java exec): add -Dfile.encoding=encoding.name() and other
    // The access token rides in `command_arguments` (see `arguments.rs`), so the
    // whole line is never logged — only the executable, the main class and the
    // argument count.
    let launch_command = build_launch_command(&java_path, &command_arguments);
    info!(
        "Launching {} with {} argument(s){}",
        java_path.display(),
        command_arguments.len(),
        format_args!(" (before: {:?})", launch_options.execute_before_launch),
    );
    for argument in &command_arguments {
        if argument.starts_with("-D") && !argument.contains("auth_access_token") {
            debug!("  {argument}");
        }
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

    #[cfg(target_os = "windows")]
    let minecraft_process = {
        let mut command = std::process::Command::new(script_path);
        command.creation_flags(0x08000000);
        command
    }
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
    #[cfg(not(target_os = "windows"))]
    let minecraft_process = {
        info!("Running chmod +x {}", script_path.display());
        let mut chmod = Command::new("chmod");
        chmod.args(["+x", script_path.to_string_lossy().to_string().as_ref()]);
        // A failed `chmod` (read-only mount, filesystem without the bit) leaves
        // a non-executable script whose only symptom is the `bash` spawn
        // failing.
        let chmod_status = chmod.status()?;
        if !chmod_status.success() {
            log::warn!(
                "chmod +x {} failed with {chmod_status}; the script may not be executable",
                script_path.display()
            );
        }
        let mut command = std::process::Command::new("bash");
        command.arg(script_path);
        command
    }
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;
    reporter.report(LaunchProgress::WaitForLaunch);
    info!("Spawning minecraft process");
    let pid = minecraft_process.id();
    // Latched by the stdout watcher on any of the three markers the wait below
    // breaks on, so the launch task does not read the reporter's own last value.
    let game_up = Arc::new(AtomicBool::new(false));
    let watcher_reporter = Arc::clone(&reporter);
    let watcher_game_up = Arc::clone(&game_up);
    // The process outlives this call. `running::register` owns it from here:
    // it takes the stdout and stderr pipes, watches them to EOF, and reaps and
    // forgets the session on exit.
    running::register(instance.clone(), minecraft_process, move |line| {
        debug!("[{pid}] {line}");
        if line.contains("Setting user:") {
            info!("The game reported {line}");
            watcher_reporter.report(LaunchProgress::LogSettingUser);
        }
        if line.to_lowercase().contains("lwjgl version") {
            info!("Found LWJGL version, the game seems to have started successfully.");
            watcher_reporter.report(LaunchProgress::LogLwjglVersion);
            watcher_game_up.store(true, Ordering::SeqCst);
        }
        if line.contains("OpenAL initialized") {
            // Logged at `info` like the LWJGL marker: it latches `game_up` and
            // ends the 20-second wait, so it is a launch milestone.
            info!("The game reported {line}");
            watcher_reporter.report(LaunchProgress::LogOpenALLoaded);
            watcher_game_up.store(true, Ordering::SeqCst);
        }
        if (line.contains("Created") && line.contains("textures") && line.contains("-atlas"))
            || line.contains("Found animation info")
        {
            info!("The game reported {line}");
            watcher_reporter.report(LaunchProgress::LogTextureLoaded);
            watcher_game_up.store(true, Ordering::SeqCst);
        }
    });
    info!("Waiting up to 20 seconds for the game to report that it started");
    let start = Instant::now();
    while start.elapsed().as_secs() < 20 {
        if game_up.load(Ordering::SeqCst) {
            info!(
                "The game reported a successful start after {} second(s)",
                start.elapsed().as_secs()
            );
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    if !game_up.load(Ordering::SeqCst) {
        log::warn!(
            "The game (pid {pid}) did not report a successful start within 20 seconds; \
             whatever it printed is above and its exit, if any, is below"
        );
    }
    // Run the hook only when set: the default empty value would otherwise spawn
    // an empty `cmd /C ` / `sh -c ` on every launch.
    if launch_options.execute_after_launch.trim().is_empty() {
        log::debug!("No after-launch command is configured");
    } else {
        log::info!(
            "Running the after-launch command: {}",
            launch_options.execute_after_launch
        );
        #[cfg(target_os = "windows")]
        let hook = {
            let mut command = Command::new("cmd");
            command
                .args(["/C", &launch_options.execute_after_launch])
                .creation_flags(0x08000000);
            command.spawn()
        };
        #[cfg(not(target_os = "windows"))]
        let hook = Command::new("sh")
            .args(["-c", &launch_options.execute_after_launch])
            .spawn();
        if let Err(error) = &hook {
            log::warn!("The after-launch command could not be started: {error}; it will not run");
        }
    }

    let statistics_profile = match launch_options.selected_account {
        Account::Microsoft(account) => StatisticsProfile::Microsoft(account.profile.uuid),
        Account::Offline(account) => StatisticsProfile::Offline(account.uuid),
        Account::Yggdrasil(account) => StatisticsProfile::Yggdrasil(account.identifier),
    };
    let _ = log_launch(statistics_profile, instance.id).await;
    Ok(pid)
}

#[cfg(test)]
mod tests {
    // Every test here exercises the Unix shell quoting, so on Windows the
    // helpers are unused.
    #[cfg(not(target_os = "windows"))]
    use std::path::PathBuf;

    #[cfg(not(target_os = "windows"))]
    use super::{build_launch_command, quote_shell_arg};

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn an_empty_argument_is_dropped_from_the_launch_command() {
        // An empty `extra_jvm_args` must not become an explicit empty word, or
        // `java` reads it as the (missing) main class.
        let command = build_launch_command(
            &PathBuf::from("/usr/bin/java"),
            &["-Xmx2G".to_string(), String::new(), "-jar".to_string()],
        );
        assert_eq!(command, "exec '/usr/bin/java' '-Xmx2G' '-jar'");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn an_argument_with_spaces_survives_shell_quoting() {
        assert_eq!(
            quote_shell_arg("-Xdock:icon=/a b/minecraft.icns"),
            "'-Xdock:icon=/a b/minecraft.icns'"
        );
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn a_single_quote_is_closed_and_reopened() {
        assert_eq!(quote_shell_arg("a'b"), "'a'\\''b'");
    }
}
