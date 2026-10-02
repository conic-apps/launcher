// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The loopback listener the Microsoft browser flow hands its authorization
//! code to.
//!
//! # Why a listener at all
//!
//! The authorization-code flow ends with the browser being redirected to a URI
//! the app is expected to receive. The alternative is a custom URI scheme
//! (`conic-launcher://oauth2/microsoft/callback`) registered with the desktop
//! environment, which means claiming single-instance ownership of it and
//! trusting the platform to route it — something each of the three desktops
//! the launcher ships on does differently. Microsoft's own guidance for a
//! native app is a loopback redirect instead, and that is what this is: the
//! browser is sent to `http://localhost:<port>/callback`, and the code arrives
//! as an ordinary `GET` to a socket this process already holds.
//!
//! # Why so little of a server
//!
//! It answers one request and goes away. A web framework would bring a
//! dependency tree, a router, a middleware stack and a connection state
//! machine for a single `GET` carrying a code in its query string, so the
//! framing is `http`'s: a request line, a status line, a body. All of it is
//! bounded — a loopback listener is reachable by every process on the machine
//! — and the bounds are in `http`.
//!
//! # Ports
//!
//! The socket is bound to port 0 and the port the OS hands back is read off
//! the bound address, so there is no port to collide over: a second launcher,
//! a leftover listener, or anything else already on the port cannot stop the
//! flow. Microsoft accepts `http://localhost` on any port for a native client,
//! so the port never has to be a fixed one either.
//!
//! `localhost` resolves to `127.0.0.1` or `::1` depending on the browser and
//! the machine, and a listener on one of them does not answer for the other.
//! The socket is bound on the IPv4 loopback and, if that same port also comes
//! free on the IPv6 loopback, on both — one port, either address. A machine
//! that can only do one of them keeps the one it can.

mod error;
mod http;
pub mod page;

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{debug, info, warn};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use uuid::Uuid;

pub use error::{Error, Result};
pub use page::{Messages, Palette, Rgba, Screen};

/// The path the browser is redirected to, and the only one that can end the
/// login; every other path only gets a page.
const CALLBACK_PATH: &str = "/callback";

/// What the browser's redirect turned out to be.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The browser came back with an authorization code, and it belongs to
    /// this listener: its `state` matched.
    Code(String),
    /// The sign-in was refused — by the user at Microsoft's consent screen, or
    /// by Microsoft itself. Carries `error_description` where there was one and
    /// `error` otherwise, which is what the caller's log line records; the
    /// browser page itself only ever shows the generic failure sentence.
    Declined(String),
    /// Nothing usable came back: the caller's deadline passed, or the listener
    /// could not go on accepting.
    Nothing,
}

/// A bound, not-yet-serving loopback callback.
///
/// [`start`](Self::start) binds and [`wait`](Self::wait) serves, and they are
/// separate for the caller's sake: the dialog needs the URL the moment it
/// draws the browser screen, and binding a loopback socket needs nothing but a
/// `std` socket, so it can happen on the UI thread. `wait` is the half that
/// blocks on a socket, and it is the half that needs a runtime. A connection
/// that arrives in between waits in the kernel's backlog, so nothing is lost
/// by the ordering.
pub struct AuthCallback {
    listeners: Vec<std::net::TcpListener>,
    port: u16,
    outcome: oneshot::Receiver<Outcome>,
    shared: Arc<Shared>,
}

impl AuthCallback {
    /// Binds the loopback socket and mints the `state` this callback answers
    /// to.
    ///
    /// Everything here is a `bind` and a `Uuid`, so this is callable from the
    /// UI thread: the sockets stay `std` until [`wait`](Self::wait) hands them
    /// to a runtime, which is what keeps this one off the async side.
    pub fn start(palette: Palette, messages: Messages) -> Result<Self> {
        let (listeners, port) = bind_loopback()?;
        // The CSRF half of OAuth, and the reason the callback cannot simply
        // take whatever arrives on the port. `state` is round-tripped through
        // the browser by Microsoft untouched, so a page that guessed the port
        // — a port scan, or another process on the machine — cannot hand this
        // launcher a code of its own, and a replay of an earlier callback
        // cannot either. It lives in the `Shared` because that is where it is
        // compared, and the caller's copy is a read of the same one.
        let (sender, outcome) = oneshot::channel();
        let shared = Arc::new(Shared {
            state: Uuid::new_v4().simple().to_string(),
            sender: Mutex::new(Some(sender)),
            finished: AtomicBool::new(false),
            page: Page { palette, messages },
        });
        info!("listening for the Microsoft sign-in callback on port {port}");
        Ok(Self {
            listeners,
            port,
            outcome,
            shared,
        })
    }

