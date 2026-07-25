#[derive(Copy, Clone)]
pub struct LibraryInfo {
    pub filename: &'static str,
    pub sha256: &'static str,
    pub sources: &'static [&'static str],
}

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "beat_this_ffi_aarch64-pc-windows-msvc.dll",
    sha256: "e171b17607704c7d1f0756d2778a13a109626ac5eef8730c86c6d26c19d21013",
    sources: &[
        "https://github.com/conic-apps/beat-this-ffi/releases/download/v0.1.0-0802533/beat_this_ffi_aarch64-pc-windows-msvc.dll",
    ],
};

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "beat_this_ffi_x86_64-pc-windows-msvc.dll",
    sha256: "97437668303a7f7896526ffc4bd1880fd4f9976b53e9a0478b574a5b40b01756",
    sources: &[
        "https://github.com/conic-apps/beat-this-ffi/releases/download/v0.1.0-0802533/beat_this_ffi_x86_64-pc-windows-msvc.dll",
    ],
};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libbeat_this_ffi_aarch64-unknown-linux-gnu.so",
    sha256: "0b9820122c54594ea0e59b61d2c3461a1561d6764ef431254fdc130da73d0283",
    sources: &[
        "https://github.com/conic-apps/beat-this-ffi/releases/download/v0.1.0-0802533/libbeat_this_ffi_aarch64-unknown-linux-gnu.so",
    ],
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libbeat_this_ffi_x86_64-unknown-linux-gnu.so",
    sha256: "8be896db713acc56ccb1b724951241535d741cdd81fcc49c6e357c256a7a8304",
    sources: &[
        "https://github.com/conic-apps/beat-this-ffi/releases/download/v0.1.0-0802533/libbeat_this_ffi_x86_64-unknown-linux-gnu.so",
    ],
};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libbeat_this_ffi_aarch64-apple-darwin.dylib",
    sha256: "f53445c00d3fcb7aca3918355a59ddf1600e341932e301b5f6ff2cf15ee22df3",
    sources: &[
        "https://github.com/conic-apps/beat-this-ffi/releases/download/v0.1.0-0802533/libbeat_this_ffi_aarch64-apple-darwin.dylib",
    ],
};

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libbeat_this_ffi_x86_64-apple-darwin.dylib",
    sha256: "12c986ca1fc4d5aae9a591e63d465a1f1bb6731a0a3b1e5451a23f6b190563a3",
    sources: &[
        "https://github.com/conic-apps/beat-this-ffi/releases/download/v0.1.0-0802533/libbeat_this_ffi_x86_64-apple-darwin.dylib",
    ],
};
