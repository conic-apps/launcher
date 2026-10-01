// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The add-account callbacks, registered on `AccountAddState`.

use super::*;

pub fn setup(ui: &App) {
    setup_shell(ui);
    setup_offline_screen(ui);
    setup_microsoft_screen(ui);
    setup_yggdrasil_form(ui);
    setup_yggdrasil_login(ui);
}

/// The shell every screen shares: the auth-service switch, dismissal, and the
/// copy/open helpers the screens use.
pub(crate) fn setup_shell(ui: &App) {
    let state = ui.global::<AccountAddState>();
    {
        let weak = ui.as_weak();
        state.on_select_auth_service(move |service| {
            // The screen swap itself is the shell's; this records the choice
            // (Vue `authServiceType`).
            let Some(ui) = weak.upgrade() else { return };
            ui.global::<AccountAddState>().set_auth_service(service);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_close(move || {
            let Some(ui) = weak.upgrade() else { return };
            // The Vue's `onUnmounted` cancels whatever was still running.
            MICROSOFT.with(|flow| flow.borrow().login.cancel());
            release_auth_code_flow(&ui);
            ui.global::<Dialogs>().set_account_add_visible(false);
        });
    }
    state.on_copy_text(|text| {
        if let Err(error) = crate::config_bridge::copy_to_clipboard(text.as_str()) {
            log::warn!("failed to write to the clipboard: {error}");
        }
    });
    state.on_open_url(|url| {
        if let Err(error) = crate::config_bridge::open_external(url.as_str()) {
            log::warn!("failed to open '{url}': {error}");
        }
    });
}

/// The offline screen: the username-to-UUID derivation and the submit.
pub(crate) fn setup_offline_screen(ui: &App) {
    let state = ui.global::<AccountAddState>();
    {
        let weak = ui.as_weak();
        state.on_reset_offline_form(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<AccountAddState>();
            state.set_offline_username("".into());
            state.set_offline_advanced(false);
            state.set_offline_uuid(get_uuid_from_username("").to_string().into());
            state.set_offline_uuid_invalid(false);
            refresh_offline_submit(&state);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_toggle_offline_advanced(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<AccountAddState>();
            let advanced = !state.get_offline_advanced();
            state.set_offline_advanced(advanced);
            state.set_offline_uuid_invalid(false);
            // The Vue's `watch(advancedMode)`: switching it on hands the field
            // over empty, switching it off takes the derived value back.
            if advanced {
                state.set_offline_uuid("".into());
            } else {
                state.set_offline_uuid(
                    get_uuid_from_username(state.get_offline_username().as_str())
                        .to_string()
                        .into(),
                );
            }
            refresh_offline_submit(&state);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_sync_offline_uuid(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<AccountAddState>();
            if state.get_offline_advanced() {
                // The Vue's `uuidInvalid`: only a typed, unparsable value is
                // wrong — an empty field simply has nothing to send yet.
                let uuid = state.get_offline_uuid().trim().to_string();
                state.set_offline_uuid_invalid(!uuid.is_empty() && Uuid::parse_str(&uuid).is_err());
            } else {
                state.set_offline_uuid_invalid(false);
                state.set_offline_uuid(
                    get_uuid_from_username(state.get_offline_username().as_str())
                        .to_string()
                        .into(),
                );
            }
            refresh_offline_submit(&state);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_add_offline_account(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<AccountAddState>();
            let username = state.get_offline_username().trim().to_string();
            let uuid = state.get_offline_uuid().trim().to_string();
            // The Vue's `submitAccount` guard, on top of the disabled button.
            if username.is_empty() {
                return;
            }
            let Ok(uuid) = Uuid::parse_str(&uuid) else {
                return;
            };

            let weak = weak.clone();
            crate::runtime::spawn(async move {
                if let Err(error) = account::offline::add_account(username, uuid).await {
                    // The Vue has no error path here either: the dialog simply
                    // stays open.
                    log::error!("failed to add the offline account: {error}");
                    return;
                }
                let _ = weak.upgrade_in_event_loop(move |ui| finish_add(&ui));
            });
        });
    }
}

/// The Microsoft screen: the device-code and browser (auth-code) flows.
pub(crate) fn setup_microsoft_screen(ui: &App) {
    let state = ui.global::<AccountAddState>();
    {
        let weak = ui.as_weak();
        state.on_use_device_code_flow(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<AccountAddState>();
            if MICROSOFT.with(|flow| flow.borrow().login.is_running()) {
                // The user is coming back to a code that is still being polled.
                state.set_ms_view("device-code".into());
                return;
            }
            // Deliberate divergence from the Vue: it keeps the previous code on
            // screen and never polls it again, which leaves the dialog stuck.
            // A fresh code is what the user asked for.
            state.set_ms_user_code("".into());
            state.set_ms_verification_uri("".into());
            start_microsoft_login(weak.clone(), LoginRequest::DeviceCode, true);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_use_auth_code_flow(move || {
            let Some(ui) = weak.upgrade() else { return };
            // The Vue's `watch(useDeviceCodeFlow)`: leaving the device code
            // cancels the login behind it. The browser screen's own listener is
            // the one this screen's swap tears down (see `release`).
            MICROSOFT.with(|flow| flow.borrow().login.cancel());
            ui.global::<AccountAddState>()
                .set_ms_view("auth-code".into());
        });
    }
    {
        let weak = ui.as_weak();
        state.on_cancel_microsoft_login(move || {
            let Some(ui) = weak.upgrade() else { return };
            // The Vue's `closeDialog()`: cancel, then dismiss.
            MICROSOFT.with(|flow| flow.borrow().login.cancel());
            release_auth_code_flow(&ui);
            ui.global::<Dialogs>().set_account_add_visible(false);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_prepare_auth_code_flow(move || {
            let Some(ui) = weak.upgrade() else { return };
            prepare_auth_code_flow(&ui);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_release_auth_code_flow(move || {
            let Some(ui) = weak.upgrade() else { return };
            release_auth_code_flow(&ui);
        });
    }
}

/// The Yggdrasil screen's form: the reset, the submit gate and the server-name
/// lookup.
pub(crate) fn setup_yggdrasil_form(ui: &App) {
    let state = ui.global::<AccountAddState>();
    {
        let weak = ui.as_weak();
        state.on_reset_yggdrasil_form(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<AccountAddState>();
            state.set_yggdrasil_view("form".into());
            state.set_yggdrasil_api_root("".into());
            state.set_yggdrasil_username("".into());
            state.set_yggdrasil_password("".into());
            state.set_yggdrasil_server_name("".into());
            state.set_yggdrasil_error("".into());
            state.set_yggdrasil_has_selection(false);
            refresh_yggdrasil_submit(&state);
        });
    }
    {
        let weak = ui.as_weak();
        state.on_sync_yggdrasil_submit(move || {
            let Some(ui) = weak.upgrade() else { return };
            refresh_yggdrasil_submit(&ui.global::<AccountAddState>());
        });
    }
    {
        let weak = ui.as_weak();
        state.on_lookup_server_name(move || {
            let Some(ui) = weak.upgrade() else { return };
            let api_root = ui
                .global::<AccountAddState>()
                .get_yggdrasil_api_root()
                .trim()
                .to_string();
            if api_root.is_empty() {
                ui.global::<AccountAddState>()
                    .set_yggdrasil_server_name("".into());
                return;
            }
            let weak = weak.clone();
            crate::runtime::spawn(async move {
                let info = yggdrasil::yggdrasil_server::get_server_info(&api_root).await;
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    // The Vue only takes a string `meta.serverName`, and leaves
                    // the previous name up when the request fails.
                    let Ok(info) = info else { return };
                    if let Some(name) = info.meta.get("serverName").and_then(|name| name.as_str()) {
                        ui.global::<AccountAddState>()
                            .set_yggdrasil_server_name(name.into());
                    }
                });
            });
        });
    }
}

/// The Yggdrasil login and the profile chooser it may open.
pub(crate) fn setup_yggdrasil_login(ui: &App) {
    let state = ui.global::<AccountAddState>();
    {
        let weak = ui.as_weak();
        state.on_yggdrasil_login(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<AccountAddState>();
            let api_root = state.get_yggdrasil_api_root().trim().to_string();
            let username = state.get_yggdrasil_username().trim().to_string();
            let password = state.get_yggdrasil_password().to_string();
            if api_root.is_empty() || username.is_empty() || password.trim().is_empty() {
                return;
            }
            state.set_yggdrasil_view("processing".into());
            state.set_yggdrasil_error("".into());

            let weak = weak.clone();
            crate::runtime::spawn(async move {
                let result = yggdrasil::yggdrasil_user_api::authenticate(
                    &api_root,
                    username.clone(),
                    password,
                )
                .await;
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let response = match result {
                        Ok(response) => response,
                        Err(error) => {
                            let state = ui.global::<AccountAddState>();
                            state.set_yggdrasil_error(error.to_string().into());
                            state.set_yggdrasil_view("error".into());
                            return;
                        }
                    };
                    // The default profile is taken before the response is split
                    // up: the Vue asks the user only when there is no default
                    // *and* a choice to make.
                    let selected = response.selected_profile;
                    let credentials = PendingYggdrasil {
                        access_token: response.access_token,
                        client_token: response.client_token,
                        profiles: response.available_profiles,
                        api_root,
                        username,
                    };
                    if credentials.profiles.len() > 1 && selected.is_none() {
                        show_profile_chooser(&ui, &credentials);
                        PENDING.with(|pending| *pending.borrow_mut() = Some(credentials));
                        return;
                    }
                    let profile = selected.or_else(|| credentials.profiles.first().cloned());
                    let Some(profile) = profile else {
                        let state = ui.global::<AccountAddState>();
                        state.set_yggdrasil_error(state.get_no_profile_message());
                        state.set_yggdrasil_view("error".into());
                        return;
                    };
                    add_yggdrasil_accounts(&ui, &credentials, &[profile]);
                });
            });
        });
    }
    {
        let weak = ui.as_weak();
        state.on_toggle_yggdrasil_profile(move |index| {
            let Some(ui) = weak.upgrade() else { return };
            PROFILES.with(|model| {
                let Ok(index) = usize::try_from(index) else {
                    return;
                };
                let Some(mut row) = model.row_data(index) else {
                    return;
                };
                // `&.disabled { pointer-events: none }`: a profile the launcher
                // already has cannot be picked.
                if row.added {
                    return;
                }
                row.selected = !row.selected;
                model.set_row_data(index, row);
                let has_selection = model.iter().any(|row| row.selected && !row.added);
                ui.global::<AccountAddState>()
                    .set_yggdrasil_has_selection(has_selection);
            });
        });
    }
    {
        let weak = ui.as_weak();
        state.on_yggdrasil_add_profiles(move || {
            let Some(ui) = weak.upgrade() else { return };
            let Some(credentials) = PENDING.with(|pending| pending.borrow().clone()) else {
                return;
            };
            let selected = PROFILES.with(|model| {
                credentials
                    .profiles
                    .iter()
                    .filter(|profile| {
                        let id = profile.id.to_string();
                        model
                            .iter()
                            .any(|row| row.id == id && row.selected && !row.added)
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            });
            if selected.is_empty() {
                return;
            }
            add_yggdrasil_accounts(&ui, &credentials, &selected);
        });
    }
}
