// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The listener over a real socket: a real bind, a real `connect`, a real
//! request line and a real response. The framing in `http` and the page in
//! `page` are unit-tested against strings; this is the part only a socket can
//! answer — that the port the redirect carries is the port the code comes back
//! to, and that anything else on the loopback cannot end the login.

use std::time::Duration;

use account::authcode::{AuthCallback, Messages, Outcome, Palette, Rgba};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;

/// Generous: a test on a loaded machine is not a 5-second budget argument.
const GENEROUS: Duration = Duration::from_secs(5);

/// Short, for the cases that assert the listener is *still* listening.
const BRIEF: Duration = Duration::from_millis(400);

fn palette() -> Palette {
    Palette {
        window: Rgba::new(0x11, 0x11, 0x1b, 1.0),
        card: Rgba::new(0x1e, 0x1e, 0x2e, 1.0),
        card_border: Rgba::new(0x38, 0x3b, 0x41, 1.0),
        title: Rgba::new(0xcd, 0xd6, 0xf4, 0.9),
        body: Rgba::new(0xcd, 0xd6, 0xf4, 0.7),
        success: Rgba::new(0xa6, 0xe3, 0xa1, 1.0),
        danger: Rgba::new(0xf3, 0x8b, 0xa8, 1.0),
        dark: true,
        font_family: "system-ui, sans-serif".to_string(),
    }
}

fn messages() -> Messages {
    Messages {
        waiting_title: "Still waiting".to_string(),
        waiting_body: "Finish signing in the browser.".to_string(),
        success_title: "Signed in".to_string(),
        success_body: "You can close this tab.".to_string(),
        failure_title: "Sign-in failed".to_string(),
        failure_body: "The launcher did not get a valid sign-in.".to_string(),
        language_tag: "en-US".to_string(),
    }
}

fn callback() -> AuthCallback {
    AuthCallback::start(palette(), messages()).expect("the loopback interface is available")
}

/// Where a callback is listening, in the two forms the tests need.
#[derive(Clone)]
struct Endpoint {
    host: String,
    port: u16,
}

impl Endpoint {
    fn of(callback: &AuthCallback) -> Self {
        let authority = callback
            .redirect_uri()
            .strip_prefix("http://")
            .expect("the redirect uri is a loopback http url")
            .split('/')
            .next()
            .expect("the uri is never empty")
            .to_string();
        let (host, port) = authority
            .split_once(':')
            .expect("the redirect uri carries a port");
        Self {
            host: host.to_string(),
            port: port.parse().expect("the port is a number"),
        }
    }

    /// The absolute form of `target`, which is what the listener matches paths
    /// against after dropping the authority.
    fn url(&self, target: &str) -> String {
        format!("http://{}:{}{target}", self.host, self.port)
    }
}

/// Starts serving and hands back the answer, the way the launcher does: the
/// `wait` is already running by the time the browser is sent anywhere.
fn serving(callback: AuthCallback, timeout: Duration) -> oneshot::Receiver<Outcome> {
    let (sender, receiver) = oneshot::channel();
    tokio::spawn(async move {
        let _ = sender.send(callback.wait(timeout).await);
    });
    receiver
}

/// Sends one `GET` and reads the whole response back.
///
/// A connection that is refused comes back as an empty string, which is what a
/// listener that has gone away looks like from the other end.
async fn get(endpoint: &Endpoint, target: &str) -> String {
    let Ok(mut stream) = TcpStream::connect((endpoint.host.as_str(), endpoint.port)).await else {
        return String::new();
    };
    // The origin form, which is what a browser sends for a redirect.
    stream
        .write_all(
            format!(
                "GET {target} HTTP/1.1\r\nHost: {}:{}\r\nAccept: */*\r\n\r\n",
                endpoint.host, endpoint.port
            )
            .as_bytes(),
        )
        .await
        .expect("write");
    let mut response = Vec::new();
    // The listener closes after answering, so the read ends on its own.
    tokio::time::timeout(GENEROUS, stream.read_to_end(&mut response))
        .await
        .expect("the response arrived")
        .expect("the read succeeded");
    String::from_utf8_lossy(&response).into_owned()
}

/// The body of a response, which is where the page is.
fn body(response: &str) -> &str {
    response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("a response has a head and a body")
}

