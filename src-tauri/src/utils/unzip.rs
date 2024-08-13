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

use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use anyhow::Result;
use tokio::fs::create_dir_all;
use zip::{read::ZipFile, CompressionMethod, DateTime, ZipArchive};

#[derive(Debug, Clone)]
pub struct Entry {
    pub version_name_by: (u8, u8),
    pub name: String,
    pub mangled_name: PathBuf,
    pub enclosed_name: Option<PathBuf>,
    pub comment: String,
    pub compression: CompressionMethod,
    pub compressed_size: u64,
    pub size: u64,
    pub last_modified: Option<DateTime>,
    pub r#type: EntryType,
    pub unix_mode: Option<u32>,
    pub crc32: u32,
    pub extra_data: Option<Vec<u8>>,
    pub data_start: u64,
    pub header_start: u64,
    pub central_header_start: u64,
    pub content: Vec<u8>,
}

impl Entry {
    pub fn from_zip_file(zip_file: &mut ZipFile<'_>) -> Entry {
        Self {
            version_name_by: zip_file.version_made_by(),
            name: zip_file.name().to_string(),
            mangled_name: zip_file.mangled_name().to_path_buf(),
            enclosed_name: match zip_file.enclosed_name() {
                None => None,
                Some(value) => Some(value.to_path_buf()),
            },
            comment: zip_file.comment().to_string(),
            compression: zip_file.compression(),
            compressed_size: zip_file.compressed_size(),
            size: zip_file.size(),
            last_modified: zip_file.last_modified(),
            r#type: {
                if zip_file.is_dir() {
                    EntryType::Dir
                } else {
                    EntryType::File
                }
            },
            unix_mode: zip_file.unix_mode(),
            crc32: zip_file.crc32(),
            extra_data: match zip_file.extra_data() {
                None => None,
                Some(x) => Some(x.to_vec()),
            },
            data_start: zip_file.data_start(),
            header_start: zip_file.header_start(),
            central_header_start: zip_file.central_header_start(),
            content: {
                let mut buf = Vec::new();
                zip_file.read_to_end(&mut buf).unwrap();
                buf
            },
        }
    }

    pub fn from_zip_archive<R: Read + io::Seek>(zip: &mut ZipArchive<R>) -> Vec<Entry> {
        let mut entries = Vec::with_capacity(zip.len());
        for i in 0..zip.len() {
            entries.push(Entry::from_zip_file(&mut zip.by_index(i).unwrap()));
        }
        entries
    }

    pub fn get_entries_record(entries: Vec<Entry>) -> HashMap<String, Entry> {
        let mut resolved_entries = HashMap::with_capacity(entries.len());
        for entry in entries {
            resolved_entries.insert(entry.name.clone(), entry);
        }
        resolved_entries
    }
}

#[derive(Debug, Clone)]
pub enum EntryType {
    Dir,
    File,
}

pub fn open(path: PathBuf) -> ZipArchive<File> {
    let file = File::open(path).unwrap();
    ZipArchive::new(file).unwrap()
}

pub fn filter_entries<R: Read + io::Seek>(
    zip: &mut ZipArchive<R>,
    entries: &Vec<String>,
) -> HashMap<String, Entry> {
    let mut resolved_entries = HashMap::with_capacity(entries.len());
    for i in 0..zip.len() {
        let mut zip_file = zip.by_index(i).unwrap();
        let name = zip_file.name().to_string();
        for entry in entries {
            let entry = entry.to_string();
            if name == entry {
                resolved_entries.insert(entry, Entry::from_zip_file(&mut zip_file));
            }
        }
    }
    resolved_entries
}

pub async fn decompression_file(zip_archive: &mut ZipArchive<File>, from: String, to: PathBuf) {
    let mut buf: Vec<u8> = Vec::new();
    zip_archive
        .by_name(&from)
        .unwrap()
        .read_to_end(&mut buf)
        .unwrap();
    tokio::fs::write(to, buf).await.unwrap();
}

pub async fn decompression_files<R: Read + io::Seek>(
    zip_archive: &mut ZipArchive<R>,
    tasks: Vec<(String, PathBuf)>,
) {
    // todo: 在线程池读取，并发写入
    for task in tasks {
        let mut buf: Vec<u8> = Vec::new();
        zip_archive
            .by_name(&task.0)
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
        create_dir_all(task.1.parent().unwrap()).await.unwrap();
        tokio::fs::write(task.1, buf).await.unwrap();
    }
}

pub fn decompression_all<R: Read + io::Seek, S: AsRef<OsStr> + ?Sized>(
    zip_archive: &mut ZipArchive<R>,
    to: &S,
) -> Result<()> {
    let to = Path::new(to).to_path_buf();
    for i in 0..zip_archive.len() {
        let mut zip_file = zip_archive.by_index(i).unwrap();
        let name = zip_file.name().to_string();
        let entry = Entry::from_zip_file(&mut zip_file);
        let path = to.join(&name);
        if zip_file.is_dir() {
            std::fs::create_dir_all(zip_file.name()).unwrap();
            continue;
        }
        std::fs::create_dir_all(
            path.parent()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?,
        )?;
        std::fs::write(path, entry.content).unwrap();
        // for entry in entries {
        //     let entry = entry.to_string();
        //     if name == entry {
        //         resolved_entries.insert(entry, Entry::from_zip_file(&mut zip_file));
        //     }
        // }
    }
    Ok(())
}
