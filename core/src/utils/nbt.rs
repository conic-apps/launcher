// Conic Launcher
// Copyright 2022-2026 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use nbt::Value;

/// Get value from nbt value
///
/// # Args
/// * `nbt` - nbt value, for more info, see [hematite-nbt crate](https://crates.io/crates/hematite-nbt)
/// * `target` - You need to use `:` to connect the path. For example, if you want to modify the
///   value of `seed`, you can to use `Data:world_gen_settings:seed` or ``.
pub fn get_value(nbt_value: Value, target: &str) -> Result<Value> {
    let mut result = nbt_value.clone();
    for name in target.split(":") {
        if let Value::Compound(map) = result {
            result = map
                .get(name)
                .ok_or(anyhow::anyhow!("nbt not found!"))?
                .clone();
        }
    }
    Ok(result)
}

/// Modify nbt settings
///
/// # Args
/// * `nbt_value` - nbt value, for more info, see [hematite-nbt crate](https://crates.io/crates/hematite-nbt)
/// * `target` - You need to use `:` to connect the path. For example, if you want to modify the
///   value of `seed`, you can to use `Data:world_gen_settings:seed` or ``.
/// * `value` - The value you want to modify
pub fn modify_nbt(nbt_value: Value, target: &str, value: Value) -> Result<Value> {
    let mut nbt_value_map = match nbt_value.clone() {
        Value::Compound(map) => map,
        _ => return Ok(nbt_value),
    };
    let mut nbt_path_mut = target.to_string();
    let nbt_path_split = target.split(":").collect::<Vec<&str>>();
    let mut buf = Value::Byte(0);
    let mut last_name = String::new();
    let mut result = std::collections::HashMap::new();
    if nbt_path_split.len() == 1 {
        nbt_value_map.insert(nbt_path_mut, value.clone());
        return Ok(Value::Compound(nbt_value_map));
    }
    for (index, name) in nbt_path_split.iter().rev().enumerate() {
        let is_last = index == nbt_path_split.len() - 1;
        let is_first = index == 0;
        if is_first {
            buf = value.clone();
            last_name = name.to_string();
            nbt_path_mut = nbt_path_mut.replace(format!(":{}", name).as_str(), "");
            continue;
        }
        let old = get_value(nbt_value.clone(), &nbt_path_mut).unwrap();
        let mut old = match old {
            Value::Compound(map) => map,
            _ => return Err(anyhow::anyhow!("nbt not found!")),
        };
        old.insert(last_name.clone(), buf.clone());
        buf = Value::Compound(old);
        last_name = name.to_string();
        if !is_last {
            nbt_path_mut = nbt_path_mut.replace(format!(":{}", name).as_str(), "");
            continue;
        }
        result = if let Value::Compound(map) = nbt_value.clone() {
            map.clone()
        } else {
            panic!("program broken")
        };
        result.insert(name.to_string(), buf.clone());
    }

    Ok(Value::Compound(result.clone()))
}
