// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The byte framing of a report, for the backends that push one through a
//! channel of their own — a socket stream, a `WM_COPYDATA` payload.
//!
//! The D-Bus backend hands the two fields to the bus as method arguments and
//! needs none of this, so on Linux the whole module is unused.
#![cfg_attr(target_os = "linux", allow(dead_code))]

use crate::Launch;

/// Separates the working directory from the arguments inside a report. The
/// arguments are separated by one, and neither a path nor an argument can hold
/// a NUL, so the first pair is always the boundary between the two fields.
const SEPARATOR: &str = "\0\0";

/// Frames a launch for the trip to the process that is already running.
pub(crate) fn encode(launch: &Launch) -> String {
    format!(
        "{cwd}{SEPARATOR}{args}",
        cwd = launch.cwd,
        args = launch.args.join("\0")
    )
}

/// Reads a framed launch back, or `None` when the report is not one of ours.
pub(crate) fn decode(raw: &str) -> Option<Launch> {
    let (cwd, args) = raw.split_once(SEPARATOR)?;
    Some(Launch {
        cwd: cwd.to_owned(),
        args: args.split('\0').map(str::to_owned).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Launch, decode, encode};

    #[test]
    fn a_report_survives_the_trip() {
        let launch = Launch {
            args: vec![
                "/usr/bin/conic-launcher".into(),
                "conic-launcher://open?code=1".into(),
            ],
            cwd: "/home/user/Games".into(),
        };
        assert_eq!(decode(&encode(&launch)), Some(launch));
    }

    #[test]
    fn a_field_may_look_like_the_boundary() {
        // `|` is not a separator; only the NUL pair splits a report.
        let launch = Launch {
            args: vec!["conic-launcher".into(), "|not a boundary|".into()],
            cwd: "/home/user/Games|weird".into(),
        };
        assert_eq!(decode(&encode(&launch)), Some(launch));
    }

    #[test]
    fn a_report_of_something_else_is_rejected() {
        assert_eq!(decode("no separator at all"), None);
    }
}
