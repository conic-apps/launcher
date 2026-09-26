// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The add-account dialog's "script": drives `slint-account` from the dialog's
//! state (src/overlays/dialogs/AccountAdd.vue and the four screens under
//! `src/views/accounts/`).
//!
//! Like `create_instance.rs`, everything that touches the network or the disk
//! runs on the tokio runtime (`crate::runtime`) and reports back through
//! `upgrade_in_event_loop`, so the window keeps drawing while a server answers.
//! The one exception is anything that builds a Slint `Image` — the profile
//! heads — which Slint only lets the drawing thread create.
//!
//! The Microsoft flow is where this module reads state it did not write: the
//! login runs as a task (see `slint_account::LoginTaskState`) whose progress
//! arrives as events, so which of the screen's four states is mounted is
//! decided here rather than in the screen.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

use slint::{ComponentHandle, Model, ModelRc, VecModel, Weak};
use url::Url;
use uuid::Uuid;

use crate::account_avatar;
use crate::slint_backend::{AccountAddState, App, Dialogs, GameState, YggdrasilProfileItem};
use slint_account::{
    Error, LoginTaskState, get_uuid_from_username,
    microsoft::{LoginEvent, LoginReporter},
    yggdrasil::{self, yggdrasil_user_api::Profile as YggdrasilProfile},
};

thread_local! {
    /// The chooser's rows. Kept across the dialog's screens so a click can flip
    /// one in place instead of re-creating every row, and reached through a
    /// thread-local rather than an `Rc` captured by the callbacks: the UI half
    /// of `upgrade_in_event_loop` has to be `Send`, so a worker cannot carry it
    /// across (`MANIFEST` in `create_instance.rs` is a thread-local for the same
    /// reason).
    static PROFILES: Rc<VecModel<YggdrasilProfileItem>> = Rc::new(VecModel::default());

    /// The credentials the chooser's rows belong to (the Vue's `authResponse`).
    static PENDING: RefCell<Option<PendingYggdrasil>> = const { RefCell::new(None) };
}

/// The credentials and profiles of a successful Yggdrasil sign-in, held between
/// the form and the profile chooser — the Vue keeps the whole `AuthResponse` in
/// its `authResponse` ref.
#[derive(Clone)]
struct PendingYggdrasil {
    api_root: String,
    username: String,
    access_token: String,
    client_token: String,
    /// `availableProfiles`, in the order the server answered with (the crate
    /// sorts them by name).
    profiles: Vec<YggdrasilProfile>,
}

/// Registers every add-account callback on the `AccountAddState` global.
///
/// No configuration is involved: an add writes an account file and then asks
/// the game view to reload (`finish_add`), which is what picks the default
/// account and persists the choice.
pub fn setup(ui: &App) {
    let state = ui.global::<AccountAddState>();
    // The original keeps this in the Tauri plugin's state (see
    // `slint_account::LoginTaskState`); here the app owns the one instance. It
    // is cloned into the callbacks rather than shared behind an `Rc`, because
    // the login task moves into the runtime.
    let login_task = LoginTaskState::default();

    // ----- the shell -----
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
        let login_task = login_task.clone();
        state.on_close(move || {
            let Some(ui) = weak.upgrade() else { return };
            // The Vue's `onUnmounted` cancels whatever was still running.
            login_task.cancel();
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

    // ----- the offline screen -----
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
                if let Err(error) = slint_account::offline::add_account(username, uuid).await {
                    // The Vue has no error path here either: the dialog simply
                    // stays open.
                    log::error!("failed to add the offline account: {error}");
                    return;
                }
                let _ = weak.upgrade_in_event_loop(move |ui| finish_add(&ui));
            });
        });
    }

    // ----- the Microsoft screen -----
    {
        let weak = ui.as_weak();
        let login_task = login_task.clone();
        state.on_use_device_code_flow(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<AccountAddState>();
            if login_task.is_running() {
                // The user is coming back to a code that is still being polled.
                state.set_ms_view("device-code".into());
                return;
            }
            // Deliberate divergence from the Vue: it keeps the previous code on
            // screen and never polls it again, which leaves the dialog stuck.
            // A fresh code is what the user asked for.
            state.set_ms_user_code("".into());
            state.set_ms_verification_uri("".into());
            start_microsoft_login(weak.clone(), login_task.clone(), true);
        });
    }
    {
        let weak = ui.as_weak();
        let login_task = login_task.clone();
        state.on_use_auth_code_flow(move || {
            let Some(ui) = weak.upgrade() else { return };
            // The Vue's `watch(useDeviceCodeFlow)`: leaving the device code
            // cancels the login behind it.
            login_task.cancel();
            ui.global::<AccountAddState>()
                .set_ms_view("auth-code".into());
        });
    }
    {
        let weak = ui.as_weak();
        let login_task = login_task.clone();
        state.on_cancel_microsoft_login(move || {
            let Some(ui) = weak.upgrade() else { return };
            // The Vue's `closeDialog()`: cancel, then dismiss.
            login_task.cancel();
            ui.global::<Dialogs>().set_account_add_visible(false);
        });
    }

    // ----- the Yggdrasil screen -----
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

