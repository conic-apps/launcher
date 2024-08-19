use crate::{config::{instance::InstanceConfig, launch::{LaunchConfig, ProcessPriority, Server, GC}}, folder::MinecraftLocation, DATA_LOCATION};

#[derive(Debug, Clone)]
pub struct GameProfile {
    pub name: String,
    pub uuid: String,
}

#[derive(Debug, Clone)]
pub struct LaunchOptions {
    /// User selected game profile.
    ///
    /// For game display name & uuid
    pub(crate) game_profile: GameProfile,

    pub(crate) properties: String,
    pub(crate) access_token: String,

    /// Min memory, this will add a jvm flag -XMS to the command result
    pub(crate) min_memory: u32,

    /// Max memory, this will add a jvm flag -Xmx to the command result
    pub(crate) max_memory: u32,

    /// Directly launch to a server. TODO: support 1.21.1
    pub(crate) server: Option<Server>,

    /// window width
    pub(crate) width: u32,

    /// window height
    pub(crate) height: u32,

    pub(crate) fullscreen: bool,

    /// User custom additional java virtual machine command line arguments.
    ///
    /// If this is empty, the `DEFAULT_EXTRA_JVM_ARGS` will be used.
    pub(crate) extra_jvm_args: Vec<String>,

    /// User custom additional minecraft command line arguments.
    pub(crate) extra_mc_args: Vec<String>,

    pub(crate) is_demo: bool,

    /// Add `-Dfml.ignoreInvalidMinecraftCertificates=true` to jvm argument
    pub(crate) ignore_invalid_minecraft_certificates: bool,

    /// Add `-Dfml.ignorePatchDiscrepancies=true` to jvm argument
    pub(crate) ignore_patch_discrepancies: bool,

    /// Add extra classpath
    pub(crate) extra_class_paths: Vec<String>,

    /// Game process priority, invalid on windows
    pub(crate) process_priority: ProcessPriority,

    // /// TODO: Support yushi's yggdrasil agent <https://github.com/to2mbn/authlib-injector/wiki>
    // pub(crate) yggdrasil_agent: Option<YggdrasilAgent>,
    pub(crate) gc: GC,

    pub(crate) minecraft_location: MinecraftLocation,
}

impl LaunchOptions {
    pub fn get(instance_config: InstanceConfig) -> Self {
        let global_config = LaunchConfig::get();
        let instance_config = instance_config.launch_config.clone();
        Self {
            game_profile: GameProfile {
                name: "Steve".to_string(),
                uuid: "00000000-0000-0000-0000-000000011111".to_string(),
            },
            access_token: "00000000000000000000000000000000".to_string(),
            min_memory: instance_config
                .min_memory
                .unwrap_or(global_config.min_memory),
            max_memory: instance_config
                .max_memory
                .unwrap_or(global_config.max_memory),
            server: instance_config.server,
            width: instance_config.width.unwrap_or(global_config.width),
            height: instance_config.height.unwrap_or(global_config.height),
            fullscreen: instance_config
                .fullscreen
                .unwrap_or(global_config.fullscreen),
            extra_jvm_args: instance_config
                .extra_jvm_args
                .unwrap_or(global_config.extra_jvm_args),
            extra_mc_args: instance_config
                .extra_mc_args
                .unwrap_or(global_config.extra_mc_args),
            is_demo: instance_config.is_demo.unwrap_or(global_config.is_demo),
            ignore_invalid_minecraft_certificates: instance_config
                .ignore_invalid_minecraft_certificates
                .unwrap_or(global_config.ignore_invalid_minecraft_certificates),
            ignore_patch_discrepancies: instance_config
                .ignore_patch_discrepancies
                .unwrap_or(global_config.ignore_patch_discrepancies),
            extra_class_paths: instance_config
                .extra_class_paths
                .unwrap_or(global_config.extra_class_paths),
            process_priority: instance_config
                .process_priority
                .unwrap_or(global_config.process_priority),
            gc: instance_config.gc.unwrap_or(global_config.gc),
            minecraft_location: MinecraftLocation::new(&DATA_LOCATION.get().unwrap().root),
            properties: "{}".to_string(),
        }
    }
}
