// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The create-instance dialog's "script": drives `install` from the dialog's
//! state and its two screens.
//!
//! Everything that touches the network or the disk runs on the tokio runtime
//! (`crate::support::runtime`) and reports back through `crate::ui::services::report::report`, so the
//! window keeps drawing while a version list is fetched. The one exception is
//! the background picker's image decode, which Slint only lets the UI thread
//! create (see `setup_background_and_wiki`).

use std::{cell::RefCell, path::Path, rc::Rc};

use chrono::{Datelike, Local};
use slint::{ComponentHandle, Image, ModelRc, SharedString, VecModel, Weak};

use crate::slint_backend::{App, CreateInstanceState, Dialogs, GameState, MinecraftVersionItem};
use crate::ui::services::app_config;
use instance::{InstanceConfig, InstanceLaunchConfig, InstanceRuntime, ModLoaderType, SortBy};

thread_local! {
    /// The whole Minecraft manifest. Only the rows of the selected category are
    /// handed to the view (the manifest has ~900 entries, and a `for` loop would
    /// instantiate one row per entry however they are filtered). Only touched on
    /// the UI thread, like `JAVA_CACHE` in `settings.rs`.
    static MANIFEST: RefCell<Vec<MinecraftVersionItem>> = const { RefCell::new(Vec::new()) };
}

/// The mod loader names, as the select and the version lists spell them.
const MOD_LOADERS: [&str; 4] = ["Fabric", "Quilt", "Forge", "Neoforge"];

/// Registers every create-instance callback on the `CreateInstanceState` global.
pub fn setup(ui: &App, config: Rc<RefCell<config::Config>>) {
    setup_minecraft_choose(ui);
    setup_create(ui);
    setup_background_and_wiki(ui, config);
}

/// The manifest, its category filter and the four mod loader fetches a picked
/// version kicks off.
fn setup_minecraft_choose(ui: &App) {
    let state = ui.global::<CreateInstanceState>();

    // Fetch the manifest, then filter it.
    {
        let weak = ui.as_weak();
        state.on_load_minecraft_versions(move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.global::<CreateInstanceState>()
                .set_minecraft_loading(true);

            let weak = weak.clone();
            crate::support::runtime::spawn(async move {
                let result = install::get_minecraft_version_list()
                    .await
                    .map(|manifest| manifest.versions.into_iter().map(version_item).collect());
                crate::ui::services::report::report(&weak, move |ui| {
                    let state = ui.global::<CreateInstanceState>();
                    match result {
                        Ok(versions) => {
                            MANIFEST.with(|manifest| *manifest.borrow_mut() = versions);
                            let category = state.get_version_category().to_string();
                            MANIFEST.with(|manifest| {
                                apply_category(&state, &manifest.borrow(), &category);
                            });
                        }
                        Err(error) => {
                            log::error!("failed to fetch the Minecraft version list: {error}");
                        }
                    }
                    state.set_minecraft_loading(false);
                });
            });
        });
    }

    // The category select: re-filter the manifest.
    {
        let weak = ui.as_weak();
        state.on_set_version_category(move |category| {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<CreateInstanceState>();
            state.set_version_category(category.clone());
            MANIFEST.with(|manifest| apply_category(&state, &manifest.borrow(), category.as_str()));
        });
    }

    // Picking a version stores it and refreshes the four loader lists.
    {
        let weak = ui.as_weak();
        state.on_select_minecraft_version(move |version| {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<CreateInstanceState>();
            // Re-picking the already selected version would otherwise refetch
            // all four loader lists; the early return makes it a no-op.
            if state.get_minecraft_version() == version {
                return;
            }
            state.set_minecraft_version(version.clone());
            state.set_fabric_loading(true);
            state.set_quilt_loading(true);
            state.set_forge_loading(true);
            state.set_neoforge_loading(true);
            state.set_fabric_available(false);
            state.set_quilt_available(false);
            state.set_forge_available(false);
            state.set_neoforge_available(false);

            for loader in MOD_LOADERS {
                spawn_mod_loader_fetch(weak.clone(), loader, version.to_string());
            }
        });
    }
}