#[tokio::test]
async fn a_callback_with_our_state_hands_over_the_code() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let target = format!("/callback?code=0.AAe-9_x&state={}", callback.state());
    let answer = serving(callback, GENEROUS);

    let response = get(&endpoint, &target).await;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(
        response.contains("Content-Type: text/html; charset=utf-8"),
        "{response}"
    );
    assert!(body(&response).contains("Signed in"), "{response}");
    // The `Content-Length` has to agree with what followed it, or the browser
    // either truncates the page or waits for bytes that never come.
    let declared = response
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .and_then(|length| length.trim().parse::<usize>().ok())
        .expect("a Content-Length");
    assert_eq!(declared, body(&response).len(), "{response}");

    assert_eq!(
        answer.await.unwrap(),
        Outcome::Code("0.AAe-9_x".to_string())
    );
}

#[tokio::test]
async fn the_absolute_form_names_the_same_resource() {
    // What `curl` sends, and what anything going through a proxy sends.
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let uri = endpoint.url(&format!("/callback?code=abs&state={}", callback.state()));
    let answer = serving(callback, GENEROUS);

    let response = get(&endpoint, &uri).await;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert_eq!(answer.await.unwrap(), Outcome::Code("abs".to_string()));
}

#[tokio::test]
async fn a_callback_with_someone_elses_state_is_refused_and_does_not_end_the_login() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let answer = serving(callback, BRIEF);

    let response = get(&endpoint, "/callback?code=stolen&state=not-ours").await;
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    assert!(body(&response).contains("Sign-in failed"), "{response}");

    // Still listening: the real callback, if one is coming, is unaffected.
    assert_eq!(answer.await.unwrap(), Outcome::Nothing);
}

#[tokio::test]
async fn a_refusal_carries_the_reason_and_ends_the_login() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let target = format!(
        "/callback?error=access_denied&error_description=The+user+said+no&state={}",
        callback.state()
    );
    let answer = serving(callback, GENEROUS);

    let response = get(&endpoint, &target).await;
    // A refusal is a perfectly good answer, so the page the browser gets is a
    // 200: there is nothing wrong with the request.
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(body(&response).contains("Sign-in failed"), "{response}");
    assert_eq!(
        answer.await.unwrap(),
        Outcome::Declined("The user said no".to_string())
    );
}

#[tokio::test]
async fn a_refusal_without_a_description_still_says_something() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let target = format!("/callback?error=access_denied&state={}", callback.state());
    let answer = serving(callback, GENEROUS);

    get(&endpoint, &target).await;
    assert_eq!(
        answer.await.unwrap(),
        Outcome::Declined("access_denied".to_string())
    );
}

#[tokio::test]
async fn the_root_says_to_go_on_waiting_and_ends_nothing() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let answer = serving(callback, BRIEF);

    let response = get(&endpoint, "/").await;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(body(&response).contains("Still waiting"), "{response}");
    assert_eq!(answer.await.unwrap(), Outcome::Nothing);
}

#[tokio::test]
async fn a_favicon_request_is_not_a_failure() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let answer = serving(callback, BRIEF);

    let response = get(&endpoint, "/favicon.ico").await;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert_eq!(answer.await.unwrap(), Outcome::Nothing);
}

#[tokio::test]
async fn an_aborted_wait_gives_the_port_back() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let waiter = tokio::spawn(async move {
        callback.wait(GENEROUS).await;
    });

    // Up before anything asks for the port to be free.
    let response = get(&endpoint, "/").await;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");

    // The dialog closing aborts the waiter the same way it does in the app.
    waiter.abort();
    let _ = waiter.await;

    // Reaping the aborted wait is the same cancellation turn; give the
    // released sockets a moment to show up as refused from the other end.
    let mut freed = String::new();
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        freed = get(&endpoint, "/").await;
        if freed.is_empty() {
            break;
        }
    }
    assert!(freed.is_empty(), "the port is still taken: {freed}");
}