    /// The `redirect_uri` the authorize request has to carry. The token request
    /// has to repeat it byte for byte, which is why it is kept as one string
    /// from here all the way to the token endpoint.
    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}{CALLBACK_PATH}", self.port)
    }

    /// The `state` the callback has to come back with.
    pub fn state(&self) -> &str {
        &self.shared.state
    }

    /// Serves until an answer arrives or the deadline passes.
    ///
    /// Consuming `self` is what ties the listener's life to the login's: a
    /// `wait` that is never awaited — the user closed the dialog, or went back
    /// to the device code — drops the sockets with it, and the port is free
    /// again at once.
    ///
    /// This is the half that needs a runtime, and the first thing it does is
    /// hand the `std` sockets over to one: a socket only becomes an async one
    /// inside a reactor.
    pub async fn wait(self, timeout: Duration) -> Outcome {
        let Self {
            listeners,
            outcome,
            shared,
            ..
        } = self;
        let mut loops = Vec::with_capacity(listeners.len());
        for listener in listeners {
            // A socket this crate bound and put in non-blocking mode is one
            // `from_std` accepts, so this cannot be reached in practice — but a
            // listener that never gets served would hold the login for the
            // whole deadline, so it is worth saying so out loud.
            match TcpListener::from_std(listener) {
                Ok(listener) => {
                    let shared = Arc::clone(&shared);
                    loops.push(tokio::spawn(accept(listener, shared)));
                }
                Err(error) => warn!("failed to serve the callback listener: {error}"),
            }
        }
        if loops.is_empty() {
            shared.finish();
            return Outcome::Nothing;
        }

        let outcome = match tokio::time::timeout(timeout, outcome).await {
            Ok(Ok(outcome)) => outcome,
            // Every accept loop gave up, or the deadline passed. Both mean the
            // same thing here: no code arrived.
            Ok(Err(_)) | Err(_) => Outcome::Nothing,
        };
        shared.finish();
        for accept_loop in loops {
            accept_loop.abort();
        }
        outcome
    }
}

/// The state every accept loop and every connection shares.
struct Shared {
    state: String,
    sender: Mutex<Option<oneshot::Sender<Outcome>>>,
    /// Set as soon as the login has its answer, so the accept loops stop and
    /// anything still in flight knows not to answer twice.
    finished: AtomicBool,
    page: Page,
}

impl Shared {
    /// Hands `outcome` to the waiting login. At most once: the first `take` to
    /// find a sender wins, and every later call finds none.
    fn deliver(&self, outcome: Outcome) {
        let sender = self.sender.lock().expect("Internal error").take();
        if let Some(sender) = sender {
            self.finished.store(true, Ordering::SeqCst);
            let _ = sender.send(outcome);
        }
    }

    fn finish(&self) {
        self.finished.store(true, Ordering::SeqCst);
    }

    /// The page one browser is to be shown, rendered on the spot rather than at
    /// bind time: a request can be minutes after the bind, and rendering it is
    /// one pass over the template.
    fn render(&self, screen: Screen) -> String {
        page::render(screen, &self.page.palette, &self.page.messages)
    }
}

/// The palette and the words, snapshotted when the listener was bound.
struct Page {
    palette: Palette,
    messages: Messages,
}

/// Binds the loopback interface on one port, on every address family that
/// port comes free on.
///
/// The first bind is the one that picks the port and the one whose failure is
/// fatal: a machine with no IPv4 loopback cannot serve the browser at all, so
/// there is nothing to fall back to. The second is a bonus, and a failure to
/// take it is a debug line.
fn bind_loopback() -> Result<(Vec<std::net::TcpListener>, u16)> {
    let ipv4 = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(Error::Bind)?;
    let port = ipv4
        .local_addr()
        .map(|address| address.port())
        .map_err(Error::Bind)?;
    ipv4.set_nonblocking(true).map_err(Error::Bind)?;
    let mut listeners = vec![ipv4];

    match std::net::TcpListener::bind((Ipv6Addr::LOCALHOST, port)) {
        Ok(ipv6) => {
            ipv6.set_nonblocking(true).map_err(Error::Bind)?;
            listeners.push(ipv6);
        }
        Err(error) => {
            // Normal on an IPv4-only machine, and normal on a machine where
            // something transient holds the port in the other family. Either
            // way the IPv4 socket answers.
            debug!("port {port} is not free on the IPv6 loopback: {error}");
        }
    }
    Ok((listeners, port))
}

