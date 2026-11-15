// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The launch settings a pack asks for in its own configuration, and how they
//! reach a new instance.
//!
//! The formats disagree on the shape but not on the intent: Prism/MultiMC
//! writes them into `instance.cfg`, MCBBS into `launchInfo`. A pack that asks
//! for nothing leaves the instance on the launcher's global settings, which is
//! why an empty [`PackLaunch`] is not applied at all.

use config::launch::Server;
use instance::InstanceLaunchConfig;

/// The launch settings a pack's configuration carries.
#[derive(Clone, Default)]
pub(crate) struct PackLaunch {
    max_memory: Option<usize>,
    extra_jvm_args: Option<String>,
    extra_mc_args: Option<String>,
    server: Option<Server>,
    width: Option<usize>,
    height: Option<usize>,
}

impl PackLaunch {
    pub(crate) fn is_empty(&self) -> bool {
        self.max_memory.is_none()
            && self.extra_jvm_args.is_none()
            && self.extra_mc_args.is_none()
            && self.server.is_none()
            && self.width.is_none()
            && self.height.is_none()
    }

    pub(crate) fn set_max_memory(&mut self, megabytes: u64) {
        if let Ok(megabytes) = usize::try_from(megabytes)
            && megabytes > 0
        {
            self.max_memory = Some(megabytes);
        }
    }

    /// Joins the arguments into one line; an empty list says nothing.
    pub(crate) fn set_extra_jvm_args<I: IntoIterator<Item = String>>(&mut self, args: I) {
        self.extra_jvm_args = joined(args);
    }

    pub(crate) fn set_extra_mc_args<I: IntoIterator<Item = String>>(&mut self, args: I) {
        self.extra_mc_args = joined(args);
    }

    pub(crate) fn set_window(&mut self, width: u64, height: u64) {
        let (Ok(width), Ok(height)) = (usize::try_from(width), usize::try_from(height)) else {
            return;
        };
        if width > 0 && height > 0 {
            self.width = Some(width);
            self.height = Some(height);
        }
    }

    /// Records the server the pack auto-joins, `host` or `host:port`.
    pub(crate) fn set_server(&mut self, address: &str) {
        let address = address.trim();
        if address.is_empty() {
            return;
        }
        let (ip, port) = match address.rsplit_once(':') {
            Some((ip, port)) => match port.parse::<u16>() {
                Ok(port) => (ip.to_string(), Some(port)),
                Err(_) => (address.to_string(), None),
            },
            None => (address.to_string(), None),
        };
        self.server = Some(Server { ip, port });
    }

    /// Writes the pack's settings over `config`.
    pub(crate) fn apply(&self, config: &mut InstanceLaunchConfig) {
        config.enable_instance_specific_settings = true;
        if let Some(memory) = self.max_memory {
            // A pack that names a heap for itself would otherwise lose it to
            // the launcher's automatic allocation.
            config.auto_memory = Some(false);
            config.max_memory = Some(memory);
        }
        config.extra_jvm_args = self.extra_jvm_args.clone();
        config.extra_mc_args = self.extra_mc_args.clone();
        config.server = self.server.clone();
        config.width = self.width;
        config.height = self.height;
    }
}

fn joined<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    let line = args
        .into_iter()
        .map(|arg| arg.trim().to_string())
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!line.is_empty()).then_some(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_pack_leaves_the_instance_on_the_global_settings() {
        let launch = PackLaunch::default();
        assert!(launch.is_empty());
    }

    #[test]
    fn splits_a_server_address_into_host_and_port() {
        let mut launch = PackLaunch::default();
        launch.set_server("mc.example.com:25566");
        let server = launch.server.clone().expect("a server");
        assert_eq!(server.ip, "mc.example.com");
        assert_eq!(server.port, Some(25566));

        launch.set_server("mc.example.com");
        let server = launch.server.clone().expect("a server");
        assert_eq!(server.ip, "mc.example.com");
        assert_eq!(server.port, None);
    }

    #[test]
    fn a_heap_disables_the_automatic_allocation() {
        let mut launch = PackLaunch::default();
        launch.set_max_memory(6144);
        let mut config = InstanceLaunchConfig::default();
        launch.apply(&mut config);
        assert_eq!(config.max_memory, Some(6144));
        assert_eq!(config.auto_memory, Some(false));
        assert!(config.enable_instance_specific_settings);
    }

    #[test]
    fn empty_argument_lists_are_not_recorded() {
        let mut launch = PackLaunch::default();
        launch.set_extra_jvm_args(vec![" ".to_string(), String::new()]);
        assert!(launch.extra_jvm_args.is_none());
    }
}
