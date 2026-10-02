// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Just enough HTTP/1.1 to answer a browser's redirect.
//!
//! A request is read up to the blank line that ends the headers, and the
//! request line is all that is looked at; every header is skipped. The response
//! is a status line, a fixed set of headers including `Content-Length`, and the
//! body, then the socket is shut down. There is no keep-alive, no chunked
//! decoding, no request body and no compression — a browser redirect carries
//! none of those, and every one of them would be code that nothing here can
//! exercise.
//!
//! Every read is bounded, in both size and time: the listener is reachable by
//! anything on the machine (and, on a machine that forwards, by anything on the
//! network), so a connection that never sends a request line, or sends one that
//! never ends, must not be able to hold a task open or a buffer growing.

use std::io;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// The blank line every HTTP request ends its headers with.
const HEADER_END: &[u8] = b"\r\n\r\n";

/// How much of a request is read before giving up on the header block. A
/// redirect's headers are a few hundred bytes; 8 KiB is a browser's default
/// request line plus a full default header set with room to spare.
const MAX_HEAD: usize = 8 * 1024;

/// How long one connection has to send its request. The browser that is being
/// redirected has the request ready before it opens the socket, so anything
/// slower than this is not that browser.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// The request line, and nothing else.
pub struct Request {
    /// The method, uppercased by the client.
    pub method: String,
    /// The path, percent-decoding applied.
    pub path: String,
    /// The query string, if the target carried one.
    pub query: String,
}

impl Request {
    /// The first value of `name` in the query string, percent-decoded.
    ///
    /// `+` is left as it is. A query string has no `+`-is-a-space rule —
    /// `application/x-www-form-urlencoded` bodies have, and this is not one of
    /// those — and the values read here are the ones that must not be mangled:
    /// a `+` in a query is a literal plus, not a space.
    pub fn param(&self, name: &str) -> Option<String> {
        self.value(name, false)
    }

    /// The first value of `name`, decoded as `application/x-www-form-urlencoded`
    /// — that is, with `+` read as a space.
    ///
    /// For `error_description` only, which RFC 6749 defines as a
    /// form-encoded value and so may well arrive as `The+user+said+no`. The
    /// other parameters are not, and go through [`param`](Self::param).
    pub fn text_param(&self, name: &str) -> Option<String> {
        self.value(name, true)
    }

    fn value(&self, name: &str, plus_is_space: bool) -> Option<String> {
        self.query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            if percent_decode(key) != name {
                return None;
            }
            Some(percent_decode(&if plus_is_space {
                value.replace('+', " ")
            } else {
                value.to_string()
            }))
        })
    }
}

/// Reads the request line of the next request on `stream`.
///
/// Fails on EOF, on a connection that runs past [`READ_TIMEOUT`], and on a head
/// that never ends within [`MAX_HEAD`]. None of the three is worth a page: the
/// browser is never the one at fault, so the caller logs and drops the
/// connection.
pub async fn read_request(stream: &mut TcpStream) -> io::Result<Request> {
    let mut head = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let room = (MAX_HEAD - head.len()).min(chunk.len());
        let read = read_within(stream, &mut chunk[..room]).await?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the connection closed before the request was complete",
            ));
        }
        head.extend_from_slice(&chunk[..read]);
        if head
            .windows(HEADER_END.len())
            .any(|window| window == HEADER_END)
        {
            break;
        }
        if head.len() >= MAX_HEAD {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the request headers did not end within the read limit",
            ));
        }
    }

    // `GET /callback?code=… HTTP/1.1`. A request line is ASCII by definition,
    // and the one field that is not — the query — is handled as bytes below, so
    // a request line that is somehow not UTF-8 still parses rather than fails.
    let line = String::from_utf8_lossy(
        head.split(|byte| *byte == b'\n')
            .next()
            .expect("the head is never empty here"),
    );
    let mut fields = line.split_ascii_whitespace();
    let method = fields.next().unwrap_or_default();
    let target = fields.next().unwrap_or_default();
    if method.is_empty() || target.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the request line is not `METHOD TARGET VERSION`",
        ));
    }
    // Browsers send the origin form (`/callback?code=…`); `curl`, and anything
    // configured to go through a proxy, sends the absolute form
    // (`http://localhost:1234/callback?code=…`). Both name the same resource,
    // so the authority is dropped when it is there — from the slash that ends
    // it, which has to stay: the path is matched against `/callback`.
    let target = match target.strip_prefix("http://") {
        Some(absolute) => match absolute.find('/') {
            Some(slash) => &absolute[slash..],
            None => "/",
        },
        None => target,
    };
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    Ok(Request {
        method: method.to_string(),
        path: percent_decode(path),
        query: query.to_string(),
    })
}

