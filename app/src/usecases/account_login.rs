// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Application rules for adding an account.
//!
//! The interactive Microsoft flow — which of the dialog's four screens is
//! mounted, when the loopback listener is bound — is the adapter's job and stays
//! in `ui/overlays/dialogs/account_add`. What lives here are the rules that do
//! not change when the screen does: whether a form may be submitted, and whether
//! two Yggdrasil servers are the same one. They are UI-neutral and unit-tested.

use url::Url;

/// Whether the offline form may be submitted: a non-empty username and a UUID
/// that is not known to be invalid.
pub fn offline_can_submit(username: &str, uuid_invalid: bool) -> bool {
    !username.trim().is_empty() && !uuid_invalid
}

/// Whether the Yggdrasil form may be submitted: all three fields non-empty.
pub fn yggdrasil_can_submit(api_root: &str, username: &str, password: &str) -> bool {
    !api_root.trim().is_empty() && !username.trim().is_empty() && !password.trim().is_empty()
}

/// Compares API roots: the same host and port, and the same path with any
/// trailing slashes ignored.
pub fn same_api_root(a: &str, b: &str) -> bool {
    let (Ok(a), Ok(b)) = (Url::parse(a), Url::parse(b)) else {
        return false;
    };
    a.host_str() == b.host_str()
        && a.port() == b.port()
        && a.path().trim_end_matches('/') == b.path().trim_end_matches('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_needs_a_name_and_a_valid_uuid() {
        assert!(offline_can_submit("Steve", false));
        assert!(!offline_can_submit("  ", false));
        assert!(!offline_can_submit("Steve", true));
    }

    #[test]
    fn yggdrasil_needs_every_field() {
        assert!(yggdrasil_can_submit("https://example.com", "user", "pass"));
        assert!(!yggdrasil_can_submit("", "user", "pass"));
        assert!(!yggdrasil_can_submit("https://example.com", "user", ""));
    }

    #[test]
    fn api_roots_match_ignoring_a_trailing_slash() {
        assert!(same_api_root(
            "https://a.example/api",
            "https://a.example/api/"
        ));
        assert!(!same_api_root(
            "https://a.example/api",
            "https://b.example/api"
        ));
        assert!(!same_api_root(
            "https://a.example:8080/api",
            "https://a.example/api"
        ));
        assert!(!same_api_root("not a url", "https://a.example/api"));
    }
}
