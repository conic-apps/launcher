// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A Markdown (or HTML) renderer for Slint, as a layout engine.
//!
//! # What this is
//!
//! Slint can draw text, but it cannot lay a document out: it has no rich-text
//! element that understands headings, lists, tables or images, and the two
//! elements it does have are each missing half of what GitHub's renderer does.
//! `Text` has `line-height-factor` but no strikethrough, no underline and no
//! text background; `StyledText` has all three but no `line-height-factor`, it
//! underlines every link whether the pointer is over it or not, and it has no
//! way to say where a span landed.
//!
//! So this crate does the part a browser's layout engine does, in Rust, and
//! hands the view a flat list of positioned items. The view's whole job is
//! `for item in items` and three cases: a run of text, a box, a bitmap.
//!
//! ```no_run
//! use slint_markdown::{Renderer, SourceFormat};
//!
//! let mut renderer = Renderer::new();
//! renderer.set_source("# Conic Launcher\n\nA **mod** loader.\n", SourceFormat::Markdown);
//! renderer.set_width(640.0);
//! let list = renderer.layout();
//! println!("{} items, {}px tall", list.items.len(), list.height);
//! ```
//!
//! # What it covers
//!
//! Everything inside a GitHub `.markdown-body`: headings, paragraphs, ordered
//! and unordered lists, task lists, code blocks, block quotes, thematic breaks,
//! tables with per-column alignment, images, footnotes and description lists,
//! plus the inline set — bold, italic, strikethrough, inline code, links,
//! superscript, hard breaks and reference-style autolinks. CurseForge's HTML
//! descriptions go through the same tree, so a body in either language renders
//! the same way.
//!
//! # The view
//!
//! `ui/markdown-view.slint` is the companion component. A caller adds its
//! directory to slint-build's include path, imports `MarkdownView`, and pushes
//! the display list into it. The engine itself has no Slint item tree in it, so
//! the same crate serves a caller that draws with `Text`/`Image` directly.
//!
//! # Two things to know
//!
//! - **Measurement needs the main thread.** Text is shaped against
//!   [`slint::fontique_011::shared_collection`], which is Slint's font
//!   collection, and that is reached through the thread that owns the Slint
//!   context. Parsing a document does not, and a caller with a big body should
//!   parse on a worker and lay out on the main one.
//! - **A wide table wraps rather than scrolls.** The launcher lets one scroll
//!   sideways; a `Text` cannot, so a table wider than its box shares the
//!   shortfall between its columns and wraps inside them instead. Set
//!   [`MdStyle::code_block_wrap`] for the same trade on a `pre`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod doc;
pub mod layout;
pub mod measure;
pub mod model;
pub mod parse;

pub use doc::{Align, Block, Cell, Inline, Inlines, ListItem};
pub use layout::layout;
pub use measure::{Cluster, Measurer, SpanMetrics, SpanStyle};
pub use model::{
    ColorRole, DisplayList, HeadingRule, ImageAsset, ImageStore, ItemKind, MdItem, MdStyle,
    SourceFormat,
};

/// A collection of the system's own fonts, for a caller that has no Slint
/// collection to hand over.
pub mod fonts {
    /// Every font the platform can see. Correct for a caller that renders its
    /// text with the platform's fonts, and wrong for one that embeds a family.
    pub fn system() -> fontique::Collection {
        fontique::Collection::new(fontique::CollectionOptions::default())
    }
}

/// The directory the companion `.slint` files live in.
///
/// A caller that wants the component adds this to slint-build's include path:
///
/// ```no_run
/// // In the caller's build.rs, with `slint-build` as a build dependency:
/// //   slint_build::CompilerConfiguration::new()
/// //       .with_include_paths(vec![slint_markdown::ui_path()])
/// ```
///
/// The crate ships them as data rather than compiling them itself: a Slint
/// struct's Rust type belongs to the *compilation* that produced it, so a
/// library that compiled the component and a caller that imported the same file
/// would end up with two unrelated types and no way to move an item between
/// them. Compiling once, in the caller, is what keeps the two the same type.
pub fn ui_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui")
}

/// A document, and the display list it lays out to.
///
/// The lifecycle is one-shot, which is what a description is: set the source,
/// set the width, ask for the layout, hand the items to a view. Nothing here
/// streams — a change to any input re-lays the whole document out, and the
/// engine keeps its shaping cache, so the second pass over an unchanged document
/// is close to free.
pub struct Renderer {
    source: String,
    format: SourceFormat,
    blocks: Vec<Block>,
    style: MdStyle,
    width: f32,
    images: ImageStore,
    measurer: Measurer,
    /// The list as it was last laid out, so `display_list()` does not have to
    /// re-run the walk.
    display: DisplayList,
    dirty: bool,
}