/// Writes a complete response and closes the connection.
///
/// `Connection: close` is what lets the body end the exchange: without a
/// `Content-Length` the browser would read until the socket closed, and with
/// one it can paint the page before that happens.
pub async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    body: &str,
    allow: Option<&str>,
) -> io::Result<()> {
    let allow = allow.map_or(String::new(), |allow| format!("Allow: {allow}\r\n"));
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\n\
         {allow}\
         Connection: close\r\n\
         \r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    // Flushes what is buffered and then closes, so the browser is not left
    // reading a page the launcher has already stopped writing.
    stream.shutdown().await
}

/// One read, bounded by a deadline rather than by the caller's own timeout: a
/// `TcpStream` is not itself wrapped in one, and a read that is never answered
/// is the case the bound exists for.
///
/// `buffer` is already trimmed to what is still allowed, so a read that lands
/// exactly on the limit is the end of the head and not a truncation of it.
async fn read_within(stream: &mut TcpStream, buffer: &mut [u8]) -> io::Result<usize> {
    tokio::time::timeout(READ_TIMEOUT, stream.read(buffer))
        .await
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "the client sent no request within the deadline",
            )
        })?
}

/// Decodes `%XX` escapes, and nothing else.
///
/// A malformed escape is left as it is: a query value that is not valid
/// percent-encoding is not a value this listener can use anyway, and guessing
/// at it is how a parser ends up accepting something the author did not mean.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escape = if index + 2 < bytes.len() {
            hex_digit(bytes[index + 1])
                .zip(hex_digit(bytes[index + 2]))
                .map(|(high, low)| high << 4 | low)
        } else {
            None
        };
        if bytes[index] == b'%'
            && let Some(byte) = escape
        {
            out.push(byte);
            index += 3;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_escapes_a_redirect_carries() {
        assert_eq!(percent_decode("/callback"), "/callback");
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("%2F%2Foauth2"), "//oauth2");
        // Base64url's alphabet survives untouched, which is the case that
        // matters: an authorization code is mostly `A-Za-z0-9-_`.
        assert_eq!(percent_decode("0.AAe-9_x"), "0.AAe-9_x");
    }

    #[test]
    fn leaves_a_malformed_escape_alone() {
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
        assert_eq!(percent_decode("%2"), "%2");
    }

    #[test]
    fn reads_one_parameter_out_of_a_query() {
        let request = Request {
            method: "GET".to_string(),
            path: "/callback".to_string(),
            query: "code=0.AAe-9&state=abc%2Ddef&session_state=zzz".to_string(),
        };
        assert_eq!(request.param("code").as_deref(), Some("0.AAe-9"));
        assert_eq!(request.param("state").as_deref(), Some("abc-def"));
        assert_eq!(request.param("session_state").as_deref(), Some("zzz"));
        assert_eq!(request.param("missing"), None);
    }

    #[test]
    fn a_valueless_parameter_is_not_a_parameter() {
        // `error` arrives bare on some refusals; the callback reads it by name
        // and must not mistake `code` for it.
        let request = Request {
            method: "GET".to_string(),
            path: "/callback".to_string(),
            query: "error=access_denied".to_string(),
        };
        assert_eq!(request.param("error").as_deref(), Some("access_denied"));
        assert_eq!(request.param("code"), None);

        let bare = Request {
            method: "GET".to_string(),
            path: "/callback".to_string(),
            query: "error".to_string(),
        };
        assert_eq!(bare.param("error"), None);
    }
}
