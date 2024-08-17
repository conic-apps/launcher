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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::version::Version;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostProcessor {
    /// The executable jar path
    pub jar: String,

    /// The classpath to run
    pub classpath: Vec<String>,
    pub args: Vec<String>,
    pub outputs: Option<HashMap<String, String>>,
    pub sides: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProfile {
    pub spec: Option<i32>,
    /// The type of this installation, like "forge"
    pub profile: Option<String>,

    /// The version of this installation
    pub version: Option<String>,

    /// The version json path
    pub json: Option<String>,

    /// The maven artifact name: \<org\>:\<artifact-id\>:\<version\>
    pub path: Value,

    /// The minecraft version
    pub minecraft: String,

    /// The processor shared variables. The key is the name of variable to replace.
    ///
    /// The value of client/server is the value of the variable.
    pub data: Option<HashMap<String, InstallProfileData>>,

    /// The post processor. Which require java to run.
    pub processors: Option<Vec<PostProcessor>>,

    /// The required install profile libraries
    pub libraries: Value,

    pub version_info: Option<Version>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProfileLegacy {
    pub install: HashMap<String, String>,
    pub version_info: Option<Version>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstallProfileData {
    pub client: Option<String>,
    pub server: Option<String>,
}
