// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The remote project detail panels.

use super::*;

/// Opens a project's detail panel from outside the content overlays — the
/// command palette's "open" on a Modrinth or CurseForge search result.
///
/// A card click and a palette result take the same `open_detail` path, so the
/// only thing to set up first is the kind and the platform. Both are the mods'
/// whatever the project actually is: a palette search is sent without facets,
/// so a resource pack opened this way opens the *mod* panel.
pub(crate) fn open_project_detail(ui: &App, platform: &str, id: &str) {
    let platform = if platform == "curseforge" {
        Platform::CurseForge
    } else {
        Platform::Modrinth
    };
    {
        let state = controller();
        let mut state = state.borrow_mut();
        state.sync_instance(ui);
        state.platform = platform;
        state.kind = RemoteKind::Mods;
    }
    ui.global::<ContentState>()
        .set_remote_kind(SharedString::from(RemoteKind::Mods.key()));
    open_detail(ui, platform, id.to_string());
}

pub(crate) fn open_detail(ui: &App, platform: Platform, id: String) {
    let (seq, kind) = {
        let state = controller();
        let mut state = state.borrow_mut();
        state.detail_seq += 1;
        state.detail = Some(OpenDetail {
            platform,
            kind: state.kind,
            id: id.clone(),
            installed_mods: Vec::new(),
        });
        (state.detail_seq, state.kind)
    };
    let ui_state = ui.global::<ContentState>();
    ui_state.set_open_detail(SharedString::from(platform.key()));
    ui_state.set_detail_id(SharedString::from(id.as_str()));
    ui_state.set_detail_loading(true);
    // Only a mod panel has an installed state; a pack or resource-pack one only
    // ever draws the download button.
    ui_state.set_detail_can_remove(kind == RemoteKind::Mods);
    // The Modrinth panels render the body's box unconditionally, the CurseForge
    // ones only when there is a description — an empty body still draws an empty
    // surface0 box on the first, and nothing on the second.
    ui_state.set_detail_body_always(platform == Platform::Modrinth);
    ui_state.set_detail_installed(false);
    ui_state.set_detail_installed_version(SharedString::default());
    ui_state.set_detail_favorited(false);
    ui_state.set_detail_body(SharedString::default());
    // The body of the panel being left, not just its string: the display list
    // belongs to the document that was in the box, and a stale one would be drawn
    // under the new one's title until the new one is measured.
    clear_detail_body(ui);
    ui_state.set_detail_gallery(ModelRc::default());

    let weak = ui.as_weak();
    crate::support::runtime::spawn(async move {
        let loaded = load_detail(platform, &id).await;
        crate::ui::services::report::report(&weak, move |ui| {
            // A different panel has been opened since this request started.
            if controller().borrow().detail_seq != seq {
                return;
            }
            ui.global::<ContentState>().set_detail_loading(false);
            match loaded {
                Ok(loaded) => {
                    let ui_state = ui.global::<ContentState>();
                    ui_state.set_detail_title(SharedString::from(loaded.title));
                    ui_state.set_detail_icon(
                        loaded
                            .icon
                            .and_then(resolve_icon)
                            .or_else(unknown_icon)
                            .unwrap_or_default(),
                    );
                    ui_state.set_detail_source(SharedString::from(loaded.source_url));
                    ui_state.set_detail_source_label(SharedString::from(loaded.source_label));
                    ui_state.set_detail_source_is_github(loaded.source_is_github);
                    ui_state.set_detail_downloads(SharedString::from(loaded.downloads));
                    ui_state.set_detail_followers(SharedString::from(loaded.followers));
                    ui_state.set_detail_has_followers(loaded.has_followers);
                    ui_state.set_detail_description(SharedString::from(loaded.description));
                    // The string goes in for the record — the panel's own
                    // conditional reads it, and so does anything that wants to
                    // know whether there is a body at all — and the layout
                    // follows from it.
                    ui_state.set_detail_body(SharedString::from(loaded.body.as_str()));
                    set_detail_body(&ui, &loaded.body, loaded.body_is_html);
                    ui_state.set_detail_gallery(ModelRc::from(Rc::new(VecModel::from(
                        loaded
                            .gallery
                            .into_iter()
                            .filter_map(resolve_gallery_shot)
                            .collect::<Vec<GalleryShot>>(),
                    ))));
                }
                Err(error) => log::error!("failed to load the detail panel: {error}"),
            }
            // A body that is only text needs nothing fetched, and this returns
            // immediately when it has no images to fetch.
            fetch_body_images(&ui);
            refresh_favorited(&ui);
            refresh_installed(&ui);
            ensure_translations(&ui, platform, vec![id]);
        });
    });
}

