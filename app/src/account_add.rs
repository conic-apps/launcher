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
//! login runs as a task (see `account::LoginTaskState`) whose progress
//! arrives as events, so which of the screen's four states is mounted is
//! decided here rather than in the screen. The browser flow's half of it —
//! waiting for the code to come back — is here too: the screen owns when the
//! listener is wanted (`AccountAddMicrosoft.slint` asks for it as the screen
//! appears and gives it up as it goes), and this module owns what it is bound
//! to and what happens when an answer arrives.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use slint::{ComponentHandle, Model, ModelRc, VecModel, Weak};
use url::Url;
use uuid::Uuid;

use crate::account_avatar;
use crate::slint_backend::{AccountAddState, App, Dialogs, GameState, YggdrasilProfileItem};
use account::{
    Error, LoginRequest, LoginTaskState, get_uuid_from_username,
    microsoft::{LoginEvent, LoginReporter},
    yggdrasil::{self, yggdrasil_user_api::Profile as YggdrasilProfile},
};
use authcode::Outcome;

/// How long the browser flow's listener waits before giving the port back.
///
/// An authorization code issued by Microsoft's `consumers` endpoint is good for
/// ten minutes, and the flow is a human being signing in somewhere else, so the
/// listener outlives anything shorter. It is the ceiling, not a prompt: the
/// listener is released the moment the code arrives, the user switches to the
/// device code, or the dialog closes.
const AUTH_CODE_TIMEOUT: Duration = Duration::from_secs(600);

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

    /// What the Microsoft screen's flow owns from one screen swap to the next.
    static MICROSOFT: RefCell<MicrosoftFlow> = RefCell::new(MicrosoftFlow::default());
}

/// The state the Microsoft screen's two flows carry between callbacks.
///
/// The login task is the Tauri plugin's `PluginState` (see
/// `account::LoginTaskState`): at most one, cancellable, and the only
/// thing that owns a running Microsoft login. The listener is the browser
/// flow's other half, and it lives in the same box because it is released in
/// the same places — leaving the screen, cancelling, closing.
#[derive(Default)]
struct MicrosoftFlow {
    login: LoginTaskState,
    /// The task blocked on the loopback listener the browser flow's code comes
    /// back on, while the browser screen is on show.
    ///
    /// An abort handle and not the [`authcode::AuthCallback`] itself: the
    /// callback moves into `wait` and comes back as an `Outcome`, and the one
    /// thing to be done to it from the UI thread is to stop waiting — which is
    /// what the two ways off this screen have in common.
    callback: Option<tokio::task::AbortHandle>,
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

