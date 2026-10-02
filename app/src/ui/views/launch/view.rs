// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Seeding the launch screen from the run's instance and account.

use super::*;

/// Resets `LaunchState` and seeds the instance and account fields.
pub(crate) fn reset_state(ui: &App, instance: Option<&Instance>, config: &Config) {
    let state = ui.global::<LaunchState>();
    state.set_error(false);
    state.set_progress_kind("preparing".into());
    state.set_progress_name("".into());
    state.set_progress_current("".into());
    state.set_progress_total("".into());
    state.set_progress_message("".into());
    state.set_error_message("".into());
    state.set_progress_loading(true);
    state.set_progress_value(0.0);
    state.set_progress_max(0.0);
    state.set_back_disabled(false);

    match instance {
        Some(instance) => {
            state.set_has_instance(true);
            state.set_instance_name(instance.config.name.clone().into());
            state.set_minecraft_version(instance.config.runtime.minecraft.clone().into());
            state.set_has_loader(instance.config.runtime.mod_loader_type.is_some());
            state.set_loader_type(
                instance
                    .config
                    .runtime
                    .mod_loader_type
                    .as_ref()
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default()
                    .into(),
            );
            state.set_loader_version(
                instance
                    .config
                    .runtime
                    .mod_loader_version
                    .clone()
                    .unwrap_or_default()
                    .into(),
            );
        }
        None => {
            state.set_has_instance(false);
            state.set_instance_name("".into());
            state.set_minecraft_version("".into());
            state.set_has_loader(false);
            state.set_loader_type("".into());
            state.set_loader_version("".into());
        }
    }

    let account = config.current_account.clone();
    match account.as_ref() {
        Some(account) => {
            state.set_has_account(true);
            state.set_account_kind(account.kind().into());
            state.set_profile_name(account.get_profile_name().into());
            state.set_account_avatar(
                crate::ui::components::account_avatar::account_head(Some(account), 48)
                    .unwrap_or_default(),
            );
        }
        None => {
            state.set_has_account(false);
            state.set_account_kind("".into());
            state.set_profile_name("".into());
            state.set_account_avatar(Default::default());
        }
    }
}
