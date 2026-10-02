// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The multiplayer download screen's presenter: a [`DownloadSnapshot`] → a
//! [`DownloadView`].
//!
//! The testable half of the pair: the phase/loading/byte-text decision is a
//! pure function of the snapshot, so it is unit-tested without a window.

use download::progress::{DownloadPhase, DownloadSnapshot};

use crate::slint_backend::MultiplayerState;
use crate::support::formatting::bytes;

/// One write to the download bar.
#[derive(Debug, PartialEq)]
pub(crate) struct DownloadView {
    /// "prepare" | "downloading".
    phase: &'static str,
    loading: bool,
    value: f32,
    max: f32,
    value_text: String,
    max_text: String,
}

impl DownloadView {
    pub(crate) fn apply(self, state: &MultiplayerState) {
        state.set_download_phase(self.phase.into());
        state.set_download_loading(self.loading);
        state.set_download_value(self.value);
        state.set_download_max(self.max);
        state.set_download_value_text(self.value_text.into());
        state.set_download_max_text(self.max_text.into());
    }
}

/// The view for one download reading. Verification (and a download whose size
/// is not known yet) shows the plain preparing bar; a counted download shows the
/// byte counters.
pub(crate) fn download_view(snapshot: &DownloadSnapshot) -> DownloadView {
    let (phase, loading, value, max) = match snapshot.phase {
        DownloadPhase::VerifyExistingFiles => ("prepare", true, 0, 10),
        DownloadPhase::DownloadFiles => (
            if snapshot.total_bytes == 0 {
                "prepare"
            } else {
                "downloading"
            },
            snapshot.total_bytes == 0,
            snapshot.completed_bytes,
            snapshot.total_bytes,
        ),
    };
    DownloadView {
        phase,
        loading,
        value: value as f32,
        max: max as f32,
        value_text: bytes(value),
        max_text: bytes(max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_prepares_with_a_small_bar() {
        let view = download_view(&DownloadSnapshot {
            phase: DownloadPhase::VerifyExistingFiles,
            completed_bytes: 0,
            total_bytes: 0,
        });
        assert_eq!(view.phase, "prepare");
        assert!(view.loading);
        assert_eq!(view.max, 10.0);
    }

    #[test]
    fn an_unknown_total_still_prepares() {
        let view = download_view(&DownloadSnapshot {
            phase: DownloadPhase::DownloadFiles,
            completed_bytes: 5,
            total_bytes: 0,
        });
        assert_eq!(view.phase, "prepare");
        assert!(view.loading);
    }

    #[test]
    fn a_counted_download_shows_bytes() {
        let view = download_view(&DownloadSnapshot {
            phase: DownloadPhase::DownloadFiles,
            completed_bytes: 1024,
            total_bytes: 2048,
        });
        assert_eq!(view.phase, "downloading");
        assert!(!view.loading);
        assert_eq!(view.value_text, "1.00 KB");
        assert_eq!(view.max_text, "2.00 KB");
    }
}