    // ----- the Microsoft screen -----
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
fn start_microsoft_login(weak: Weak<App>, request: LoginRequest, device_flow: bool) {
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

    // `LoginTaskState` is the at-most-one slot, so this is the one task it is
    // for: taking it here rather than passing it around is what makes "is
    // anything running" a question with one answer.
    let task = MICROSOFT.with(|flow| flow.borrow().login.clone());
    crate::runtime::spawn(async move {
        let result = task.spawn(request, reporter).await;
        let _ = weak.upgrade_in_event_loop(move |ui| match result {
            Ok(_) => finish_add(&ui),
            Err(error) => handle_login_error(&ui, error, device_flow),
        });
    });
}

/// Binds the loopback listener the browser flow's code comes back on, and
/// starts waiting for it.
///
/// Called by the screen as the browser flow comes on (see
/// `AccountAddMicrosoft.slint`), and a no-op when a listener is already bound —
/// the screen asks on every show, and the URL it is showing has to be one that
/// is being served for as long as it is showing. The port is the OS's, so
/// there is no fixed one to fall back to, and a bind failure is the one case
/// that leaves the screen with nothing to offer: the log line says so, and the
/// "Log in" button opens an empty URL, which does nothing.
///
/// Binding is two `bind` calls and needs no runtime, so this runs on the UI
/// thread; the waiting half is a task.
fn prepare_auth_code_flow(ui: &App) {
    if MICROSOFT.with(|flow| flow.borrow().callback.is_some()) {
        return;
    }
    let state = ui.global::<AccountAddState>();
    let callback = match authcode::AuthCallback::start(callback_palette(ui), callback_messages(ui))
    {
        Ok(callback) => callback,
        Err(error) => {
            log::error!("cannot listen for the Microsoft sign-in callback: {error}");
            return;
        }
    };

    // The Vue's `AUTH_CODE_LOGIN_URL`, with the deep link's `redirect_uri`
    // replaced by the listener's and the `state` added — OAuth's own CSRF
    // token, which the callback is checked against so that nothing else on the
    // machine (or on the network, on a machine that forwards) can hand this
    // launcher a code of its own choosing.
    let redirect_uri = callback.redirect_uri();
    state.set_auth_code_login_url(authorize_url(&redirect_uri, callback.state()).into());

    let weak = ui.as_weak();
    // The URI is carried to the token endpoint rather than rebuilt there: RFC
    // 6749 §4.1.3 requires the same bytes the code was issued against, and the
    // port in it is the OS's.
    let for_the_token_request = redirect_uri.clone();
    let waiter = crate::runtime::spawn(async move {
        let outcome = callback.wait(AUTH_CODE_TIMEOUT).await;
        let _ = weak.upgrade_in_event_loop(move |ui| {
            auth_code_flow_finished(&ui, &for_the_token_request, outcome)
        });
    });
    MICROSOFT.with(|flow| flow.borrow_mut().callback = Some(waiter.abort_handle()));
}

/// Gives the port back and takes the URL off the screen.
///
/// The listener is the screen's, so this is the swap's other half: the user
/// switched to the device code, or the dialog closed, and a URL pointing at a
/// socket that has gone away is worse than no URL.
fn release_auth_code_flow(ui: &App) {
    let released = MICROSOFT.with(|flow| flow.borrow_mut().callback.take());
    let Some(waiter) = released else { return };
    // Aborting the task drops the `AuthCallback` with it, sockets and all, so
    // the port is free the moment this returns.
    waiter.abort();
    ui.global::<AccountAddState>()
        .set_auth_code_login_url("".into());
}

/// What the browser's redirect turned out to be.
fn auth_code_flow_finished(ui: &App, redirect_uri: &str, outcome: Outcome) {
    // The listener has served its one request, or given up; either way the port
    // is the screen's to give back. Done first, so a bind of the port again is
    // not waiting behind the release.
    release_auth_code_flow(ui);
    match outcome {
        Outcome::Code(code) => start_microsoft_login(
            ui.as_weak(),
            LoginRequest::AuthCode {
                code,
                redirect_uri: redirect_uri.to_string(),
            },
            false,
        ),
        // A refusal and a timeout both go back to the browser screen rather than
        // to the error one, and neither says anything in the dialog. Two
        // reasons, and they are the reason this flow has a page at all:
        //
        //   * the error screen has no buttons. The Vue's does not either, so
        //     this is not a regression — but it means "Log in" is not
        //     reachable from there, and a message that says to press it would
        //     be a message about a button that is not on the screen;
        //   * the browser has just shown a page — themed, translated, and
        //     carrying Microsoft's own `error_description` for a refusal —
        //     that says what happened and what to do. The dialog saying it a
        //     second time, worse, is the thing worth avoiding.
        //
        // Returning to the browser screen is also what the device-code flow
        // does for the same kind of answer (`DeviceCodeExpired` asks for a new
        // code without a word), and the screen's own handler binds a fresh
        // listener for it, so "Log in" is one click away.
        Outcome::Declined(reason) => {
            log::info!("the browser refused the sign-in: {reason}");
            ui.global::<AccountAddState>()
                .set_ms_view("auth-code".into());
        }
        Outcome::Nothing => {
            log::info!("no browser came back to the sign-in callback in {AUTH_CODE_TIMEOUT:?}");
            ui.global::<AccountAddState>()
                .set_ms_view("auth-code".into());
        }
    }
}

/// The callback page's colours, off the live `Theme` tokens.
///
/// Read on the UI thread, where the global lives, and handed to the listener as
/// plain numbers: the page is rendered on a worker, minutes later, and a Slint
/// handle is not something that can cross.
fn callback_palette(ui: &App) -> authcode::Palette {
    let theme = ui.global::<AccountAddState>().get_auth_code_page_theme();
    authcode::Palette {
        window: rgba(theme.window),
        card: rgba(theme.card),
        card_border: rgba(theme.card_border),
        title: rgba(theme.title),
        body: rgba(theme.body),
        success: rgba(theme.success),
        danger: rgba(theme.danger),
        dark: theme.dark,
        font_family: theme.font_family.to_string(),
    }
}

/// The callback page's sentences, off the `@tr` catalog.
fn callback_messages(ui: &App) -> authcode::Messages {
    let text = ui.global::<AccountAddState>().get_auth_code_page_text();
    authcode::Messages {
        waiting_title: text.waiting_title.to_string(),
        waiting_body: text.waiting_body.to_string(),
        success_title: text.success_title.to_string(),
        success_body: text.success_body.to_string(),
        failure_title: text.failure_title.to_string(),
        failure_body: text.failure_body.to_string(),
        // The tag of the catalog the sentences above came from, not of the
        // config's `language` — see `config_bridge::active_language_tag`.
        language_tag: crate::config_bridge::active_language_tag(),
    }
}

/// A `slint::Color` as the page's own colour, alpha and all.
///
/// The alpha is kept rather than composited: the app's text tokens are
/// `default-text-color.transparentize(0.1)`, and handing the browser the same
/// 10% over the same card colour lands on the same pixel the window does. It
/// is read as `f32` rather than `u8` for the same reason — a `transparentize`
/// is a float until something rounds it, and rounding it here would make the
/// page's text a shade off the window's.
fn rgba(color: slint::Color) -> authcode::Rgba {
    let color = color.to_argb_f32();
    authcode::Rgba::new(
        (color.red * 255.0).round() as u8,
        (color.green * 255.0).round() as u8,
        (color.blue * 255.0).round() as u8,
        color.alpha,
    )
}

/// The authorize URL of the browser flow (the Vue's `AUTH_CODE_LOGIN_URL`),
/// pointed at the loopback listener and carrying its `state`.
///
/// The original's is a constant with a `conic-launcher://` `redirect_uri` in
/// it. This cannot be a constant, for two reasons that are the same reason: the
/// port is the OS's, and the `state` is per login. Everything else — the
/// endpoint, the client id, `response_mode`, `prompt`, the scope — is the Vue's,
/// unchanged, so the two frontends are asking the same question of Microsoft.
///
/// `redirect_uri` is percent-encoded because a whole URL is going into a query
/// value, and the `:` and the two `/` left bare are the difference between a
/// redirect Microsoft accepts and one it does not. It has to come back out of
/// the query byte for byte, which is why the same encoded form is what
/// `account::microsoft::redeem_access_token` is given.
fn authorize_url(redirect_uri: &str, state: &str) -> String {
    format!(
        "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize\
         ?client_id=94a1414e-e9ad-4bda-94f0-3368d979b0cc\
         &response_type=code\
         &redirect_uri={}\
         &response_mode=query\
         &prompt=select_account\
         &scope=XboxLive.signin%20offline_access\
         &state={}",
        urlencoding(redirect_uri),
        urlencoding(state),
    )
}

/// Percent-encodes a URL for a query value, the two characters that need it.
///
/// The authorize URL is hand-built rather than handed to `reqwest`'s serializer
/// because the dialog shows it to the user: it has to be the exact string the
/// browser is sent to, and a query written by a serializer is not the string
/// anyone reads.
fn urlencoding(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(byte))
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
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
fn handle_login_error(ui: &App, error: Error, device_flow: bool) {
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
            start_microsoft_login(ui.as_weak(), LoginRequest::DeviceCode, true);
        }
        error => {
            state.set_ms_error(error.to_string().into());
            state.set_ms_view("error".into());
        }
    }
}