impl Renderer {
    /// Creates a renderer with GitHub's `.markdown-body` defaults, no document,
    /// and the system's own fonts.
    pub fn new() -> Self {
        Self::with_collection(fonts::system())
    }

    /// Creates a renderer over the caller's font collection, which for a Slint
    /// application is the one the renderer draws with:
    ///
    /// ```ignore
    /// let renderer = slint_markdown::Renderer::with_collection(
    ///     slint::fontique_011::shared_collection(),
    /// );
    /// ```
    ///
    /// Measuring against anything else leaves every advance, and so every line
    /// break, slightly wrong: a family the renderer does not have falls back, and
    /// the fallback is not the same glyphs. This crate does not reach for the
    /// collection itself — it is behind an unstable Slint feature, and a library
    /// has no business enabling a renderer to be handed a font database.
    pub fn with_collection(collection: fontique::Collection) -> Self {
        Self {
            source: String::new(),
            format: SourceFormat::Markdown,
            blocks: Vec::new(),
            style: MdStyle::default(),
            width: 0.0,
            images: ImageStore::default(),
            measurer: Measurer::with_collection(collection),
            display: DisplayList::default(),
            dirty: true,
        }
    }

    /// Replaces the document. Nothing is measured yet; the layout happens in
    /// [`Self::layout`].
    pub fn set_source(&mut self, source: impl Into<String>, format: SourceFormat) {
        self.source = source.into();
        self.format = format;
        self.blocks = match format {
            SourceFormat::Markdown => parse::parse(&self.source),
            SourceFormat::Html => parse::html::parse(&self.source),
        };
        self.dirty = true;
    }

    /// The block tree the source produced, for a caller that wants to walk it
    /// itself (to collect image URLs, say — see [`Self::referenced_images`]).
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Every image URL the document references, in document order, without
    /// repeats.
    pub fn referenced_images(&self) -> Vec<String> {
        self.images.referenced(&self.blocks)
    }

    /// Replaces the style. Every field is a measurement input, so this marks the
    /// document for a re-layout.
    pub fn set_style(&mut self, style: MdStyle) {
        if style != self.style {
            self.style = style;
            self.dirty = true;
        }
    }

    /// The style in effect.
    pub fn style(&self) -> &MdStyle {
        &self.style
    }

    /// Sets the width the document is laid out for. Sub-pixel changes are
    /// ignored, because they cannot change a line break by enough to matter and
    /// a window drag produces one on every frame.
    ///
    /// Returns whether the document needs laying out again.
    pub fn set_width(&mut self, width: f32) -> bool {
        if (width - self.width).abs() > 0.5 {
            self.width = width;
            self.dirty = true;
        }
        self.dirty
    }

    /// Hands over a decoded bitmap for one of the document's URLs, and reports
    /// whether it changed anything — a new image means the document has to be
    /// laid out again, one that was already known does not.
    pub fn set_image(&mut self, url: &str, image: slint::Image, width: u32, height: u32) -> bool {
        if self.images.contains(url) {
            return false;
        }
        self.images.insert(
            url,
            ImageAsset {
                image,
                size: (width, height),
            },
        );
        self.dirty = true;
        true
    }

    /// The images handed over so far.
    pub fn images(&self) -> &ImageStore {
        &self.images
    }

    /// Sets the device scale factor the measurement is taken at, and returns
    /// whether it changed. A caller that wants its advances to agree with the
    /// renderer's to the last sub-pixel passes `window.scale_factor()` here.
    pub fn set_scale(&mut self, scale: f32) -> bool {
        let before = self.measurer.scale();
        self.measurer.set_scale(scale);
        before != self.measurer.scale()
    }

    /// Lays the document out if anything changed, and returns the list.
    pub fn layout(&mut self) -> &DisplayList {
        if self.dirty {
            self.display = layout::layout(
                &self.blocks,
                &self.style,
                self.width,
                &self.images,
                &mut self.measurer,
            );
            self.dirty = false;
        }
        &self.display
    }

    /// The list as last laid out, without re-running anything. The width to lay
    /// out for is whatever was last set with [`Self::set_width`].
    pub fn display_list(&self) -> &DisplayList {
        &self.display
    }

    /// Whether anything changed since the last layout.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
