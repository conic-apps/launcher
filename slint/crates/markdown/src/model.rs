// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The types the renderer produces and the knobs it is driven with.
//!
//! The renderer is a *layout engine*, not a widget: it turns a Markdown (or
//! HTML) document into a flat list of absolutely positioned [`MdItem`]s. A view
//! then draws that list. Nothing here knows about Slint's item tree, and the
//! only Slint type that appears is [`slint::Image`], which is how a decoded
//! bitmap travels from the caller to the display list.

use std::collections::HashMap;

use slint::Image;

/// Which grammar a document is written in.
///
/// Modrinth sends Markdown and CurseForge sends HTML, and both are served to the
/// same view, so the format is part of the document rather than a global.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceFormat {
    /// CommonMark with the GitHub extensions (tables, task lists,
    /// strikethrough, autolinks, footnotes, …).
    #[default]
    Markdown,
    /// A tag soup, the way CurseForge serves its `markup: true` descriptions.
    Html,
}

/// A slot in the view's palette.
///
/// Colours travel as a role rather than as a colour so that a palette switch is
/// a repaint and nothing more: the display list never has to be rebuilt (which
/// would re-measure, re-break and re-position every line) just because the app
/// switched from a dark flavour to a light one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorRole {
    /// Body text, and the fallback for anything not listed below.
    #[default]
    Text,
    /// Heading text. GitHub's stylesheet does not give headings their own colour
    /// (they inherit the container's), but a view may want to.
    Heading,
    /// The text of a link. Deliberately *not* the same as `Text`: links are the
    /// one thing the Vue's stylesheet always recolours.
    Link,
    /// Inline `code` glyphs.
    CodeText,
    /// The capsule behind inline `code`.
    CodeBackground,
    /// A fenced/indented code block's glyphs.
    CodeBlockText,
    /// A fenced/indented code block's background.
    CodeBlockBackground,
    /// A block quote's text, which the Vue dims to `--ctp-overlay2`.
    QuoteText,
    /// A block quote's left rule.
    QuoteBar,
    /// Table cell borders.
    TableBorder,
    /// A table header cell's text.
    TableHeadText,
    /// A table header cell's background.
    TableHeadBackground,
    /// A `---` thematic break, and the rule under `h1`/`h2`.
    Rule,
    /// A strikethrough drawn over deleted text.
    Strike,
    /// A list bullet, an ordered marker, a task box's tick.
    Marker,
    /// An image that could not be fetched, drawn as its alt text.
    Placeholder,
}

impl ColorRole {
    /// The stable name a view switches on. Kept in sync with the `ColorRole`
    /// function of the `markdown-view.slint` companion by hand.
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

/// What an [`MdItem`] draws. Three kinds cover the whole stylesheet: text, a
/// rounded (optionally outlined) box, and a bitmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ItemKind {
    /// A single run of glyphs in one style. One run per line per style, which is
    /// what lets the view hand each one to a `Text` and lets the strike-through
    /// and underline rules land on exactly the run they belong to.
    #[default]
    Text,
    /// A box: a code capsule, a code block's background, a quote's rule, a
    /// table cell, a thematic break, a link's hit area or its hover underline.
    Rect,
    /// A decoded bitmap, clipped to `radius`.
    Image,
}

impl ItemKind {
    /// The stable name a view switches on. Kept in sync with the `kind` strings
    /// of the companion component by hand.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Rect => "rect",
            Self::Image => "image",
        }
    }
}

/// One drawable in the display list.
///
/// Coordinates are in logical pixels relative to the display list's own top-left
/// corner. The list is in paint order: a code block's background precedes the
/// glyphs drawn on it, and a link's transparent hit area follows the text it
/// covers.
#[derive(Clone)]
pub struct MdItem {
    /// Which of the three things this item draws.
    pub kind: ItemKind,
    /// Left edge, in logical pixels from the list's own origin.
    pub x: f32,
    /// Top edge. A `Text` item's box is the run's own ascent and descent, not
    /// the line box, so that runs of different sizes on one line share a
    /// baseline; a view centres the glyphs inside it.
    pub y: f32,
    /// The box's width. For a `Text` item this is the run's advance, which is
    /// what a link's rule and a strikethrough are drawn across.
    pub width: f32,
    /// The box's height.
    pub height: f32,

