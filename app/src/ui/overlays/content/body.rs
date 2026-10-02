// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The detail panel's Markdown/HTML body and its layout.

use super::*;

thread_local! {
    /// The Markdown engine behind the detail panel's body.
    ///
    /// A `thread_local` for the same reason `CONTROLLER` is one: a
    /// `fontique::Collection` is not `Send`, and the engine holds one. It is also
    /// worth keeping between panels, because the collection is a font cache and
    /// throwing it away per panel would re-read every font file on every open.
    static BODY: RefCell<Option<BodyRenderer>> = const { RefCell::new(None) };
}

pub(crate) struct BodyRenderer {
    renderer: crate::ui::components::markdown::Renderer,
    /// The width the display list was last laid out for, so a repeat of the same
    /// width — which a `changed width` binding produces while nothing has moved —
    /// does not re-measure the document.
    width: f32,
    /// The two models, kept rather than rebuilt per layout for the reason the
    /// grids keep theirs (`set_cards`): a document is re-laid out on every pixel
    /// of a window drag, and replacing the model would destroy and re-create
    /// every item element each time.
    /// The runs, and each run's own items and bitmaps. A `VecModel` per run rather
    /// than one for the document, because a run is what a view stacks and what a
    /// `<details>` moves: the items have to be nested inside it or the view cannot
    /// put them back where they belong. The count is the number of `<details>` in
    /// the document plus one, so these are few.
    chunks: Rc<VecModel<MdChunk>>,
    /// A run's items and bitmaps, kept beside it so a re-layout rewrites them in
    /// place: a window drag re-lays the document out, and replacing a model would
    /// destroy and re-create every item element each time.
    run_items: Vec<Rc<VecModel<MdItem>>>,
    run_images: Vec<Rc<VecModel<MarkdownImage>>>,
    /// Which runs are open, index-aligned with `chunks`, held as a model for the
    /// same reason: a layout replaces it wholesale, and a toggle writes the one
    /// row that changed.
    section_open: Rc<VecModel<bool>>,
    /// Which sections the reader has opened, by section id.
    ///
    /// Kept here rather than left to the view because a width change re-lays the
    /// document out, and that rebuilds the sections: state the view held in a
    /// property would be gone the first time the window moved a pixel, and a
    /// `<details>` that collapsed itself on a window drag is worse than one that
    /// cannot collapse at all.
    open_sections: Vec<(u32, bool)>,
    /// Whether the models have been handed to the global yet. They are created
    /// with the engine and outlive a `clear`, so a panel that is reopened finds
    /// them already bound.
    bound: bool,
}

impl BodyRenderer {
    /// An engine for a test: the platform's fonts rather than Slint's.
    ///
    /// `shared_collection()` initialises the window backend, which has to happen
    /// once and on the main thread, so a test that built one would fail for a
    /// reason that has nothing to do with the body. These tests are about the
    /// sequencing, and the family does not change whether a document is still in
    /// the engine when the width arrives.
    #[cfg(test)]
    fn for_test() -> Self {
        let mut renderer = crate::ui::components::markdown::Renderer::with_collection(
            crate::ui::components::markdown::fonts::system(),
        );
        renderer.set_style(
            crate::ui::components::markdown::MdStyle::default()
                .with_families(theme_font_family(), "monospace"),
        );
        Self {
            renderer,
            width: 0.0,
            chunks: Rc::new(VecModel::default()),
            run_items: Vec::new(),
            run_images: Vec::new(),
            section_open: Rc::new(VecModel::default()),
            open_sections: Vec::new(),
            bound: false,
        }
    }

    /// A new engine, with the body's Markdown style.
    ///
    /// The collection is Slint's own, and that is not a convenience: the engine
    /// measures with it and the panel draws with it, so two collections would be
    /// two sets of advances and every line break would be wrong. The families are
    /// named because the same has to be true of the family — the engine shapes
    /// with `Comfortaa Nunito` and the view draws with `Theme.font-family`, which
    /// is that face.
    fn new() -> Self {
        let mut renderer = crate::ui::components::markdown::Renderer::with_collection(
            slint::fontique_011::shared_collection(),
        );
        let mono = monospace_family();
        renderer.set_style(
            crate::ui::components::markdown::MdStyle::default()
                .with_families(theme_font_family(), mono.clone()),
        );
        // The view is told the same name, and it has to be told: see
        // [`monospace_family`].
        MONO_FAMILY.with(|slot| *slot.borrow_mut() = mono);
        Self {
            renderer,
            width: 0.0,
            chunks: Rc::new(VecModel::default()),
            run_items: Vec::new(),
            run_images: Vec::new(),
            section_open: Rc::new(VecModel::default()),
            open_sections: Vec::new(),
            bound: false,
        }
    }
}

