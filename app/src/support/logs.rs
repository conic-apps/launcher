// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The log sink.
//!
//! A debug build writes to stdout, with the level coloured when stdout is a
//! terminal, so a line is readable while the app is being worked on. A release
//! build writes to a file in the data directory's `logs` folder, rotating at
//! 50 kB and keeping ten files, in the plain format earlier installs wrote: a
//! log read later with a text editor must not carry escape codes.
//! `Settings → About → "View launcher logs"` opens that folder, so an app that
//! writes nothing to it opens an empty directory.
//!
//! The file name and the archive naming match what earlier installs wrote, so a
//! folder shared across versions looks the same.
//!
//! `env_logger` has no rotation, so the writer is a `Target::Pipe` around a
//! small one: it appends, counts what it has written, and moves the file aside
//! once it passes the limit. The count is in bytes rather than lines — it is what
//! a 50 kB file limit means.

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use chrono::Local;
use env_logger::fmt::Formatter;
use log::Record;
use storage::LOCATIONS;

/// The active log file's stem. See the module comment.
const FILE_NAME: &str = "conic-launcher";

/// Rotate once the active file passes this many bytes.
const MAX_FILE_SIZE: u64 = 50_000;

/// How many files to keep in the folder, the active one included.
const KEEP: usize = 10;

/// The archive suffix: `[year]-[month]-[day]_[hour]-[minute]-[second]`.
const DATE_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";

/// The size of the active file, or where it is if there is none yet.
struct Rotating {
    path: PathBuf,
    written: u64,
}

impl Rotating {
    /// Opens (or accounts for) the active file, then prunes the folder down to
    /// [`KEEP`]. Pruning on open as well as on rotate keeps a folder that was
    /// copied from another machine from growing.
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
        // already there; it is renamed to `.bak` rather than overwritten.
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
/// modification time, so it survives a folder that was copied around. Only names
/// of the form `<stem>_<timestamp>.log` are touched, so anything else in the
/// folder is left alone.
fn prune_archives(directory: &Path, stem: &str, keep: usize) {
    let Ok(entries) = fs::read_dir(directory) else {
        // Pruning is best-effort and cannot report through the log (it may run
        // before the logger exists), so stderr is the only channel here.
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

/// `env_logger` writes each record through a `Write`, so the rotation is a
/// `Write` of its own rather than a target: every record is a line, and
/// `env_logger` holds the pipe behind a `Mutex`, so `&mut self` is already the
/// only writer.
struct FileLog {
    inner: Rotating,
}

impl FileLog {
    /// Reports the first failure on stderr, and only the first.
    ///
    /// Nothing here can reach `log!` — this *is* what `log!` writes to — and
    /// `env_logger` discards the writer's `io::Error` entirely, so every `?` in
    /// `Rotating` would otherwise vanish without a trace. That makes a "cannot
    /// write the log" bug the one failure this crate could never report, which is
    /// why it goes to stderr, and why it is reported once rather than per record:
    /// a release build whose log file became unwritable would otherwise emit one
    /// line per record for the rest of the session.
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
        // `env_logger` hands a record to `write` in one piece, so this is a whole
        // line by the time it gets here. It is written as one string rather than
        // delegated, because the size counted against the rotation limit has to
        // be the whole line: `write_all` would come back here in chunks and
        // could rotate between the halves of a message.
        let line = String::from_utf8_lossy(buf);
        if let Err(error) = self.inner.append(&line) {
            let path = self.inner.path.clone();
            self.report_once(&error, &path);
            return Err(error);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // `Rotating::append` opens and closes the file per record, so there is
        // nothing buffered here to push out.
        Ok(())
    }
}

/// `[date][time][target][LEVEL] message` — the format earlier installs wrote, so
/// lines read the same in a shared file.
///
/// `env_logger` formats every sink with one formatter, so this is also what
/// stdout prints. One format beats a per-sink pair.
fn format_record(buffer: &mut Formatter, record: &Record) -> std::io::Result<()> {
    let now = Local::now();
    // The level carries the colour. `default_level_style` is already empty for
    // `WriteStyle::Never`, which the file and a redirected stdout both use, so
    // the same format serves a terminal and a log file: no bracket needs to know
    // which one it is going to.
    let style = buffer.default_level_style(record.level());
    // `Formatter` is an `io::Write`, not a `fmt::Write`, so this is
    // `write_fmt` and not the `write!` macro. `{style:#}` writes the reset that
    // closes the level's colour; both sides of a `Style` render as nothing when
    // the style is empty.
    //
    // The trailing newline is this function's to write: `env_logger`'s own
    // default formatter is what normally ends a record, and this replaces it.
    buffer.write_fmt(format_args!(
        "{} - {style}{:<5}{style:#} / {}: {}\n",
        now.format("%d/%m/%Y %H:%M:%S"),
        record.level(),
        record.target(),
        record.args()
    ))
}

/// Installs the logger, choosing the sink by build: stdout for a debug build,
/// the rotating file for a release build.
///
/// The level is `info`, and `RUST_LOG` overrides it. That default goes through
/// `default_filter_or("info")` rather than a `builder.filter_level(Info)`: the
/// latter is `self.filter = Some(level)`, applied *after* `RUST_LOG` is parsed,
/// so it discards whatever `RUST_LOG` asked for and `RUST_LOG=debug` produced
/// nothing at all — which is how a hook that never runs stays invisible, since
/// the `debug!` lines are the ones that would have named it. The `debug!` lines
/// this app does emit are about the window chrome and the background, and a
/// launch log that has to be read by hand is a launch log that is not.
pub fn init() {
    // `cfg!` rather than two `#[cfg]`-gated statements: it keeps both arms
    // compiled, so the release-only file machinery is still checked by a debug
    // `cargo check` instead of quietly rotting.
    if cfg!(debug_assertions) {
        init_stdout();
    } else {
        init_file();
    }
}

/// A debug build's logger: stdout, with the level coloured when stdout is a
/// terminal.
fn init_stdout() {
    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug"));
    builder.format(format_record);
    // `Auto` colours a terminal and leaves a redirected stdout plain, which is
    // what both a pipe and `NO_COLOR` expect.
    builder.write_style(env_logger::WriteStyle::Auto);
    builder.target(env_logger::Target::Stdout);
    init_or_report(builder, "stdout");
}

/// Installs `builder`, or says on stderr that it could not.
///
/// `try_init` fails when a logger is already installed — including a second call
/// to [`init`], which is `pub` and unguarded — and the discarded `Result` made
/// that indistinguishable from success, so a run whose log went nowhere looked
/// exactly like one that worked.
fn init_or_report(mut builder: env_logger::Builder, sink: &str) {
    match builder.try_init() {
        Ok(()) => println!("logging to {sink}"),
        Err(error) => eprintln!("the logger could not be installed for {sink}: {error}"),
    }
}

/// A release build's logger: the log file, with stderr as the fallback when the
/// file cannot be opened.
fn init_file() {
    let directory = LOCATIONS.launcher.logs.clone();
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

    let mut builder =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    builder.format(format_record);
    // A log file is read with a text editor, not a terminal that can render
    // colour. The escape codes would be the first thing in the file nobody
    // wants.
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
