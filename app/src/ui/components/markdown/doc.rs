// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The block tree both front ends produce, the contract between parsing and
//! layout.

/// A block-level node covering everything GitHub draws inside a `.markdown-body`.
#[derive(Debug, Clone, PartialEq)]
// `CodeBlock` reads clearer than `Code` beside the inline `Code`.
#[allow(clippy::enum_variant_names)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Inlines,
    },
    Paragraph {
        inlines: Inlines,
    },
    CodeBlock {
        /// The fence's info string, lowercased and cut at the first space. The
        /// layout draws no label for it.
        info: String,
        code: String,
    },
    List {
        ordered: bool,
        start: u64,
        /// A tight list's items are not wrapped in paragraphs.
        tight: bool,
        items: Vec<ListItem>,
    },
    Quote {
        blocks: Vec<Block>,
    },
    Rule,
    Table {
        head: Vec<Cell>,
        rows: Vec<Vec<Cell>>,
        align: Vec<Align>,
    },
    Image {
        url: String,
        title: String,
        alt: String,
    },
    /// `<details>`: a summary line and the blocks it hides; the one piece of
    /// GitHub's markup that behaves like a control.
    Details {
        open: bool,
        /// Empty for a `<details>` written without a summary.
        summary: Inlines,
        blocks: Vec<Block>,
    },
    /// Raw HTML reduced to its text.
    RawText {
        text: String,
    },
    /// A footnote's definition, lifted out of the flow.
    Footnote {
        name: String,
        /// comrak's reference-order number, or the order it was found in.
        number: usize,
        blocks: Vec<Block>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// `Some(true)` for checked, `Some(false)` for unchecked, `None` for a
    /// non-task item.
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub head: bool,
    pub inlines: Inlines,
    pub align: Align,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    None,
    Left,
    Center,
    Right,
}

impl Align {
    /// 0 for left, 0.5 for centre, 1 for right.
    pub const fn align_factor(self) -> f32 {
        match self {
            Self::None | Self::Left => 0.0,
            Self::Center => 0.5,
            Self::Right => 1.0,
        }
    }
}

pub type Inlines = Vec<Inline>;

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    /// Literal text with entities resolved and soft breaks already spaces.
    Text(String),
    Emph(Inlines),
    Strong(Inlines),
    Strike(Inlines),
    Code(String),
    Link {
        url: String,
        title: String,
        /// A linked image has an image and no text, so this is the image alone.
        inlines: Inlines,
    },
    Image {
        url: String,
        title: String,
        alt: String,
    },
    HardBreak,
    FootnoteRef {
        name: String,
        /// Zero for a reference with no definition.
        number: usize,
    },
    Sup(Inlines),
    /// An inline HTML tag; the tag is dropped, the surrounding text kept.
    Raw(String),
}

impl Block {
    /// Calls `f` with every image URL this block contains, at any depth.
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
