// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The add-account dialog: drives `account` from the dialog's state and its four
//! screens. Network and disk work runs on the tokio runtime and reports through
//! `crate::ui::services::report`; the Microsoft login runs as a task whose events
//! decide which screen is mounted, and this module owns the browser listener.

use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use slint::{ComponentHandle, Model, ModelRc, VecModel, Weak};
use uuid::Uuid;

use crate::slint_backend::{AccountAddState, App, Dialogs, GameState, YggdrasilProfileItem};
use crate::ui::components::account_avatar;
use account::authcode::BrowserRedirectOutcome;
use account::{
    Error, LoginRequest, LoginTaskState, get_uuid_from_username,
    microsoft::{LoginEvent, LoginReporter},
    yggdrasil::{self, yggdrasil_user_api::Profile as YggdrasilProfile},
};

pub(crate) mod login;
mod wiring;

pub(crate) use login::*;
pub(crate) use wiring::*;

/// The authorization code is good for ten minutes; the listener is released
/// sooner the moment the code arrives, the user switches to the device code, or
/// the dialog closes.
const AUTH_CODE_TIMEOUT: Duration = Duration::from_secs(600);

thread_local! {
    /// Kept across the dialog's screens so a click can flip one row in place.
    /// It is a thread-local rather than an `Rc` captured by callbacks because
    /// the UI half of `crate::ui::services::report` has to be `Send`.
    static PROFILES: Rc<VecModel<YggdrasilProfileItem>> = Rc::new(VecModel::default());

    /// The credentials the chooser's rows belong to.
    static PENDING: RefCell<Option<PendingYggdrasil>> = const { RefCell::new(None) };

    static MICROSOFT: RefCell<MicrosoftFlow> = RefCell::new(MicrosoftFlow::default());
}

/// The state the Microsoft screen's two flows carry between callbacks.
///
/// The login task is at most one and cancellable; the listener lives in the same
/// box because it is released in the same places — leaving the screen,
/// cancelling, closing.
#[derive(Default)]
pub(crate) struct MicrosoftFlow {
    login: LoginTaskState,
    /// An abort handle rather than the callback itself, which moves into `wait`;
    /// the only thing the UI thread can do to it is stop waiting.
    callback: Option<tokio::task::AbortHandle>,
}

/// The credentials and profiles of a successful Yggdrasil sign-in, held between
/// the form and the profile chooser.
#[derive(Clone)]
pub(crate) struct PendingYggdrasil {
    api_root: String,
    username: String,
    access_token: String,
    client_token: String,
    /// `availableProfiles`, sorted by name (the crate reorders whatever the
    /// server answered with).
    profiles: Vec<YggdrasilProfile>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
        // And no `conic-launcher://` deep link is in it — it is not something
        // this app can serve.
        assert!(!url.contains("conic-launcher"), "{url}");
    }

    #[test]
    fn the_authorize_url_is_the_vue_one_with_two_changes() {
        let query = query_of(&authorize_url("http://localhost:53421/callback", "abc123"));
        // The endpoint, the client id, `response_mode`, `prompt` and the scope,
        // unchanged.
        assert_eq!(query["client_id"], "94a1414e-e9ad-4bda-94f0-3368d979b0cc");
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["response_mode"], "query");
        assert_eq!(query["prompt"], "select_account");
        assert_eq!(query["scope"], "XboxLive.signin%20offline_access");
        // Plus the `state` this flow adds.
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

    /// The whole of the browser flow's front end, on Slint's own testing backend:
    /// no window, no event loop, no display. It covers the one piece nothing else
    /// can reach — the `changed` handler in `AccountAddMicrosoft.slint` that binds
    /// the listener when the screen appears and gives it up when it goes. The
    /// platform can only be installed once per process, so this test does its one
    /// thing and the rest stay free of the UI.
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
        // A `localhost` redirect, on a port the OS chose, not a
        // `conic-launcher://` one.
        assert!(
            query["redirect_uri"].starts_with("http%3A%2F%2Flocalhost%3A"),
            "{url}"
        );
        assert!(!query["redirect_uri"].ends_with("%3A0%2Fcallback"), "{url}");
        assert!(!url.contains("conic-launcher"), "{url}");

        // And the socket that URL names is really serving, in the app's own theme:
        // `/` is the page a browser reaches without the OAuth parameters, which is
        // what proves `callback_palette` and `callback_messages` reached it. The
        // `redirect_uri` is decoded rather than picked apart, so that also checks the
        // encoding is reversible.
        //
        // The listener is served from a worker, so this is a plain blocking read.
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
        crate::ui::services::app_config::apply_locale("zh_CN");
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
