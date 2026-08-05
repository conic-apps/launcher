// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

#[derive(Copy, Clone)]
pub(crate) struct LibraryInfo {
    pub(crate) filename: &'static str,
    pub(crate) sha256: &'static str,
    pub(crate) size: u64,
    pub(crate) sources: &'static [&'static str],
}

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "conic_nexus_aarch64-pc-windows-msvc.dll",
    sha256: "ee5cb29167a01a4ce602cac228d35f113627e9146fec2082fb1171f05cb63718",
    size: 6878208,
    sources: &[
        "https://github.com/conic-apps/conic-nexus/releases/download/v0.1.0-dcfd183/conic_nexus_aarch64-pc-windows-msvc.dll",
    ],
};

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "conic_nexus_x86_64-pc-windows-msvc.dll",
    sha256: "7aad7fdf674ceb0a6f18378150bd74eda21dbe72ca601edbabbd66dd2128e219",
    size: 8448512,
    sources: &[
        "https://github.com/conic-apps/conic-nexus/releases/download/v0.1.0-dcfd183/conic_nexus_x86_64-pc-windows-msvc.dll",
    ],
};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libconic_nexus_aarch64-unknown-linux-gnu.so",
    sha256: "4dfbf1ed58988e9839e9bb3c3d8d8318b49738a01aa92664b7510278e978279e",
    size: 9552592,
    sources: &[
        "https://github.com/conic-apps/conic-nexus/releases/download/v0.1.0-dcfd183/libconic_nexus_aarch64-unknown-linux-gnu.so",
    ],
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libconic_nexus_x86_64-unknown-linux-gnu.so",
    sha256: "7b3e4f541a514169553180804d2bb39ea5e625a0474fab2399e926058bd5a34e",
    size: 12006064,
    sources: &[
        "https://github.com/conic-apps/conic-nexus/releases/download/v0.1.0-dcfd183/libconic_nexus_x86_64-unknown-linux-gnu.so",
    ],
};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libconic_nexus_aarch64-apple-darwin.dylib",
    sha256: "bea4e0b15b504739172249388f348558a26bb20c83730ef6a41d2abe953365b1",
    size: 6802944,
    sources: &[
        "https://github.com/conic-apps/conic-nexus/releases/download/v0.1.0-dcfd183/libconic_nexus_aarch64-apple-darwin.dylib",
    ],
};

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libconic_nexus_x86_64-apple-darwin.dylib",
    sha256: "2140135b4977445ead085f5ede0a24954e354c0a868249ce2a8179ec777fbce1",
    size: 8849728,
    sources: &[
        "https://github.com/conic-apps/conic-nexus/releases/download/v0.1.0-dcfd183/libconic_nexus_x86_64-apple-darwin.dylib",
    ],
};
