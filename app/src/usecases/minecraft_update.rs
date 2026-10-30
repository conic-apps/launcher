// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Whether a Minecraft release or snapshot was published since the last check.
//!
//! The two reminder switches are off by default. The first check after one is
//! turned on records the current latest version without a notice: enabling a
//! reminder should not immediately announce a version that may be months old.
//! Turning a switch off forgets its recorded version, so switching it back on
//! starts from the then-current version again.
//!
//! The recorded pair lives beside `config.toml` rather than in it: it is a
//! record of what was shown, not a setting the user edits.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use storage::LOCATIONS;

fn state_file() -> PathBuf {
    LOCATIONS.launcher.root.join("minecraft_reminder.json")
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
struct ReminderState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    release: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    snapshot: Option<String>,
}

impl ReminderState {
    /// Drops the versions whose reminder was turned off. Returns whether
    /// anything was dropped, so an unchanged state does not rewrite the file.
    fn clear_disabled(&mut self, release_enabled: bool, snapshot_enabled: bool) -> bool {
        let release_cleared = if release_enabled {
            false
        } else {
            self.release.take().is_some()
        };
        let snapshot_cleared = if snapshot_enabled {
            false
        } else {
            self.snapshot.take().is_some()
        };
        release_cleared || snapshot_cleared
    }

    /// Records the latest versions and returns the ones that are newer than
    /// what was recorded.
    ///
    /// The first sight of a version is never a release: there is nothing to
    /// compare it against, and a fresh state file would otherwise announce the
    /// current latest. When both switches are on and both latest versions match
    /// — a release is also the latest snapshot — one notice is enough.
    fn observe(
        &mut self,
        release_enabled: bool,
        snapshot_enabled: bool,
        latest_release: &str,
        latest_snapshot: &str,
    ) -> Vec<String> {
        let mut published = Vec::new();
        if release_enabled {
            match self.release.as_deref() {
                Some(seen) if seen == latest_release => {}
                Some(_) => {
                    self.release = Some(latest_release.to_owned());
                    published.push(latest_release.to_owned());
                }
                None => self.release = Some(latest_release.to_owned()),
            }
        }
        if snapshot_enabled {
            match self.snapshot.as_deref() {
                Some(seen) if seen == latest_snapshot => {}
                Some(_) => {
                    self.snapshot = Some(latest_snapshot.to_owned());
                    published.push(latest_snapshot.to_owned());
                }
                None => self.snapshot = Some(latest_snapshot.to_owned()),
            }
        }
        if published.len() == 2 && published[0] == published[1] {
            published.pop();
        }
        published
    }
}

/// Compares Mojang's latest versions with the last ones shown to the user and
/// returns the versions worth a notice.
///
/// A failed fetch leaves the recorded state alone, so the check can be retried
/// on the next start instead of losing the version that was not announced.
pub(crate) async fn check(release_enabled: bool, snapshot_enabled: bool) -> Vec<String> {
    let mut state = load();
    if state.clear_disabled(release_enabled, snapshot_enabled) {
        save(&state);
    }
    if !release_enabled && !snapshot_enabled {
        return Vec::new();
    }

    let manifest = match install::get_minecraft_version_list().await {
        Ok(manifest) => manifest,
        Err(error) => {
            log::warn!("Minecraft version reminder check failed: {error}");
            return Vec::new();
        }
    };
    let published = state.observe(
        release_enabled,
        snapshot_enabled,
        &manifest.latest.release,
        &manifest.latest.snapshot,
    );
    save(&state);
    for version in &published {
        log::info!("Minecraft {version} was published since the last check");
    }
    published
}

fn load() -> ReminderState {
    let Ok(raw) = std::fs::read(state_file()) else {
        return ReminderState::default();
    };
    serde_json::from_slice(&raw).unwrap_or_else(|error| {
        log::warn!(
            "{} is not valid Minecraft reminder json ({error}); starting from no record",
            state_file().display()
        );
        ReminderState::default()
    })
}

fn save(state: &ReminderState) {
    let path = state_file();
    let Ok(bytes) = serde_json::to_vec_pretty(state) else {
        log::warn!("the Minecraft reminder state could not be serialised");
        return;
    };
    if let Err(error) = std::fs::write(&path, bytes) {
        log::warn!(
            "could not save the Minecraft reminder state to {}: {error}",
            path.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::ReminderState;

    #[test]
    fn the_first_sight_is_recorded_without_a_notice() {
        let mut state = ReminderState::default();
        let published = state.observe(true, true, "1.21.8", "26w33a");

        assert!(published.is_empty());
        assert_eq!(state.release.as_deref(), Some("1.21.8"));
        assert_eq!(state.snapshot.as_deref(), Some("26w33a"));
    }

    #[test]
    fn only_a_changed_version_is_announced_once() {
        let mut state = ReminderState::default();
        state.observe(true, true, "1.21.8", "26w33a");

        assert!(state.observe(true, true, "1.21.8", "26w33a").is_empty());
        assert_eq!(state.observe(true, true, "1.21.9", "26w33a"), ["1.21.9"]);
        assert!(state.observe(true, true, "1.21.9", "26w33a").is_empty());
    }

    #[test]
    fn a_release_that_is_also_the_latest_snapshot_is_announced_once() {
        let mut state = ReminderState::default();
        state.observe(true, true, "1.21.8", "26w33a");

        assert_eq!(state.observe(true, true, "1.21.9", "1.21.9"), ["1.21.9"]);
    }

    #[test]
    fn both_switches_can_announce_their_own_version() {
        let mut state = ReminderState::default();
        state.observe(true, true, "1.21.8", "26w33a");

        assert_eq!(
            state.observe(true, true, "1.21.9", "26w34a"),
            ["1.21.9", "26w34a"]
        );
    }

    #[test]
    fn only_the_enabled_switch_is_compared() {
        let mut state = ReminderState::default();
        state.observe(true, true, "1.21.8", "26w33a");
        state.clear_disabled(false, true);

        assert_eq!(state.release, None);
        assert!(state.observe(false, true, "1.21.9", "26w33a").is_empty());
        assert_eq!(state.observe(false, true, "1.21.9", "26w34a"), ["26w34a"]);
    }

    #[test]
    fn turning_a_switch_off_forgets_its_version() {
        let mut state = ReminderState {
            release: Some("1.21.8".into()),
            snapshot: Some("26w33a".into()),
        };

        assert!(state.clear_disabled(true, false));
        assert_eq!(state.release.as_deref(), Some("1.21.8"));
        assert!(state.snapshot.is_none());
        assert!(!state.clear_disabled(true, false));
    }
}