#[tokio::test]
async fn only_the_first_code_ends_the_login() {
    let (endpoint, replay) = {
        let callback = callback();
        let endpoint = Endpoint::of(&callback);
        let state = callback.state().to_string();
        let replay = format!("/callback?code=second&state={state}");
        let answer = serving(callback, GENEROUS);

        assert!(
            get(&endpoint, &format!("/callback?code=first&state={state}"))
                .await
                .contains("200 OK")
        );
        assert_eq!(answer.await.unwrap(), Outcome::Code("first".to_string()));
        (endpoint, replay)
    };

    // `wait` has returned, so the listeners are gone: a replay of the same
    // callback is a refused connection rather than a second sign-in.
    assert!(get(&endpoint, &replay).await.is_empty());
    assert!(
        TcpStream::connect((endpoint.host.as_str(), endpoint.port))
            .await
            .is_err(),
        "the listener is still accepting after the code was taken"
    );
}

/// Sends one bare request line and reads the whole response back, for the
/// methods `get` does not cover.
async fn request_line(endpoint: &Endpoint, line: &[u8]) -> String {
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
        .await
        .expect("connect");
    stream.write_all(line).await.expect("write");
    let mut response = Vec::new();
    tokio::time::timeout(GENEROUS, stream.read_to_end(&mut response))
        .await
        .expect("the response arrived")
        .expect("the read succeeded");
    String::from_utf8_lossy(&response).into_owned()
}

#[tokio::test]
async fn a_method_that_is_not_a_get_or_a_head_is_answered_and_ends_nothing() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let answer = serving(callback, BRIEF);

    let response = request_line(&endpoint, b"POST /callback HTTP/1.1\r\nHost: x\r\n\r\n").await;
    assert!(
        response.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"),
        "{response}"
    );
    // The one response that lists what this listener does allow.
    assert!(response.contains("Allow: GET"), "{response}");
    // And, unlike a `HEAD`'s, it may carry the page.
    assert!(body(&response).contains("Sign-in failed"), "{response}");
    assert_eq!(answer.await.unwrap(), Outcome::Nothing);
}

#[tokio::test]
async fn a_head_gets_the_status_without_a_body_it_could_not_read() {
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    let answer = serving(callback, BRIEF);

    // On `/`, which is what "is anything listening?" asks about.
    let response = request_line(&endpoint, b"HEAD / HTTP/1.1\r\nHost: x\r\n\r\n").await;
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    // The trap this avoids: a `Content-Length` for bytes the client is not
    // allowed to read, and will sit waiting for until it gives up.
    assert_eq!(body(&response), "", "{response}");
    assert_eq!(answer.await.unwrap(), Outcome::Nothing);
}

#[tokio::test]
async fn a_dropped_callback_releases_its_port() {
    let (host, port) = {
        let callback = callback();
        let endpoint = Endpoint::of(&callback);
        // `callback` goes out of scope here, sockets and all: the user closed
        // the dialog, or went back to the device code.
        (endpoint.host, endpoint.port)
    };
    // Nothing here ever connected *to* the listener, so there is no TIME_WAIT
    // of our own to wait out — the port is free the moment the socket is
    // dropped, which is what makes the OS-chosen port safe to hand out again.
    std::net::TcpListener::bind((host.as_str(), port))
        .expect("the port was released when the callback was dropped");
}

#[test]
fn the_port_is_one_the_os_picked_and_the_uri_names_it() {
    // Not an assertion about a specific number — that is the point: there is
    // none. Two callbacks must be able to exist at once, neither of them having
    // to give way, and neither of them knowing the other's port.
    let first = callback();
    let second = callback();
    let first_endpoint = Endpoint::of(&first);
    let second_endpoint = Endpoint::of(&second);

    assert_ne!(first_endpoint.port, 0);
    assert_ne!(second_endpoint.port, 0);
    assert_ne!(first_endpoint.port, second_endpoint.port);
    assert!(first.redirect_uri().ends_with("/callback"));
    assert_eq!(
        first_endpoint.host, "localhost",
        "what `redirect_uri` names"
    );
    assert_eq!(first.state().len(), 32, "a uuid without its dashes");
    assert_ne!(first.state(), second.state(), "two logins, two states");
}

#[test]
fn the_host_the_redirect_names_is_the_one_that_answers() {
    // `localhost` is in the redirect because that is the host Microsoft's
    // loopback rule is written for; this is the socket it has to reach. The
    // listener binds the IPv4 loopback, and `localhost` resolves to it on every
    // machine that can serve the browser at all.
    let callback = callback();
    let endpoint = Endpoint::of(&callback);
    std::net::TcpStream::connect((endpoint.host.as_str(), endpoint.port))
        .expect("the IPv4 loopback is bound");
}