/// The create callback (`on_create`): a free id, the instance, the background,
/// then the game view refreshes and the dialog closes.
fn setup_create(ui: &App) {
    let state = ui.global::<CreateInstanceState>();
    {
        let weak = ui.as_weak();
        state.on_create(move || {
            let Some(ui) = weak.upgrade() else { return };
            let state = ui.global::<CreateInstanceState>();
            if state.get_creating() {
                return;
            }
            let custom_name = state.get_custom_name().to_string();
            let name = if custom_name.is_empty() {
                state.get_default_name().to_string()
            } else {
                custom_name
            };
            let minecraft = state.get_minecraft_version().to_string();
            let loader_type = state.get_mod_loader_type().to_string();
            let loader_version = state.get_mod_loader_version().to_string();
            let background = state.get_background_path().to_string();
            state.set_creating(true);

            let weak = weak.clone();
            // Creating an instance writes files and walks the instance folder, so
            // it belongs off the UI thread.
            crate::support::runtime::spawn(async move {
                let created = create_instance(
                    &name,
                    &minecraft,
                    &loader_type,
                    &loader_version,
                    &background,
                )
                .await;
                if let Err(error) = &created {
                    log::error!("failed to create the instance: {error}");
                }
                crate::ui::services::report::report(&weak, move |ui| {
                    ui.global::<CreateInstanceState>().set_creating(false);
                    match created {
                        // The new instance becomes current: the reload carries
                        // its id so the list selects it as it lands.
                        Ok(id) => ui.global::<GameState>().invoke_refresh_selecting(id.into()),
                        // Reload after a failure too, since the dialog is not a
                        // place to report an error.
                        Err(_) => ui.global::<GameState>().invoke_refresh(),
                    }
                    ui.global::<Dialogs>().set_create_instance_visible(false);
                });
            });
        });
    }
}

/// The background picker's preview and the version wiki link.
fn setup_background_and_wiki(ui: &App, config: Rc<RefCell<config::Config>>) {
    let state = ui.global::<CreateInstanceState>();

    // The background callback (`on_pick_background`): the native file picker,
    // then the preview image.
    {
        let weak = ui.as_weak();
        state.on_pick_background(move |filter_name| {
            let Some(ui) = weak.upgrade() else { return };
            let Some(path) = app_config::pick_image_file_named(filter_name.as_str()) else {
                return;
            };
            let state = ui.global::<CreateInstanceState>();
            state.set_background_path(path.to_string_lossy().to_string().into());
            // The picture decodes as soon as it is picked. Slint can only build
            // an `Image` on the calling thread (it is not `Send`), so unlike the
            // other work here this decode stays on the UI thread.
            match Image::load_from_path(&path) {
                Ok(image) => state.set_background_image(image),
                Err(error) => log::warn!("failed to load '{}': {error}", path.display()),
            }
        });
    }

    // Every version has a wiki page — on the Chinese wiki for the Chinese
    // locales, on the English one otherwise.
    let language = app_config::active_language_code(config.borrow().language.as_deref());
    let chinese = language.starts_with("zh");
    state.on_open_version_wiki(move |version| {
        let url = if chinese {
            format!("https://zh.minecraft.wiki/w/Java%E7%89%88{version}")
        } else {
            format!("https://minecraft.wiki/w/Java_Edition_{version}")
        };
        if let Err(error) = app_config::open_external(&url) {
            log::warn!("failed to open '{url}': {error}");
        }
    });
}

/// Replaces the chooser's rows with the ones of `category`; the "old" category
/// matches any kind containing "old".
fn apply_category(state: &CreateInstanceState, manifest: &[MinecraftVersionItem], category: &str) {
    let rows: Vec<MinecraftVersionItem> = manifest
        .iter()
        .filter(|item| match category {
            "releases" => item.kind == "release",
            "snapshot" => item.kind == "snapshot",
            "old" => item.kind.contains("old"),
            _ => item.kind == "release",
        })
        .cloned()
        .collect();
    state.set_minecraft_versions(ModelRc::new(VecModel::from(rows)));
}