thread_local! {
    /// The monospace family the view draws code with, resolved once by
    /// [`monospace_family`].
    static MONO_FAMILY: RefCell<String> = const { RefCell::new(String::new()) };
}

/// The monospace family, named the way *this* platform's font stack names it.
///
/// CSS resolves the generic `monospace` itself. Slint's `font-family` is not
/// CSS: it takes a family *name*, and the generic names mean nothing to it, so
/// a view given `"monospace"` falls back to the window's default face. The
/// engine, on the other hand, resolves `GenericFamily::Monospace` and shaped
/// with a real monospace font — so the two
/// halves measured and drew in *different faces*, which is the worst of both
/// worlds: the code came out in the body typeface, and its advances were narrower
/// than the box the engine had reserved for it, which showed as a band of empty
/// space at the right of every inline-code capsule.
///
/// Asking fontique which family it registers as the generic monospace is portable
/// and names a real family when it has an answer. On this machine it answers
/// *Courier*, though, which is a monospace face from 1954 and looks like it next
/// to a rounded sans — and a browser asked the same question on the same machine
/// says Menlo or SF Mono. So a short preference list is tried first, and
/// fontique's own answer is the fallback rather than the first choice.
pub(crate) fn monospace_family() -> String {
    use slint::fontique_011::fontique::GenericFamily;
    let mut collection = slint::fontique_011::shared_collection();

    // Best first, per platform. The names a system registers privately are
    // spelled with the leading dot they are registered under.
    #[cfg(target_os = "macos")]
    const PREFERRED: &[&str] = &[
        ".SF NS Mono",
        "SF Mono",
        "Menlo",
        "Monaco",
        "Andale Mono",
        "PT Mono",
        "Courier New",
    ];
    #[cfg(target_os = "windows")]
    const PREFERRED: &[&str] = &["Cascadia Mono", "Consolas", "Lucida Console"];
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    const PREFERRED: &[&str] = &[
        "DejaVu Sans Mono",
        "Liberation Mono",
        "Noto Sans Mono",
        "Ubuntu Mono",
    ];

    for name in PREFERRED {
        if collection.family_id(name).is_some() {
            return (*name).to_string();
        }
    }

    let id = collection.generic_families(GenericFamily::Monospace).next();
    id.and_then(|id| collection.family_name(id).map(str::to_owned))
        .unwrap_or_else(|| {
            log::warn!("no monospace family is registered; code will draw in the body face");
            "monospace".to_string()
        })
}

/// The monospace family the body was laid out with, for the view to draw with.
pub(crate) fn body_mono_family() -> String {
    MONO_FAMILY.with(|family| family.borrow().clone())
}

/// The body face, which has to be the one `Theme.font-family` names.
///
/// It is a constant (`theme.slint`) rather than a global Rust can read, and a
/// second place to change it is a second place to get wrong: a body laid out
/// with one face and drawn with another has every line the wrong length. A
/// `Theme` token that moved would have to move here too.
pub(crate) fn theme_font_family() -> &'static str {
    "Comfortaa Nunito"
}

/// Gives the panel a body and lays it out for the width it already has.
///
/// The view reports the width through `body-resized`, so a body set before the
/// panel has been measured waits for that; one set after — a second project
/// opened into a panel that is already on screen — is laid out at once.
pub(crate) fn set_detail_body(ui: &App, body: &str, is_html: bool) {
    if set_body_source(body, is_html) {
        push_detail_body(ui);
    } else {
        // The panel has not been measured yet, so there is nothing to lay the
        // document out for and nothing to draw. Only the *model* is emptied. The
        // document is not, because the width arrives a moment later — the view's
        // `changed width` — and by then the document has to still be there.
        clear_detail_body_model(ui);
    }
}

