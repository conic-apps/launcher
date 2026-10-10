// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The block tree both front ends produce.
//!
//! Neither comrak nor an HTML parser is the interesting part: this tree is the
//! contract between "how the document is written" and "how it is laid out", so
//! a new front end only has to produce it.

/// A block-level node. The variants cover everything GitHub's renderer draws
/// inside a `.markdown-body`.
#[derive(Debug, Clone, PartialEq)]
// `CodeBlock` reads clearer than `Code` beside the inline `Code`.
#[allow(clippy::enum_variant_names)]
pub enum Block {
    /// `# …` through `###### …`, or a setext heading. `level` is 0-based, so
    /// `h1` is `0`.
    Heading { level: u8, inlines: Inlines },
    /// A paragraph, or a loose list item's paragraph.
    Paragraph { inlines: Inlines },
    /// A fenced or indented code block. `info` is the fence's language, which
    /// GitHub shows in the box's corner.
    CodeBlock {
        /// The fence's info string, lowercased and cut at the first space. The
        /// layout keeps it for a caller that wants a language label; it draws
        /// none itself, because the launcher's stylesheet styles none either.
        info: String,
        /// The code, without the fence and without a trailing newline.
        code: String,
    },
    /// A bullet or numbered list.
    List {
        ordered: bool,
        start: u64,
        /// A tight list does not wrap its items in paragraphs, so its items are
        /// not separated by a paragraph margin.
        tight: bool,
        items: Vec<ListItem>,
    },
    /// `> …`
    Quote { blocks: Vec<Block> },
    /// A `---`, `***` or `___` line.
    Rule,
    /// A GFM table. `align` is per column and drives `text-align`.
    Table {
        /// The header row. Empty for a table with no header.
        head: Vec<Cell>,
        /// The body rows. A GFM table without a header still has one.
        rows: Vec<Vec<Cell>>,
        align: Vec<Align>,
    },
    /// A paragraph that holds nothing but one image, which GitHub renders as a
    /// block of its own rather than as a line of text.
    Image {
        url: String,
        /// The title attribute, which no stylesheet uses.
        title: String,
        /// The text shown when the bitmap cannot be fetched.
        alt: String,
    },
    /// `<details>`: a summary line and the blocks it hides.
    ///
    /// A README uses this to keep a long settings table out of the way until
    /// somebody wants it, and it is the one piece of GitHub's own markup that
    /// behaves like a control rather than like text.
    Details {
        /// Whether the tag carried `open`, which is the only thing that opens
        /// it. GitHub shows a `<details>` closed unless the attribute is there,
        /// and so does this; a caller that wants a different default overrides
        /// it in the view rather than here, because the source said nothing
        /// either way.
        open: bool,
        /// The `<summary>`'s own inline content, which is shown either way. It
        /// is empty for a `<details>` written without one, and a view that has
        /// nothing to label the control with has to say so.
        summary: Inlines,
        /// The blocks the summary hides. They are laid out whether the section
        /// is open or not — see `MdSection` — so a view can reveal them without
        /// a second pass over the document.
        blocks: Vec<Block>,
    },
    /// Raw HTML that neither front end can model. It is shown as the text it
    /// contains, which is what a browser does with an unknown element too.
    RawText { text: String },
    /// A footnote's definition, lifted out of the flow by the renderer.
    Footnote {
        name: String,
        /// Its ordinal: comrak's reference-order number, or the order the
        /// definition was found in for one the extension did not number.
        number: usize,
        blocks: Vec<Block>,
    },
}

/// One `li`.
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// `Some(true)` for `- [x]`, `Some(false)` for `- [ ]`, `None` for an item
    /// that is not a task.
    pub task: Option<bool>,
    /// The item's own blocks, which for a one-line item is a single paragraph.
    pub blocks: Vec<Block>,
}

/// One `th`/`td`.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// A `th`, whose cell is filled and whose text is bold.
    pub head: bool,
    pub inlines: Inlines,
    pub align: Align,
}

/// `text-align` of a table column, from its `---:` style delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// No `align` attribute and no `---:` delimiter: the text sits where the
    /// layout puts it, which is the left edge.
    #[default]
    None,
    /// `:---`, or `align="left"`.
    Left,
    /// `:---:`, or `align="center"`.
    Center,
    /// `---:`, or `align="right"`.
    Right,
}

impl Align {
    /// The factor an item's box is offset by inside its column, as a fraction of
    /// the leftover space: 0 for left, 0.5 for centre, 1 for right.
    pub const fn align_factor(self) -> f32 {
        match self {
            Self::None | Self::Left => 0.0,
            Self::Center => 0.5,
            Self::Right => 1.0,
        }
    }
}