/// A manifest entry as the chooser shows it: the id, its type and the release
/// date split into parts, in local time.
fn version_item(info: install::vanilla::VersionInfo) -> MinecraftVersionItem {
    let date = chrono::DateTime::parse_from_rfc3339(&info.release_time)
        .map(|time| time.with_timezone(&Local));
    MinecraftVersionItem {
        id: info.id.into(),
        kind: info.r#type.into(),
        year: date.map(|date| date.year()).unwrap_or_default(),
        month: date.map(|date| date.month() as i32).unwrap_or_default(),
        day: date.map(|date| date.day() as i32).unwrap_or_default(),
    }
}

/// Fetches one mod loader's versions for `mcversion` and reports the result
/// into the dialog's state.
fn spawn_mod_loader_fetch(weak: Weak<App>, loader: &'static str, mcversion: String) {
    crate::support::runtime::spawn(async move {
        let result: Result<Vec<String>, install::Error> = match loader {
            "Fabric" => install::get_fabric_version_list(&mcversion)
                .await
                .map(|list| {
                    list.as_slice()
                        .iter()
                        .map(|artifact| artifact.loader.version.clone())
                        .collect()
                }),
            "Quilt" => install::get_quilt_version_list(&mcversion)
                .await
                .map(|list| {
                    list.as_slice()
                        .iter()
                        .map(|version| version.loader.version.clone())
                        .collect()
                }),
            "Forge" => install::get_forge_version_list().await.map(|list| {
                list.get(&mcversion)
                    .unwrap_or_default()
                    .iter()
                    .map(|version| forge_version_label(version))
                    .collect()
            }),
            _ => install::get_neoforge_version_list()
                .await
                .map(|list| filter_neoforge_version_list(&mcversion, &list)),
        };

        crate::ui::services::report::report(&weak, move |ui| {
            let state = ui.global::<CreateInstanceState>();
            match result {
                Ok(versions) => {
                    // A loader is available only when its list is not empty.
                    let available = !versions.is_empty();
                    let versions: ModelRc<SharedString> = ModelRc::new(VecModel::from(
                        versions
                            .into_iter()
                            .map(SharedString::from)
                            .collect::<Vec<_>>(),
                    ));
                    match loader {
                        "Fabric" => {
                            state.set_fabric_versions(versions);
                            state.set_fabric_available(available);
                        }
                        "Quilt" => {
                            state.set_quilt_versions(versions);
                            state.set_quilt_available(available);
                        }
                        "Forge" => {
                            state.set_forge_versions(versions);
                            state.set_forge_available(available);
                        }
                        _ => {
                            state.set_neoforge_versions(versions);
                            state.set_neoforge_available(available);
                        }
                    }
                }
                Err(error) => {
                    log::error!("failed to fetch the {loader} version list: {error}");
                }
            }
            // Regardless of the outcome, the loader is no longer loading.
            match loader {
                "Fabric" => state.set_fabric_loading(false),
                "Quilt" => state.set_quilt_loading(false),
                "Forge" => state.set_forge_loading(false),
                _ => state.set_neoforge_loading(false),
            }
        });
    });
}

/// A Forge version as the list shows it: everything after the Minecraft
/// version.
fn forge_version_label(version: &str) -> String {
    version.split('-').skip(1).collect::<Vec<_>>().join("-")
}

/// A Neoforge version as the filter picks them: those whose version string
/// encodes the given Minecraft version.
///
/// It is a UI-layer helper, so it lives here rather than in `install`.
fn filter_neoforge_version_list(mcversion: &str, versions: &[String]) -> Vec<String> {
    versions
        .iter()
        .filter(|version| {
            neoforge_minecraft_version(version).is_some_and(|minecraft| minecraft == mcversion)
        })
        // The list stays in the order the API returned it (newest first).
        .cloned()
        .collect()
}