/// Fills the profile chooser (Vue `shouldChooseProfile`).
fn show_profile_chooser(ui: &App, credentials: &PendingYggdrasil) {
    let existing = account::list_accounts();
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
    let accounts: Vec<account::yggdrasil::YggdrasilAccount> = profiles
        .iter()
        .map(|profile| account::yggdrasil::YggdrasilAccount {
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
            if let Err(error) = account::yggdrasil::add_account(account).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The query of a URL, as the pairs it is made of.
    fn query_of(url: &str) -> HashMap<String, String> {
        let (base, query) = url.split_once('?').expect("the authorize url has a query");
        assert_eq!(
            base,
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize"
        );
        query
            .split('&')
            .map(|pair| {
                let (key, value) = pair.split_once('=').expect("every pair has an '='");
                (key.to_string(), value.to_string())
            })
            .collect()
    }

    #[test]
    fn the_authorize_url_points_at_the_listener() {
        let url = authorize_url("http://localhost:53421/callback", "abc123");
        let query = query_of(&url);

        // The one thing that has to be exactly right: Microsoft compares this
        // against the value the token request repeats, byte for byte.
        assert_eq!(
            query["redirect_uri"],
            "http%3A%2F%2Flocalhost%3A53421%2Fcallback"
        );
        // And the deep link's is gone — it is not something this app can serve.
        assert!(!url.contains("conic-launcher"), "{url}");
    }

    #[test]
    fn the_authorize_url_is_the_vue_one_with_two_changes() {
        let query = query_of(&authorize_url("http://localhost:53421/callback", "abc123"));
        // Everything the Vue's `AUTH_CODE_LOGIN_URL` had, unchanged, so both
        // frontends ask Microsoft the same question.
        assert_eq!(query["client_id"], "94a1414e-e9ad-4bda-94f0-3368d979b0cc");
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["response_mode"], "query");
        assert_eq!(query["prompt"], "select_account");
        assert_eq!(query["scope"], "XboxLive.signin%20offline_access");
        // Plus the two this flow needs and the deep link did not.
        assert_eq!(query["state"], "abc123");
    }

    #[test]
    fn a_signed_uri_breaks_the_query_it_is_in() {
        // A `state` that could close the query and add a parameter of its own
        // must not be able to: `state` is minted, but a URL is a URL.
        let query = query_of(&authorize_url(
            "http://localhost:1/callback",
            "a&scope=admin",
        ));
        assert_eq!(query["state"], "a%26scope%3Dadmin");
        assert_eq!(query["scope"], "XboxLive.signin%20offline_access");
    }

    /// The whole of the browser flow's front end, on Slint's own testing
    /// backend: no window, no event loop, no display. What it covers is the one
    /// piece nothing else can reach — the `changed` handler in
    /// `AccountAddMicrosoft.slint` that asks for the listener when the screen
    /// appears and gives it up when it goes. If that handler did not fire the
    /// feature would do nothing at all, silently, and every other test here
    /// would still pass.
    ///
    /// The platform can only be installed once per process, so this test does
    /// its one thing and the rest of them stay free of the UI. (`cargo test`
    /// runs them in threads of one binary; the binding is in a thread-local
    /// anyway, so which thread asks first does not matter.)
    #[test]
    fn the_browser_screen_brings_its_listener_up_and_lets_it_go() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = App::new().expect("the app");
        setup(&ui);

        // Nothing is on screen, so nothing is listening and there is no URL to
        // point anywhere.
        assert!(
            ui.global::<AccountAddState>()
                .get_auth_code_login_url()
                .is_empty()
        );

        // The dialog opens on the Microsoft screen, which is the browser flow.
        // The set posts the change; the screen's `changed` handler runs when
        // the loop is given a turn, which is what `mock_elapsed_time` is.
        ui.global::<Dialogs>().set_account_add_visible(true);
        i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
        let url = ui
            .global::<AccountAddState>()
            .get_auth_code_login_url()
            .to_string();
        let query = query_of(&url);
        // A `localhost` redirect, on a port the OS chose, and not the deep link.
        assert!(
            query["redirect_uri"].starts_with("http%3A%2F%2Flocalhost%3A"),
            "{url}"
        );
        assert!(!query["redirect_uri"].ends_with("%3A0%2Fcallback"), "{url}");
        assert!(!url.contains("conic-launcher"), "{url}");

        // And the socket that URL names is really serving, in the app's own
        // theme: `/` is the page a browser reaches without the OAuth
        // parameters, and it is the one that proves `callback_palette` and
        // `callback_messages` — the two `Theme`/`@tr` reads — reached the
        // template. (The listener is served from a worker, so this is a plain
        // blocking read; the `wait` it is in is on the runtime and does not
        // need the event loop that `init_no_event_loop` turns off.)
        //
        // The `redirect_uri` is percent-encoded, so it is decoded rather than
        // picked apart — which is also the one check that the encoding is
        // reversible, which is what the token request depends on.
        let redirect_uri = query["redirect_uri"]
            .replace("%3A", ":")
            .replace("%2F", "/");
        assert!(
            redirect_uri.starts_with("http://localhost:"),
            "{redirect_uri} (from {url})"
        );
        let port: u16 = redirect_uri
            .trim_start_matches("http://localhost:")
            .split('/')
            .next()
            .and_then(|port| port.parse().ok())
            .unwrap_or_else(|| panic!("no port in {redirect_uri}"));
        let page = fetch(port, "/");
        assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
        assert!(page.contains("<!doctype html>"), "{page}");
        // The waiting page's own sentence, from the `@tr` catalog.
        assert!(page.contains("Finish signing in"), "{page}");
        // And a colour off `Theme`, which the page would have no way to invent.
        assert!(page.contains("--card: rgb("), "{page}");

        // Closing the dialog is the other half: the port goes back and the URL
        // goes with it, rather than being left pointing at a socket that is no
        // longer there.
        ui.global::<Dialogs>().set_account_add_visible(false);
        i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
        assert!(
            ui.global::<AccountAddState>()
                .get_auth_code_login_url()
                .is_empty()
        );

        // The page is in the launcher's language, not the launcher's build
        // language: switching it and bringing the screen back up is what makes
        // the listener bind again, against the new catalog.
        crate::config_bridge::apply_locale("zh_CN");
        ui.global::<Dialogs>().set_account_add_visible(true);
        i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
        let url = ui
            .global::<AccountAddState>()
            .get_auth_code_login_url()
            .to_string();
        let redirect_uri = query_of(&url)["redirect_uri"]
            .replace("%3A", ":")
            .replace("%2F", "/");
        let port: u16 = redirect_uri
            .trim_start_matches("http://localhost:")
            .split('/')
            .next()
            .and_then(|port| port.parse().ok())
            .expect("a port in the redirect uri");
        let page = fetch(port, "/");
        assert!(page.contains(r#"lang="zh-CN""#), "{page}");
        assert!(page.contains("正在等待登录"), "{page}");
        assert!(!page.contains("Waiting for the sign-in"), "{page}");
    }

    /// One `GET` to the callback the app just bound, and the whole response.
    ///
    /// Bounded, because a listener that is not being served is a test failure
    /// and a blocking read would turn that into a hung test binary.
    fn fetch(port: u16, target: &str) -> String {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("the listener");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a read timeout");
        stream
            .write_all(
                format!("GET {target} HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n").as_bytes(),
            )
            .expect("write");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("the page, which the listener closes after answering");
        response
    }

    #[test]
    fn the_url_has_no_stray_whitespace_from_its_own_formatting() {
        // The literal is written across lines, which Rust folds; a space left at
        // the seam would be a parameter name the endpoint does not know.
        let url = authorize_url("http://localhost:1/callback", "abc");
        assert!(!url.contains(' '), "{url}");
        assert!(!url.contains('\n'), "{url}");
        assert!(
            url.starts_with("https://login.microsoftonline.com/"),
            "{url}"
        );
    }
}
