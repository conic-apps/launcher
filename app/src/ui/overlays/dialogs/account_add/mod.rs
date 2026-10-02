// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The add-account dialog's "script": drives `account` from the dialog's state
//! and its four screens.
//!
//! Like `create_instance.rs`, everything that touches the network or the disk
//! runs on the tokio runtime (`crate::support::runtime`) and reports back through
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

use crate::slint_backend::{AccountAddState, App, Dialogs, GameState, YggdrasilProfileItem};
use crate::ui::components::account_avatar;
use account::authcode::Outcome;
use account::{
    Error, LoginRequest, LoginTaskState, get_uuid_from_username,
    microsoft::{LoginEvent, LoginReporter},
    yggdrasil::{self, yggdrasil_user_api::Profile as YggdrasilProfile},
};

pub(crate) mod login;
mod wiring;

pub(crate) use login::*;
pub(crate) use wiring::*;

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

    /// The credentials the chooser's rows belong to.
    static PENDING: RefCell<Option<PendingYggdrasil>> = const { RefCell::new(None) };

    /// What the Microsoft screen's flow owns from one screen swap to the next.
    static MICROSOFT: RefCell<MicrosoftFlow> = RefCell::new(MicrosoftFlow::default());
}

/// The state the Microsoft screen's two flows carry between callbacks.
///
/// The login task (see `account::LoginTaskState`) is at most one, cancellable,
/// and the only thing that owns a running Microsoft login. The listener is the
/// browser flow's other half, and it lives in the same box because it is
/// released in the same places — leaving the screen, cancelling, closing.
#[derive(Default)]
pub(crate) struct MicrosoftFlow {
    login: LoginTaskState,
    /// The task blocked on the loopback listener the browser flow's code comes
    /// back on, while the browser screen is on show.
    ///
    /// An abort handle and not the [`account::authcode::AuthCallback`] itself: the
    /// callback moves into `wait` and comes back as an `Outcome`, and the one
    /// thing to be done to it from the UI thread is to stop waiting — which is
    /// what the two ways off this screen have in common.
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
        // A `localhost` redirect, on a port the OS chose, not a
        // `conic-launcher://` one.
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