/// Recomputes whether the offline form can be submitted — the Vue's `:disabled`
/// test, which trims the username; Slint has no `trim`, so the test lives here.
fn refresh_offline_submit(state: &AccountAddState) {
    let username = state.get_offline_username().trim().to_string();
    state.set_offline_can_submit(!username.is_empty() && !state.get_offline_uuid_invalid());
}

/// The same for the Yggdrasil form, whose three fields the Vue trims as well.
fn refresh_yggdrasil_submit(state: &AccountAddState) {
    let ready = [
        state.get_yggdrasil_api_root(),
        state.get_yggdrasil_username(),
        state.get_yggdrasil_password(),
    ]
    .iter()
    .all(|value| !value.trim().is_empty());
    state.set_yggdrasil_can_submit(ready);
}

/// Picks a default account, reloads the account list and dismisses the dialog —
/// the three lines the Vue runs after *every* successful add.
///
/// The default is picked through a `GameState` callback rather than here: the
/// config lives on the UI thread, and `upgrade_in_event_loop`'s closure has to
/// be `Send`, so a worker cannot carry it.
fn finish_add(ui: &App) {
    ui.global::<GameState>().invoke_select_first_account();
    ui.global::<GameState>().invoke_refresh();
    ui.global::<Dialogs>().set_account_add_visible(false);
}

/// Names the step a login event reports, as `AccountAddMicrosoft`'s
/// `progress-text` spells it.
///
/// The event itself never reaches the UI: the sentence for each step lives in
/// the `.slint` file, so a language change re-renders it.
fn progress_kind(event: &LoginEvent) -> Option<&'static str> {
    Some(match event {
        LoginEvent::Prepare => "prepare",
        LoginEvent::RequestDeviceCode => "request-device-code",
        LoginEvent::RedeemAccessToken => "redeem-access-token",
        LoginEvent::XboxAuthenticate => "xbox-authenticate",
        LoginEvent::XstsAuthenticate => "xsts-authenticate",
        LoginEvent::MinecraftAuthenticate => "minecraft-authenticate",
        LoginEvent::GetProfile => "get-profile",
        LoginEvent::SaveAccount => "save-account",
        // Handled as a screen change of its own, below.
        LoginEvent::WaitingForAuthorization { .. } => return None,
    })
}

/// Runs the login task and reports its outcome back into the dialog (the Vue's
/// `startLogin`).
fn start_microsoft_login(weak: Weak<App>, task: LoginTaskState, device_flow: bool) {
    let Some(ui) = weak.upgrade() else { return };
    let state = ui.global::<AccountAddState>();
    state.set_ms_view("processing".into());
    state.set_ms_progress("prepare".into());
    state.set_ms_error("".into());

    let reporter = {
        let weak = weak.clone();
        LoginReporter::new(move |event| {
            let weak = weak.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| apply_login_event(&ui, event));
        })
    };

    crate::runtime::spawn(async move {
        let result = task.spawn(None, reporter).await;
        let _ = weak.upgrade_in_event_loop(move |ui| match result {
            Ok(_) => finish_add(&ui),
            Err(error) => handle_login_error(&ui, &task, error, device_flow),
        });
    });
}

