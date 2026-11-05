// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! comrak AST → [`Block`], with CommonMark and the GitHub extensions.

use std::collections::HashMap;

use std::cell::RefCell;

use comrak::arena_tree::Node as AstNode;

/// `children()` hands out `&'a Node<'a, T>`, so every walk carries the arena's
/// lifetime rather than its own.
type Node<'a> = AstNode<'a, RefCell<Ast>>;
use comrak::nodes::{Ast, NodeValue, TableAlignment};
use comrak::{Arena, ComrakOptions, parse_document};

use crate::ui::components::markdown::doc::{Align, Block, Cell, Inline, Inlines, ListItem};

/// The GitHub extensions the renderer understands. Render options are left
/// alone because comrak's HTML formatter is never called.
fn options() -> ComrakOptions {
    let mut options = ComrakOptions::default();
    let ext = &mut options.extension;
    ext.strikethrough = true;
    // GitHub escapes the handful of tags that could otherwise swap the
    // rendering mode, so `<title>` and friends end up as text rather than as
    // raw HTML.
    ext.tagfilter = true;
    ext.table = true;
    ext.autolink = true;
    ext.tasklist = true;
    ext.superscript = true;
    ext.footnotes = true;
    ext.description_lists = true;
    // A soft break stays a space: GitHub's renderer collapses it, and so does
    // the launcher, so a hard break is only what the document asked for.
    options.render.hardbreaks = false;
    options
}

/// Footnote definitions are hoisted to the end, as GitHub does, and the
/// references renumbered to match.
pub fn parse(source: &str) -> Vec<Block> {
    let arena = Arena::new();
    let root = parse_document(&arena, source, &options());
    let mut out = Sink::new();
    let mut footnotes: Vec<(String, usize, Vec<Block>)> = Vec::new();
    walk_children(root, 0, &mut out, &mut footnotes);
    for (name, number, blocks) in footnotes {
        out.push(Block::Footnote {
            name,
            number,
            blocks,
        });
    }
    let mut root_blocks = out.into_root();
    renumber_footnotes(&mut root_blocks);
    root_blocks
}

type Collected = Vec<(String, usize, Vec<Block>)>;

struct Open {
    open: bool,
    summary: Inlines,
    blocks: Vec<Block>,
}

/// The destination is the innermost open `<details>`, or the document. comrak
/// ends a raw HTML block at a blank line, so the tags arrive as siblings of the
/// markdown between them; re-parsing the joined source would lose that markdown.
/// `Deref` keeps the walker reading as if it wrote to a `Vec`.
struct Sink {
    root: Vec<Block>,
    open: Vec<Open>,
}

impl Sink {
    fn new() -> Self {
        Self {
            root: Vec::new(),
            open: Vec::new(),
        }
    }

    fn into_root(mut self) -> Vec<Block> {
        // A truncated fragment can still leave a section open.
        while !self.open.is_empty() {
            self.close_details();
        }
        self.root
    }

    fn close_details(&mut self) {
        let Some(open) = self.open.pop() else {
            return;
        };
        self.target_mut().push(Block::Details {
            open: open.open,
            summary: open.summary,
            blocks: open.blocks,
        });
    }

    fn target_mut(&mut self) -> &mut Vec<Block> {
        match self.open.last_mut() {
            Some(open) => &mut open.blocks,
            None => &mut self.root,
        }
    }
}

impl std::ops::Deref for Sink {
    type Target = Vec<Block>;
    fn deref(&self) -> &Self::Target {
        match self.open.last() {
            Some(open) => &open.blocks,
            None => &self.root,
        }
    }
}

impl std::ops::DerefMut for Sink {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.target_mut()
    }
}

fn walk_children<'a>(
    parent: &'a Node<'a>,
    list_depth: usize,
    out: &mut Sink,
    footnotes: &mut Collected,
) {
    for child in parent.children() {
        convert(child, list_depth, out, footnotes);
    }
}

fn inner_blocks<'a>(
    node: &'a Node<'a>,
    list_depth: usize,
    footnotes: &mut Collected,
) -> Vec<Block> {
    let mut out = Sink::new();
    walk_children(node, list_depth, &mut out, footnotes);
    out.into_root()
}

