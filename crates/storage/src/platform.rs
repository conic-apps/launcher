// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Choosing the default root from the platform and the packaging.

use std::path::PathBuf;

use platform::{OsFamily, PLATFORM_INFO};

/// The launcher's folder name inside the platform's base directory.
///
/// macOS names it after the bundle identifier (`app.conicmc.launcher`), which is
/// what [`platform_anchor`] joins the Application Support directory with; the
/// `-debug` suffix keeps a debug build out of a release install's data.
#[cfg(target_os = "macos")]
pub const APP_DIR_NAME: &str = if cfg!(debug_assertions) {
    "app.conicmc.launcher-debug"
} else {
    "app.conicmc.launcher"
};
#[cfg(not(target_os = "macos"))]
pub const APP_DIR_NAME: &str = if cfg!(debug_assertions) {
    "conic-debug"
} else {
    "conic"
};

/// The platform's base directory for application data.
pub fn platform_base() -> PathBuf {
    match PLATFORM_INFO.os_family {
        OsFamily::Windows => {
            PathBuf::from(std::env::var("APPDATA").expect("Could not find APPDATA directory"))
        }
        OsFamily::Macos => PathBuf::from(std::env::var("HOME").expect("Could not find home"))
            .join("Library/Application Support"),
        OsFamily::Linux => std::env::var("XDG_DATA_HOME")
            .ok()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var("HOME").expect("Could not find home"))
                    .join(".local/share")
            }),
    }
}

/// The fixed directory the bootstrap file lives in and the default root is
/// derived from.
pub fn platform_anchor() -> PathBuf {
    platform_base().join(APP_DIR_NAME)
}

/// The root the three locations hang off when the user has not overridden them.
///
/// On Windows the single-file portable build keeps everything next to the
/// executable (`.minecraft-conic`) so it stays self-contained; an MSI install
/// uses the platform anchor. Everywhere else the anchor is the default.
pub fn default_root() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(root) = portable_root() {
            return root;
        }
    }
    platform_anchor()
}

/// The data directory of a portable Windows build, or `None` for an MSI install.
///
/// The MSI records `Installed=1` and `InstallLocation`; the build is installed
/// only when both are present and this executable really is inside the recorded
/// directory. Anything else — no marker, or an executable that was copied out —
/// is the portable build.
#[cfg(target_os = "windows")]
fn portable_root() -> Option<PathBuf> {
    // Each of the three readings can fail or answer differently, and all three
    // collapse into one boolean here. That boolean picks the portable updater over
    // the MSI one, so a registry read that fails is not a cosmetic loss — it is the
    // wrong update path, chosen with nothing to say why.
    match windows_registry::install_location() {
        Some(directory) => match std::env::current_exe() {
            Ok(exe) => {
                let installed = path_is_under(&exe, &directory);
                if !installed {
                    log::debug!(
                        "the MSI marker points at {} but this executable is at {}, so this is \
                         a portable build",
                        directory.display(),
                        exe.display()
                    );
                }
                if installed {
                    return None;
                }
            }
            Err(error) => {
                log::debug!(
                    "the MSI marker points at {} but the running executable is unknown \
                     ({error})",
                    directory.display()
                );
            }
        },
        None => log::debug!("no MSI install marker was found; treating this as a portable build"),
    }
    let exe = std::env::current_exe().ok()?;
    exe.parent()
        .map(|directory| directory.join(".minecraft-conic"))
}

#[cfg(target_os = "windows")]
fn path_is_under(path: &std::path::Path, base: &std::path::Path) -> bool {
    let normalize = |value: &std::path::Path| {
        value
            .to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    };
    let path = normalize(path);
    let base = normalize(base);
    !base.is_empty() && (path == base || path.starts_with(&(base + "\\")))
}

#[cfg(target_os = "windows")]
mod windows_registry {
    use std::path::PathBuf;

    use windows::{
        Win32::{
            Foundation::ERROR_SUCCESS,
            System::Registry::{
                HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, REG_VALUE_TYPE, RegCloseKey,
                RegOpenKeyExW, RegQueryValueExW,
            },
        },
        core::w,
    };

    const KEY_PATH: windows::core::PCWSTR = w!("Software\\ConicMC\\Conic Launcher");

    /// The install directory the MSI recorded, when it installed this app.
    pub(super) fn install_location() -> Option<PathBuf> {
        unsafe {
            let mut key = HKEY::default();
            if RegOpenKeyExW(HKEY_LOCAL_MACHINE, KEY_PATH, None, KEY_READ, &mut key)
                != ERROR_SUCCESS
            {
                return None;
            }
            let location = read_install_location(key);
            let _ = RegCloseKey(key);
            location
        }
    }

    /// # Safety
    /// `key` must be an open registry key.
    unsafe fn read_install_location(key: HKEY) -> Option<PathBuf> {
        if !unsafe { installed_flag(key) }? {
            return None;
        }
        let mut kind = REG_VALUE_TYPE::default();
        let mut size = 0u32;
        let status = unsafe {
            RegQueryValueExW(
                key,
                w!("InstallLocation"),
                None,
                Some(&mut kind),
                None,
                Some(&mut size),
            )
        };
        if status != ERROR_SUCCESS || size == 0 {
            return None;
        }
        // `size` is in bytes; the value is a NUL-terminated UTF-16 string.
        let mut buffer = vec![0u16; (size as usize).div_ceil(2)];
        let status = unsafe {
            RegQueryValueExW(
                key,
                w!("InstallLocation"),
                None,
                Some(&mut kind),
                Some(buffer.as_mut_ptr() as *mut u8),
                Some(&mut size),
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let length = buffer
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(buffer.len());
        let value = String::from_utf16_lossy(&buffer[..length]);
        (!value.is_empty()).then(|| PathBuf::from(value))
    }

    /// # Safety
    /// `key` must be an open registry key.
    unsafe fn installed_flag(key: HKEY) -> Option<bool> {
        let mut kind = REG_VALUE_TYPE::default();
        let mut value = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            RegQueryValueExW(
                key,
                w!("Installed"),
                None,
                Some(&mut kind),
                Some(&mut value as *mut u32 as *mut u8),
                Some(&mut size),
            )
        };
        (status == ERROR_SUCCESS && kind.0 == REG_DWORD.0).then_some(value == 1)
    }
}
