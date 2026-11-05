// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The display-list types and the knobs the layout engine is driven with.

use std::collections::HashMap;

use slint::Image;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceFormat {
    #[default]
    Markdown,
    Html,
}

/// A palette slot rather than a colour, so a theme switch only repaints instead
/// of rebuilding the display list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorRole {
    #[default]
    Text,
    Heading,
    Link,
    CodeText,
    CodeBackground,
    CodeBlockText,
    CodeBlockBackground,
    QuoteText,
    QuoteBar,
    TableBorder,
    TableHeadText,
    TableHeadBackground,
    Rule,
    Strike,
    Marker,
    /// A failed inline image drawn as alt text; a block image uses the body
    /// colour instead.
    Placeholder,
}

impl ColorRole {
    /// Kept in sync by hand with `MarkdownColors.resolve` in `markdown-view.slint`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Heading => "heading",
            Self::Link => "link",
            Self::CodeText => "code-text",
            Self::CodeBackground => "code-background",
            Self::CodeBlockText => "code-block-text",
            Self::CodeBlockBackground => "code-block-background",
            Self::QuoteText => "quote-text",
            Self::QuoteBar => "quote-bar",
            Self::TableBorder => "table-border",
            Self::TableHeadText => "table-head-text",
            Self::TableHeadBackground => "table-head-background",
            Self::Rule => "rule",
            Self::Strike => "strike",
            Self::Marker => "marker",
            Self::Placeholder => "placeholder",
        }
    }
}

/// What an [`MdItem`] draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemKind {
    /// One run of glyphs in one style.
    #[default]
    Text,
    Rect,
    /// A decoded bitmap, clipped to `radius`.
    Image,
}

impl ItemKind {
    /// Kept in sync by hand with the companion component's `kind` strings.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Rect => "rect",
            Self::Image => "image",
        }
    }
}

/// One drawable in the display list, in logical pixels from the list's
/// top-left, in paint order.
#[derive(Clone)]
pub struct MdItem {
    pub kind: ItemKind,
    pub x: f32,
    /// A `Text` item's box is the run's ascent and descent, not the line box,
    /// so runs of different sizes share a baseline.
    pub y: f32,
    pub width: f32,
    pub height: f32,

    /// The glyphs, or an image's alt text.
    pub text: String,
    pub font_size: f32,
    pub font_weight: u16,
    pub font_italic: bool,
    pub mono: bool,
    pub color: ColorRole,

    pub radius: f32,
    pub filled: bool,
    pub border_width: f32,
    pub border_color: ColorRole,

    /// Set on a `Rect` that acts as a link and takes the press.
    pub link_url: String,
    /// Groups a link's per-line boxes so a view can test them at once.
    pub link_id: String,
    /// Where the revealed rule sits, down from the box top.
    pub rule_offset: f32,
    /// False for a linked image, which a browser does not underline.
    pub rule: bool,

    pub image: Option<Image>,
    /// Slint's `Image` cannot report its size, so the caller supplies it.
    pub image_size: Option<(u32, u32)>,
}

impl Default for MdItem {
    fn default() -> Self {
        Self {
            kind: ItemKind::Text,
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            text: String::new(),
            font_size: 14.0,
            font_weight: 400,
            font_italic: false,
            mono: false,
            color: ColorRole::Text,
            radius: 0.0,
            filled: true,
            border_width: 0.0,
            border_color: ColorRole::Text,
            link_url: String::new(),
            link_id: String::new(),
            rule_offset: 0.0,
            rule: false,
            image: None,
            image_size: None,
        }
    }
}

/// Hand-written because [`slint::Image`] has no `Debug`, and a display list that
/// cannot be printed cannot be debugged.
impl std::fmt::Debug for MdItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MdItem")
            .field("kind", &self.kind)
            .field("x", &self.x)
            .field("y", &self.y)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("text", &self.text)
            .field("font-size", &self.font_size)
            .field("font-weight", &self.font_weight)
            .field("italic", &self.font_italic)
            .field("mono", &self.mono)
            .field("color", &self.color)
            .field("radius", &self.radius)
            .field("filled", &self.filled)
            .field("border-width", &self.border_width)
            .field("border-color", &self.border_color)
            .field("link-id", &self.link_id)
            .field("link-url", &self.link_url)
            .field("rule-offset", &self.rule_offset)
            .field("rule", &self.rule)
            .field("image", &self.image_size)
            .finish()
    }
}

