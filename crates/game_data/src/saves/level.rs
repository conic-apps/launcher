// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
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

/// Get level data
///
/// Note: This function will return the `Data` tag in `level.dat`
pub fn parse_level_data<P: AsRef<Path>>(leveldat_path: P) -> Result<Value> {
    let file = fs::File::open(leveldat_path)?;
    let mut decoder = GzDecoder::new(file);
    let mut bytes = vec![];
    decoder.read_to_end(&mut bytes)?;
    Ok(fastnbt::from_bytes(&bytes)?)
}

/// Modify level
///
/// * `value_path` - You need to use a colon to connect the path. For example, if you want to modify the
///   seed, you should use `Data:world_gen_settings:seed`.
pub fn modify_level<P: AsRef<Path>>(level_path: P, value_path: &str, value: Value) -> Result<()> {
    let file = fs::File::options().read(true).open(&level_path)?;
    let mut decoder = GzDecoder::new(file);
    let mut bytes = vec![];
    decoder.read_to_end(&mut bytes)?;
    let leveldat: fastnbt::Value = fastnbt::from_bytes(&bytes)?;

    let modified_laveldat = modify_nbt(leveldat, value_path, value)?;

    let out_file = fs::File::options()
        .write(true)
        .truncate(true)
        .open(level_path)
        .unwrap();
    let new_bytes = fastnbt::to_bytes(&modified_laveldat)?;
    let mut encoder = GzEncoder::new(out_file, Compression::fast());
    encoder.write_all(&new_bytes)?;
    Ok(())
}

/// Get all levels from 'saves' folder
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
