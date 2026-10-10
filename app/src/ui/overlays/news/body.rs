// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The changelog detail's HTML body and its layout.
//!
//! The engine is the app's one Markdown/HTML renderer
//! (`app/src/ui/components/markdown`); only the properties the display list
//! lands in are the news surface's own. The body is laid out for a width the
//! view reports and re-laid out whenever that width changes.
//!
//! Two deliberate simplifications over the content detail's body, both because
//! of what a changelog is: it has no `<details>` (so there are no collapsible
//! sections to animate or remember) and no images (an image item has no bitmap
//! to draw and is skipped, not reserved as an empty box).

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::slint_backend::{App, MdChunk, MdItem, NewsState};
use crate::ui::components::markdown::{ItemKind, MdStyle, Renderer, SourceFormat};
use crate::ui::overlays::content::{monospace_family, theme_font_family};

thread_local! {
    /// The Markdown engine behind the changelog body.
    ///
    /// A `thread_local` for the same reason the content body's is one: a
    /// `fontique::Collection` is not `Send`, and the engine holds one. Keeping
    /// it between changelogs also keeps the font cache, which a fresh engine
    /// would re-read from every font file.
    static BODY: RefCell<Option<Body>> = const { RefCell::new(None) };
}

struct Body {
    renderer: Renderer,
    width: f32,
    /// The family the engine measured code with; the view draws with it too.
    mono: String,
    /// The runs the view stacks.
    chunks: Rc<VecModel<MdChunk>>,
    /// A run's items, kept beside it so a re-layout rewrites them in place.
    run_items: Vec<Rc<VecModel<MdItem>>>,
    /// Which runs are open, index-aligned with `chunks`. A changelog has no
    /// `<details>`, so every value is false — but the view indexes against the
    /// model, so it still has to exist.
    section_open: Rc<VecModel<bool>>,
    /// Whether the models have been handed to the global yet.
    bound: bool,
}

impl Body {
    fn new() -> Self {
        let mut renderer = Renderer::with_collection(slint::fontique_011::shared_collection());
        let mono = monospace_family();
        renderer.set_style(MdStyle::default().with_families(theme_font_family(), mono.clone()));
        Self {
            renderer,
            width: 0.0,
            mono,
            chunks: Rc::new(VecModel::default()),
            run_items: Vec::new(),
            section_open: Rc::new(VecModel::default()),
            bound: false,
        }
    }
}

/// Puts a document in the engine and draws it if the view has reported a width
/// yet.
pub(crate) fn set_body(ui: &App, html: &str) {
    let (mono, measured) = BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let body = slot.get_or_insert_with(Body::new);
        body.renderer.set_source(html, SourceFormat::Html);
        (body.mono.clone(), body.width > 0.0)
    });
    ui.global::<NewsState>()
        .set_body_mono_family(SharedString::from(mono));
    if measured {
        push_display_list(ui);
    } else {
        clear_models(ui);
    }
}

pub(crate) fn layout(ui: &App, width: f32) {
    let changed = BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(body) = slot.as_mut() else {
            return false;
        };
        // `set_width` ignores sub-pixel changes on its own and reports whether
        // anything needs laying out again.
        if body.renderer.set_width(width) {
            body.width = width;
            true
        } else {
            false
        }
    });
    if changed {
        push_display_list(ui);
    }
}

/// Empties the engine's document *and* what the panel draws, for a detail panel
/// that is opening another changelog. The width stays: it belongs to the view,
/// which does not go away between changelogs, and clearing it would mean waiting
/// for a `changed width` that never comes.
pub(crate) fn clear(ui: &App) {
    clear_models(ui);
    BODY.with(|slot| {
        if let Some(body) = slot.borrow_mut().as_mut() {
            body.renderer
                .set_source(String::new(), SourceFormat::Markdown);
        }
    });
}

/// Empties the models and nothing else. The document stays, because the width
/// that lets it be drawn may be a moment away.
fn clear_models(ui: &App) {
    let state = ui.global::<NewsState>();
    state.set_body_hovered(SharedString::default());
    state.set_body_height(0.0);
    BODY.with(|slot| {
        if let Some(body) = slot.borrow_mut().as_mut() {
            body.chunks.set_vec(Vec::new());
            body.run_items.clear();
            body.section_open.set_vec(Vec::new());
        }
    });
}

/// One run of the laid-out document: the row's value and its own item model.
struct Run {
    chunk: MdChunk,
    items: Vec<MdItem>,
}