/// Opens or closes a `<details>`, by the id the engine gave it.
///
/// The id rather than an index, because the view is the one holding the array and
/// an index would only be meaningful to whoever last laid the document out. The
/// flip is applied to the bound model and *then* remembered: the model is what the
/// next layout would rebuild from, and a section that reopened by itself after a
/// window drag would be a worse bug than a lost toggle.
pub(crate) fn toggle_detail_section(_ui: &App, id: u32) {
    // The state lives in the bound model, so there is nothing to write through the
    // UI here: `section_open` is what the view reads, and it is updated below.
    let toggled = BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let body = slot.as_mut()?;
        // The run is named by the section id the view read off it, so the two do
        // not have to agree on an index.
        let index = (0..body.chunks.row_count()).find(|index| {
            body.chunks
                .row_data(*index)
                .is_some_and(|run| run.section_index == id as i32)
        })?;
        let open = body.section_open.row_data(index)?;
        let next = !open;
        body.section_open.set_row_data(index, next);
        // One entry per id: a second document numbers its sections from zero
        // again, and a remembered answer from the last one would open this
        // document's first section by accident.
        body.open_sections.retain(|(seen, _)| *seen != id);
        body.open_sections.push((id, next));
        Some(next)
    });
    if toggled.is_none() {
        log::warn!("a <details> was toggled that the engine no longer has: {id}");
    }
}

/// Puts a document in the engine, and reports whether there is a width to lay it
/// out for.
pub(crate) fn set_body_source(body: &str, is_html: bool) -> bool {
    set_body_source_with(body, is_html, BodyRenderer::new)
}

/// [`set_body_source`], with the engine's constructor handed in.
///
/// A constructor rather than a collection, because the collection itself is a
/// `fontique` type the app does not depend on directly, and because a test has
/// to be able to build an engine without the window backend — which
/// `shared_collection()` initialises, once, on the main thread.
pub(crate) fn set_body_source_with(
    body: &str,
    is_html: bool,
    new_engine: fn() -> BodyRenderer,
) -> bool {
    let format = if is_html {
        crate::ui::components::markdown::SourceFormat::Html
    } else {
        crate::ui::components::markdown::SourceFormat::Markdown
    };
    BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let engine = slot.get_or_insert_with(new_engine);
        engine.renderer.set_source(body, format);
        // Which `<details>` the reader opened belongs to the document they opened
        // it in. The engine numbers its sections from zero every time, so a
        // remembered answer would land on whatever section happens to sit at that
        // index in the *new* page.
        engine.open_sections.clear();
        engine.width > 0.0
    })
}

/// Whether the engine holds a document. Only in the tests.
#[cfg(test)]
pub(crate) fn body_has_document() -> bool {
    BODY.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|body| !body.renderer.blocks().is_empty())
    })
}

/// Lays the body out for a width the view reported, and pushes the result.
pub(crate) fn layout_detail_body(ui: &App, width: f32) {
    let changed = BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(body) = slot.as_mut() else {
            return false;
        };
        // `set_width` ignores sub-pixel changes on its own and reports whether
        // anything needs laying out again, which is also how a repeated width is
        // skipped.
        if body.renderer.set_width(width) {
            body.width = width;
            true
        } else {
            false
        }
    });
    if changed {
        push_detail_body(ui);
    }
}

/// What the engine says the panel should draw, with the UI left out of it.
///
/// The split is what makes the sequencing testable: whether a document set
/// before the panel has been measured survives until the width arrives is a
/// question about the engine and nothing else, and a function that took an `App`
/// could only be answered by opening a window.
pub(crate) struct BodyLayout {
    /// The runs, each with the items and bitmaps it owns.
    chunks: Vec<BodyRun>,
    /// Which runs the reader has opened, index-aligned with `chunks`. A run nobody
    /// has an answer for takes the document's own `open`, which is the
    /// `<details open>` attribute: a README's sections have none, so they start
    /// closed.
    section_open: Vec<bool>,
    /// How tall the engine laid the document out, with every section open. The
    /// panel does not use it — the box measures itself from the view, which
    /// shrinks as sections close, and the engine's height would leave a band of
    /// empty space under the document — but it is what the layout tests assert
    /// on, and what says whether the engine produced anything at all.
    height: f32,
}

