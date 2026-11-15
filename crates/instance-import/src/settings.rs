// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The settings a source asks for in its own configuration, and how they reach
//! a new instance.
//!
//! Most of them are launch settings, but not all of them have to be: a source
//! may also name its instance or carry a group, and batch imports will need that
//! as much as the JVM arguments. They live together because they are one thing —
//! what the source's configuration says the imported instance is — and they
//! reach the instance through [`Settings::apply`].
//!
//! The formats disagree on the shape but not on the intent: Prism/MultiMC
//! writes them into `instance.cfg`, MCBBS into `launchInfo`. A source that asks
//! for nothing leaves the instance on the launcher's defaults, which is why an
//! empty [`Settings`] changes nothing.

use config::launch::Server;
use instance::InstanceConfig;

/// The settings a source's configuration carries.
#[derive(Clone, Default)]
pub(crate) struct Settings {
    max_memory: Option<usize>,
    extra_jvm_args: Option<String>,
    extra_mc_args: Option<String>,
    server: Option<Server>,
    width: Option<usize>,
    height: Option<usize>,
}

impl Settings {
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

    /// Records the server the source auto-joins, `host` or `host:port`.
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

    /// Writes the source's settings into a new instance's configuration.
    pub(crate) fn apply(&self, config: &mut InstanceConfig) {
        if self.is_empty() {
            return;
        }
        let launch = &mut config.launch_config;
        launch.enable_instance_specific_settings = true;
        if let Some(memory) = self.max_memory {
            // A source that names a heap for itself would otherwise lose it to
            // the launcher's automatic allocation.
            launch.auto_memory = Some(false);
            launch.max_memory = Some(memory);
        }
        launch.extra_jvm_args = self.extra_jvm_args.clone();
        launch.extra_mc_args = self.extra_mc_args.clone();
        launch.server = self.server.clone();
        launch.width = self.width;
        launch.height = self.height;
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
    fn an_empty_source_leaves_the_instance_on_the_launcher_defaults() {
        let launch = Settings::default();
        assert!(launch.is_empty());
    }

    #[test]
    fn splits_a_server_address_into_host_and_port() {
        let mut launch = Settings::default();
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
        let mut launch = Settings::default();
        launch.set_max_memory(6144);
        let mut applied = InstanceConfig::default();
        launch.apply(&mut applied);
        assert_eq!(applied.launch_config.max_memory, Some(6144));
        assert_eq!(applied.launch_config.auto_memory, Some(false));
        assert!(applied.launch_config.enable_instance_specific_settings);
    }

    #[test]
    fn empty_argument_lists_are_not_recorded() {
        let mut launch = Settings::default();
        launch.set_extra_jvm_args(vec![" ".to_string(), String::new()]);
        assert!(launch.extra_jvm_args.is_none());
    }
}
