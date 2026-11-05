// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The loopback listener the Microsoft browser flow hands its authorization
//! code to.

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

const CALLBACK_PATH: &str = "/callback";

#[derive(Debug, PartialEq, Eq)]
pub enum BrowserRedirectOutcome {
    AuthorizationCode(String),
    Declined { reason: String },
    NoRedirect,
}

pub struct BoundAuthCallback {
    listeners: Vec<std::net::TcpListener>,
    port: u16,
    redirect_outcome_rx: oneshot::Receiver<BrowserRedirectOutcome>,
    shared_state: Arc<Shared>,
}

impl BoundAuthCallback {
    /// Binds the loopback socket and mints the `state` this callback answers
    /// to.
    pub fn start(palette: Palette, messages: Messages) -> Result<Self> {
        let (listeners, port) = bind_loopback()?;
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
            redirect_outcome_rx: outcome,
            shared_state: shared,
        })
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://localhost:{}{CALLBACK_PATH}", self.port)
    }

    /// The `state` the callback has to come back with.
    pub fn state(&self) -> &str {
        &self.shared_state.state
    }

    /// Serves until an answer arrives or the deadline passes.
    ///
    /// Consuming `self` is what ties the listener's life to the login's.
    pub async fn wait(self, timeout: Duration) -> BrowserRedirectOutcome {
        let Self {
            listeners,
            redirect_outcome_rx: outcome,
            shared_state: shared,
            ..
        } = self;
        let mut loops = AcceptLoops::new();
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
            return BrowserRedirectOutcome::NoRedirect;
        }

        let outcome = match tokio::time::timeout(timeout, outcome).await {
            Ok(Ok(outcome)) => outcome,
            // Every accept loop gave up, or the deadline passed. Both mean the
            // same thing here: no code arrived.
            Ok(Err(_)) | Err(_) => BrowserRedirectOutcome::NoRedirect,
        };
        shared.finish();
        drop(loops);
        outcome
    }
}

/// The accept-loop handles, kept so every end of `wait` — return, `?`-less
/// early exit, or a dropped (aborted) future — takes the loops down with it.
/// Without the guard, aborting the task that awaited `wait` drops the
/// `JoinHandle`s, which detaches the loops instead of cancelling them: they
/// would hold their listeners, and the port, until the process ended.
struct AcceptLoops(Vec<tokio::task::JoinHandle<()>>);

impl AcceptLoops {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, loop_handle: tokio::task::JoinHandle<()>) {
        self.0.push(loop_handle);
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Drop for AcceptLoops {
    fn drop(&mut self) {
        for loop_handle in &self.0 {
            loop_handle.abort();
        }
    }
}

/// The state every accept loop and every connection shares.
struct Shared {
    state: String,
    sender: Mutex<Option<oneshot::Sender<BrowserRedirectOutcome>>>,
    /// Set as soon as the login has its answer, so the accept loops stop and
    /// anything still in flight knows not to answer twice.
    finished: AtomicBool,
    page: Page,
}

impl Shared {
    /// Hands `outcome` to the waiting login. At most once: the first `take` to
    /// find a sender wins, and every later call finds none.
    fn deliver(&self, outcome: BrowserRedirectOutcome) {
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
    outcome: Option<BrowserRedirectOutcome>,
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

fn route(request: &http::RequestLine, shared: &Shared) -> Response {
    match request.path.as_str() {
        CALLBACK_PATH => callback(request, shared),
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

fn callback(request: &http::RequestLine, shared: &Shared) -> Response {
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
            outcome: Some(BrowserRedirectOutcome::Declined { reason: detail }),
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
                outcome: Some(BrowserRedirectOutcome::AuthorizationCode(code)),
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
