// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The log file (`src/log.ts` + `tauri-plugin-log`'s folder target).
//!
//! The Tauri app wrote to three places: stdout, the webview console, and a file
//! in the data directory's `logs` folder, rotating at 50 kB and keeping ten
//! files. `Settings → About → "View launcher logs"` opens that folder, so a
//! native app that writes nothing to it opens an empty directory.
//!
//! The webview's half has no counterpart and needs none: everything the frontend
//! used to print is now printed by the Rust that replaced it, and this module is
//! what puts those lines on disk.
//!
//! The file name and the archive naming are the plugin's own, so the folder looks
//! the same whichever frontend wrote to it. It is not this crate's name: the
//! plugin derives it from the Tauri app's, and the two frontends are meant to be
//! running against the same `~/.conic[-debug]`.
//!
//! `env_logger` has no rotation, so the writer is a `Target::Pipe` around a
//! small one: it appends, counts what it has written, and moves the file aside
//! once it passes the limit. The count is in bytes rather than lines for the same
//! reason the plugin counts bytes — it is what a 50 kB file limit means.

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use chrono::Local;
use env_logger::fmt::Formatter;
use log::Record;
use slint_folder::DATA_LOCATION;

/// The active log file's stem. See the module comment for why it is not the
/// crate's name.
const FILE_NAME: &str = "conic-launcher";

/// Rotate once the active file passes this many bytes
/// (`max_file_size(50_000)`).
const MAX_FILE_SIZE: u64 = 50_000;

/// How many files to keep in the folder, the active one included
/// (`RotationStrategy::KeepSome(10)`).
const KEEP: usize = 10;

/// The archive suffix the plugin writes:
/// `[year]-[month]-[day]_[hour]-[minute]-[second]`.
const DATE_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";

/// The size of the active file, or where it is if there is none yet.
struct Rotating {
    path: PathBuf,
    written: u64,
}

impl Rotating {
    /// Opens (or accounts for) the active file, then prunes the folder down to
    /// [`KEEP`]. The plugin prunes on open as well as on rotate, which is what
    /// keeps a folder that was copied from another machine from growing.
    fn open(directory: &Path) -> std::io::Result<Self> {
        let stem = FILE_NAME.to_string();
        prune_archives(directory, &stem, KEEP.saturating_sub(1));
        let path = directory.join(format!("{stem}.log"));
        // An existing file is *appended* to, so a crash does not throw away the
        // run that led up to it. Its length is the starting count.
        let written = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        Ok(Self { path, written })
    }

    /// Appends one already-formatted line, rotating first if the file is full.
    fn append(&mut self, line: &str) -> std::io::Result<()> {
        if self.written >= MAX_FILE_SIZE {
            self.rotate()?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        self.written += line.len() as u64;
        Ok(())
    }

    /// Moves the active file aside under a timestamped name and prunes the
    /// archives down to what is left of the budget once the new file is counted.
    fn rotate(&mut self) -> std::io::Result<()> {
        let directory = self
            .path
            .parent()
            .ok_or_else(|| std::io::Error::other("the log file has no parent directory"))?;
        let stem = self
            .path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| FILE_NAME.to_string());

        let to = directory.join(format!("{stem}_{}.log", Local::now().format(DATE_FORMAT)));
        // Two rotations inside the same second would land on a file that is
        // already there; the plugin renames that one to `.bak` rather than
        // overwriting it, and so does this.
        if to.exists() {
            let mut to_bak = to.clone();
            to_bak.set_file_name(format!(
                "{}.bak",
                to_bak.file_name().unwrap_or_default().to_string_lossy()
            ));
            fs::rename(&to, &to_bak)?;
        }
        fs::rename(&self.path, &to)?;
        self.written = 0;
        // One slot of the budget has just been taken by the file that was moved.
        prune_archives(directory, &stem, KEEP.saturating_sub(2));
        Ok(())
    }
}

