#[derive(Copy, Clone)]
pub(crate) struct LibraryInfo {
    pub(crate) filename: &'static str,
    pub(crate) sha256: &'static str,
    pub(crate) size: u64,
    pub(crate) sources: &'static [&'static str],
}

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "conic_terracotta_aarch64-pc-windows-msvc.dll",
    sha256: "d15677fbb03c10902c8a185619fdfc2f9e09303f43d0f2b0a3b5bd7daa42b46c",
    size: 6833152,
    sources: &[
        "https://github.com/conic-apps/conic-terracotta/releases/download/v0.1.0-7ed121a/conic_terracotta_aarch64-pc-windows-msvc.dll",
    ],
};

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "conic_terracotta_x86_64-pc-windows-msvc.dll",
    sha256: "1fe1090fb309058ba03d386eb22eb53022ac6df87747a8870ccc797270274f83",
    size: 8427008,
    sources: &[
        "https://github.com/conic-apps/conic-terracotta/releases/download/v0.1.0-7ed121a/conic_terracotta_x86_64-pc-windows-msvc.dll",
    ],
};

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub(crate) static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libconic_terracotta_aarch64-unknown-linux-gnu.so",
    sha256: "592a40f832bf2f422812f1bbde0f7fc338f8be9d1c9092ad5d1d71c6913b9329",
    size: 9446040,
    sources: &[
        "https://github.com/conic-apps/conic-terracotta/releases/download/v0.1.0-7ed121a/libconic_terracotta_aarch64-unknown-linux-gnu.so",
    ],
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libconic_terracotta_x86_64-unknown-linux-gnu.so",
    sha256: "8e9c8f9fe244554a5e1c5ab2512ccc7f3efc4c9b489a913ea82e3fe9d8c264c0",
    size: 11885736,
    sources: &[
        "https://github.com/conic-apps/conic-terracotta/releases/download/v0.1.0-7ed121a/libconic_terracotta_x86_64-unknown-linux-gnu.so",
    ],
};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libconic_terracotta_aarch64-apple-darwin.dylib",
    sha256: "36940f5a723676d13f882d78ca22347f742fa14f6c4a50fd777cd518e8d2ac94",
    size: 6703120,
    sources: &[
        "https://github.com/conic-apps/conic-terracotta/releases/download/v0.1.0-7ed121a/libconic_terracotta_aarch64-apple-darwin.dylib",
    ],
};

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub static LIBRARY: LibraryInfo = LibraryInfo {
    filename: "libconic_terracotta_x86_64-apple-darwin.dylib",
    sha256: "dd5621df06fceee575733d0f51b33c88e4369c7df1319862e83cc7d4ca436a80",
    size: 8754936,
    sources: &[
        "https://github.com/conic-apps/conic-terracotta/releases/download/v0.1.0-7ed121a/libconic_terracotta_x86_64-apple-darwin.dylib",
    ],
};
