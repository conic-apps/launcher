// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The Microsoft and Yggdrasil login flows behind those callbacks.

use super::*;

use account::authcode;

/// Recomputes whether the offline form can be submitted. The test trims the
/// username; Slint has no `trim`, so it lives here.
pub(crate) fn refresh_offline_submit(state: &AccountAddState) {
    let username = state.get_offline_username().trim().to_string();
    state.set_offline_can_submit(!username.is_empty() && !state.get_offline_uuid_invalid());
}

/// The same for the Yggdrasil form, whose three fields are trimmed as well.
pub(crate) fn refresh_yggdrasil_submit(state: &AccountAddState) {
    let ready = [
        state.get_yggdrasil_api_root(),
        state.get_yggdrasil_username(),
        state.get_yggdrasil_password(),
    ]
    .iter()
    .all(|value| !value.trim().is_empty());
    state.set_yggdrasil_can_submit(ready);
}

/// Picks a default account, reloads the account list and dismisses the dialog
/// after every successful add.
///
/// The default is picked through a `GameState` callback rather than here: the
/// config lives on the UI thread, and `upgrade_in_event_loop`'s closure has to
/// be `Send`, so a worker cannot carry it.
pub(crate) fn finish_add(ui: &App) {
    ui.global::<GameState>().invoke_select_first_account();
    ui.global::<GameState>().invoke_refresh();
    ui.global::<Dialogs>().set_account_add_visible(false);
}

/// Names the step a login event reports, as `AccountAddMicrosoft`'s
/// `progress-text` spells it.
///
/// The event itself never reaches the UI: the sentence for each step lives in
/// the `.slint` file, so a language change re-renders it.
pub(crate) fn progress_kind(event: &LoginEvent) -> Option<&'static str> {
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