fn convert<'a>(node: &'a Node<'a>, list_depth: usize, out: &mut Sink, footnotes: &mut Collected) {
    // Cloned so the `Ref` into the arena is not held while `out` is mutated.
    let value = node.data.borrow().value.clone();
    let tight = is_tight(node, list_depth);

    match &value {
        NodeValue::Document | NodeValue::FrontMatter(_) => {
            walk_children(node, list_depth, out, footnotes)
        }

        NodeValue::Paragraph => {
            let inlines = convert_inlines(node);
            // A paragraph holding nothing but one image is GitHub's block
            // image; anything else stays a paragraph.
            if !tight && let [Inline::Image { url, title, alt }] = inlines.as_slice() {
                out.push(Block::Image {
                    url: url.clone(),
                    title: title.clone(),
                    alt: alt.clone(),
                });
                return;
            }
            out.push(Block::Paragraph { inlines });
        }

        NodeValue::Heading(heading) => {
            out.push(Block::Heading {
                level: heading.level.saturating_sub(1).min(5),
                inlines: convert_inlines(node),
            });
        }

        NodeValue::CodeBlock(code) => {
            out.push(Block::CodeBlock {
                // ```` ```js title="x" ```` carries the language first; the rest
                // is a caption GitHub ignores.
                info: code
                    .info
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_lowercase(),
                code: code.literal.trim_end_matches('\n').to_string(),
            });
        }

        NodeValue::HtmlBlock(raw) => {
            // HTML is reduced to its text, except `<details>`, which is a
            // control held open across sibling blocks (see `Sink`).
            for event in details_events(&raw.literal) {
                match event {
                    DetailsEvent::Open { open } => out.open.push(Open {
                        open,
                        summary: Vec::new(),
                        blocks: Vec::new(),
                    }),
                    DetailsEvent::Summary(text) => {
                        // Only the first `<summary>` is the label.
                        if let Some(section) = out.open.last_mut()
                            && section.summary.is_empty()
                        {
                            section.summary = html::fragment_inlines(&text);
                        }
                    }
                    DetailsEvent::Close => out.close_details(),
                }
            }
            if !out.open.is_empty() {
                return;
            }
            let text = html::text_content(&raw.literal);
            if !text.trim().is_empty() {
                out.push(Block::RawText { text });
            }
        }

        NodeValue::BlockQuote => {
            let blocks = inner_blocks(node, list_depth, footnotes);
            out.push(Block::Quote { blocks });
        }

        NodeValue::List(list) => {
            let mut items = Vec::new();
            for item in node.children() {
                let item_value = item.data.borrow().value.clone();
                let task = match &item_value {
                    NodeValue::TaskItem(checked) => Some(checked.is_some()),
                    _ => None,
                };
                let blocks = inner_blocks(item, list_depth + 1, footnotes);
                items.push(ListItem { task, blocks });
            }
            out.push(Block::List {
                ordered: matches!(list.list_type, comrak::nodes::ListType::Ordered),
                start: list.start as u64,
                tight: list.tight,
                items,
            });
        }

        // Not built by comrak's parser, but a hand-made arena could.
        NodeValue::Item(_) => {
            let blocks = inner_blocks(node, list_depth, footnotes);
            out.extend(blocks);
        }

        NodeValue::Table(align) => convert_table(node, align, out),

        NodeValue::ThematicBreak => out.push(Block::Rule),

        NodeValue::FootnoteDefinition(name) => {
            let blocks = inner_blocks(node, list_depth, footnotes);
            // The same: comrak's own ordinal, or the order it was found in for
            // a definition the extension did not number.
            let number = name.parse().unwrap_or(footnotes.len() + 1);
            footnotes.push((name.clone(), number, blocks));
        }

        NodeValue::TaskItem(checked) => out.push(Block::RawText {
            text: format!("[{}]", if checked.is_some() { "x" } else { " " }),
        }),

        NodeValue::Text(text) => out.push(Block::RawText { text: text.clone() }),

        // Keep the children; if there are none, keep the literal so no line
        // disappears.
        _ => {
            let blocks = inner_blocks(node, list_depth, footnotes);
            if blocks.is_empty() {
                let text = match &value {
                    NodeValue::Code(code) => code.literal.clone(),
                    NodeValue::HtmlInline(raw) => html::text_content(raw),
                    _ => String::new(),
                };
                if !text.trim().is_empty() {
                    out.push(Block::RawText { text });
                }
            } else {
                out.extend(blocks);
            }
        }
    }
}

