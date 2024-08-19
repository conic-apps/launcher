// TODO: launch options

// now just use default launch config

use serde::{Deserialize, Serialize};

/// Game process priority, invalid on windows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum ProcessPriority {
    High,
    AboveNormal,
    Normal,
    BelowNormal,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct Server {
    pub ip: String,
    pub port: Option<u16>,
}

/// User custom jvm gc
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub enum GC {
    Serial,
    Parallel,
    ParallelOld,
    G1,
    Z,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd, Hash)]
pub struct LaunchConfig {
    /// Min memory, this will add a jvm flag -XMS to the command result
    pub(crate) min_memory: u32,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub(crate) max_memory: u32,
    pub(crate) server: Option<Server>,
    /// window width
    pub(crate) width: u32,

    /// window height
    pub(crate) height: u32,

    pub(crate) fullscreen: bool,

    /// User custom additional java virtual machine command line arguments.
    pub(crate) extra_jvm_args: Vec<String>,

    /// User custom additional minecraft command line arguments.
    pub(crate) extra_mc_args: Vec<String>,

    pub(crate) is_demo: bool,
    /// Game process priority, invalid on windows
    pub(crate) process_priority: ProcessPriority,

    /// Add `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub(crate) ignore_invalid_minecraft_certificates: bool,

    /// Add `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub(crate) ignore_patch_discrepancies: bool,

    /// Add extra classpath
    pub(crate) extra_class_paths: Vec<String>,

    pub(crate) gc: GC,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            min_memory: 0,
            max_memory: 1024,
            server: None,
            width: 854,
            height: 480,
            fullscreen: false,
            extra_jvm_args: vec![],
            extra_mc_args: vec![],
            is_demo: false,
            process_priority: ProcessPriority::Normal,
            ignore_invalid_minecraft_certificates: false,
            ignore_patch_discrepancies: false,
            extra_class_paths: vec![],
            gc: GC::G1,
        }
    }
}

impl LaunchConfig {
    pub fn get() -> Self {
        Self::default()
    }
}
