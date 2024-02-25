/*
 * Amethyst Launcher Core
 * Copyright (C) 2023 Broken-Deer <old_driver__@outlook.com> and contributors
 *
 * This program is free software, you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::{ffi::OsStr, fs, io::Read, path::Path};

use anyhow::Result;
use zip::ZipArchive;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PackMetadata {
    pub description: String,
    pub pack_format: u8,
    #[serde(flatten)]
    pub other: Option<serde_json::Value>
}

pub fn get_metadata<S: AsRef<OsStr> + ?Sized>(s: &S) -> Result<PackMetadata> {
    let path = Path::new(s).to_path_buf();
    let metadata = if path.is_dir() {
        fs::read_to_string(path.join("pack.mcmeta"))?
    } else {
        let file = fs::File::open(path)?;
        let mut zip_archive = ZipArchive::new(file)?;
        let mut zip_file = zip_archive.by_name("pack.metadata")?;
        let mut buf = String::new();
        zip_file.read_to_string(&mut buf)?;
        buf
    };
    Ok(serde_json::from_str(&metadata)?)
}

pub fn parse_resourcespack<P: AsRef<Path>>(_path: P) {}