fn is_tight(node: &Node<'_>, list_depth: usize) -> bool {
    if list_depth == 0 {
        return false;
    }
    let mut current = node;
    for _ in 0..list_depth {
        let Some(parent) = current.parent() else {
            return false;
        };
        if let NodeValue::List(list) = &parent.data.borrow().value {
            return list.tight;
        }
        current = parent;
    }
    false
}

fn convert_table<'a>(node: &'a Node<'a>, align: &[TableAlignment], out: &mut Sink) {
    let aligns: Vec<Align> = align
        .iter()
        .map(|alignment| match alignment {
            TableAlignment::Left => Align::Left,
            TableAlignment::Center => Align::Center,
            TableAlignment::Right => Align::Right,
            TableAlignment::None => Align::None,
        })
        .collect();
    let mut head = Vec::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for row in node.children() {
        let is_head = matches!(row.data.borrow().value, NodeValue::TableRow(true));
        let mut cells = Vec::new();
        for (index, cell) in row.children().enumerate() {
            cells.push(Cell {
                head: is_head,
                inlines: convert_inlines(cell),
                align: aligns.get(index).copied().unwrap_or(Align::None),
            });
        }
        if is_head {
            head = cells;
        } else {
            rows.push(cells);
        }
    }
    if head.is_empty() && rows.is_empty() {
        return;
    }
    out.push(Block::Table {
        head,
        rows,
        align: aligns,
    });
}

fn convert_inlines<'a>(node: &'a Node<'a>) -> Inlines {
    let mut out = Vec::new();
    convert_inline_children(node, &mut out);
    out
}

fn convert_inline_children<'a>(node: &'a Node<'a>, out: &mut Inlines) {
    for child in node.children() {
        let value = child.data.borrow().value.clone();
        match &value {
            NodeValue::Text(text) => out.push(Inline::Text(text.clone())),
            NodeValue::SoftBreak => out.push(Inline::Text(" ".into())),
            NodeValue::LineBreak => out.push(Inline::HardBreak),
            NodeValue::Code(code) => out.push(Inline::Code(code.literal.clone())),
            NodeValue::Emph => out.push(Inline::Emph(convert_inlines(child))),
            NodeValue::Strong => out.push(Inline::Strong(convert_inlines(child))),
            NodeValue::Strikethrough => out.push(Inline::Strike(convert_inlines(child))),
            NodeValue::Superscript => out.push(Inline::Sup(convert_inlines(child))),
            NodeValue::Link(link) => out.push(Inline::Link {
                url: link.url.clone(),
                title: link.title.clone(),
                inlines: convert_inlines(child),
            }),
            NodeValue::Image(image) => out.push(Inline::Image {
                url: image.url.clone(),
                title: image.title.clone(),
                alt: child
                    .children()
                    .filter_map(|grandchild| match &grandchild.data.borrow().value {
                        NodeValue::Text(text) => Some(text.clone()),
                        _ => None,
                    })
                    .collect(),
            }),
            NodeValue::FootnoteReference(name) => out.push(Inline::FootnoteRef {
                // With the extension on, comrak has already renamed the reference
                // to its ordinal, so the name is the number.
                name: name.clone(),
                number: name.parse().unwrap_or(0),
            }),
            NodeValue::HtmlInline(raw) => {
                let text = html::text_content(raw);
                if !text.is_empty() {
                    out.push(Inline::Text(text));
                }
            }
            _ => convert_inline_children(child, out),
        }
    }
}