/// The engine's display list for the width it has, or `None` until the view has
/// reported one — which is not the same as an empty document.
fn body_layout() -> Option<(Vec<Run>, f32)> {
    BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let body = slot.as_mut()?;
        if body.width <= 0.0 {
            return None;
        }
        let list = body.renderer.layout();
        let runs = list
            .chunks
            .iter()
            .map(|run| Run {
                chunk: MdChunk {
                    section_index: -1,
                    top: run.top,
                    height: run.height,
                    ..Default::default()
                },
                items: run_items(list, run),
            })
            .collect();
        Some((runs, list.height))
    })
}

/// One run's items, in the engine's paint order.
fn run_items(
    list: &crate::ui::components::markdown::DisplayList,
    run: &crate::ui::components::markdown::model::MdChunk,
) -> Vec<MdItem> {
    let mut items = Vec::with_capacity(run.items.len());
    for item in &list.items[run.items.clone()] {
        if item.kind == ItemKind::Image {
            continue;
        }
        items.push(MdItem {
            kind: SharedString::from(item.kind.name()),
            x: item.x,
            // An item's own `y` is relative to its run, because the view stacks
            // the runs and places a run's items against it.
            y: item.y - run.top,
            width: item.width,
            height: item.height,
            text: SharedString::from(item.text.as_str()),
            font_size: item.font_size,
            font_weight: i32::from(item.font_weight),
            font_italic: item.font_italic,
            mono: item.mono,
            color: SharedString::from(item.color.name()),
            radius: item.radius,
            filled: item.filled,
            border_width: item.border_width,
            border_color: SharedString::from(item.border_color.name()),
            link_url: SharedString::from(item.link_url.as_str()),
            link_id: SharedString::from(item.link_id.as_str()),
            rule_offset: item.rule_offset,
            rule: item.rule,
        });
    }
    items
}

fn push_display_list(ui: &App) {
    let state = ui.global::<NewsState>();
    let Some((runs, height)) = body_layout() else {
        clear_models(ui);
        return;
    };
    BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(body) = slot.as_mut() else {
            return;
        };
        bind_models(body, &state);
        write_runs(body, runs);
        write_sections(body);
    });
    state.set_body_height(height);
}

/// Hands the models to the global the first time a document is drawn. From then
/// on they are written in place, so the view keeps its elements.
fn bind_models(body: &mut Body, state: &NewsState) {
    if body.bound {
        return;
    }
    body.bound = true;
    state.set_body_chunks(ModelRc::from(body.chunks.clone()));
    state.set_body_section_open(ModelRc::from(body.section_open.clone()));
}

/// Rewrites the run models from a fresh layout, reusing each run's item model
/// where its item count has not changed.
fn write_runs(body: &mut Body, runs: Vec<Run>) {
    if body.run_items.len() != runs.len() {
        body.run_items = runs
            .iter()
            .map(|run| Rc::new(VecModel::from(run.items.clone())))
            .collect();
        let rows: Vec<MdChunk> = runs
            .iter()
            .enumerate()
            .map(|(index, run)| MdChunk {
                items: ModelRc::from(body.run_items[index].clone()),
                ..run.chunk.clone()
            })
            .collect();
        body.chunks.set_vec(rows);
        return;
    }
    for (index, run) in runs.into_iter().enumerate() {
        set_items(&body.run_items[index], run.items);
        let current = body.chunks.row_data(index).unwrap_or_default();
        let items = ModelRc::from(body.run_items[index].clone());
        if current.items != items || current != run.chunk {
            body.chunks
                .set_row_data(index, MdChunk { items, ..run.chunk });
        }
    }
}

/// Writes one run's items into its model, replacing the model only when the
/// item count changed.
fn set_items(model: &Rc<VecModel<MdItem>>, items: Vec<MdItem>) {
    if model.row_count() != items.len() {
        model.set_vec(items);
    } else {
        for (row, item) in items.into_iter().enumerate() {
            model.set_row_data(row, item);
        }
    }
}

/// Keeps the section-open model index-aligned with the runs. Every entry is
/// false, because a changelog has no `<details>`.
fn write_sections(body: &mut Body) {
    let open = vec![false; body.chunks.row_count()];
    if body.section_open.row_count() != open.len() {
        body.section_open.set_vec(open);
    } else {
        for (index, value) in open.into_iter().enumerate() {
            body.section_open.set_row_data(index, value);
        }
    }
}