impl MdItem {
    pub fn text(x: f32, y: f32, width: f32, height: f32, text: impl Into<String>) -> Self {
        Self {
            kind: ItemKind::Text,
            x,
            y,
            width,
            height,
            text: text.into(),
            ..Default::default()
        }
    }

    pub fn rect(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            kind: ItemKind::Rect,
            x,
            y,
            width,
            height,
            ..Default::default()
        }
    }
}

/// A decoded bitmap, keyed by the URL it was fetched from.
#[derive(Clone)]
pub struct ImageAsset {
    pub image: Image,
    /// The bitmap's own size, which Slint cannot read back.
    pub size: (u32, u32),
}

impl std::fmt::Debug for ImageAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageAsset")
            .field("size", &self.size)
            .finish()
    }
}

/// The bitmaps a document references, handed in by the caller as they arrive.
/// Until one arrives, its alt text is drawn in its place.
#[derive(Default, Clone)]
pub struct ImageStore {
    assets: HashMap<String, ImageAsset>,
}

impl ImageStore {
    pub fn insert(&mut self, url: impl Into<String>, asset: ImageAsset) {
        self.assets.insert(url.into(), asset);
    }

    pub fn get(&self, url: &str) -> Option<&ImageAsset> {
        self.assets.get(url)
    }

    pub fn contains(&self, url: &str) -> bool {
        self.assets.contains_key(url)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// The URLs a document references, in document order and without repeats.
    pub fn referenced(&self, blocks: &[crate::ui::components::markdown::Block]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for block in blocks {
            block.walk_images(&mut |url| {
                if !out.iter().any(|seen| seen == url) {
                    out.push(url.to_string());
                }
            });
        }
        out
    }
}

/// Every measurement, margin and radius the renderer uses. The defaults are
/// GitHub's `.markdown-body` at a 14px body, with the `<details>` row sized like
/// a launcher settings row.
#[derive(Debug, Clone, PartialEq)]
pub struct MdStyle {
    pub font_family: String,
    pub mono_family: String,
    pub font_size: f32,
    pub line_height: f32,
    pub weight_normal: u16,
    pub weight_bold: u16,
    pub heading_weight: u16,

    pub heading_sizes: [f32; 6],
    pub heading_line_height: f32,

    pub paragraph_margin_bottom: f32,
    /// Not read: a heading's separation comes from the preceding block's margin.
    pub heading_margin_top: f32,
    pub heading_margin_bottom: f32,
    pub heading_rule: [Option<HeadingRule>; 6],
    pub list_margin_bottom: f32,
    pub list_indent: f32,
    pub list_item_spacing: f32,
    pub list_marker_gap: f32,
    pub quote_margin_bottom: f32,
    pub quote_padding_x: f32,
    pub quote_bar_width: f32,
    pub code_block_margin_bottom: f32,
    pub code_block_padding: f32,
    pub code_block_radius: f32,
    pub rule_margin: f32,
    pub rule_thickness: f32,
    pub table_margin_bottom: f32,
    pub cell_padding_x: f32,
    pub cell_padding_y: f32,
    pub cell_border: f32,
    pub task_marker_gap: f32,
    pub task_marker_size: f32,
    /// Not read: the separator uses `footnote_rule_margin`.
    pub footnote_margin_top: f32,
    pub footnote_rule_margin: f32,

    /// A `<details>` is a control, so it is sized like a launcher settings row
    /// rather than GitHub's bare summary.
    pub details_head_font_size: f32,
    pub details_head_padding_x: f32,
    pub details_head_padding_y: f32,
    pub details_head_radius: f32,
    pub details_marker_size: f32,
    pub details_marker_gap: f32,
    pub details_content_inset: f32,
    pub details_gap: f32,
    pub details_margin_bottom: f32,

    pub code_font_scale: f32,
    pub code_line_height: f32,
    pub code_padding_x: f32,
    /// The gap outside the capsule, which has to be room in the line or the
    /// capsule is drawn over the glyphs next to it.
    pub code_margin_x: f32,
    pub code_padding_y: f32,
    pub code_radius: f32,
    pub image_radius: f32,
    /// A block image is its own block, so without this the next line starts
    /// against the picture.
    pub image_margin_bottom: f32,
    /// The thickness of a revealed link rule and of a strikethrough.
    pub decoration_width: f32,