/// Moves the Microsoft screen to the state a login event calls for.
fn apply_login_event(ui: &App, event: LoginEvent) {
    let state = ui.global::<AccountAddState>();
    if let LoginEvent::WaitingForAuthorization {
        user_code,
        verification_uri,
        expires_in,
        ..
    } = event
    {
        state.set_ms_user_code(user_code.clone().into());
        state.set_ms_verification_uri(verification_uri.into());
        state.set_ms_expires_in(expires_in as i32);
        // The Vue hands the code straight to the clipboard, so the user only
        // has to paste it.
        if let Err(error) = crate::config_bridge::copy_to_clipboard(&user_code) {
            log::warn!("failed to copy the device code: {error}");
        }
        state.set_ms_view("device-code".into());
        return;
    }
    if let Some(kind) = progress_kind(&event) {
        state.set_ms_progress(kind.into());
        state.set_ms_view("processing".into());
    }
}

/// The Vue's `handleError`.
fn handle_login_error(ui: &App, task: &LoginTaskState, error: Error, device_flow: bool) {
    let state = ui.global::<AccountAddState>();
    match error {
        // The user cancelled: the Vue silently returns to the screen it was
        // on, which for a device login is the code it is still showing.
        Error::Aborted(_) => {
            state.set_ms_view(
                if device_flow {
                    "device-code"
                } else {
                    "auth-code"
                }
                .into(),
            );
        }
        // The Vue asks for a new code and carries on.
        Error::DeviceCodeExpired if device_flow => {
            start_microsoft_login(ui.as_weak(), task.clone(), true);
        }
        error => {
            state.set_ms_error(error.to_string().into());
            state.set_ms_view("error".into());
        }
    }
}

/// Fills the profile chooser (Vue `shouldChooseProfile`).
fn show_profile_chooser(ui: &App, credentials: &PendingYggdrasil) {
    let existing = slint_account::list_accounts();
    let rows: Vec<YggdrasilProfileItem> = credentials
        .profiles
        .iter()
        .map(|profile| {
            let id = profile.id.to_string();
            YggdrasilProfileItem {
                // Slint can only crop the skin on the drawing thread, which is
                // where this runs.
                avatar: account_avatar::avatar_image(
                    yggdrasil::get_skin_url(profile).as_deref(),
                    &id,
                    28,
                )
                .unwrap_or_default(),
                // Vue `profileDisabled`: the same profile on the same server.
                added: existing.yggdrasil.iter().any(|account| {
                    same_api_root(&account.api_root, &credentials.api_root)
                        && account.profile.name == profile.name
                        && account.profile.id == profile.id
                }),
                selected: false,
                name: profile.name.clone().into(),
                id: id.into(),
            }
        })
        .collect();
    let state = ui.global::<AccountAddState>();
    PROFILES.with(|model| {
        model.set_vec(rows);
        state.set_yggdrasil_profiles(ModelRc::new(Rc::clone(model)));
    });
    state.set_yggdrasil_has_selection(false);
    state.set_yggdrasil_view("profiles".into());
}

/// Stores one account per chosen profile (the Vue's `addSelectedProfiles`, and
/// the single-profile tail of its `login`).
///
/// Every profile gets its own identifier and its own copy of the credentials,
/// exactly as the Vue's loop does.
fn add_yggdrasil_accounts(ui: &App, credentials: &PendingYggdrasil, profiles: &[YggdrasilProfile]) {
    let accounts: Vec<slint_account::yggdrasil::YggdrasilAccount> = profiles
        .iter()
        .map(|profile| slint_account::yggdrasil::YggdrasilAccount {
            api_root: credentials.api_root.clone(),
            username: credentials.username.clone(),
            access_token: credentials.access_token.clone(),
            client_token: credentials.client_token.clone(),
            identifier: Uuid::new_v4(),
            profile: profile.clone(),
            textures: Default::default(),
            added_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_millis() as u64)
                .unwrap_or_default(),
        })
        .collect();

    let weak = ui.as_weak();
    crate::runtime::spawn(async move {
        for account in accounts {
            if let Err(error) = slint_account::yggdrasil::add_account(account).await {
                log::error!("failed to add the Yggdrasil account: {error}");
                return;
            }
        }
        let _ = weak.upgrade_in_event_loop(move |ui| finish_add(&ui));
    });
}

/// The Vue's `sameApiRoot`: the same host and port, and the same path with any
/// trailing slashes ignored.
fn same_api_root(a: &str, b: &str) -> bool {
    let (Ok(a), Ok(b)) = (Url::parse(a), Url::parse(b)) else {
        return false;
    };
    a.host_str() == b.host_str()
        && a.port() == b.port()
        && a.path().trim_end_matches('/') == b.path().trim_end_matches('/')
}