/// One run, in the two forms it has to be in: the values the model row is built
/// from, and the two nested models the row holds.
pub(crate) struct BodyRun {
    chunk: MdChunk,
    items: Vec<MdItem>,
    images: Vec<MarkdownImage>,
}

/// Lays the engine's document out for the width it has, if it has one.
///
/// `None` until the view has reported a width. It is not the same as an empty
/// layout: an empty document is a real answer at a real width, and a document
/// nobody has measured yet has to be kept, because the width arrives a moment
/// later and by then it still has to be there.
pub(crate) fn body_layout() -> Option<BodyLayout> {
    BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let body = slot.as_mut()?;
        if body.width <= 0.0 {
            return None;
        }
        let list = body.renderer.layout();
        let mut chunks = Vec::with_capacity(list.chunks.len());
        let mut section_open = Vec::with_capacity(list.chunks.len());
        for run in &list.chunks {
            let mut items: Vec<MdItem> = Vec::with_capacity(run.items.len());
            let mut images: Vec<MarkdownImage> = Vec::new();
            for item in &list.items[run.items.clone()] {
                // A run's items are placed against the *run*, not the document:
                // the view stacks the runs and a run is what a `<details>` moves,
                // so an item's own `y` is only right relative to its own run. The
                // document's coordinates are the engine's, and stay absolute there.
                let y = item.y - run.top;
                // A bitmap is not an `MdItem`: a `slint::Image` cannot be left
                // empty in a struct literal, so an item without one would not be a
                // value the panel could build at all. The two lists share one
                // coordinate space and the view draws each in its own loop.
                if item.kind == crate::ui::components::markdown::ItemKind::Image {
                    images.push(MarkdownImage {
                        x: item.x,
                        y,
                        width: item.width,
                        height: item.height,
                        radius: item.radius,
                        alt: SharedString::from(item.text.as_str()),
                        bitmap: item.image.clone().unwrap_or_default(),
                        link_url: SharedString::from(item.link_url.as_str()),
                        link_id: SharedString::from(item.link_id.as_str()),
                    });
                    continue;
                }
                items.push(MdItem {
                    kind: SharedString::from(item.kind.name()),
                    x: item.x,
                    y,
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
            // A run that is not a `<details>` has nothing to open, and the value
            // the array holds for it is never read — but the array is
            // index-aligned with the runs, so it has a slot either way.
            let open = match &run.section {
                None => false,
                Some(section) => body
                    .open_sections
                    .iter()
                    .find(|(id, _)| *id == section.id)
                    .map(|(_, open)| *open)
                    .unwrap_or(section.open),
            };
            section_open.push(open);
            let (
                head_x,
                head_y,
                head_width,
                head_height,
                head_radius,
                marker_x,
                marker_y,
                marker_size,
                hidden,
            ) = match &run.section {
                None => (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                Some(section) => (
                    section.head_x,
                    section.head_y,
                    section.head_width,
                    section.head_height,
                    section.head_radius,
                    section.marker_x,
                    section.marker_y,
                    section.marker_size,
                    section.hidden,
                ),
            };
            chunks.push(BodyRun {
                chunk: MdChunk {
                    section_index: run.section.as_ref().map_or(-1, |section| section.id as i32),
                    top: run.top,
                    height: run.height,
                    hidden,
                    head_x,
                    head_y,
                    head_width,
                    head_height,
                    head_radius,
                    marker_x,
                    marker_y,
                    marker_size,
                    // Filled in below, once the run's own models exist: a model
                    // cannot be part of the value the row is built from and part of
                    // the model at the same time.
                    items: ModelRc::default(),
                    images: ModelRc::default(),
                },
                items,
                images,
            });
        }
        Some(BodyLayout {
            chunks,
            section_open,
            height: list.height,
        })
    })
}

/// Pushes the display list into the model the panel draws.
pub(crate) fn push_detail_body(ui: &App) {
    let Some(BodyLayout {
        chunks,
        section_open,
        height,
    }) = body_layout()
    else {
        // The panel has not been measured, so there is nothing to lay the
        // document out for. Only the model goes: the document itself stays, for
        // the width that is about to arrive.
        clear_detail_body_model(ui);
        return;
    };
    push_detail_body_rows(ui, chunks, section_open, height);
}

/// The mechanical half of [`push_detail_body`], once there is something to push.
pub(crate) fn push_detail_body_rows(
    ui: &App,
    runs: Vec<BodyRun>,
    section_open: Vec<bool>,
    height: f32,
) {
    let state = ui.global::<ContentState>();
    // The view needs the same monospace family the engine measured with, and this
    // is the first time the engine exists — so this is the first time the answer
    // is known. Pushing it on every layout would be a no-op after the first.
    state.set_detail_body_mono_family(SharedString::from(body_mono_family()));
    BODY.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(body) = slot.as_mut() else { return };
        // The first layout is also the first time the panel is shown the models,
        // and from then on they are written in place: replacing one destroys and
        // re-creates every item element, and a body is re-laid out on every pixel
        // of a window drag.
        if !body.bound {
            body.bound = true;
            state.set_detail_body_chunks(ModelRc::from(body.chunks.clone()));
            state.set_detail_body_section_open(ModelRc::from(body.section_open.clone()));
        }
        // A run's items and bitmaps are models of their own, nested inside the run
        // that holds them, so they are created once and rewritten after that.
        if body.run_items.len() != runs.len() {
            body.run_items = runs
                .iter()
                .map(|run| Rc::new(VecModel::from(run.items.clone())))
                .collect();
            body.run_images = runs
                .iter()
                .map(|run| Rc::new(VecModel::from(run.images.clone())))
                .collect();
            let rows: Vec<MdChunk> = runs
                .iter()
                .enumerate()
                .map(|(index, run)| MdChunk {
                    items: ModelRc::from(body.run_items[index].clone()),
                    images: ModelRc::from(body.run_images[index].clone()),
                    ..run.chunk.clone()
                })
                .collect();
            body.chunks.set_vec(rows);
        } else {
            for (index, run) in runs.into_iter().enumerate() {
                if body.run_items[index].row_count() != run.items.len() {
                    body.run_items[index].set_vec(run.items);
                } else {
                    for (row, item) in run.items.into_iter().enumerate() {
                        body.run_items[index].set_row_data(row, item);
                    }
                }
                if body.run_images[index].row_count() != run.images.len() {
                    body.run_images[index].set_vec(run.images);
                } else {
                    for (row, image) in run.images.into_iter().enumerate() {
                        body.run_images[index].set_row_data(row, image);
                    }
                }
                let current = body.chunks.row_data(index).unwrap_or_default();
                let items = ModelRc::from(body.run_items[index].clone());
                let images = ModelRc::from(body.run_images[index].clone());
                if current.items != items || current.images != images || current != run.chunk {
                    body.chunks.set_row_data(
                        index,
                        MdChunk {
                            items,
                            images,
                            ..run.chunk
                        },
                    );
                }
            }
        }
        if body.section_open.row_count() != section_open.len() {
            body.section_open.set_vec(section_open);
        } else {
            for (index, open) in section_open.into_iter().enumerate() {
                body.section_open.set_row_data(index, open);
            }
        }
    });
    state.set_detail_body_height(height);
}