/// A run of inline nodes.
pub type Inlines = Vec<Inline>;

/// An inline node.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    /// Literal text. Entity references are already resolved, and a soft break
    /// is already a space.
    Text(String),
    /// `*a*`
    Emph(Inlines),
    /// `**a**`
    Strong(Inlines),
    /// `~~a~~`
    Strike(Inlines),
    /// `` `a` `` — a span of code, which the stylesheet draws as a capsule.
    Code(String),
    /// `[label](url)`, a reference link, or an autolink.
    Link {
        url: String,
        /// The title attribute, which no stylesheet uses.
        title: String,
        /// The label. A GitHub image link has an image and no text, in which
        /// case this is the image alone.
        inlines: Inlines,
    },
    /// `![alt](url)`, inline (inside a sentence) as opposed to [`Block::Image`].
    Image {
        url: String,
        /// The title attribute, which no stylesheet uses.
        title: String,
        /// The text shown when the bitmap cannot be fetched.
        alt: String,
    },
    /// A hard line break: two trailing spaces, or a backslash.
    HardBreak,
    /// A footnote reference, already numbered.
    FootnoteRef {
        name: String,
        /// The definition's ordinal, or zero for a reference with no definition.
        number: usize,
    },
    /// `^a` with the superscript extension on.
    Sup(Inlines),
    /// An inline HTML tag. The text of the document around it is kept, the tag
    /// itself is dropped.
    Raw(String),
}

impl Block {
    /// Calls `f` with the URL of every image this block contains, at any depth.
    ///
    /// The renderer uses it to report what a document references so the caller
    /// knows what to fetch; `RawText` is not descended into because a tag soup
    /// that reached it has already been reduced to its text.
    pub fn walk_images(&self, f: &mut impl FnMut(&str)) {
        match self {
            Self::Image { url, .. } => f(url),
            Self::Paragraph { inlines } | Self::Heading { inlines, .. } => walk_images(inlines, f),
            Self::Quote { blocks } | Self::Footnote { blocks, .. } => {
                for block in blocks {
                    block.walk_images(f)
                }
            }
            Self::Details {
                summary, blocks, ..
            } => {
                walk_images(summary, f);
                for block in blocks {
                    block.walk_images(f)
                }
            }
            Self::List { items, .. } => {
                for item in items {
                    for block in &item.blocks {
                        block.walk_images(f)
                    }
                }
            }
            Self::Table { head, rows, .. } => {
                for cell in head.iter().chain(rows.iter().flatten()) {
                    walk_images(&cell.inlines, f)
                }
            }
            Self::CodeBlock { .. } | Self::Rule | Self::RawText { .. } => {}
        }
    }
}

/// Calls `f` with the URL of every image these inlines contain, at any depth.
pub fn walk_images(inlines: &[Inline], f: &mut impl FnMut(&str)) {
    {
        for inline in inlines {
            match inline {
                Inline::Image { url, .. } => f(url),
                Inline::Emph(inner)
                | Inline::Strong(inner)
                | Inline::Strike(inner)
                | Inline::Sup(inner) => walk_images(inner, f),
                Inline::Link { inlines, .. } => walk_images(inlines, f),
                Inline::Text(_)
                | Inline::Code(_)
                | Inline::HardBreak
                | Inline::FootnoteRef { .. }
                | Inline::Raw(_) => {}
            }
        }
    }
}

/// The text of these inlines with every marker removed.
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    push_plain_text(inlines, &mut out);
    out
}

fn push_plain_text(inlines: &[Inline], out: &mut String) {
    {
        for inline in inlines {
            match inline {
                Inline::Text(text) => out.push_str(text),
                Inline::Code(text) => out.push_str(text),
                Inline::Raw(text) => out.push_str(text),
                Inline::Emph(inner)
                | Inline::Strong(inner)
                | Inline::Strike(inner)
                | Inline::Sup(inner) => push_plain_text(inner, out),
                Inline::Link { inlines, .. } => push_plain_text(inlines, out),
                // An image contributes its alt text, which is what a browser
                // shows for one that failed to load.
                Inline::Image { alt, .. } => out.push_str(alt),
                Inline::HardBreak => out.push(' '),
                Inline::FootnoteRef { number, .. } => {
                    out.push('[');
                    out.push_str(&number.to_string());
                    out.push(']');
                }
            }
        }
    }
}
