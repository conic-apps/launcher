// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::path::PathBuf;

use base64::{Engine, engine::general_purpose};
use fastnbt::Value;
use storage::LOCATIONS;

use crate::error::*;

pub mod datapack;
pub mod level;
mod nbt;

/// The fields a save's card shows, read out of its `level.dat` root.
///
/// Pulling the handful of fields out here keeps the NBT shape inside this crate
/// and gives the app a plain, `Send` summary to carry across threads.
///
/// The spawn position is a second kind of field: no card shows it, but the
/// world map opens centred on it.
#[derive(Debug, Clone, Default)]
pub struct LevelSummary {
    /// The world's display name, absent when `level.dat` does not carry one.
    pub name: Option<String>,
    /// `Data.GameType`: 0 survival, 1 creative, 2 adventure, 3 spectator.
    pub game_type: Option<i32>,
    pub allow_commands: bool,
    /// `Data.LastPlayed`, in milliseconds since the epoch.
    pub last_played: Option<u64>,
    /// `Data.spawn.pos` — the world's spawn point, `[x, y, z]` in blocks.
    pub spawn: Option<[i32; 3]>,
}

/// Reads a [`LevelSummary`] out of a `level.dat` root (`level::get_all_levels`).
pub fn summarize_level(root: &Value) -> LevelSummary {
    let Some(data) = compound_field(root, "Data") else {
        return LevelSummary::default();
    };
    LevelSummary {
        name: match compound_field(data, "LevelName") {
            Some(Value::String(name)) => Some(name.clone()),
            _ => None,
        },
        game_type: integer_field(data, "GameType").map(|value| value as i32),
        allow_commands: integer_field(data, "allowCommands").is_some_and(|value| value != 0),
        // The launch script stores milliseconds, the unit the frontend's date
        // formatting expects.
        last_played: integer_field(data, "LastPlayed").map(|value| value as u64),
        spawn: spawn_field(data),
    }
}

/// `Data.spawn.pos`, the `ListTag` of three ints the Java writes as
/// `[x, y, z]`. A world that predates the tag (or a `level.dat` that carries
/// `SpawnX`/`SpawnZ` instead, which the format did before 1.2) has none.
/// `worldmap.rs` falls back to the spawn `conic-worldmap` reads for itself only
/// when a render request carries no centre.
fn spawn_field(data: &Value) -> Option<[i32; 3]> {
    let pos = compound_field(compound_field(data, "spawn")?, "pos")?;
    let coords: &[i32] = match pos {
        Value::IntArray(values) => values,
        _ => return None,
    };
    let [x, y, z] = coords else {
        return None;
    };
    Some([*x, *y, *z])
}

fn compound_field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    match value {
        Value::Compound(fields) => fields.get(name),
        _ => None,
    }
}

/// The integer tags, as `i64`. NBT writes these fields as whichever width fits,
/// so all four are accepted.
fn integer_field(value: &Value, name: &str) -> Option<i64> {
    match compound_field(value, name)? {
        Value::Byte(value) => Some(i64::from(*value)),
        Value::Short(value) => Some(i64::from(*value)),
        Value::Int(value) => Some(i64::from(*value)),
        Value::Long(value) => Some(*value),
        _ => None,
    }
}

fn save_folder(instance_id: &str, folder_name: &str) -> PathBuf {
    LOCATIONS
        .instances
        .get_instance_root(instance_id)
        .join("saves")
        .join(folder_name)
}

pub fn get_all_levels(instance_id: &str) -> Result<HashMap<String, Value>> {
    level::get_all_levels(
        LOCATIONS
            .instances
            .get_instance_root(instance_id)
            .join("saves"),
    )
}

pub async fn get_save_icon(instance_id: &str, folder_name: &str) -> Result<String> {
    let icon_path = save_folder(instance_id, folder_name).join("icon.png");
    let icon = tokio::fs::read(icon_path).await?;
    Ok(format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(icon)
    ))
}

pub fn get_save_path(instance_id: &str, folder_name: &str) -> Result<String> {
    Ok(save_folder(instance_id, folder_name)
        .to_string_lossy()
        .to_string())
}

pub async fn delete_save(instance_id: &str, folder_name: &str) -> Result<()> {
    tokio::fs::remove_dir_all(save_folder(instance_id, folder_name)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use fastnbt::{IntArray, LongArray, Value};

    use super::*;

    /// A `Data` compound out of the three shapes `summarize_level` has to tell
    /// apart: the modern `spawn.pos` list, the pre-1.2 `SpawnX`/`SpawnZ` pair
    /// with no list at all, and a list of the wrong length.
    fn data_with_spawn(spawn: Option<Value>) -> Value {
        let mut fields = HashMap::new();
        fields.insert(
            "LevelName".to_string(),
            Value::String("New World".to_string()),
        );
        fields.insert("GameType".to_string(), Value::Int(1));
        fields.insert("LastPlayed".to_string(), Value::Long(1_700_000_000_000));
        if let Some(spawn) = spawn {
            fields.insert(
                "spawn".to_string(),
                Value::Compound(HashMap::from([("pos".to_string(), spawn)])),
            );
        }
        Value::Compound(HashMap::from([(
            "Data".to_string(),
            Value::Compound(fields),
        )]))
    }

    #[test]
    fn reads_the_spawn_point_out_of_the_modern_tag() {
        let root = data_with_spawn(Some(Value::IntArray(IntArray::new(vec![-128, 70, 512]))));
        let summary = summarize_level(&root);
        assert_eq!(summary.spawn, Some([-128, 70, 512]));
        assert_eq!(summary.name.as_deref(), Some("New World"));
        assert_eq!(summary.game_type, Some(1));
        assert_eq!(summary.last_played, Some(1_700_000_000_000));
    }

    #[test]
    fn a_world_without_a_spawn_tag_has_none() {
        // A missing spawn tag yields `None`; `worldmap.rs` asks the world for
        // its own spawn only when a request carries no centre, not for `(0, 0)`.
        assert_eq!(summarize_level(&data_with_spawn(None)).spawn, None);
    }

    #[test]
    fn a_spawn_list_of_the_wrong_shape_has_none() {
        assert_eq!(
            summarize_level(&data_with_spawn(Some(Value::IntArray(IntArray::new(
                vec![1, 2]
            )))))
            .spawn,
            None
        );
        assert_eq!(
            summarize_level(&data_with_spawn(Some(Value::IntArray(IntArray::new(
                Vec::new()
            )))))
            .spawn,
            None
        );
        // A list of the right length but the wrong tag is not a position either.
        assert_eq!(
            summarize_level(&data_with_spawn(Some(Value::LongArray(LongArray::new(
                vec![1, 2, 3]
            )))))
            .spawn,
            None
        );
    }
}
