// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use fastnbt::Value;

use crate::error::*;

/// Modify the value at a colon-separated path, e.g. `Data:world_gen_settings:seed`.
pub fn modify_nbt(nbt_value: Value, target: &str, value: Value) -> Result<Value> {
    let path: Vec<&str> = target.split(':').collect();
    let mut nbt_value = nbt_value.clone();
    modify_nbt_inner(&mut nbt_value, &path, value)?;
    Ok(nbt_value)
}

fn modify_nbt_inner(current: &mut Value, path: &[&str], value: Value) -> Result<()> {
    if path.is_empty() {
        *current = value;
        return Ok(());
    }
    if let Value::Compound(map) = current {
        let next = map.get_mut(path[0]).ok_or(Error::NBTPathNotFound)?;
        modify_nbt_inner(next, &path[1..], value)
    } else {
        Err(Error::IsNotCompound)
    }
}
