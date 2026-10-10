// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The content detail panel's presenter: a [`DownloadSnapshot`] → the panel's
//! progress fields.
//!
//! The testable half of the pair: the byte label and the bar's value/max are a
//! pure function of the snapshot.

use download::progress::DownloadSnapshot;

use crate::slint_backend::ContentState;
use crate::support::formatting::bytes;

#[derive(Debug, PartialEq)]
pub(crate) struct ContentDownloadView {
    value: f32,
    max: f32,
    text: String,
}

impl ContentDownloadView {
    pub(crate) fn apply(self, state: &ContentState) {
        state.set_detail_progress(self.value);
        state.set_detail_progress_max(self.max);
        state.set_detail_progress_text(self.text.into());
    }
}

pub(crate) fn download_view(snapshot: &DownloadSnapshot) -> ContentDownloadView {
    ContentDownloadView {
        value: snapshot.completed_bytes as f32,
        max: snapshot.total_bytes as f32,
        text: format!(
            "{} / {}",
            bytes(snapshot.completed_bytes),
            bytes(snapshot.total_bytes)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use download::progress::DownloadPhase;

    #[test]
    fn the_label_counts_both_ends() {
        let view = download_view(&DownloadSnapshot {
            phase: DownloadPhase::DownloadFiles,
            completed_bytes: 1024,
            total_bytes: 2048,
        });
        assert_eq!(view.value, 1024.0);
        assert_eq!(view.max, 2048.0);
        assert_eq!(view.text, "1.00 KB / 2.00 KB");
    }
}
