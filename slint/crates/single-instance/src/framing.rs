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

/// Separates the working directory from the arguments inside a report. Two of
/// them, because a path is never empty and a command line always has `argv[0]`,
/// so the first pair can only be the boundary between the two.
const SEPARATOR: &str = "\0\0";

/// Frames a launch for the trip to the process that is already running.
///
/// The fields are NUL separated, and a path or an argument cannot contain a
/// NUL, so nothing in a report can shift the boundary the reader splits on.
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
        // A NUL cannot appear in either field, so the first pair of them is
        // always the boundary — a `|` in a path, which the Tauri app's
        // pipe-separated Windows protocol has to special-case, is just a byte.
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
