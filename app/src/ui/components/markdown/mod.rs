// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A Markdown (or HTML) renderer implemented as a layout engine: it parses a
//! document, measures text and hands the view a flat list of positioned items.
//!
//! Text must be measured on the thread that owns Slint's font collection; parsing
//! can happen on a worker.

#![forbid(unsafe_code)]
// The engine exposes a fuller API than any one caller uses, so unused accessors
// and re-exports here are expected rather than a sign of rot.
#![allow(dead_code, unused_imports)]

pub mod doc;
pub mod layout;
pub mod measure;
pub mod model;
pub mod parse;

#[cfg(test)]
mod tests;

pub use doc::{Align, Block, Cell, Inline, Inlines, ListItem};
pub use layout::layout;
pub use measure::{Cluster, Measurer, SpanMetrics, SpanStyle};
pub use model::{
    ColorRole, DisplayList, HeadingRule, ImageAsset, ImageStore, ItemKind, MdItem, MdStyle,
    SourceFormat,
};

/// A collection of the system's own fonts, for a caller with no Slint collection.
pub mod fonts {
    /// Wrong for a caller that embeds its own family.
    pub fn system() -> fontique::Collection {
        fontique::Collection::new(fontique::CollectionOptions::default())
    }
}

/// A document and the display list it lays out to. Any change re-lays the whole
/// document out; the shaping cache makes the second pass cheap.
pub struct Renderer {
    source: String,
    format: SourceFormat,
    blocks: Vec<Block>,
    style: MdStyle,
    width: f32,
    images: ImageStore,
    measurer: Measurer,
    display: DisplayList,
    dirty: bool,
}

impl Renderer {
    pub fn new() -> Self {
        Self::with_collection(fonts::system())
    }

    /// For a Slint application, pass
    /// `slint::fontique_011::shared_collection()`, or every advance and line
    /// break will be slightly wrong. The crate does not reach for it itself
    /// because it is behind an unstable Slint feature.
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

    /// Nothing is measured until [`Self::layout`].
    pub fn set_source(&mut self, source: impl Into<String>, format: SourceFormat) {
        self.source = source.into();
        self.format = format;
        self.blocks = match format {
            SourceFormat::Markdown => parse::parse(&self.source),
            SourceFormat::Html => parse::html::parse(&self.source),
        };
        self.dirty = true;
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Every image URL the document references, in document order, without
    /// repeats.
    pub fn referenced_images(&self) -> Vec<String> {
        self.images.referenced(&self.blocks)
    }

    /// Every style field is a measurement input, so a change marks a re-layout.
    pub fn set_style(&mut self, style: MdStyle) {
        if style != self.style {
            self.style = style;
            self.dirty = true;
        }
    }

    pub fn style(&self) -> &MdStyle {
        &self.style
    }

    /// Half-pixel changes are ignored: a window drag produces one every frame.
    /// Returns whether a re-layout is needed.
    pub fn set_width(&mut self, width: f32) -> bool {
        if (width - self.width).abs() > 0.5 {
            self.width = width;
            self.dirty = true;
        }
        self.dirty
    }

    /// Reports whether this was a new image, which needs a re-layout.
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

    pub fn images(&self) -> &ImageStore {
        &self.images
    }

    /// Pass `window.scale_factor()` to agree with Slint's hinting to the last
    /// sub-pixel. Returns whether it changed.
    pub fn set_scale(&mut self, scale: f32) -> bool {
        let before = self.measurer.scale();
        self.measurer.set_scale(scale);
        before != self.measurer.scale()
    }

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

    pub fn display_list(&self) -> &DisplayList {
        &self.display
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
