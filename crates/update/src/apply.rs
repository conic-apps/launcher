// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Applying a staged bundle.
//!
//! This runs once, on the way out of the process: the window has closed and
//! nothing will read the running executable again. Every branch replaces the
//! running copy in place (or hands it to the platform's installer) so the next
//! launch starts the new version, which is why the user is never interrupted by
//! an install.

// `Path`/`PathBuf` and `extract_tar_gz` are only reached from the Linux and
// macOS branches below; on Windows the same work is handed to a script, so the
// imports would be unused there.
#[cfg(not(windows))]
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::install::InstallKind;
#[cfg(not(windows))]
use crate::staged::extract_tar_gz;
use crate::staged::{Staged, clear_pending, read_pending, updates_dir};

/// Applies the pending bundle, if any. Returns whether something was applied.
pub fn apply_pending() -> Result<bool> {
    let Some(staged) = read_pending()? else {
        return Ok(false);
    };

    if !staged.artifact.exists() {
        log::warn!(
            "staged update {} is gone; dropping the record",
            staged.artifact.display()
        );
        clear_pending()?;
        return Ok(false);
    }

    log::info!(
        "applying update {} ({})",
        staged.version,
        staged.kind.as_str()
    );
    // The bundle that is about to be swapped in — the file the user would need to
    // put back by hand if anything below goes wrong.
    log::debug!("the staged bundle is at {}", staged.artifact.display());
    match staged.kind {
        InstallKind::AppImage => apply_appimage(&staged)?,
        InstallKind::App => apply_mac_app(&staged)?,
        InstallKind::Portable => apply_portable(&staged)?,
        InstallKind::Msi => apply_msi(&staged)?,
        InstallKind::Nsis => apply_nsis(&staged)?,
    }
    Ok(true)
}

/// Removes the staged bundle now that it has been applied.
///
/// [`clear_pending`] deliberately drops only the record: on Windows the bundle
/// has to outlive this process, because a detached script is what consumes it.
/// On the platforms that apply in-process the artifact is dead weight the moment
/// the swap succeeds, and leaving it behind would let a used `updates/` directory
/// keep one tarball per update.
#[cfg(not(windows))]
fn discard_artifact(staged: &Staged) {
    if let Err(error) = std::fs::remove_file(&staged.artifact)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!(
            "could not remove the applied update {}: {error}",
            staged.artifact.display()
        );
    }
}

/// Replaces `target` with `source` through a scratch file in the same
/// directory, so the swap is a directory-entry rename rather than an in-place
/// overwrite of the file the running process may still hold open.
#[cfg(target_os = "linux")]
fn replace_with(source: &Path, target: &Path) -> Result<()> {
    let mut scratch = target.as_os_str().to_owned();
    scratch.push(".new");
    let scratch = PathBuf::from(scratch);
    let _ = std::fs::remove_file(&scratch);
    std::fs::copy(source, &scratch)?;
    std::fs::rename(&scratch, target)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

fn apply_appimage(staged: &Staged) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // The AppImage runtime exports the image's own path; it is the file the
        // next launch will execute, so it is the one to replace.
        let image = std::env::var_os("APPIMAGE").ok_or(Error::NoCurrentExe)?;
        let image = PathBuf::from(image);
        let mut scratch = image.as_os_str().to_owned();
        scratch.push(".new");
        let scratch = PathBuf::from(scratch);
        let _ = std::fs::remove_file(&scratch);
        std::fs::copy(&staged.artifact, &scratch)?;
        make_executable(&scratch)?;
        std::fs::rename(&scratch, &image)?;
        clear_pending()?;
        discard_artifact(staged);
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = staged;
        Err(Error::Unsupported("an AppImage update on a non-Linux host"))
    }
}

fn apply_mac_app(staged: &Staged) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let exe = std::env::current_exe()?;
        // <bundle>.app/Contents/MacOS/conic-launcher
        let bundle = exe
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or(Error::NoCurrentExe)?
            .to_path_buf();
        if bundle.extension().and_then(|e| e.to_str()) != Some("app") {
            return Err(Error::Unsupported(
                "the running executable is not in a .app",
            ));
        }

        let work = updates_dir().join("mac-extract");
        let _ = std::fs::remove_dir_all(&work);
        extract_tar_gz(&staged.artifact, &work)?;
        let new_app = top_level_app(&work)
            .ok_or(Error::Unsupported("the update archive has no .app bundle"))?;

        // A running bundle may be renamed even though it cannot be replaced in
        // place; moving the old one aside first is what makes the swap atomic.
        let backup = bundle.with_extension("app.old");
        let _ = std::fs::remove_dir_all(&backup);
        std::fs::rename(&bundle, &backup)?;
        if let Err(error) = std::fs::rename(&new_app, &bundle) {
            // Roll back to the running bundle. A failure here is the worst case
            // in this crate: the user is left with *no* launcher, and the only
            // record would otherwise be the app's single `failed to apply` line
            // — which says nothing about where their bundle went.
            log::error!(
                "the new bundle could not be moved into place ({error}); restoring the \
                 previous one from {}",
                backup.display()
            );
            if let Err(rollback) = std::fs::rename(&backup, &bundle) {
                log::error!(
                    "the previous bundle could not be restored from {} either ({rollback}); \
                     the launcher is at {} and must be put back by hand",
                    backup.display(),
                    bundle.display()
                );
            } else {
                log::error!("the previous launcher was restored; the update was not applied");
            }
            return Err(error.into());
        }
        let _ = std::fs::remove_dir_all(&backup);
        let _ = std::fs::remove_dir_all(&work);
        clear_pending()?;
        discard_artifact(staged);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = staged;
        Err(Error::Unsupported("a macOS bundle update on another host"))
    }
}