/// Renumbers references to the order the definitions were collected in.
fn renumber_footnotes(blocks: &mut [Block]) {
    let mut numbers: HashMap<&str, usize> = HashMap::new();
    for block in blocks.iter() {
        if let Block::Footnote { name, number, .. } = block {
            numbers.insert(name.as_str(), *number);
        }
    }
    let numbers: HashMap<String, usize> = numbers.into_iter().map(|(k, v)| (k.into(), v)).collect();
    for block in blocks.iter_mut() {
        renumber_block(block, &numbers);
    }
}

fn renumber_block(block: &mut Block, numbers: &HashMap<String, usize>) {
    match block {
        Block::Heading { inlines, .. } | Block::Paragraph { inlines } => {
            renumber_inlines(inlines, numbers)
        }
        Block::Quote { blocks } | Block::Footnote { blocks, .. } => renumber_footnotes(blocks),
        Block::Details {
            summary, blocks, ..
        } => {
            renumber_inlines(summary, numbers);
            renumber_footnotes(blocks);
        }
        Block::List { items, .. } => {
            for item in items {
                renumber_footnotes(&mut item.blocks);
            }
        }
        Block::Table { head, rows, .. } => {
            for cell in head.iter_mut().chain(rows.iter_mut().flatten()) {
                renumber_inlines(&mut cell.inlines, numbers);
            }
        }
        Block::Image { .. } | Block::CodeBlock { .. } | Block::Rule | Block::RawText { .. } => {}
    }
}

fn renumber_inlines(inlines: &mut Inlines, numbers: &HashMap<String, usize>) {
    for inline in inlines {
        match inline {
            Inline::FootnoteRef { name, number } => {
                if let Some(found) = numbers.get(name) {
                    *number = *found;
                }
            }
            Inline::Emph(inner)
            | Inline::Strong(inner)
            | Inline::Strike(inner)
            | Inline::Sup(inner)
            | Inline::Link { inlines: inner, .. } => renumber_inlines(inner, numbers),
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::Image { .. }
            | Inline::HardBreak
            | Inline::Raw(_) => {}
        }
    }
}

pub mod html;

/// What a raw HTML block did to the `<details>` around it.
enum DetailsEvent {
    Open { open: bool },
    Summary(String),
    Close,
}

/// One block can hold the open tag, the summary and the close tag at once, so
/// the events come back in source order for the walker to apply.
fn details_events(literal: &str) -> Vec<DetailsEvent> {
    let mut events = Vec::new();
    let mut at = 0usize;
    while at < literal.len() {
        let Some(rel) = literal[at..].find('<') else {
            break;
        };
        let start = at + rel;
        let Some(rel) = literal[start..].find('>') else {
            break;
        };
        let end = start + rel;
        let tag = &literal[start..=end];

        let body = tag
            .trim_start_matches('<')
            .trim_start_matches('/')
            .trim_end_matches('>');
        let name: String = body
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        let closing = tag.starts_with("</");

        match (closing, name.as_str()) {
            (false, "details") => events.push(DetailsEvent::Open {
                // `open`, `open=open` and `open=""` all mean the same thing, and
                // so does any capitalisation of the tag itself.
                open: body
                    .split(|c: char| c.is_whitespace())
                    .filter(|word| !word.is_empty())
                    .skip(1)
                    .any(|word| {
                        word.eq_ignore_ascii_case("open") || {
                            word.len() > 4 && word[..4].eq_ignore_ascii_case("open")
                        }
                    }),
            }),
            (false, "summary") => {
                // The label runs to `</summary>`, or to the end of the block when
                // the blank line that ends the block came first — which is what a
                // summary holding a blank line of its own does.
                let after = end + 1;
                let stop = find_tag(&literal[after..], "summary")
                    .map(|rel| after + rel)
                    .unwrap_or(literal.len());
                events.push(DetailsEvent::Summary(literal[after..stop].to_string()));
                at = stop;
                continue;
            }
            (true, "details") => events.push(DetailsEvent::Close),
            _ => {}
        }
        at = end + 1;
    }
    events
}

/// Where a closing `</name>` tag starts, case-insensitively.
fn find_tag(source: &str, name: &str) -> Option<usize> {
    let lower = source.to_ascii_lowercase();
    lower.find(&format!("</{name}"))
}
