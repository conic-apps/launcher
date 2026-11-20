// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The shared Minecraft installation.

use std::{
    borrow::Cow,
    fmt::Display,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// The file the Forge and NeoForge installers look for before they install.
const LAUNCHER_PROFILES: &str = "launcher_profiles.json";
/// The Microsoft Store launcher's variant of the same file. Only ever read: the
/// installers accept either, so one of them being usable is enough and there is
/// no reason to write this one.
const LAUNCHER_PROFILES_MICROSOFT_STORE: &str = "launcher_profiles_microsoft_store.json";
/// The names a Forge or NeoForge installer reads, in the order it reads them.
/// The second is the Microsoft Store launcher's variant, which is only ever
/// written when it exists and cannot be parsed.
const LAUNCHER_PROFILE_NAMES: [&str; 2] = [LAUNCHER_PROFILES, LAUNCHER_PROFILES_MICROSOFT_STORE];
const DEFAULT_LAUNCHER_PROFILE: &[u8] = include_bytes!("./launcher_profiles.json");

/// The Minecraft folder structure. Every method returns a path under a
/// Minecraft root such as `.minecraft`.
///
/// One of these is shared by every instance — it is where `versions`,
/// `libraries` and `assets` live — and it is not the launcher's data root: the
/// two are chosen separately (see [`crate::LauncherLocation`]).
#[derive(Debug, Clone)]
pub struct MinecraftLocation {
    pub root: PathBuf,
    pub libraries: PathBuf,
    pub assets: PathBuf,
    pub versions: PathBuf,
    /// The Mojang Java runtimes the launcher installs.
    pub runtime: PathBuf,
}

impl MinecraftLocation {
    pub fn new(root: impl AsRef<Path>) -> MinecraftLocation {
        let root = root.as_ref().to_path_buf();
        MinecraftLocation {
            assets: root.join("assets"),
            libraries: root.join("libraries"),
            versions: root.join("versions"),
            runtime: root.join("runtime"),
            root,
        }
    }

    pub fn get_natives_root<P: AsRef<Path>>(&self, version_id: P) -> PathBuf {
        self.get_version_root(version_id).join("conic-natives")
    }

    pub fn get_version_root<P: AsRef<Path>>(&self, version_id: P) -> PathBuf {
        self.versions.join(version_id)
    }

    pub fn get_version_json<P: AsRef<Path> + Display>(&self, version_id: P) -> PathBuf {
        self.get_version_root(&version_id)
            .join(format!("{version_id}.json"))
    }

    pub fn get_version_jar<P: AsRef<Path> + Display>(
        &self,
        version: P,
        version_jar_type: Option<&str>,
    ) -> PathBuf {
        if let Some(version_jar_type) = version_jar_type
            && version_jar_type != "client"
        {
            self.get_version_root(&version)
                .join(format!("{version}-{}.jar", version_jar_type))
        } else {
            self.get_version_root(&version)
                .join(format!("{version}.jar"))
        }
    }

    pub fn get_library_by_path<P: AsRef<Path>>(&self, library_path: P) -> PathBuf {
        self.libraries.join(library_path)
    }

    pub fn get_assets_index(&self, version_assets: &str) -> PathBuf {
        self.assets
            .join("indexes")
            .join(format!("{version_assets}.json"))
    }

    pub fn get_log_config<P: AsRef<Path>>(&self, version_id: P) -> PathBuf {
        self.get_version_root(version_id).join("log4j2.xml")
    }

    /// Makes sure an installer finds a launcher profile it can consume. See
    /// [`ProfilesPolicy`] for how much this is allowed to touch.
    pub fn ensure_launcher_profiles(&self, policy: ProfilesPolicy) -> ProfilesOutcome {
        ensure_launcher_profiles_at(&self.root, policy)
    }

    /// Creates the root, and the `launcher_profiles.json` the Forge and NeoForge
    /// installers insist on when there is none yet.
    ///
    /// Only the absent case is created here: this runs on every start and the
    /// file belongs to the official launcher when the root was pointed at one,
    /// so rewriting it unconditionally would take the user's profiles and login
    /// with it. A file an installer cannot use is repaired by
    /// [`ProfilesPolicy::Repair`] right before an install needs it.
    pub fn init(&self) {
        if let Err(error) = std::fs::create_dir_all(&self.root) {
            log::warn!(
                "Could not create the Minecraft directory {}: {error}",
                self.root.display()
            );
        }
        if let ProfilesOutcome::Unusable(reason) =
            self.ensure_launcher_profiles(ProfilesPolicy::MissingOnly)
        {
            log::warn!(
                "The launcher profile the Forge and NeoForge installers need is not usable: {reason}"
            );
        }
    }
}

/// Whether a launcher profile can be handed to a Forge or NeoForge installer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfilesState {
    /// Parses as a JSON object whose `profiles`, when present, is an object too.
    Usable,
    /// Neither variant of the file exists.
    Missing,
    /// Present, but an installer would crash on it: the check Forge and
    /// NeoForge perform is an existence test followed by a parse that reports a
    /// stack trace rather than an error.
    Unusable(&'static str),
}

/// What [`ensure_launcher_profiles_at`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfilesOutcome {
    AlreadyUsable,
    Created,
    /// Was unusable; every original was moved aside to `backups` first.
    Repaired {
        backups: Vec<PathBuf>,
    },
    /// Could not be read or written (read-only directory, an unreadable file,
    /// ...). Carries the reason for the caller to log: never fatal, since an
    /// install that needs the file fails with the installer's own message.
    Unusable(String),
}

