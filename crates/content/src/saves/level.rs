// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::io::Write;
use std::path::Path;
use std::{collections::HashMap, io::Read};

use fastnbt::Value;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use crate::error::*;
use crate::saves::nbt::modify_nbt;

/// Returns the `level.dat` root compound; the level fields live under its `Data`
/// tag.
pub fn parse_level_data<P: AsRef<Path>>(leveldat_path: P) -> Result<Value> {
    let file = fs::File::open(leveldat_path)?;
    let mut decoder = GzDecoder::new(file);
    let mut bytes = vec![];
    decoder.read_to_end(&mut bytes)?;
    Ok(fastnbt::from_bytes(&bytes)?)
}

pub fn modify_level<P: AsRef<Path>>(level_path: P, value_path: &str, value: Value) -> Result<()> {
    let level_path = level_path.as_ref();
    let file = fs::File::options().read(true).open(level_path)?;
    let mut decoder = GzDecoder::new(file);
    let mut bytes = vec![];
    decoder.read_to_end(&mut bytes)?;
    let leveldat: fastnbt::Value = fastnbt::from_bytes(&bytes)?;

    let modified_laveldat = modify_nbt(leveldat, value_path, value)?;
    // Everything that can fail happens before the save file is touched: the
    // original opened the same path with `truncate(true)`, which empties it as
    // part of the open, so a failure after that point left the world with no
    // level data at all.
    let new_bytes = fastnbt::to_bytes(&modified_laveldat)?;

    // A sibling temporary file, renamed over the original. Same directory, so the
    // rename is atomic and a failed write never destroys what was there.
    let temporary = level_path.with_extension("dat.tmp");
    let mut encoder = GzEncoder::new(fs::File::create(&temporary)?, Compression::fast());
    encoder.write_all(&new_bytes)?;
    encoder.finish()?.sync_all()?;
    fs::rename(&temporary, level_path)?;
    Ok(())
}

pub fn get_all_levels<P: AsRef<Path>>(saves_folder_path: P) -> Result<HashMap<String, Value>> {
    Ok(fs::read_dir(saves_folder_path)?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .flat_map(|entry| {
            let folder_name = entry.file_name().display().to_string();
            let leveldat_path = entry.path().join("level.dat");
            let leveldat = parse_level_data(leveldat_path)?;
            Result::Ok((folder_name, leveldat))
        })
        .collect::<HashMap<_, _>>())
}