    /// Not consulted: wrapping always allows the break.
    pub break_long_words: bool,
    /// Wrap a code block instead of clipping it, for a view that cannot scroll.
    pub code_block_wrap: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadingRule {
    /// `padding-bottom`, relative to the heading's own size.
    pub padding: f32,
    pub width: f32,
}

impl MdStyle {
    /// A caller that embeds a typeface must set this, or the document is measured
    /// with one family and drawn with another, clipping every line.
    pub fn with_families(
        mut self,
        family: impl Into<String>,
        mono_family: impl Into<String>,
    ) -> Self {
        self.font_family = family.into();
        self.mono_family = mono_family.into();
        self
    }
}

impl Default for MdStyle {
    /// GitHub's `.markdown-body` at a 14px body, with the `<details>` row sized
    /// like a launcher settings row.
    fn default() -> Self {
        Self {
            font_family: String::new(),
            mono_family: "monospace".into(),
            font_size: 14.0,
            line_height: 1.6,
            weight_normal: 400,
            weight_bold: 700,
            heading_weight: 600,
            heading_sizes: [2.0, 1.5, 1.25, 1.0, 0.83, 0.67],
            heading_line_height: 1.25,

            paragraph_margin_bottom: 16.0,
            heading_margin_top: 24.0,
            heading_margin_bottom: 16.0,
            heading_rule: [
                Some(HeadingRule {
                    width: 1.0,
                    padding: 0.3,
                }),
                Some(HeadingRule {
                    width: 1.0,
                    padding: 0.3,
                }),
                None,
                None,
                None,
                None,
            ],

            list_margin_bottom: 16.0,
            list_indent: 28.0,
            list_item_spacing: 3.5,
            list_marker_gap: 7.0,
            quote_margin_bottom: 16.0,
            quote_padding_x: 14.0,
            quote_bar_width: 3.5,
            code_block_margin_bottom: 16.0,
            code_block_padding: 16.0,
            code_block_radius: 8.0,
            rule_margin: 24.0,
            rule_thickness: 3.5,
            table_margin_bottom: 16.0,
            cell_padding_x: 13.0,
            cell_padding_y: 6.0,
            cell_border: 1.0,
            task_marker_gap: 7.0,
            task_marker_size: 13.0,
            footnote_margin_top: 16.0,
            footnote_rule_margin: 16.0,

            details_head_font_size: 13.0,
            details_head_padding_x: 14.0,
            details_head_padding_y: 10.0,
            details_head_radius: 8.0,
            details_marker_size: 17.0,
            details_marker_gap: 8.0,
            details_content_inset: 14.0,
            details_gap: 8.0,
            details_margin_bottom: 16.0,

            code_font_scale: 0.85,
            code_line_height: 1.45,
            code_padding_x: 0.4,
            code_margin_x: 0.25,
            code_padding_y: 0.2,
            code_radius: 6.0,
            image_radius: 8.0,
            image_margin_bottom: 16.0,
            decoration_width: 1.0,

            break_long_words: true,
            code_block_wrap: false,
        }
    }
}

#[derive(Default, Clone)]
pub struct DisplayList {
    pub items: Vec<MdItem>,
    /// The height a scroll container has to offer.
    pub height: f32,
    pub width: f32,
    /// The widest line, for a shrink-to-fit container.
    pub content_width: f32,
    /// The document in runs a view stacks vertically. Hiding a section means
    /// taking its height out of the flow, which is why the engine hands over runs
    /// rather than letting a view cover the content.
    pub chunks: Vec<MdChunk>,
}

/// A run of the document that moves as one thing.
#[derive(Debug, Clone, PartialEq)]
pub struct MdChunk {
    pub section: Option<MdSection>,
    /// Where the run starts in the open layout, the origin its items use.
    pub top: f32,
    /// The run's height with its section open.
    pub height: f32,
    /// A contiguous range into [`DisplayList::items`]; a copy would have to be
    /// kept correct.
    pub items: std::ops::Range<usize>,
}

/// A `<details>`, laid out **open** either way so a view can animate the reveal
/// without re-measuring.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSection {
    /// A view holds its open/closed state against this across re-layouts.
    pub id: u32,
    pub head_x: f32,
    pub head_y: f32,
    pub head_width: f32,
    pub head_height: f32,
    pub head_radius: f32,
    /// The chevron's left edge; the engine has no icon font, so a view draws it.
    pub marker_x: f32,
    pub marker_y: f32,
    pub marker_size: f32,
    pub content_x: f32,
    pub content_y: f32,
    pub content_height: f32,
    /// The flow taken away while closed: the content **and** the margin under
    /// it, so a closed run is only its row and the gap.
    pub hidden: f32,
    /// Where the document's flow carries on after the section.
    pub flow_bottom: f32,
    /// Seeds a view's own state from `<details open>`.
    pub open: bool,
}