/// Empties the two models and nothing else.
///
/// The document stays, and that is the whole point of it being a function of its
/// own: an unmeasured document has nothing to draw *yet*, and the width that lets
/// it be drawn is a moment away. Anything that empties the engine has to say so
/// itself, in [`forget_body_document`], so the two cannot be mistaken.
pub(crate) fn clear_body_models() {
    BODY.with(|slot| {
        if let Some(body) = slot.borrow_mut().as_mut() {
            body.chunks.set_vec(Vec::new());
            body.run_items.clear();
            body.run_images.clear();
            body.section_open.set_vec(Vec::new());
        }
    });
}

/// Empties what the panel draws, leaving the engine's document alone.
pub(crate) fn clear_detail_body_model(ui: &App) {
    let state = ui.global::<ContentState>();
    state.set_detail_body_hovered(SharedString::default());
    state.set_detail_body_height(0.0);
    clear_body_models();
}

/// Empties the body *and* the engine's document, for a panel that is about to
/// load another one. The display list belongs to the document that was in the
/// box, so it goes with it.
///
/// The *width* stays. It belongs to the view, not to the document, and the view
/// does not go away between panels — so clearing it would mean waiting for a
/// `changed width` that never comes, because a panel reopened at the same width
/// reports the same width, and the new body would never be laid out.
pub(crate) fn clear_detail_body(ui: &App) {
    clear_detail_body_model(ui);
    forget_body_document();
}

