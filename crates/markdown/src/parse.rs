// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! comrak → [`Block`].
//!
//! comrak is asked for CommonMark plus the GitHub extensions, because that is
//! what a Modrinth README is written against, and its arena AST is walked once
//! into the block tree the layout engine consumes. Nothing is rendered through
//! comrak's own HTML formatter: the point of taking the AST is that the
//! geometry is ours.

use std::collections::HashMap;

use std::cell::RefCell;

use comrak::arena_tree::Node as AstNode;

/// comrak's AST node. `Node<'a, T>` is invariant in `'a` and `children()`
/// hands out `&'a Node<'a, T>`, so every walk below carries the *arena's*
/// lifetime rather than its own borrow's. That is why the signatures are
/// noisier than they would otherwise be.
type Node<'a> = AstNode<'a, RefCell<Ast>>;
use comrak::nodes::{Ast, NodeValue, TableAlignment};
use comrak::{Arena, ComrakOptions, parse_document};

use crate::doc::{Align, Block, Cell, Inline, Inlines, ListItem};

/// The extensions the renderer understands.
///
/// Everything else keeps comrak's default. The render options are left alone
/// too: the HTML formatter is never called, and the parser does not consult
/// them — `render.unsafe_` in particular only decides whether raw HTML reaches
/// the formatter, which never runs.
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

/// Parses a document into its top-level blocks.
///
/// Footnote *definitions* are hoisted to the end of the list, which is where
/// GitHub puts them, and the references are renumbered to match afterwards.
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

/// A `<details>` that has been opened and not yet closed.
struct Open {
    /// Whether its tag carried `open`.
    open: bool,
    /// Its `<summary>`'s content, read from whichever raw HTML block held it.
    summary: Inlines,
    /// Everything the walker converted between the open and the close.
    blocks: Vec<Block>,
}

/// Where the walker puts the blocks it converts.
///
/// A `<details>` is not a node in comrak's tree: comrak ends a raw HTML block at
/// a blank line, so `<details>` and its `</details>` arrive as two siblings with
/// real markdown — tables, lists, headings — parsed *between* them. Re-parsing
/// the joined source would lose all of that, and feeding the tags to the HTML
/// parser would too, so the tags are read here and the blocks stay comrak's.
///
/// That makes the destination a function of the walk's position rather than a
/// fixed list, which is all this is: the innermost open `<details>` if there is
/// one, and the document otherwise. `Deref` is what keeps the rest of the walker
/// reading as if it were still writing into a `Vec`.
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

    /// Hands back what a container (a quote, a list item) collected.
    fn into_root(mut self) -> Vec<Block> {
        // An unclosed `<details>` is still a section: a document truncated
        // mid-tag is one a README is not, but a fragment handed to the API is.
        while !self.open.is_empty() {
            self.close_details();
        }
        self.root
    }

    fn close_details(&mut self) {
        let Some(open) = self.open.pop() else {
            return;
        };
        // The target is the *new* innermost, which is the one this section's own
        // block belongs in: a nested `<details>` closes into its parent.
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

/// The blocks of a container element, which a `<details>` inside it opens and
/// closes within.
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
    // Borrowed once: `node.data.borrow()` is a `Ref` into the arena, and the
    // conversions below need `&mut out` at the same time.
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
            // A raw HTML block is reduced to its text: the renderer has no way to
            // lay out an unknown element, and a browser shows the text too. A
            // comment, a doctype or a CDATA section has no text at all.
            //
            // `<details>` is the exception, and it is the one tag here that is a
            // control rather than a container. comrak ends a raw HTML block at a
            // blank line, so the tags arrive as siblings of the markdown between
            // them, and the walker has to hold the section open across them —
            // see `Sink`.
            for event in details_events(&raw.literal) {
                match event {
                    DetailsEvent::Open { open } => out.open.push(Open {
                        open,
                        summary: Vec::new(),
                        blocks: Vec::new(),
                    }),
                    DetailsEvent::Summary(text) => {
                        // The first `<summary>` is the label; a second one is
                        // content the document put in the wrong place, and
                        // dropping it keeps the label unique.
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
                // Inside a `<details>`, so the tags were the block: there is no
                // text of their own to show.
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

        // A list item outside a list. Only the HTML front end can produce this,
        // and it renders the item's contents.
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

        // Reached only through a `TaskItem` whose content is not a block.
        NodeValue::TaskItem(checked) => out.push(Block::RawText {
            text: format!("[{}]", if checked.is_some() { "x" } else { " " }),
        }),

        // Text at block level, which only a `li` of a tight list can produce.
        NodeValue::Text(text) => out.push(Block::RawText { text: text.clone() }),

        // Anything else: keep the children, and if there are none, keep the
        // node's own literal so that no line of a document disappears.
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

/// Whether the innermost list wraps its items in paragraphs, which is what tells
/// a tight list's items from a loose one's.
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
            // A soft break is a space in a rendered paragraph, which is what
            // GitHub shows: its renderer does not preserve the source newline.
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
                // With the footnotes extension on, comrak has already renamed
                // every definition and reference to the ordinal the first
                // reference gave it, so the name *is* the number. A reference
                // to a definition comrak did not find keeps its own name and is
                // numbered by the second pass below.
                name: name.clone(),
                number: name.parse().unwrap_or(0),
            }),
            NodeValue::HtmlInline(raw) => {
                let text = html::text_content(raw);
                if !text.is_empty() {
                    out.push(Inline::Text(text));
                }
            }
            // A block that an inline walk reached: recurse rather than lose it.
            _ => convert_inline_children(child, out),
        }
    }
}

/// Renumbers the footnote references to match the order the definitions were
/// collected in, which is the order GitHub numbers them in.
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
            // The summary is a run of inlines like any other heading, so a
            // footnote reference in it is numbered like one too.
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

/// The HTML front end's tag-soup reader, which also owns the entity table.
pub mod html;

/// What a raw HTML block did to the `<details>` around it.
enum DetailsEvent {
    /// A `<details>` tag, and whether it carried `open`.
    Open { open: bool },
    /// A `<summary>`'s raw content, which is HTML rather than text.
    Summary(String),
    /// A `</details>`.
    Close,
}

/// Reads the `<details>` structure out of one raw HTML block.
///
/// comrak gives a raw HTML block whole, so a block can hold the open tag, the
/// summary and the close tag at once — `<details><summary>x</summary></details>`
/// on one line is the usual way a README writes an empty one — and the events
/// come back in source order for the walker to apply one at a time.
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
