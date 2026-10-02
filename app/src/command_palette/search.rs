// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The palette's two remote searches and the JSON they parse.

use super::*;

/// Searches Modrinth with no facets, so the hits are mods, resource packs,
/// modpacks and shaders alike — which is what the row's category subtitle is
/// for.
pub(crate) async fn search_modrinth(keyword: &str) -> Result<Vec<RemoteResult>, String> {
    let params = modrinth::SearchParameters {
        query: Some(keyword.to_string()),
        facets: None,
        index: None,
        offset: Some(0),
        limit: Some(SEARCH_LIMIT),
    };
    let response = modrinth::search_projects(&params)
        .await
        .map_err(|error| error.to_string())?;
    let hits = response
        .get("hits")
        .and_then(Value::as_array)
        .ok_or("the search response carried no hits")?;
    Ok(hits
        .iter()
        .map(|hit| {
            let project_id = crate::json::string(hit, "project_id");
            RemoteResult {
                key: format!("modrinth-{project_id}"),
                title: first_present(hit, &["title", "slug", "project_id"]),
                author: crate::json::string(hit, "author"),
                icon_url: crate::json::string(hit, "icon_url"),
                loaders: strings(hit.get("display_categories")),
                subtitle_kind: modrinth_type(&crate::json::string(hit, "project_type")),
                platform: "modrinth",
                id: project_id,
            }
        })
        .collect())
}

/// Searches CurseForge mods for the keyword.
pub(crate) async fn search_curseforge(keyword: &str) -> Result<Vec<RemoteResult>, String> {
    let params = serde_json::json!({
        "searchFilter": keyword,
        "index": 0,
        "pageSize": SEARCH_LIMIT,
    });
    let response = curseforge::search_mods(&params)
        .await
        .map_err(|error| error.to_string())?;
    let mods = response
        .get("data")
        .and_then(Value::as_array)
        .ok_or("the search response carried no data")?;
    Ok(mods
        .iter()
        .map(|entry| {
            let id = entry.get("id").and_then(Value::as_i64).unwrap_or_default();
            RemoteResult {
                key: format!("curseforge-{id}"),
                title: crate::json::string(entry, "name"),
                author: entry
                    .get("authors")
                    .and_then(Value::as_array)
                    .and_then(|authors| authors.first())
                    .map(|author| crate::json::string(author, "name"))
                    .unwrap_or_default(),
                icon_url: entry
                    .pointer("/logo/thumbnailUrl")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                loaders: entry
                    .get("categories")
                    .and_then(Value::as_array)
                    .map(|categories| {
                        categories
                            .iter()
                            .filter_map(|category| category.get("slug").and_then(Value::as_str))
                            .filter(|slug| crate::content::LOADER_SLUGS.contains(slug))
                            .map(ToString::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                subtitle_kind: curseforge_type(entry.get("classId").and_then(Value::as_i64)),
                platform: "curseforge",
                id: id.to_string(),
            }
        })
        .collect())
}

/// Maps a CurseForge class id to the row's subtitle kind: 6 mods, 12 resource
/// packs, 4471 modpacks. A class with no entry — a shader, say — leaves the row
/// without a subtitle.
pub(crate) fn curseforge_type(class_id: Option<i64>) -> &'static str {
    match class_id {
        Some(6) => "mod",
        Some(12) => "resourcepack",
        Some(4471) => "modpack",
        _ => "",
    }
}

/// The Modrinth `project_type`, which the search types as a plain string.
pub(crate) fn modrinth_type(project_type: &str) -> &'static str {
    match project_type {
        "mod" => "mod",
        "modpack" => "modpack",
        "resourcepack" => "resourcepack",
        "shader" => "shader",
        _ => "",
    }
}

/// The loader slugs of a Modrinth hit's `display_categories`, the ones the
/// palette has a tag for.
pub(crate) fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .filter(|slug| crate::content::LOADER_SLUGS.contains(slug))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The first of `names` that `value` carries with a non-empty string.
pub(crate) fn first_present(value: &Value, names: &[&str]) -> String {
    names
        .iter()
        .filter_map(|name| value.get(name).and_then(Value::as_str))
        .find(|found| !found.is_empty())
        .unwrap_or_default()
        .to_string()
}

/// Fetches and decodes the results' icons, detached from the results themselves.
///
/// They are not worth waiting for: the icon fills in beside a row that is
/// already there, and holding the list back for twenty of them made the palette
/// look dead for as long as the slowest CDN took. Each is a separate task, so
/// twenty of them take one round trip rather than twenty, and each gives up on
/// its own so one dead host cannot hold the rest.
pub(crate) fn spawn_icon_fetch(weak: Weak<App>, token: u64, wanted: Vec<(usize, String)>) {
    crate::runtime::spawn(async move {
        let mut tasks = Vec::with_capacity(wanted.len());
        for (index, url) in wanted {
            tasks.push(crate::runtime::spawn(async move {
                let bytes =
                    match tokio::time::timeout(ICON_TIMEOUT, content::fetch_icon_bytes(&url)).await
                    {
                        Ok(bytes) => bytes,
                        Err(_) => {
                            log::warn!("command palette icon timed out: {url}");
                            None
                        }
                    };
                (index, url, bytes)
            }));
        }
        let mut icons: Vec<(usize, String, Option<Vec<u8>>)> = Vec::with_capacity(tasks.len());
        for task in tasks {
            match task.await {
                Ok(entry) => icons.push(entry),
                Err(error) => log::error!("the icon fetch panicked: {error}"),
            }
        }
        // Only the decode is left, and that is the part that wants a blocking
        // thread.
        let decoded = crate::runtime::spawn_blocking(move || {
            icons
                .into_iter()
                .map(|(index, url, bytes)| {
                    (
                        index,
                        bytes.and_then(|bytes| content::decode_icon(&url, bytes)),
                    )
                })
                .collect::<Vec<_>>()
        })
        .await
        .unwrap_or_default();
        let _ = weak.upgrade_in_event_loop(move |ui| {
            let controller = controller();
            let mut state = controller.borrow_mut();
            if state.token != token {
                return;
            }
            state.attach_icons(&ui, decoded);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curseforge_class_ids_map_to_a_subtitle() {
        assert_eq!(curseforge_type(Some(6)), "mod");
        assert_eq!(curseforge_type(Some(12)), "resourcepack");
        assert_eq!(curseforge_type(Some(4471)), "modpack");
        // A shader has no entry and therefore no subtitle.
        assert_eq!(curseforge_type(Some(999)), "");
        assert_eq!(curseforge_type(None), "");
    }

    #[test]
    fn modrinth_project_types_map_to_a_subtitle() {
        assert_eq!(modrinth_type("mod"), "mod");
        assert_eq!(modrinth_type("modpack"), "modpack");
        assert_eq!(modrinth_type("shader"), "shader");
        assert_eq!(modrinth_type("plugin"), "");
    }

    #[test]
    fn only_loader_slugs_survive_the_category_filter() {
        let value = serde_json::json!(["fabric", "shader", "forge", "quilt", 3, null]);
        assert_eq!(strings(Some(&value)), ["fabric", "forge", "quilt"]);
        assert!(strings(None).is_empty());
    }

    #[test]
    fn the_first_non_empty_named_field_wins() {
        let value = serde_json::json!({ "title": "", "slug": "lithium", "project_id": "abc" });
        assert_eq!(
            first_present(&value, &["title", "slug", "project_id"]),
            "lithium"
        );
        assert_eq!(first_present(&value, &["missing"]), "");
    }
}