/// Throws the engine's document away, for a panel that is about to show another
/// one.
///
/// The *width* stays. It belongs to the view, and the view does not go away
/// between panels — so clearing it would mean waiting for a `changed width` that
/// never comes, because a panel reopened at the width it already had reports the
/// same width, and the new body would never be laid out.
pub(crate) fn forget_body_document() {
    BODY.with(|slot| {
        if let Some(body) = slot.borrow_mut().as_mut() {
            body.renderer.set_source(
                String::new(),
                crate::ui::components::markdown::SourceFormat::Markdown,
            );
        }
    });
}

/// Fetches whatever the body is still missing, then lays it out again.
///
/// An image changes a document's height, because the engine reserves a box from
/// the bitmap's own size, so a body whose images arrive late is re-laid out once
/// per batch. The fetches are the same `fetch_icon` path the project icons take,
/// including its cache, so a logo and a README that show the same image decode it
/// once.
pub(crate) fn fetch_body_images(ui: &App) {
    let wanted = BODY.with(|slot| {
        let slot = slot.borrow();
        let Some(body) = slot.as_ref() else {
            return Vec::new();
        };
        body.renderer
            .referenced_images()
            .into_iter()
            .filter(|url| !body.renderer.images().contains(url))
            .collect::<Vec<_>>()
    });
    if wanted.is_empty() {
        return;
    }
    let weak = ui.as_weak();
    crate::support::runtime::spawn_blocking(move || {
        let fetched = wanted
            .iter()
            .filter_map(|url| fetch_icon(url).map(|image| (url.clone(), image)))
            .collect::<Vec<_>>();
        if fetched.is_empty() {
            return;
        }
        crate::ui::services::report::report(&weak, move |ui| {
            let mut changed = false;
            for (url, image) in &fetched {
                changed |= BODY.with(|slot| {
                    let mut slot = slot.borrow_mut();
                    match slot.as_mut() {
                        // The engine is not `Send` and the bitmap is not either, so
                        // the `Image` is built on this side of the hop, from the
                        // buffer that did cross.
                        Some(body) => body.renderer.set_image(
                            url,
                            Image::from_rgba8(SharedPixelBuffer::clone_from_slice(
                                &image.rgba,
                                image.width,
                                image.height,
                            )),
                            image.width,
                            image.height,
                        ),
                        None => false,
                    }
                });
            }
            if changed {
                push_detail_body(&ui);
            }
        });
    });
}

/// `owner/repo`, when the URL is a GitHub one. Only the real host is tested,
/// never a `"://github.com"` substring, which can never match a parsed host.
pub(crate) fn github_repo(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url.trim()).ok()?;
    if parsed.host_str() != Some("github.com") {
        return None;
    }
    let segments: Vec<&str> = parsed
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.len() < 2 {
        return None;
    }
    let repo = segments[1].strip_suffix(".git").unwrap_or(segments[1]);
    Some(format!("{}/{}", segments[0], repo))
}