    /// The glyphs of a `Text` item, and empty for the other kinds. An `Image`
    /// item carries its alternative text here instead, for a view that wants a
    /// label or a tooltip.
    pub text: String,
    /// The `Text` item's font size. Slint centres the glyphs in `height`, which
    /// is the line's box rather than the run's own, so a run of a different size
    /// sitting next to a larger one still lands on the same baseline.
    pub font_size: f32,
    /// The `font-weight`, 100 to 900. A variable font interpolates it, which is
    /// how a heading gets 600 rather than the boldest weight the family has.
    pub font_weight: u16,
    /// Whether the run is slanted.
    pub font_italic: bool,
    /// Draw with the code font instead of the text font.
    /// Draw with the code font instead of the text font.
    pub mono: bool,
    /// Which slot of the view's palette this item takes its colour from.
    pub color: ColorRole,

    /// A `Rect` item's corner radius.
    pub radius: f32,
    /// Whether a `Rect` item is filled. A table cell that has no background of
    /// its own — every cell but a header's — is an outline and nothing else.
    pub filled: bool,
    /// A `Rect` item's outline width, drawn inside the box.
    pub border_width: f32,
    /// A `Rect` item's outline colour.
    pub border_color: ColorRole,

    /// Set on a `Rect` item that acts as a link: a transparent box that takes the
    /// press. A link is *one* item per line rather than two — the box and the
    /// rule it reveals — because they are the same rectangle, and a view that had
    /// to match them up by position would be doing arithmetic the engine has
    /// already done.
    pub link_url: String,
    /// Groups every piece of one link — its per-line boxes — so a view can
    /// resolve "is the pointer inside this link" from one string, and draw the
    /// rule under all of the link's lines at once.
    pub link_id: String,
    /// Where the rule a view reveals for this link sits, measured down from the
    /// top of the box. Zero for an item that is not a link.
    pub rule_offset: f32,
    /// Whether this link reveals a rule when the pointer is over it.
    ///
    /// Almost always true, and false for a link whose content is an image: an
    /// image is a replaced element, `text-decoration` does not reach one, and a
    /// browser underlines nothing under a linked picture. A view that drew the
    /// rule anyway would put a line across the bottom of every badge in a README,
    /// which is the one place a README is guaranteed to have a row of them.
    pub rule: bool,

    /// An `Image` item's bitmap.
    pub image: Option<Image>,
    /// The natural size of that bitmap, which the caller supplies because
    /// Slint's `Image` cannot report it.
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

/// The item's geometry and style, without the bitmap.
///
/// Hand-written because [`slint::Image`] has no `Debug`, and a layout engine
/// whose items cannot be printed is one whose bugs cannot be seen: `{:?}` of a
/// display list is the first thing anyone reaches for.
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
    /// A text run.
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

    /// A box.
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
    /// The decoded bitmap, ready for an `Image` element.
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

/// The images a document references, as handed to [`crate::Renderer::set_image`].
///
/// A README can reference dozens of images; fetching and decoding them is the
/// caller's job (it needs a network client and a background thread), so the
/// renderer takes them one at a time and re-layouts whenever one arrives. A
/// document that has not been given an image yet draws the image's alt text in
/// its place, at the width an image would have had if its size were known, or
/// across the full line when it is not — which is what a browser does for a
/// broken image.
#[derive(Default, Clone)]
pub struct ImageStore {
    assets: HashMap<String, ImageAsset>,
}

impl ImageStore {
    /// Adds or replaces the asset for a URL.
    pub fn insert(&mut self, url: impl Into<String>, asset: ImageAsset) {
        self.assets.insert(url.into(), asset);
    }

    /// The asset for a URL, if one has been handed over.
    pub fn get(&self, url: &str) -> Option<&ImageAsset> {
        self.assets.get(url)
    }

    /// Whether a URL has an asset already.
    pub fn contains(&self, url: &str) -> bool {
        self.assets.contains_key(url)
    }