/// The Minecraft version a Neoforge version targets.
///
/// A version is dot-separated numeric groups plus an optional `-suffix`. A
/// four-group version's first three groups are the Minecraft version, with a
/// trailing `0` dropped (`26.3.0.39-beta` yields `26.3`); a three-group version
/// omits the Minecraft version's leading `1`, which is restored here
/// (`20.2.57-beta` yields `1.20.2`).
fn neoforge_minecraft_version(version: &str) -> Option<String> {
    // `^(\d+)\.(\d+)\.(\d+)\.(\d+)(?:-([a-zA-Z0-9_]+))?$`
    if let Some(groups) = neoforge_groups(version, 4) {
        let (major, minor, patch) = (groups[0], groups[1], groups[2]);
        return Some(if patch == "0" {
            if minor == "0" {
                major.to_string()
            } else {
                format!("{major}.{minor}")
            }
        } else {
            format!("{major}.{minor}.{patch}")
        });
    }
    // The three-group form omits the leading `1`, so its first group reads as
    // `minor` and its second as `patch`:
    // `^(\d+)\.(\d+)\.(\d+)(?:-([a-zA-Z0-9_]+))?$`.
    if let Some(groups) = neoforge_groups(version, 3) {
        let (minor, patch) = (groups[0], groups[1]);
        return Some(if patch == "0" {
            format!("1.{minor}")
        } else {
            format!("1.{minor}.{patch}")
        });
    }
    None
}

/// Splits `version` into `count` dot-separated numeric groups plus an optional
/// `-suffix`.
fn neoforge_groups(version: &str, count: usize) -> Option<Vec<&str>> {
    let (numbers, suffix) = match version.split_once('-') {
        Some((numbers, suffix)) => (numbers, Some(suffix)),
        None => (version, None),
    };
    if let Some(suffix) = suffix
        && (suffix.is_empty()
            || !suffix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'))
    {
        return None;
    }
    let groups: Vec<&str> = numbers.split('.').collect();
    let numeric = |group: &&str| !group.is_empty() && group.chars().all(|c| c.is_ascii_digit());
    (groups.len() == count && groups.iter().all(numeric)).then_some(groups)
}

/// Writes the instance and copies the background over.
///
/// A name that is already taken gets a " 1", " 2", … suffix, compared against
/// the instance *ids*.
async fn create_instance(
    name: &str,
    minecraft: &str,
    loader_type: &str,
    loader_version: &str,
    background: &str,
) -> Result<String, instance::Error> {
    let instances = instance::list_instances(SortBy::Name)
        .await
        .unwrap_or_default();
    let mut suffix = 0;
    let id = loop {
        let candidate = if suffix == 0 {
            name.to_string()
        } else {
            format!("{name} {suffix}")
        };
        if !instances.iter().any(|instance| instance.id == candidate) {
            break candidate;
        }
        suffix += 1;
    };

    let modded = loader_type != "None";
    let config = InstanceConfig {
        name: name.to_string(),
        runtime: InstanceRuntime {
            minecraft: minecraft.to_string(),
            mod_loader_type: if modded {
                mod_loader_type(loader_type)
            } else {
                None
            },
            mod_loader_version: modded.then(|| loader_version.to_string()),
        },
        launch_config: InstanceLaunchConfig {
            enable_instance_specific_settings: false,
            ..Default::default()
        },
        ..Default::default()
    };
    instance::create_instance(config, Some(&id)).await?;
    if !background.is_empty() {
        instance::add_background_image(Path::new(background), &id).await?;
    }
    Ok(id)
}

/// The loader name of the dialog's select as the instance config's enum.
fn mod_loader_type(value: &str) -> Option<ModLoaderType> {
    match value {
        "Fabric" => Some(ModLoaderType::Fabric),
        "Quilt" => Some(ModLoaderType::Quilt),
        "Forge" => Some(ModLoaderType::Forge),
        "Neoforge" => Some(ModLoaderType::Neoforge),
        _ => None,
    }
}
