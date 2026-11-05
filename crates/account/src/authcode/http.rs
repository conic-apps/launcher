// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A simple HTTP/1.1 implement to answer a browser's redirect.

use std::io;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const HEADER_END: &[u8] = b"\r\n\r\n";
const MAX_HEAD: usize = 8 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(10);

pub struct RequestLine {
    pub method: String,
    pub path: String,
    pub query: String,
}

impl RequestLine {
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
pub async fn read_request(stream: &mut TcpStream) -> io::Result<RequestLine> {
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
    let target = match target.strip_prefix("http://") {
        Some(absolute) => match absolute.find('/') {
            Some(slash) => &absolute[slash..],
            None => "/",
        },
        None => target,
    };
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    Ok(RequestLine {
        method: method.to_string(),
        path: percent_decode(path),
        query: query.to_string(),
    })
}

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
         Server: Conic Launcher\r\n\
         {allow}\
         Connection: close\r\n\
         \r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.shutdown().await
}

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
    }

    #[test]
    fn authorization_code_untouch() {
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
        let request = RequestLine {
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
        let request = RequestLine {
            method: "GET".to_string(),
            path: "/callback".to_string(),
            query: "error=access_denied".to_string(),
        };
        assert_eq!(request.param("error").as_deref(), Some("access_denied"));
        assert_eq!(request.param("code"), None);

        let bare = RequestLine {
            method: "GET".to_string(),
            path: "/callback".to_string(),
            query: "error".to_string(),
        };
        assert_eq!(bare.param("error"), None);
    }
}