/// Runs the login task and reports its outcome back into the dialog.
pub(crate) fn start_microsoft_login(weak: Weak<App>, request: LoginRequest, device_flow: bool) {
    let Some(ui) = weak.upgrade() else { return };
    let state = ui.global::<AccountAddState>();
    state.set_ms_view("processing".into());
    state.set_ms_progress("prepare".into());
    state.set_ms_error("".into());

    let reporter = {
        let weak = weak.clone();
        LoginReporter::new(move |event| {
            let weak = weak.clone();
            crate::report::report(&weak, move |ui| apply_login_event(&ui, event));
        })
    };

    // `LoginTaskState` is the at-most-one slot, so this is the one task it is
    // for: taking it here rather than passing it around is what makes "is
    // anything running" a question with one answer.
    let task = MICROSOFT.with(|flow| flow.borrow().login.clone());
    crate::runtime::spawn(async move {
        let result = task.spawn(request, reporter).await;
        crate::report::report(&weak, move |ui| match result {
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
pub(crate) fn prepare_auth_code_flow(ui: &App) {
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

    // The authorize URL carries the listener's `redirect_uri`, and the `state`
    // is added — OAuth's own CSRF token, which the callback is checked against
    // so that nothing else on the machine (or on the network, on a machine that
    // forwards) can hand this launcher a code of its own choosing.
    let redirect_uri = callback.redirect_uri();
    state.set_auth_code_login_url(authorize_url(&redirect_uri, callback.state()).into());

    let weak = ui.as_weak();
    // The URI is carried to the token endpoint rather than rebuilt there: RFC
    // 6749 §4.1.3 requires the same bytes the code was issued against, and the
    // port in it is the OS's.
    let for_the_token_request = redirect_uri.clone();
    let waiter = crate::runtime::spawn(async move {
        let outcome = callback.wait(AUTH_CODE_TIMEOUT).await;
        crate::report::report(&weak, move |ui| {
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
pub(crate) fn release_auth_code_flow(ui: &App) {
    let released = MICROSOFT.with(|flow| flow.borrow_mut().callback.take());
    let Some(waiter) = released else { return };
    // Aborting the task drops the `AuthCallback` with it, sockets and all, so
    // the port is free the moment this returns.
    waiter.abort();
    ui.global::<AccountAddState>()
        .set_auth_code_login_url("".into());
}

/// What the browser's redirect turned out to be.
pub(crate) fn auth_code_flow_finished(ui: &App, redirect_uri: &str, outcome: Outcome) {
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
        //   * the error screen has no buttons, so "Log in" is not reachable
        //     from there, and a message that says to press it would be a
        //     message about a button that is not on the screen;
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
pub(crate) fn callback_palette(ui: &App) -> authcode::Palette {
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
pub(crate) fn callback_messages(ui: &App) -> authcode::Messages {
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
pub(crate) fn rgba(color: slint::Color) -> authcode::Rgba {
    let color = color.to_argb_f32();
    authcode::Rgba::new(
        (color.red * 255.0).round() as u8,
        (color.green * 255.0).round() as u8,
        (color.blue * 255.0).round() as u8,
        color.alpha,
    )
}

/// The authorize URL of the browser flow, pointed at the loopback listener and
/// carrying its `state`.
///
/// The endpoint, the client id, `response_mode`, `prompt` and the scope are
/// fixed; the `redirect_uri` and the `state` are the two that vary. That is why
/// this cannot be a constant: the port is the OS's, and the `state` is per
/// login.
///
/// `redirect_uri` is percent-encoded because a whole URL is going into a query
/// value; it has to come back out of the query byte for byte, which is why the
/// same encoded form is what `account::microsoft::redeem_access_token` is
/// given.
pub(crate) fn authorize_url(redirect_uri: &str, state: &str) -> String {
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

/// Percent-encodes a string for a query value: every byte outside the
/// unreserved set (`A-Z a-z 0-9 - _ . ~`) becomes a `%XX` escape.
///
/// The authorize URL is hand-built rather than handed to `reqwest`'s serializer
/// because the dialog shows it to the user: it has to be the exact string the
/// browser is sent to, and a query written by a serializer is not the string
/// anyone reads.
pub(crate) fn urlencoding(value: &str) -> String {
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
pub(crate) fn apply_login_event(ui: &App, event: LoginEvent) {
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
        // The code goes straight to the clipboard, so the user only has to
        // paste it.
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

/// Handles a failed login.
pub(crate) fn handle_login_error(ui: &App, error: Error, device_flow: bool) {
    let state = ui.global::<AccountAddState>();
    match error {
        // The user cancelled: silently return to the screen it was on, which
        // for a device login is the code it is still showing.
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
        // Ask for a new code and carry on.
        Error::DeviceCodeExpired if device_flow => {
            start_microsoft_login(ui.as_weak(), LoginRequest::DeviceCode, true);
        }
        error => {
            state.set_ms_error(error.to_string().into());
            state.set_ms_view("error".into());
        }
    }
}

/// Fills the profile chooser.
pub(crate) fn show_profile_chooser(ui: &App, credentials: &PendingYggdrasil) {
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
                // The same profile on the same server is already added.
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

/// Stores one account per chosen profile.
///
/// Every profile gets its own identifier and its own copy of the credentials.
pub(crate) fn add_yggdrasil_accounts(
    ui: &App,
    credentials: &PendingYggdrasil,
    profiles: &[YggdrasilProfile],
) {
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
        crate::report::report(&weak, move |ui| finish_add(&ui));
    });
}

/// Compares API roots: the same host and port, and the same path with any
/// trailing slashes ignored.
pub(crate) fn same_api_root(a: &str, b: &str) -> bool {
    let (Ok(a), Ok(b)) = (Url::parse(a), Url::parse(b)) else {
        return false;
    };
    a.host_str() == b.host_str()
        && a.port() == b.port()
        && a.path().trim_end_matches('/') == b.path().trim_end_matches('/')
}