/// How hard [`ensure_launcher_profiles_at`] should try.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilesPolicy {
    /// Write the default only when there is no usable profile. Used at startup,
    /// where replacing an existing file would destroy data that is not ours.
    MissingOnly,
    /// Also back up and replace one an installer cannot consume. Used right
    /// before Forge and NeoForge are installed, where leaving it broken only
    /// turns into an unexplained installer crash.
    Repair,
}

/// How an installer would take the launcher profile under `root`.
fn profiles_state_at(root: &Path) -> ProfilesState {
    let mut found = false;
    // The installer injects into every variant that exists, so one broken file
    // fails the install even when the other is fine.
    for name in LAUNCHER_PROFILE_NAMES {
        match std::fs::read(root.join(name)) {
            Ok(bytes) => {
                found = true;
                match profiles_state(&bytes) {
                    ProfilesState::Usable => {}
                    unusable => return unusable,
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(_) => return ProfilesState::Unusable("it could not be read"),
        }
    }
    if found {
        ProfilesState::Usable
    } else {
        ProfilesState::Missing
    }
}

/// Makes sure a Forge or NeoForge installer finds a launcher profile it can
/// consume, following `policy` for what may be touched.
pub fn ensure_launcher_profiles_at(root: &Path, policy: ProfilesPolicy) -> ProfilesOutcome {
    match profiles_state_at(root) {
        ProfilesState::Usable => ProfilesOutcome::AlreadyUsable,
        ProfilesState::Missing => write_default_profiles(root, LAUNCHER_PROFILES),
        ProfilesState::Unusable(reason) => match policy {
            // Startup leaves the decision to the install that needs the file.
            ProfilesPolicy::MissingOnly => ProfilesOutcome::Unusable(reason.to_string()),
            ProfilesPolicy::Repair => repair_launcher_profiles(root, reason),
        },
    }
}

/// Backs every unusable variant up and puts the default in its place.
///
/// The originals are kept: a launcher profile holds the official launcher's
/// profiles and login, and it is not this launcher's to throw away.
fn repair_launcher_profiles(root: &Path, reason: &str) -> ProfilesOutcome {
    let mut backups = Vec::new();
    for name in LAUNCHER_PROFILE_NAMES {
        let path = root.join(name);
        match std::fs::read(&path) {
            Ok(bytes) if profiles_state(&bytes) == ProfilesState::Usable => continue,
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            // A directory or an unreadable file is not something to move aside.
            Err(_) => {
                return ProfilesOutcome::Unusable(format!(
                    "{reason}, and {} could not be read",
                    path.display()
                ));
            }
        }

        let backup = backup_path(&path);
        if let Err(error) = std::fs::rename(&path, &backup) {
            log::warn!(
                "Could not move the unusable {} aside: {error}",
                path.display()
            );
            return ProfilesOutcome::Unusable(format!("{reason}, and it could not be moved aside"));
        }
        if let ProfilesOutcome::Unusable(error) = write_default_profiles(root, name) {
            // Put the backup back rather than leave the directory holding nothing.
            if let Err(restore_error) = std::fs::rename(&backup, &path) {
                log::error!(
                    "Could not restore {} after a failed repair: {restore_error}",
                    backup.display()
                );
            }
            return ProfilesOutcome::Unusable(error);
        }
        backups.push(backup);
    }
    if backups.is_empty() {
        ProfilesOutcome::AlreadyUsable
    } else {
        ProfilesOutcome::Repaired { backups }
    }
}

/// Writes the embedded default under `name` through a temporary file, so a
/// crash mid-write cannot leave a truncated profile behind.
fn write_default_profiles(root: &Path, name: &str) -> ProfilesOutcome {
    let path = root.join(name);
    if let Err(error) = std::fs::create_dir_all(root) {
        return ProfilesOutcome::Unusable(format!(
            "{} could not be created: {error}",
            root.display()
        ));
    }
    let staged = root.join(format!("{name}.tmp"));
    if let Err(error) = std::fs::write(&staged, DEFAULT_LAUNCHER_PROFILE) {
        let _ = std::fs::remove_file(&staged);
        return ProfilesOutcome::Unusable(format!(
            "{} could not be written: {error}",
            staged.display()
        ));
    }
    if let Err(error) = std::fs::rename(&staged, &path) {
        let _ = std::fs::remove_file(&staged);
        return ProfilesOutcome::Unusable(format!(
            "{} could not be replaced: {error}",
            path.display()
        ));
    }
    ProfilesOutcome::Created
}

/// `launcher_profiles.json.bak-<unix seconds>`, suffixed until it is free.
fn backup_path(path: &Path) -> PathBuf {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let base = format!("{}.bak-{seconds}", path.display());
    let mut candidate = PathBuf::from(&base);
    let mut attempt = 1;
    while candidate.exists() {
        candidate = PathBuf::from(format!("{base}-{attempt}"));
        attempt += 1;
    }
    candidate
}

/// The check `ClientInstall.injectProfile` performs, without the crash.
fn profiles_state(bytes: &[u8]) -> ProfilesState {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => Cow::Borrowed(text),
        // PCL writes this file as GB18030, so failing UTF-8 alone does not make
        // it unusable: the damage is confined to string literals and a lossy
        // decode keeps the structure an installer reads.
        Err(_) => String::from_utf8_lossy(bytes),
    };
    let text = text.trim_start_matches('\u{feff}');
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return ProfilesState::Unusable("it is not valid JSON");
    };
    let serde_json::Value::Object(object) = value else {
        return ProfilesState::Unusable("its top level is not a JSON object");
    };
    match object.get("profiles") {
        // The installer creates `profiles` when it is absent.
        None => ProfilesState::Usable,
        Some(serde_json::Value::Object(_)) => ProfilesState::Usable,
        Some(_) => ProfilesState::Unusable("its `profiles` is not a JSON object"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory of its own, so the tests do not share state.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("conic-profiles-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path).expect("a scratch directory");
            Self(path)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        fn profiles(&self) -> PathBuf {
            self.0.join(LAUNCHER_PROFILES)
        }

        fn write(&self, bytes: &[u8]) {
            std::fs::write(self.profiles(), bytes).expect("the fixture");
        }

        fn read(&self) -> Vec<u8> {
            std::fs::read(self.profiles()).expect("the profile")
        }

        fn backups(&self) -> Vec<PathBuf> {
            // Covers both variants: `<name>.json.bak-<seconds>`.
            let marker = ".json.bak-";
            let Ok(entries) = std::fs::read_dir(&self.0) else {
                return Vec::new();
            };
            entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.contains(marker))
                })
                .collect()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_missing_profile_is_created_with_the_default() {
        let scratch = Scratch::new();
        assert_eq!(profiles_state_at(scratch.root()), ProfilesState::Missing);
        assert_eq!(
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::MissingOnly),
            ProfilesOutcome::Created
        );
        assert_eq!(scratch.read(), DEFAULT_LAUNCHER_PROFILE);
        // The staging file must not survive the rename.
        assert!(
            !scratch
                .root()
                .join(format!("{LAUNCHER_PROFILES}.tmp"))
                .exists()
        );
    }

    #[test]
    fn a_usable_profile_is_not_rewritten() {
        let scratch = Scratch::new();
        let existing = br#"{"profiles":{"mine":{"name":"mine","lastVersionId":"1.20.1"}}}"#;
        scratch.write(existing);
        assert_eq!(
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair),
            ProfilesOutcome::AlreadyUsable
        );
        assert_eq!(scratch.read(), existing);
        assert!(scratch.backups().is_empty());
    }

    #[test]
    fn a_profile_without_profiles_is_usable() {
        // `injectProfile` adds `profiles` itself, so this needs no repair.
        let scratch = Scratch::new();
        scratch.write(b"{}");
        assert_eq!(profiles_state_at(scratch.root()), ProfilesState::Usable);
    }

    #[test]
    fn a_byte_order_mark_is_not_corruption() {
        let scratch = Scratch::new();
        scratch.write(b"\xef\xbb\xbf{\"profiles\":{}}");
        assert_eq!(profiles_state_at(scratch.root()), ProfilesState::Usable);
    }

    #[test]
    fn a_gb18030_profile_is_usable() {
        // PCL writes this file as GB18030; only the names are damaged by a
        // UTF-8 read, and the structure an installer reads survives.
        let scratch = Scratch::new();
        scratch.write(b"{\"profiles\":{\"\xb9\xfa\xc4\xda\":{\"name\":\"\xb9\xfa\xc4\xda\"}}}");
        assert_eq!(profiles_state_at(scratch.root()), ProfilesState::Usable);
        assert_eq!(
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair),
            ProfilesOutcome::AlreadyUsable
        );
    }

    #[test]
    fn a_microsoft_store_profile_is_usable_on_its_own() {
        let scratch = Scratch::new();
        std::fs::write(
            scratch.root().join(LAUNCHER_PROFILES_MICROSOFT_STORE),
            br#"{"profiles":{}}"#,
        )
        .expect("the fixture");
        assert_eq!(profiles_state_at(scratch.root()), ProfilesState::Usable);
        assert_eq!(
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair),
            ProfilesOutcome::AlreadyUsable
        );
        assert!(!scratch.profiles().exists());
    }

    #[test]
    fn a_truncated_profile_is_backed_up_and_replaced() {
        let scratch = Scratch::new();
        let broken = br#"{"profiles": {"#;
        scratch.write(broken);
        assert_eq!(
            profiles_state_at(scratch.root()),
            ProfilesState::Unusable("it is not valid JSON")
        );

        let ProfilesOutcome::Repaired { backups } =
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair)
        else {
            panic!("the broken profile should have been repaired");
        };
        assert_eq!(backups.len(), 1);
        assert_eq!(std::fs::read(&backups[0]).expect("the backup"), broken);
        assert_eq!(scratch.backups(), backups);
        assert_eq!(scratch.read(), DEFAULT_LAUNCHER_PROFILE);
        assert_eq!(profiles_state_at(scratch.root()), ProfilesState::Usable);
    }

    #[test]
    fn a_broken_microsoft_store_profile_is_repaired_too() {
        // The installer injects into every variant it finds, so leaving this one
        // broken fails the install even though the classic file is fine.
        let scratch = Scratch::new();
        scratch.write(br#"{"profiles":{}}"#);
        let store = scratch.root().join(LAUNCHER_PROFILES_MICROSOFT_STORE);
        std::fs::write(&store, b"not json").expect("the fixture");
        assert!(matches!(
            profiles_state_at(scratch.root()),
            ProfilesState::Unusable(_)
        ));

        let ProfilesOutcome::Repaired { backups } =
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair)
        else {
            panic!("the broken store profile should have been repaired");
        };
        assert_eq!(backups.len(), 1);
        assert_eq!(std::fs::read(&backups[0]).expect("the backup"), b"not json");
        assert_eq!(
            std::fs::read(&store).expect("the replacement"),
            DEFAULT_LAUNCHER_PROFILE
        );
        assert_eq!(profiles_state_at(scratch.root()), ProfilesState::Usable);
    }

    #[test]
    fn both_variants_are_backed_up_when_both_are_broken() {
        let scratch = Scratch::new();
        scratch.write(b"[]");
        std::fs::write(
            scratch.root().join(LAUNCHER_PROFILES_MICROSOFT_STORE),
            b"[]",
        )
        .expect("the fixture");
        let ProfilesOutcome::Repaired { backups } =
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair)
        else {
            panic!("both profiles should have been repaired");
        };
        assert_eq!(backups.len(), 2);
        assert_eq!(scratch.backups().len(), 2);
        assert_eq!(profiles_state_at(scratch.root()), ProfilesState::Usable);
    }

    #[test]
    fn a_top_level_array_is_repaired() {
        let scratch = Scratch::new();
        scratch.write(b"[]");
        assert!(matches!(
            profiles_state_at(scratch.root()),
            ProfilesState::Unusable(_)
        ));
        assert!(matches!(
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair),
            ProfilesOutcome::Repaired { .. }
        ));
        assert_eq!(scratch.backups().len(), 1);
    }

    #[test]
    fn profiles_that_is_not_an_object_is_repaired() {
        let scratch = Scratch::new();
        scratch.write(br#"{"profiles": []}"#);
        assert_eq!(
            profiles_state_at(scratch.root()),
            ProfilesState::Unusable("its `profiles` is not a JSON object")
        );
        assert!(matches!(
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair),
            ProfilesOutcome::Repaired { .. }
        ));
    }

    #[test]
    fn an_empty_file_is_repaired() {
        let scratch = Scratch::new();
        scratch.write(b"");
        assert!(matches!(
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::Repair),
            ProfilesOutcome::Repaired { .. }
        ));
    }

    #[test]
    fn missing_only_leaves_a_broken_profile_alone() {
        // Startup must not decide for the user: an official launcher's file that
        // is broken today may still be worth recovering by hand.
        let scratch = Scratch::new();
        let broken = br#"{"profiles": {"#;
        scratch.write(broken);
        assert!(matches!(
            ensure_launcher_profiles_at(scratch.root(), ProfilesPolicy::MissingOnly),
            ProfilesOutcome::Unusable(_)
        ));
        assert_eq!(scratch.read(), broken);
        assert!(scratch.backups().is_empty());
    }

    #[test]
    fn an_unreadable_profile_is_reported_instead_of_panicking() {
        // A directory under the profile's name fails the read with something
        // other than `NotFound`, which is the read-only and permission-denied
        // path without depending on the test user's rights.
        let scratch = Scratch::new();
        std::fs::create_dir(scratch.profiles()).expect("the fixture");
        assert_eq!(
            profiles_state_at(scratch.root()),
            ProfilesState::Unusable("it could not be read")
        );
        for policy in [ProfilesPolicy::MissingOnly, ProfilesPolicy::Repair] {
            match ensure_launcher_profiles_at(scratch.root(), policy) {
                ProfilesOutcome::Unusable(reason) => assert!(!reason.is_empty()),
                other => panic!("expected an unusable outcome, got {other:?}"),
            }
        }
        // Nothing was moved or written: the directory is still in the way.
        assert!(scratch.profiles().is_dir());
        assert!(scratch.backups().is_empty());
    }
}
