// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The app's log sink: stdout in a debug build, a rotating file (50 kB, ten
//! files, under `<data>/logs`) in a release build, and the newest records kept in
//! memory for the crash report.

use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex, PoisonError},
};

use chrono::Local;
use env_logger::fmt::Formatter;
use log::Record;
use storage::LOCATIONS;

const FILE_NAME: &str = "conic-launcher";

const MAX_FILE_SIZE: u64 = 50_000;

pub(crate) fn log_path(directory: &Path) -> PathBuf {
    directory.join(format!("{FILE_NAME}.log"))
}

/// Records the crash report embeds.
const RECENT_LINES: usize = 1_000;

static RECENT: LazyLock<Mutex<VecDeque<String>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(RECENT_LINES)));

/// Files kept in the folder, the active one included.
const KEEP: usize = 10;

/// The archive suffix: `[year]-[month]-[day]_[hour]-[minute]-[second]`.
const DATE_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";

struct Rotating {
    path: PathBuf,
    written: u64,
}

impl Rotating {
    fn open(directory: &Path) -> std::io::Result<Self> {
        let stem = FILE_NAME.to_string();
        prune_archives(directory, &stem, KEEP.saturating_sub(1));
        let path = directory.join(format!("{stem}.log"));
        // Append to an existing file so a crash does not discard the run before it.
        let written = fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        Ok(Self { path, written })
    }

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
        // Two rotations in one second would collide; move the old one aside.
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
        prune_archives(directory, &stem, KEEP.saturating_sub(2));
        Ok(())
    }
}

/// Deletes all but the newest `keep` of `<stem>_<timestamp>.log`; the timestamp
/// is the one in the name, so a copied folder keeps its order.
fn prune_archives(directory: &Path, stem: &str, keep: usize) {
    let Ok(entries) = fs::read_dir(directory) else {
        // This may run before the logger exists, so stderr is the only channel.
        eprintln!(
            "the log folder at {} could not be read; it will not be pruned",
            directory.display()
        );
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

struct FileLog {
    inner: Rotating,
}

impl FileLog {
    /// Reports a write failure on stderr, once per process.
    fn report_once(&self, error: &io::Error, path: &std::path::Path) {
        static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !REPORTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "the log file at {} could not be written ({error}); further failures are \
                 not reported",
                path.display()
            );
        }
    }
}

impl io::Write for FileLog {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // One record, so rotate between records rather than inside one.
        let line = String::from_utf8_lossy(buf);
        if let Err(error) = self.inner.append(&line) {
            let path = self.inner.path.clone();
            self.report_once(&error, &path);
            return Err(error);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `<date> - <LEVEL> / <target>: message`, the line every sink writes.
fn format_record(buffer: &mut Formatter, record: &Record) -> std::io::Result<()> {
    let now = Local::now();
    let style = buffer.default_level_style(record.level());
    let plain = format!(
        "{} - {:<5} / {}: {}\n",
        now.format("%d/%m/%Y %H:%M:%S"),
        record.level(),
        record.target(),
        record.args()
    );
    push_recent(&plain);
    // `{style:#}` writes the reset; the newline is ours because this replaces
    // `env_logger`'s formatter.
    buffer.write_fmt(format_args!(
        "{} - {style}{:<5}{style:#} / {}: {}\n",
        now.format("%d/%m/%Y %H:%M:%S"),
        record.level(),
        record.target(),
        record.args()
    ))
}

fn push_recent(line: &str) {
    let mut recent = RECENT.lock().unwrap_or_else(PoisonError::into_inner);
    if recent.len() == RECENT_LINES {
        recent.pop_front();
    }
    recent.push_back(line.to_string());
}

/// The newest `lines` records, oldest first, without trailing newlines.
pub(crate) fn recent(lines: usize) -> String {
    let recent = RECENT.lock().unwrap_or_else(PoisonError::into_inner);
    let start = recent.len().saturating_sub(lines);
    recent
        .iter()
        .skip(start)
        .map(|line| line.trim_end_matches('\n'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Installs the logger. `RUST_LOG` overrides the default level.
pub fn init() {
    // `cfg!` keeps both arms compiled, so the release file path is still checked.
    if cfg!(debug_assertions) {
        init_stdout();
    } else {
        init_file();
    }
}

fn init_stdout() {
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"));
    builder.format(format_record);
    builder.write_style(env_logger::WriteStyle::Auto);
    builder.target(env_logger::Target::Stdout);
    init_or_report(builder, "stdout");
}

/// Installs `builder`, or says on stderr that it could not.
fn init_or_report(mut builder: env_logger::Builder, sink: &str) {
    match builder.try_init() {
        Ok(()) => println!("logging to {sink}"),
        Err(error) => eprintln!("the logger could not be installed for {sink}: {error}"),
    }
}

fn init_file() {
    let directory = LOCATIONS.launcher.logs.clone();
    let rotating = match fs::create_dir_all(&directory).and_then(|()| Rotating::open(&directory)) {
        Ok(rotating) => rotating,
        Err(error) => {
            eprintln!(
                "could not open the log file in {}: {error}",
                directory.display()
            );
            init_stderr();
            return;
        }
    };

    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    builder.format(format_record);
    // A log file is read with a text editor, so never write colour escapes.
    builder.write_style(env_logger::WriteStyle::Never);
    builder.target(env_logger::Target::Pipe(Box::new(FileLog {
        inner: rotating,
    })));
    init_or_report(builder, &directory.display().to_string());
}

fn init_stderr() {
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    builder.format(format_record);
    builder.write_style(env_logger::WriteStyle::Never);
    builder.target(env_logger::Target::Stderr);
    init_or_report(builder, "stderr");
}