pub(crate) fn number_label(value: Option<i64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

/// Checks whether the open project is installed and writes the result.
pub(crate) fn refresh_installed(ui: &App) {
    let Some(detail) = controller().borrow().detail.clone() else {
        return;
    };
    if detail.kind != RemoteKind::Mods {
        return;
    }
    ui.global::<ContentState>()
        .set_detail_checking_installed(true);
    let weak = ui.as_weak();
    let instance = controller().borrow().instance_id.clone();
    crate::support::runtime::spawn(async move {
        let info =
            content::mods::remote::check_installed(&instance, detail.platform.api(), &detail.id)
                .await;
        crate::ui::services::report::report(&weak, move |ui| {
            {
                let state = controller();
                let mut state = state.borrow_mut();
                if let Some(open) = state.detail.as_mut()
                    && open.id == detail.id
                {
                    open.installed_mods = info.mods.clone();
                }
            }
            let ui_state = ui.global::<ContentState>();
            ui_state.set_detail_checking_installed(false);
            ui_state.set_detail_installed(info.installed);
            ui_state.set_detail_installed_version(SharedString::from(
                info.mods
                    .first()
                    .and_then(|mod_info| mod_info.version.clone())
                    .unwrap_or_default(),
            ));
        });
    });
}

pub(crate) fn refresh_favorited(ui: &App) {
    let state = controller();
    let state = state.borrow();
    if let Some(detail) = &state.detail {
        let favorited = state.is_favorited(detail.platform, &detail.id);
        ui.global::<ContentState>().set_detail_favorited(favorited);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The body state, with nothing in it. Every test here drives the same
    /// `thread_local` the panel does, so they are ordered against nothing and
    /// have to clean up after themselves.
    fn reset_body() {
        BODY.with(|slot| *slot.borrow_mut() = None);
    }

    /// [`set_body_source`], with an engine that needs no window backend.
    fn set_source(body: &str) -> bool {
        set_body_source_with(body, false, BodyRenderer::for_test)
    }

    /// An engine that already knows its width, which is the state a panel is in
    /// once its view has reported one.
    fn measured_body() {
        BODY.with(|slot| {
            let mut slot = slot.borrow_mut();
            let engine = slot.get_or_insert_with(BodyRenderer::for_test);
            engine.width = 600.0;
            engine.renderer.set_width(600.0);
        });
    }

    #[test]
    fn a_body_set_before_the_panel_is_measured_survives_until_the_width_arrives() {
        // The bug this is here for. Sending a document with no width through the
        // "empty the body" path would empty the engine too -- so the width would
        // arrive a moment later, by which point the engine would hold an empty
        // string and lay *that* out. The panel would draw nothing, for a body
        // that had arrived perfectly intact.
        reset_body();

        assert!(!set_source("# A heading\n\nA paragraph.\n"));
        assert!(body_has_document(), "the document arrived");

        // No width, so nothing to draw. The model goes; the document does not.
        clear_body_models();
        assert!(
            body_has_document(),
            "emptying what is drawn must not empty the engine"
        );
        assert!(
            body_layout().is_none(),
            "an unmeasured document has no layout, and that is not a failure"
        );

        // Then the view reports its width, and the document is still there.
        measured_body();
        let layout = body_layout().expect("a layout once the width is known");
        assert!(!layout.chunks.is_empty(), "the document was lost");
        assert!(layout.height > 0.0, "a document with text has a height");
        reset_body();
    }

    #[test]
    fn a_body_keeps_its_width_across_panels() {
        // The width belongs to the view, not to the document. A panel reopened at
        // the width it already had reports the same width, so `changed width`
        // never fires -- and a body that waited for one would never be laid out.
        reset_body();
        measured_body();
        assert!(set_source("first"));
        assert!(body_layout().is_some(), "the first body lays out");

        // A second panel: the document is cleared, the width is not.
        forget_body_document();
        assert!(!body_has_document());
        BODY.with(|slot| {
            let body = slot.borrow();
            let body = body.as_ref().expect("the engine");
            assert_eq!(body.width, 600.0, "the width went with the document");
        });

        // And the new body is laid out at once, without waiting for a width that
        // is not coming.
        assert!(set_source("second"), "the width survived");
        let layout = body_layout().expect("the second body lays out at once");
        assert!(!layout.chunks.is_empty(), "the second document was lost");
        reset_body();
    }

    #[test]
    fn an_empty_document_is_a_layout_of_nothing_rather_than_no_layout() {
        // The distinction that made the first test's bug invisible: both answer
        // "there is nothing to draw", and only one of them is a reason to keep
        // the document.
        reset_body();
        measured_body();
        forget_body_document();
        let layout = body_layout().expect("a width is a width, empty or not");
        assert!(layout.chunks.is_empty());
        assert_eq!(layout.height, 0.0);
        reset_body();
    }
}