    /// How many assets have been handed over.
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Whether nothing has been handed over yet.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// The URLs a document references, in document order and without repeats.
    ///
    /// The caller uses this to decide what to fetch, and the renderer uses it to
    /// reserve a box for an image that is on its way.
    pub fn referenced(&self, blocks: &[crate::Block]) -> Vec<String> {
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

/// Every measurement, margin and radius the renderer uses.
///
/// The defaults are GitHub's `.markdown-body` values scaled to a 14px body,
/// which is what the launcher renders its detail panel with; a caller overrides
/// whichever of them its own stylesheet disagrees about. Nothing is read from
/// the environment: the renderer never looks at a palette, a font file or a
/// window, it only produces geometry from what it is given.
#[derive(Debug, Clone, PartialEq)]
pub struct MdStyle {
    // ----- type -----
    /// The family body text is shaped with, before the generic fallbacks.
    pub font_family: String,
    /// The family inline `code` and code blocks are shaped with.
    pub mono_family: String,
    /// The body size. Every `em` below is relative to this.
    pub font_size: f32,
    /// The body line height, a multiple of the body size (CSS `line-height`).
    pub line_height: f32,
    /// The weight of body text, and of the `ui` element that shows this in a
    /// component's own documentation.
    pub weight_normal: u16,
    /// The weight of a task list's tick and of `**strong**`, which is a document's
    /// own choice and not a style.
    pub weight_bold: u16,
    /// `font-weight` of every heading level.
    pub heading_weight: u16,

    /// `h1`…`h6` sizes as multiples of `font_size`. The last two are the
    /// browser's user-agent defaults, which the launcher's stylesheet leaves
    /// alone because it only overrides `h1`–`h4`.
    pub heading_sizes: [f32; 6],
    /// The line height of every heading level.
    pub heading_line_height: f32,

    // ----- block spacing (logical pixels) -----
    /// `p { margin: 0 0 16px }`
    pub paragraph_margin_bottom: f32,
    /// `h1..h6 { margin: 24px 0 16px }`
    pub heading_margin_top: f32,
    /// The space below a heading, before the next block's own top margin.
    pub heading_margin_bottom: f32,
    /// `h1, h2 { border-bottom: 1px solid }` and `padding-bottom: 0.3em`, both
    /// resolved against the heading's own size. `false` for `h3`–`h6`, which
    /// carry no rule.
    pub heading_rule: [Option<HeadingRule>; 6],
    /// `ul, ol { margin: 0 0 16px }`
    pub list_margin_bottom: f32,
    /// `ul, ol { padding-left: 2em }`
    pub list_indent: f32,
    /// `li { margin-top: 0.25em }` — the gap between items. A user agent also
    /// puts it above the first one, and so does the launcher's stylesheet.
    pub list_item_spacing: f32,
    /// The gap between a list marker and the item's text. A user agent places
    /// the marker outside the content box; the distance here is the same
    /// `0.5em` the task list's checkbox gets.
    pub list_marker_gap: f32,
    /// `blockquote { margin: 0 0 16px 0 }`
    pub quote_margin_bottom: f32,
    /// `blockquote { padding: 0 1em }`
    pub quote_padding_x: f32,
    /// `blockquote { border-left: 0.25em solid }`
    pub quote_bar_width: f32,
    /// `pre { margin: 0 0 16px }`
    pub code_block_margin_bottom: f32,
    /// `pre { padding: 16px }`
    pub code_block_padding: f32,
    /// `pre { border-radius: 8px }`
    pub code_block_radius: f32,
    /// `hr { margin: 24px 0 }`
    pub rule_margin: f32,
    /// `hr { height: 0.25em }`
    pub rule_thickness: f32,
    /// `table { margin: 0 0 16px }`
    pub table_margin_bottom: f32,
    /// `th, td { padding: 6px 13px }`
    pub cell_padding_x: f32,
    /// The space above and below a cell's text.
    pub cell_padding_y: f32,
    /// `th, td { border: 1px solid }`
    pub cell_border: f32,
    /// The gap between a task box and its label (`input { margin-right: 0.5em }`).
    pub task_marker_gap: f32,
    /// The edge of a task list's checkbox. GitHub uses the platform control; a
    /// 13px box is what a webkit checkbox comes out as at this text size.
    pub task_marker_size: f32,
    /// The gap between a table and the paragraph that follows it when the table
    /// is the last block. Folded into `table_margin_bottom`.
    pub footnote_margin_top: f32,
    /// The separator drawn above the collected footnote definitions.
    pub footnote_rule_margin: f32,

    // ----- <details> -----
    //
    // A `<details>` is a control, not prose, so it is sized like the launcher's
    // settings rows rather than like GitHub's own `<summary>`: a 13px title in a
    // padded, rounded, hoverable row with a chevron. GitHub draws a bare
    // `<summary>` at body size with a small triangle, which in a body already
    // carrying the launcher's own typography reads as an unstyled leftover, and
    // the settings rows are the one place in the app where "a row you can click
    // to see more" already has a look.
    /// The summary line's font size, which is a settings row's title rather than
    /// the body's.
    pub details_head_font_size: f32,
    /// The row's left and right padding.
    pub details_head_padding_x: f32,
    /// The row's top and bottom padding.
    pub details_head_padding_y: f32,
    /// The row's corner radius, the same 8px a settings group has.
    pub details_head_radius: f32,
    /// The edge of the disclosure chevron.
    pub details_marker_size: f32,
    /// The gap between the chevron and the title.
    pub details_marker_gap: f32,
    /// The indent the hidden content is laid out at, so its first line sits under
    /// the summary's title rather than under the chevron.
    pub details_content_inset: f32,
    /// The gap between the summary row and the content it hides.
    ///
    /// Not the 1px `SettingCollapse` puts between its rows. That is a seam
    /// *inside* one group, where the rows below it carry on; a `<summary>` is the
    /// end of a group and the beginning of another, and at 1px the first line of
    /// the content sits against the row's own edge. 8px is a little over half the
    /// row's padding, which puts the gap and the padding in the same family.
    pub details_gap: f32,
    /// The space under a collapsed section's summary, before the next block's own
    /// top margin. Folded into the section's own height.
    pub details_margin_bottom: f32,

    // ----- relative sizing -----
    /// `code, pre { font-size: 85% }`, applied to both inline code and code
    /// blocks. GitHub's own inline code is a touch smaller still; the launcher
    /// stylesheet uses 85% for both.
    pub code_font_scale: f32,
    /// `pre { line-height: 1.45 }`
    pub code_line_height: f32,
    /// `code { padding: 0.2em 0.4em }`, relative to the code font's size.
    pub code_padding_x: f32,
    /// The space between an inline-code capsule and the text beside it, relative
    /// to the code font's size. Half a space, which is what reads as "this is
    /// code" without opening a hole in the paragraph.
    ///
    /// A margin and not more padding: padding is inside the capsule, and the gap
    /// the reader is asking about is the one *outside* it. `code_padding_x` alone
    /// leaves the capsule drawn straight over the glyphs next to it, because
    /// nothing in the line's width was ever asked for it.
    pub code_margin_x: f32,
    /// The space above and below an inline code span's glyphs.
    pub code_padding_y: f32,
    /// `code { border-radius: 6px }`
    pub code_radius: f32,
    /// `img { border-radius: 8px }`
    pub image_radius: f32,
    /// The space under a block image, before the next block's own top margin.
    ///
    /// `markdown-body.less` has no rule for it, because a browser needs none: a
    /// lone `img` sits in a paragraph, and the paragraph's `margin-bottom: 16px`
    /// is the gap. There is no paragraph here — the image *is* the block — so
    /// without this the next line starts against the picture's edge.
    pub image_margin_bottom: f32,
    /// The thickness of the rule a hovered link reveals, and of a
    /// strikethrough. One pixel is what a 1px CSS decoration comes out as.
    pub decoration_width: f32,

    // ----- behaviour -----
    /// `word-wrap: break-word` on the container: a word wider than the line is
    /// split instead of overflowing.
    pub break_long_words: bool,
    /// `pre { overflow: auto }`. When false a code block is one long line that
    /// is clipped; when true it is wrapped, which is not what a browser does
    /// but is the only option for a view that cannot scroll sideways.
    pub code_block_wrap: bool,
}

/// The `border-bottom` of a heading that GitHub underlines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadingRule {
    /// The rule's thickness.
    pub width: f32,
    /// `padding-bottom`, relative to the heading's own font size.
    pub padding: f32,
}

impl MdStyle {
    /// The style with `family` and `mono_family` set.
    ///
    /// A caller that embeds a typeface *has* to do this: the measurement is
    /// taken with this family, and the view draws with whatever the caller told
    /// it to. An empty family measures with the generic fallbacks, and a
    /// document laid out with Helvetica's advances and drawn in Comfortaa's is a
    /// document whose lines are the wrong length — which shows up as text clipped
    /// at the end of every line.
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
    /// GitHub's `.markdown-body` at a 14px body, which is what
    /// `src/overlays/content/styles/markdown-body.less` describes.
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

/// The result of a layout pass: the display list and how tall it is.
#[derive(Default, Clone)]
pub struct DisplayList {
    /// The items, in paint order.
    pub items: Vec<MdItem>,
    /// The height a scroll container has to offer, which is the bottom of the
    /// last item plus the container's bottom margin.
    pub height: f32,
    /// The width the list was laid out for.
    pub width: f32,
    /// The widest single line, which is what a shrink-to-fit container wants.
    pub content_width: f32,
    /// The document in runs, in document order. Never empty: a document with no
    /// `<details>` in it is one run of everything.
    ///
    /// This is what a view lays out *vertically*, and it is here because a
    /// collapsible region cannot be drawn over: hiding a section's content means
    /// taking that height out of the flow, and every line below it has to move up
    /// by the same amount. A view that painted a cover over the content and
    /// shrank the height it reported would reclaim no space at all — the gap would
    /// still be there, and everything past it would be scrolled out of reach.
    ///
    /// So the engine hands over the runs, and a view that can stack things stacks
    /// these: a section's run is as tall as its content allows and as short as
    /// its summary row alone, and animating between the two moves everything below
    /// it for free. `SettingCollapse` is the same idea with a `clip: true` box.
    pub chunks: Vec<MdChunk>,
}

/// A run of the document that moves as one thing.
#[derive(Debug, Clone, PartialEq)]
pub struct MdChunk {
    /// The `<details>` this run is, when it is one. A run between two sections
    /// has none, and is never hidden.
    pub section: Option<MdSection>,
    /// Where the run starts in the open layout, which is the origin its items'
    /// own `y` are measured from.
    pub top: f32,
    /// How tall the run is with its section open. A run with no section is always
    /// this tall.
    pub height: f32,
    /// Which of [`DisplayList::items`] belong to this run, as a contiguous range.
    ///
    /// A range rather than a copy: the display list is the same items either way,
    /// and a second copy of a long README's every line is a second copy to keep
    /// correct. Runs are contiguous by construction — the document is one flow.
    pub items: std::ops::Range<usize>,
}

/// A `<details>`: the summary line that is always shown, and the blocks below it
/// that a view hides until the summary is clicked.
///
/// The engine lays a section's content out **whether it is open or not**, and
/// reports the content's full height here rather than laying it out collapsed.
/// That is what makes the open/close animation possible at all: a view that had
/// to ask the engine to re-measure would get a document that jumps to its new
/// height instead of easing into it, and a view that hid the content by
/// re-laying out would watch every line below it move. Instead the items are
/// drawn as they are and the view covers what is hidden, which is the same
/// trick `SettingCollapse` uses with a `clip: true` box and the same reason.
#[derive(Debug, Clone, PartialEq)]
pub struct MdSection {
    /// Identifies the section. A view holds its open/closed state against this,
    /// so the state survives the re-layout a width change causes.
    pub id: u32,
    /// The summary row's left edge.
    pub head_x: f32,
    /// The summary row's top edge.
    pub head_y: f32,
    /// The summary row's width, which is the width the document was laid out
    /// for rather than the summary text's own.
    pub head_width: f32,
    /// The summary row's height: the title's line box between the row's padding.
    pub head_height: f32,
    /// The corner radius of that box.
    pub head_radius: f32,
    /// The disclosure marker's left edge — the chevron a view rotates by half a
    /// turn when the section opens. A box rather than an item because the engine
    /// has no icon font: a view draws what it has.
    pub marker_x: f32,
    /// The marker's top edge, centred in the row.
    pub marker_y: f32,
    /// The chevron's edge length.
    pub marker_size: f32,
    /// Where the hidden content starts, and the indent it is laid out at.
    pub content_x: f32,
    /// The hidden content's top edge, just under the row and its gap.
    pub content_y: f32,
    /// How tall the content is with the section fully open.
    pub content_height: f32,
    /// How much of the document's flow the section takes away while it is closed:
    /// its content **and** the margin under it, which stays either way.
    ///
    /// Not `content_height`, and the difference is the last of the content: the
    /// space under a closed section is its own, so a run that shrank by the
    /// content alone would still be a margin taller than its summary row and the
    /// top of the first hidden line would sit inside it. What is left when the
    /// section is closed is the row and the gap under it, and nothing else.
    pub hidden: f32,
    /// The y below the section, its own margin included — which is where the
    /// document's flow carries on after it. A view that stacks runs needs to know
    /// where this one ends to know where the next begins.
    pub flow_bottom: f32,
    /// Whether the document asked for the section open (`<details open>`). A view
    /// that keeps its own state seeds it from here; see [`Block::Details`].
    pub open: bool,
}