#[cfg(target_os = "macos")]
fn top_level_app(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|e| e.to_str()) == Some("app"))
}

fn apply_portable(staged: &Staged) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let exe = std::env::current_exe()?;
        let work = updates_dir().join("portable-extract");
        let _ = std::fs::remove_dir_all(&work);
        extract_tar_gz(&staged.artifact, &work)?;
        let new_bin = find_executable(&work)
            .ok_or(Error::Unsupported("the portable archive has no executable"))?;
        replace_with(&new_bin, &exe)?;
        make_executable(&exe)?;
        // The archive may carry the install policy beside the binary; keep it.
        if let Some(parent) = exe.parent()
            && let Some(policy) = find_named(&work, crate::install::POLICY_FILE)
        {
            let _ = std::fs::copy(policy, parent.join(crate::install::POLICY_FILE));
        }
        let _ = std::fs::remove_dir_all(&work);
        clear_pending()?;
        discard_artifact(staged);
        Ok(())
    }
    #[cfg(windows)]
    {
        // Windows will not let a running process replace its own image, so a
        // detached script waits for this process to exit and swaps the file.
        let target = std::env::current_exe()?;
        clear_pending()?;
        let script = format!(
            "@echo off\r\n\
             setlocal\r\n\
             :wait\r\n\
             tasklist /FI \"PID eq {pid}\" 2>NUL | find \"{pid}\" >NUL\r\n\
             if not errorlevel 1 (\r\n\
             \x20 ping -n 2 127.0.0.1 >NUL\r\n\
             \x20 goto wait\r\n\
             )\r\n\
             move /Y \"{new}\" \"{target}\" >NUL\r\n\
             start \"\" \"{target}\"\r\n\
             del \"%~f0\"\r\n",
            pid = std::process::id(),
            new = staged.artifact.display(),
            target = target.display(),
        );
        run_detached_script(&script)?;
        Ok(())
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = staged;
        Err(Error::Unsupported("a portable update on this host"))
    }
}

#[cfg(target_os = "linux")]
fn find_executable(root: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    // The portable archive is expected to carry the launcher at its root; fall
    // back to the first file with the executable bit set.
    find_named(root, "conic-launcher").or_else(|| {
        std::fs::read_dir(root).ok()?.flatten().find_map(|entry| {
            let path = entry.path();
            let mode = entry.metadata().ok()?.permissions().mode();
            (path.is_file() && mode & 0o111 != 0).then_some(path)
        })
    })
}

#[cfg(target_os = "linux")]
fn find_named(root: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.file_name().and_then(|n| n.to_str()) == Some(name))
}

fn apply_msi(staged: &Staged) -> Result<()> {
    #[cfg(windows)]
    {
        // The installer has to run once this process is gone, or Windows
        // Installer cannot replace the executable it is running from.
        clear_pending()?;
        let script = format!(
            "@echo off\r\n\
             setlocal\r\n\
             :wait\r\n\
             tasklist /FI \"PID eq {pid}\" 2>NUL | find \"{pid}\" >NUL\r\n\
             if not errorlevel 1 (\r\n\
             \x20 ping -n 2 127.0.0.1 >NUL\r\n\
             \x20 goto wait\r\n\
             )\r\n\
             start \"\" /WAIT msiexec /i \"{msi}\" /passive /norestart\r\n\
             del \"{msi}\" >NUL 2>NUL\r\n\
             del \"%~f0\"\r\n",
            pid = std::process::id(),
            msi = staged.artifact.display(),
        );
        run_detached_script(&script)?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = staged;
        Err(Error::Unsupported("an MSI update on a non-Windows host"))
    }
}

fn apply_nsis(staged: &Staged) -> Result<()> {
    #[cfg(windows)]
    {
        clear_pending()?;
        let script = format!(
            "@echo off\r\n\
             setlocal\r\n\
             :wait\r\n\
             tasklist /FI \"PID eq {pid}\" 2>NUL | find \"{pid}\" >NUL\r\n\
             if not errorlevel 1 (\r\n\
             \x20 ping -n 2 127.0.0.1 >NUL\r\n\
             \x20 goto wait\r\n\
             )\r\n\
             start \"\" /WAIT \"{setup}\" /S\r\n\
             del \"{setup}\" >NUL 2>NUL\r\n\
             del \"%~f0\"\r\n",
            pid = std::process::id(),
            setup = staged.artifact.display(),
        );
        run_detached_script(&script)?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = staged;
        Err(Error::Unsupported("an NSIS update on a non-Windows host"))
    }
}

#[cfg(windows)]
fn run_detached_script(script: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW | DETACHED_PROCESS: the script outlives this process and
    // must not flash a console.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    let path = updates_dir().join("apply.cmd");
    std::fs::write(&path, script)?;
    // `spawn` succeeding only means the shell started; the script may still fail.
    // Logging the hand-off is what distinguishes "queued and run" from "queued
    // and lost", which were otherwise the same silence.
    log::info!(
        "handing the update to the detached script at {}",
        path.display()
    );
    std::process::Command::new("cmd")
        .arg("/C")
        .arg(&path)
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .spawn()?;
    Ok(())
}