pub(crate) struct LoadedDetail {
    title: String,
    icon: Option<PendingImage>,
    source_url: String,
    source_label: String,
    source_is_github: bool,
    downloads: String,
    followers: String,
    has_followers: bool,
    description: String,
    body: String,
    /// Whether `body` is HTML rather than Markdown. A CurseForge description
    /// arrives as HTML, a Modrinth body as Markdown, and the two go through
    /// different parsers.
    body_is_html: bool,
    gallery: Vec<PendingImage>,
}

pub(crate) async fn load_detail(platform: Platform, id: &str) -> Result<LoadedDetail, String> {
    match platform {
        Platform::Modrinth => load_modrinth_detail(id).await,
        Platform::CurseForge => load_curseforge_detail(id).await,
    }
}

/// A Modrinth project's detail panel. Its README is Markdown.
pub(crate) async fn load_modrinth_detail(id: &str) -> Result<LoadedDetail, String> {
    let project = modrinth::get_project(id)
        .await
        .map_err(|error| error.to_string())?;
    let source_url = project
        .get("source_url")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    // A GitHub URL is shown as `owner/repo`.
    let github = github_repo(&source_url);
    let gallery: Vec<PendingImage> = project
        .get("gallery")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("url").and_then(Value::as_str))
                .filter_map(fetch_icon)
                .collect()
        })
        .unwrap_or_default();
    Ok(LoadedDetail {
        title: crate::support::json::string(&project, "title"),
        icon: project
            .get("icon_url")
            .and_then(Value::as_str)
            .and_then(fetch_icon),
        source_label: github.clone().unwrap_or_else(|| source_url.clone()),
        source_is_github: github.is_some(),
        source_url,
        downloads: number_label(project.get("downloads").and_then(Value::as_i64)),
        followers: number_label(project.get("followers").and_then(Value::as_i64)),
        has_followers: project
            .get("followers")
            .is_some_and(|value| !value.is_null()),
        description: crate::support::json::string(&project, "description"),
        // The README, as the project ships it. The body is parsed and laid out
        // by the `markdown` module and drawn by `markdown-body.slint`, so the
        // markup reaches the panel intact.
        body: crate::support::json::string(&project, "body"),
        body_is_html: false,
        gallery,
    })
}

/// A CurseForge mod's detail panel. Its description arrives as HTML, so the
/// body is parsed by the tag-soup front end instead of the Markdown one; both
/// converge on one block tree, so a heading is the same block either way.
pub(crate) async fn load_curseforge_detail(id: &str) -> Result<LoadedDetail, String> {
    let mod_id: i64 = id
        .parse()
        .map_err(|_| "not a CurseForge mod id".to_string())?;
    let response = curseforge::get_mod(mod_id)
        .await
        .map_err(|error| error.to_string())?;
    let mod_info = response.get("data").cloned().unwrap_or(Value::Null);
    let source_url = mod_info
        .pointer("/links/sourceUrl")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let description = curseforge::get_mod_description(mod_id, &json!({ "markup": true }))
        .await
        .map_err(|error| error.to_string())?;
    let html = description
        .get("data")
        .and_then(Value::as_str)
        .unwrap_or_default();
    // A description carrying a `<script>` or `<style>` block is refused outright
    // rather than sanitised.
    let unsafe_html = {
        let lower = html.to_lowercase();
        lower.contains("<script") || lower.contains("<style")
    };
    let thumbs_up = mod_info.get("thumbsUpCount").and_then(Value::as_i64);
    Ok(LoadedDetail {
        title: crate::support::json::string(&mod_info, "name"),
        icon: mod_info
            .pointer("/logo/url")
            .and_then(Value::as_str)
            .and_then(fetch_icon),
        source_label: source_url.clone(),
        source_is_github: github_repo(&source_url).is_some(),
        source_url,
        downloads: number_label(mod_info.get("downloadCount").and_then(Value::as_i64)),
        followers: number_label(thumbs_up),
        // The thumbs-up entry is only drawn when it is non-zero.
        has_followers: thumbs_up.is_some_and(|count| count > 0),
        description: crate::support::json::string(&mod_info, "summary"),
        body: if unsafe_html {
            String::new()
        } else {
            html.trim().to_string()
        },
        body_is_html: true,
        // Galleries are a Modrinth-only feature.
        gallery: Vec::new(),
    })
}