/// Removes the oldest archives until `keep` of them are left.
///
/// The sort is on the timestamp the name carries rather than on the file's
/// modification time, which is the plugin's ordering and survives a folder that
/// was copied around. Only names of the form `<stem>_<timestamp>.log` are
/// touched, so anything else in the folder is left alone.
fn prune_archives(directory: &Path, stem: &str, keep: usize) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut archives: Vec<(PathBuf, String)> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with(stem) || name == format!("{stem}.log") {
                return None;
            }
            let date = name
                .strip_prefix(stem)?
                .strip_prefix('_')?
                .strip_suffix(".log")?
                .to_string();
            Some((entry.path(), date))
        })
        .collect();
    archives.sort_by(|a, b| a.1.cmp(&b.1));
    let excess = archives.len().saturating_sub(keep);
    for (path, _) in archives.into_iter().take(excess) {
        let _ = fs::remove_file(path);
    }
}

/// `env_logger` writes each record through a `Write`, so the rotation is a
/// `Write` of its own rather than a target: every record is a line, and
/// `env_logger` holds the pipe behind a `Mutex`, so `&mut self` is already the
/// only writer.
struct FileLog {
    inner: Rotating,
}

impl io::Write for FileLog {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // `env_logger` hands a record to `write` in one piece, so this is a whole
        // line by the time it gets here. It is written as one string rather than
        // delegated, because the size counted against the rotation limit has to
        // be the whole line: `write_all` would come back here in chunks and
        // could rotate between the halves of a message.
        let line = String::from_utf8_lossy(buf);
        self.inner.append(&line)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // `Rotating::write` opens and closes the file per record, so there is
        // nothing buffered here to push out.
        Ok(())
    }
}

/// `[date][time][target][LEVEL] message` — the plugin's own line
/// (`tauri-plugin-log`'s `Builder::default`), so the two frontends' lines read the
/// same in a file they share.
///
/// `env_logger` formats every target with one formatter, so this is also what
/// stderr prints now. It is the shape a log file wants in either place, and one
/// format beats a per-target pair.
fn format_record(buffer: &mut Formatter, record: &Record) -> std::io::Result<()> {
    let now = Local::now();
    // `Formatter` is an `io::Write`, not a `fmt::Write`, so this is
    // `write_fmt` and not the `write!` macro.
    //
    // The plugin writes its brackets one part at a time so a style can be applied
    // to each; there is nothing to colour here, so they are written plain.
    //
    // The trailing newline is this function's to write: `env_logger`'s own
    // default formatter is what normally ends a record, and this replaces it.
    buffer.write_fmt(format_args!(
        "[{}][{}][{}][{}] {}\n",
        now.format("%Y-%m-%d"),
        now.format("%H:%M:%S"),
        record.target(),
        record.level(),
        record.args()
    ))
}

/// Installs the logger: stderr as before, plus the file.
///
/// The level is the same `LevelFilter::Info` the stderr logger had, and
/// `RUST_LOG` still overrides it (`from_default_env`). The Tauri app ran at
/// `Debug`; the `debug!` lines this app does emit are about the window chrome
/// and the background, and a launch log that has to be read by hand is a launch
/// log that is not.
pub fn init() {
    let directory = DATA_LOCATION.logs.clone();
    let rotating = match fs::create_dir_all(&directory).and_then(|()| Rotating::open(&directory)) {
        Ok(rotating) => rotating,
        Err(error) => {
            // Nothing to log to but stderr, which the logger below still gets.
            eprintln!(
                "could not open the log file in {}: {error}",
                directory.display()
            );
            init_stderr();
            return;
        }
    };

    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(log::LevelFilter::Info);
    builder.format(format_record);
    // A log file is read with a text editor, not a terminal that can render
    // colour, and the plugin's file target was plain in a release build. The
    // escape codes would be the first thing in the file nobody wants.
    builder.write_style(env_logger::WriteStyle::Never);
    builder.target(env_logger::Target::Stderr);
    builder.target(env_logger::Target::Pipe(Box::new(FileLog {
        inner: rotating,
    })));
    let _ = builder.try_init();
}

fn init_stderr() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(log::LevelFilter::Info);
    builder.format(format_record);
    builder.target(env_logger::Target::Stderr);
    let _ = builder.try_init();
}
