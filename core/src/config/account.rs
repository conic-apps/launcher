// Amethyst Launcher
// Copyright 2022-2024 Broken-Deer and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Skin {
    pub id: String,
    pub state: String,
    #[serde(rename(serialize = "texture_key", deserialize = "textureKey"))]
    pub texture_key: String,
    pub url: String,
    pub variant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Cape {
    pub alias: String,
    pub id: String,
    pub state: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Profile {
    pub profile_name: String,
    pub uuid: String,
    pub skins: Vec<Skin>,
    pub capes: Vec<Cape>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum AccountType {
    Microsoft,
    Offline,
}