/// Accepts until [`Shared::finished`], one task per connection.
async fn accept(listener: TcpListener, shared: Arc<Shared>) {
    loop {
        let accepted = listener.accept().await;
        if shared.finished.load(Ordering::SeqCst) {
            return;
        }
        match accepted {
            Ok((stream, peer)) => {
                let shared = Arc::clone(&shared);
                tokio::spawn(async move {
                    if let Err(error) = serve(stream, &shared).await {
                        debug!("the callback connection from {peer} failed: {error}");
                    }
                });
            }
            Err(error) => {
                // A failed `accept` is one connection (a reset between the SYN
                // and the queue), not a broken listener; the next poll tries
                // again. The pause is the usual answer to a persistent EMFILE.
                warn!("failed to accept a callback connection: {error}");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

/// Answers one browser, and decides whether that browser ended the login.
async fn serve(mut stream: TcpStream, shared: &Shared) -> io::Result<()> {
    let request = http::read_request(&mut stream).await?;
    let response = match request.method.as_str() {
        "GET" | "HEAD" => route(&request, shared),
        // RFC 9110's own advice for a method with no answer is to list the ones
        // there are. It costs a header, and nothing here is ever asked.
        _ => Response::method_not_allowed(),
    };
    if let Some(outcome) = response.outcome {
        // Before the page is written, not after: the answer is the reason the
        // browser is here, and a login that is waiting must not wait for a
        // socket write to finish.
        shared.deliver(outcome);
    }
    // `HEAD` is a `GET` without the body. Answering it with one puts a
    // `Content-Length` in the response for bytes the client is not allowed to
    // read and will then wait for. Nothing sends a `HEAD` here except whatever
    // is checking whether anything is listening, and that wants the status.
    let body = match request.method.as_str() {
        "HEAD" => String::new(),
        _ => shared.render(response.screen),
    };
    http::write_response(
        &mut stream,
        response.status,
        response.reason,
        &body,
        response.allow,
    )
    .await
}

/// What one connection is answered with, and — when it ends the login — with
/// what.
struct Response {
    status: u16,
    reason: &'static str,
    screen: Screen,
    outcome: Option<Outcome>,
    /// The `Allow` header, on the one response that has one.
    allow: Option<&'static str>,
}

impl Response {
    /// A method this listener has no answer for.
    fn method_not_allowed() -> Self {
        Self {
            status: 405,
            reason: "Method Not Allowed",
            screen: Screen::Failure,
            outcome: None,
            allow: Some("GET"),
        }
    }
}

/// Decides what a request means.
fn route(request: &http::Request, shared: &Shared) -> Response {
    match request.path.as_str() {
        CALLBACK_PATH => callback(request, shared),
        // `/` is where a stray visit lands, and `/favicon.ico` is what every
        // browser asks for on its own. Neither is a failure and neither ends
        // anything: the page says to go on waiting, and the listener stays up.
        "/" | "/favicon.ico" => Response {
            status: 200,
            reason: "OK",
            screen: Screen::Waiting,
            outcome: None,
            allow: None,
        },
        _ => Response {
            status: 404,
            reason: "Not Found",
            screen: Screen::Failure,
            outcome: None,
            allow: None,
        },
    }
}

/// The one request that matters.
fn callback(request: &http::Request, shared: &Shared) -> Response {
    if request.param("state").as_deref() != Some(shared.state.as_str()) {
        // Someone else's callback, or a replay of an old one. Saying so and
        // staying up is the right pair: the real callback may still be on its
        // way, and the page must not report a sign-in this launcher never
        // asked for.
        warn!("a callback arrived with a state that is not ours; ignoring it");
        return Response {
            status: 400,
            reason: "Bad Request",
            screen: Screen::Failure,
            outcome: None,
            allow: None,
        };
    }
    if let Some(error) = request.param("error") {
        // `error_description` is a form-encoded value, so a `+` in it is a
        // space (see `Request::text_param`); `error` is a bare token.
        let detail = request
            .text_param("error_description")
            .unwrap_or(error.clone());
        info!("the browser reported '{error}': {detail}");
        return Response {
            status: 200,
            reason: "OK",
            screen: Screen::Failure,
            outcome: Some(Outcome::Declined(detail)),
            allow: None,
        };
    }
    match request.param("code") {
        Some(code) if !code.is_empty() => {
            info!("the browser came back with an authorization code");
            Response {
                status: 200,
                reason: "OK",
                screen: Screen::Success,
                outcome: Some(Outcome::Code(code)),
                allow: None,
            }
        }
        _ => {
            // Our own `state` and nothing else in it. Not a refusal, so the
            // listener stays up for the real callback.
            warn!("a callback carried our state but no code");
            Response {
                status: 400,
                reason: "Bad Request",
                screen: Screen::Failure,
                outcome: None,
                allow: None,
            }
        }
    }
}
