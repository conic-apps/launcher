// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! A tag-soup HTML → [`Block`], which is how CurseForge serves descriptions.
//!
//! An unknown tag is transparent, so its contents keep being parsed; `script`,
//! `style`, `head`, `title` and `noscript` are dropped with their contents.

use crate::ui::components::markdown::doc::{Align, Block, Cell, Inline, Inlines, ListItem};

pub fn parse(source: &str) -> Vec<Block> {
    let tokens = tokenize(source);
    let mut parser = Parser { tokens, pos: 0 };
    parser.blocks(&[])
}

/// The inline content of one element's source, such as a `<summary>` label.
pub fn fragment_inlines(source: &str) -> Inlines {
    let tokens = tokenize(source);
    let mut parser = Parser { tokens, pos: 0 };
    parser.inlines(&InlineContext::default(), &[])
}

#[derive(Debug, Clone)]
enum Token {
    Text(String),
    Open {
        name: String,
        attrs: Vec<(String, String)>,
        self_closing: bool,
    },
    Close(String),
}

impl Token {
    fn name(&self) -> Option<&str> {
        match self {
            Self::Open { name, .. } | Self::Close(name) => Some(name),
            Self::Text(_) => None,
        }
    }
}

fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut text = String::new();
    let mut index = 0usize;

    while index < bytes.len() {
        if bytes[index] != b'<' {
            let start = index;
            while index < bytes.len() && bytes[index] != b'<' {
                index += 1;
            }
            text.push_str(&source[start..index]);
            continue;
        }

        if source[index..].starts_with("<!--") {
            let end = source[index..]
                .find("-->")
                .map(|end| index + end + 3)
                .unwrap_or(bytes.len());
            index = end;
            continue;
        }
        if source[index..].starts_with("<!") || source[index..].starts_with("<?") {
            let end = source[index..]
                .find('>')
                .map(|end| index + end + 1)
                .unwrap_or(bytes.len());
            index = end;
            continue;
        }

        let end = match find_tag_end(source, index) {
            Some(end) => end,
            // A `<` that never closes a tag is text, as in `a < b`.
            None => {
                text.push('<');
                index += 1;
                continue;
            }
        };
        let inner = &source[index + 1..end];
        index = if source[..end].ends_with("/>") {
            end
        } else {
            end + 1
        };

        let Some(token) = parse_tag(inner) else {
            continue;
        };
        if !text.is_empty() {
            tokens.push(Token::Text(unescape_entities(&text)));
            text.clear();
        }
        tokens.push(token);
    }

    if !text.is_empty() {
        tokens.push(Token::Text(unescape_entities(&text)));
    }
    collapse_whitespace(tokens)
}

/// Collapses whitespace and drops it at block edges, but keeps it elsewhere
/// because it separates neighbouring runs.
fn collapse_whitespace(tokens: Vec<Token>) -> Vec<Token> {
    let mut block_edge = vec![true; tokens.len() + 1];
    for (index, token) in tokens.iter().enumerate() {
        block_edge[index] = match token {
            Token::Text(_) => false,
            Token::Open { name, .. } | Token::Close(name) => is_block_tag(name),
        };
    }
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    for (index, token) in tokens.into_iter().enumerate() {
        let Token::Text(raw) = token else {
            out.push(token);
            continue;
        };
        let mut collapsed = String::with_capacity(raw.len());
        let mut in_space = false;
        for ch in raw.chars() {
            if ch.is_whitespace() {
                if !in_space {
                    collapsed.push(' ');
                    in_space = true;
                }
            } else {
                collapsed.push(ch);
                in_space = false;
            }
        }
        if block_edge[index] {
            let trimmed = collapsed.trim_start().to_string();
            out.push(Token::Text(trimmed));
        } else {
            out.push(Token::Text(collapsed));
        }
        if block_edge[index + 1]
            && let Some(Token::Text(text)) = out.last_mut()
        {
            let trimmed = text.trim_end().to_string();
            *text = trimmed;
        }
    }
    out.retain(|token| !matches!(token, Token::Text(text) if text.is_empty()));
    out
}

/// Respects quoted attribute values, so an `href` containing `>` does not end
/// the tag early.
fn find_tag_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    let mut quote: Option<u8> = None;
    while index < bytes.len() {
        let byte = bytes[index];
        match quote {
            Some(open) if byte == open => quote = None,
            Some(_) => {}
            None if byte == b'"' || byte == b'\'' => quote = Some(byte),
            None if byte == b'>' => return Some(index),
            None => {}
        }
        index += 1;
    }
    None
}

fn parse_tag(inner: &str) -> Option<Token> {
    let inner = inner.trim();
    if inner.is_empty() {
        return None;
    }
    if let Some(rest) = inner.strip_prefix('/') {
        let name = tag_name(rest)?;
        return Some(Token::Close(name));
    }
    let self_closing = inner.ends_with('/');
    let inner = inner.trim_end_matches('/');
    let mut chars = inner.char_indices();
    let (_, first) = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let name_end = inner
        .find(|c: char| c.is_whitespace())
        .unwrap_or(inner.len());
    let name = inner[..name_end].to_lowercase();
    let attrs = parse_attributes(&inner[name_end..]);
    Some(Token::Open {
        name,
        attrs,
        self_closing,
    })
}

fn tag_name(rest: &str) -> Option<String> {
    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    let name = rest[..end].to_lowercase();
    (!name.is_empty()).then_some(name)
}

fn parse_attributes(source: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        while index < bytes.len() && (bytes[index] as char).is_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let start = index;
        while index < bytes.len()
            && bytes[index] != b'='
            && !(bytes[index] as char).is_whitespace()
            && bytes[index] != b'/'
        {
            index += 1;
        }
        let name = source[start..index].to_lowercase();
        while index < bytes.len() && (bytes[index] as char).is_whitespace() {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'=' {
            if !name.is_empty() {
                attrs.push((name, String::new()));
            }
            continue;
        }
        index += 1;
        while index < bytes.len() && (bytes[index] as char).is_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            break;
        }
        let value = if bytes[index] == b'"' || bytes[index] == b'\'' {
            let quote = bytes[index];
            index += 1;
            let start = index;
            while index < bytes.len() && bytes[index] != quote {
                index += 1;
            }
            let value = &source[start..index];
            index = (index + 1).min(bytes.len());
            value
        } else {
            let start = index;
            while index < bytes.len() && !(bytes[index] as char).is_whitespace() {
                index += 1;
            }
            &source[start..index]
        };
        if !name.is_empty() {
            attrs.push((name, unescape_entities(value)));
        }
    }
    attrs
}

/// The named entities descriptions actually carry; numeric ones are handled
/// generically below.
const ENTITIES: &[(&str, &str)] = &[
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("quot", "\""),
    ("apos", "'"),
    ("nbsp", "\u{a0}"),
    ("ensp", "\u{2002}"),
    ("emsp", "\u{2003}"),
    ("thinsp", "\u{2009}"),
    ("copy", "©"),
    ("reg", "®"),
    ("trade", "™"),
    ("hellip", "…"),
    ("mdash", "—"),
    ("ndash", "–"),
    ("lsquo", "‘"),
    ("rsquo", "’"),
    ("ldquo", "“"),
    ("rdquo", "”"),
    ("laquo", "«"),
    ("raquo", "»"),
    ("lsaquo", "‹"),
    ("rsaquo", "›"),
    ("bull", "•"),
    ("middot", "·"),
    ("deg", "°"),
    ("plusmn", "±"),
    ("times", "×"),
    ("divide", "÷"),
    ("frac12", "½"),
    ("frac14", "¼"),
    ("frac34", "¾"),
    ("sup2", "²"),
    ("sup3", "³"),
    ("micro", "µ"),
    ("para", "¶"),
    ("sect", "§"),
    ("dagger", "†"),
    ("euro", "€"),
    ("pound", "£"),
    ("yen", "¥"),
    ("cent", "¢"),
    ("larr", "←"),
    ("rarr", "→"),
    ("harr", "↔"),
    ("darr", "↓"),
    ("uarr", "↑"),
    ("infin", "∞"),
    ("ne", "≠"),
    ("le", "≤"),
    ("ge", "≥"),
    ("check", "✓"),
    ("star", "★"),
];

/// An unknown reference is left as written, as a browser does.
pub fn unescape_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        // A reference is at most a handful of characters; anything longer is an
        // ampersand of its own.
        let Some(offset) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..offset];
        let replacement = ENTITIES
            .iter()
            .find(|(name, _)| *name == entity)
            .map(|(_, value)| (*value).to_string())
            .or_else(|| {
                let body = entity.strip_prefix('#')?.trim_end_matches(';');
                let code = match body.strip_prefix(['x', 'X']) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => body.parse().ok(),
                }?;
                char::from_u32(code).map(|c| c.to_string())
            });
        match replacement {
            Some(replacement) => {
                out.push_str(&replacement);
                rest = &rest[offset + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The text of a run of HTML with the tags removed.
pub fn text_content(source: &str) -> String {
    let mut out = String::new();
    for token in tokenize(source) {
        if let Token::Text(text) = token {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&text);
        }
    }
    out.trim().to_string()
}

/// Tags whose contents are code, not prose.
const DROPPED: &[&str] = &[
    "script", "style", "head", "title", "noscript", "template", "iframe", "object", "svg", "math",
    "canvas", "audio", "video", "select", "textarea", "option",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Flags {
    strong: bool,
    emphasis: bool,
    strike: bool,
    code: bool,
    sup: bool,
    sub: bool,
}

impl Flags {
    fn with(&self, name: &str) -> Self {
        let mut flags = *self;
        match name {
            "strong" | "b" => flags.strong = true,
            "em" | "i" | "cite" | "var" => flags.emphasis = true,
            "del" | "s" | "strike" => flags.strike = true,
            "code" | "kbd" | "samp" | "tt" => flags.code = true,
            "sup" => flags.sup = true,
            "sub" => flags.sub = true,
            _ => {}
        }
        flags
    }
}

#[derive(Debug, Clone, Default)]
struct InlineContext {
    flags: Flags,
    link: Option<(String, String)>,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next_token(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn skip_element(&mut self, name: &str) {
        let mut depth = 1usize;
        while let Some(token) = self.next_token() {
            match token.name() {
                Some(open) if open == name => {
                    if matches!(token, Token::Open { .. }) {
                        depth += 1;
                    } else {
                        depth -= 1;
                    }
                }
                Some(close) if close == name => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return;
            }
        }
    }

    fn blocks(&mut self, stop: &[&str]) -> Vec<Block> {
        self.blocks_until(stop, &[])
    }

    /// A second stop set matched only by close tags, which a list item needs: it
    /// must not stop at a nested `<ul>` but must stop at the enclosing `</ul>`.
    fn blocks_until(&mut self, stop: &[&str], closes: &[&str]) -> Vec<Block> {
        let mut out: Vec<Block> = Vec::new();
        let mut pending: Inlines = Vec::new();
        let context = InlineContext::default();

        while let Some(token) = self.peek().cloned() {
            match token {
                Token::Text(text) => {
                    self.pos += 1;
                    if !text.trim().is_empty() {
                        pending.push(Inline::Text(text));
                    }
                }
                Token::Close(name) => {
                    if stop.contains(&name.as_str()) || closes.contains(&name.as_str()) {
                        break;
                    }
                    self.pos += 1;
                }
                Token::Open {
                    ref name,
                    ref attrs,
                    self_closing,
                } => {
                    if DROPPED.contains(&name.as_str()) {
                        self.pos += 1;
                        if !self_closing {
                            self.skip_element(name);
                        }
                        continue;
                    }
                    if stop.contains(&name.as_str()) {
                        break;
                    }
                    if is_block_tag(name) {
                        flush_paragraph(&mut pending, &mut out);
                        self.pos += 1;
                        self.block(name, attrs, &mut out);
                    } else {
                        self.pos += 1;
                        pending.extend(self.inlines(&context, &[name.as_str()]));
                    }
                }
            }
        }
        flush_paragraph(&mut pending, &mut out);
        out
    }

    fn block(&mut self, name: &str, attrs: &[(String, String)], out: &mut Vec<Block>) {
        let attr = |key: &str| {
            attrs
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        match name {
            "p" | "div" | "section" | "article" | "header" | "footer" | "main" | "aside"
            | "figure" | "dd" | "dt" | "address" => {
                if name == "p" {
                    let inlines = self.inlines(&InlineContext::default(), &["p"]);
                    if let [Inline::Image { url, title, alt }] = inlines.as_slice() {
                        out.push(Block::Image {
                            url: url.clone(),
                            title: title.clone(),
                            alt: alt.clone(),
                        });
                    } else if !inlines.is_empty() {
                        out.push(Block::Paragraph { inlines });
                    }
                } else {
                    out.extend(self.blocks(&[]));
                }
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = name.as_bytes()[1] - b'1';
                let inlines = self.inlines(&InlineContext::default(), &[name]);
                out.push(Block::Heading { level, inlines });
            }
            "blockquote" => {
                let blocks = self.blocks(&["blockquote"]);
                out.push(Block::Quote { blocks });
            }
            "ul" | "ol" => {
                let ordered = name == "ol";
                let start = attr("start").and_then(|v| v.parse().ok()).unwrap_or(1);
                let items = self.list_items(&[name], ordered);
                out.push(Block::List {
                    ordered,
                    start,
                    // A description's list is written one tag per line with no
                    // blank lines, which is what "tight" means.
                    tight: true,
                    items,
                });
            }
            "li" => {
                let mut items = self.list_items(&["li", "ul", "ol"], false);
                if let Some(item) = items.pop() {
                    out.extend(item.blocks);
                }
            }
            "pre" => {
                let code = self.raw_text();
                out.push(Block::CodeBlock {
                    info: String::new(),
                    code: code.trim_end_matches('\n').to_string(),
                });
            }
            "hr" => out.push(Block::Rule),
            "details" => {
                // The one element here that is a control rather than a container.
                let open = attr("open").is_some();
                let mut blocks: Vec<Block> = self.blocks(&["summary", "details"]);
                let mut summary: Inlines = Vec::new();
                // `blocks` stops on the summary tag rather than consuming it.
                if matches!(
                    self.peek(),
                    Some(Token::Open { name, self_closing: false, .. }) if name == "summary"
                ) {
                    self.pos += 1;
                    summary = self.inlines(&InlineContext::default(), &["summary"]);
                    blocks.extend(self.blocks(&["details"]));
                }
                out.push(Block::Details {
                    open,
                    summary,
                    blocks,
                });
            }
            "br" => out.push(Block::RawText { text: "\n".into() }),
            "table" => {
                let table = self.table();
                out.extend(table);
            }
            // Unknown: transparent, so the children keep their own structure.
            _ => out.extend(self.blocks(&[])),
        }
    }

    fn list_items(&mut self, stop: &[&str], _ordered: bool) -> Vec<ListItem> {
        let mut items = Vec::new();
        while let Some(token) = self.peek().cloned() {
            match &token {
                Token::Open {
                    name, self_closing, ..
                } if name == "li" && !self_closing => {
                    self.pos += 1;
                    items.push(ListItem {
                        task: None,
                        blocks: self.blocks_until(&["li"], stop),
                    });
                }
                Token::Close(name) if stop.contains(&name.as_str()) => {
                    // Consume the list's own close; a nested `</ul>` left behind
                    // would cut the outer list short.
                    self.pos += 1;
                    break;
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        items
    }

    fn table(&mut self) -> Vec<Block> {
        let mut head: Vec<Cell> = Vec::new();
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        while let Some(token) = self.peek().cloned() {
            let is_row = matches!(&token, Token::Open { name, .. } if name == "tr");
            if matches!(&token, Token::Close(name) if name == "table") {
                self.pos += 1;
                break;
            }
            if !is_row {
                self.pos += 1;
                continue;
            }
            self.pos += 1;
            let mut cells = Vec::new();
            let mut is_head = false;
            while let Some(token) = self.peek().cloned() {
                if matches!(&token, Token::Close(name) if name == "tr" || name == "table") {
                    break;
                }
                let cell = match &token {
                    Token::Open { name, attrs, .. } if name == "th" || name == "td" => {
                        is_head = is_head || name == "th";
                        self.pos += 1;
                        let inlines = self.inlines(&InlineContext::default(), &[name.as_str()]);
                        Cell {
                            head: is_head,
                            inlines,
                            align: align_of(
                                &attrs
                                    .iter()
                                    .map(|(k, v)| (k.as_str(), v.as_str()))
                                    .collect::<Vec<_>>(),
                            ),
                        }
                    }
                    _ => {
                        self.pos += 1;
                        continue;
                    }
                };
                cells.push(cell);
            }
            if is_head && head.is_empty() {
                head = cells;
            } else {
                rows.push(cells);
            }
        }
        if head.is_empty() && rows.is_empty() {
            return Vec::new();
        }
        let align = (0..head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0)))
            .map(|index| {
                head.get(index)
                    .or_else(|| rows.iter().filter_map(|row| row.get(index)).next())
                    .map(|cell| cell.align)
                    .unwrap_or(Align::None)
            })
            .collect();
        vec![Block::Table { head, rows, align }]
    }

    fn raw_text(&mut self) -> String {
        let mut out = String::new();
        let mut depth = 0usize;
        while let Some(token) = self.next_token() {
            match token {
                Token::Text(text) => out.push_str(&text),
                Token::Open { ref name, .. } if name == "pre" || name == "code" => depth += 1,
                Token::Close(ref name) if name == "pre" || name == "code" => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
        out
    }

    fn inlines(&mut self, context: &InlineContext, stop: &[&str]) -> Inlines {
        let mut out: Inlines = Vec::new();
        while let Some(token) = self.peek().cloned() {
            match token {
                Token::Text(text) => {
                    self.pos += 1;
                    if !text.trim().is_empty() {
                        out.push(Inline::Text(text));
                    }
                }
                Token::Close(name) => {
                    if stop.contains(&name.as_str()) {
                        self.pos += 1;
                        break;
                    }
                    self.pos += 1;
                }
                Token::Open {
                    ref name,
                    ref attrs,
                    self_closing,
                } => {
                    let name = name.clone();
                    if stop.contains(&name.as_str()) {
                        break;
                    }
                    if DROPPED.contains(&name.as_str()) {
                        self.pos += 1;
                        if !self_closing {
                            self.skip_element(&name);
                        }
                        continue;
                    }
                    self.pos += 1;
                    match name.as_str() {
                        "br" => out.push(Inline::HardBreak),
                        "img" => out.push(Inline::Image {
                            url: attr_value(attrs, "src").unwrap_or_default().to_string(),
                            title: attr_value(attrs, "title").unwrap_or_default().to_string(),
                            alt: attr_value(attrs, "alt").unwrap_or_default().to_string(),
                        }),
                        "a" => {
                            let url = attr_value(attrs, "href").unwrap_or_default().to_string();
                            let title = attr_value(attrs, "title").unwrap_or_default().to_string();
                            let mut inner = context.clone();
                            inner.link = Some((url.clone(), title.clone()));
                            let label = self.inlines(&inner, &["a"]);
                            // An image link has no label; keep the image's box.
                            if !label.is_empty() {
                                let (url, title) = inner.link.clone().unwrap();
                                out.push(Inline::Link {
                                    url,
                                    title,
                                    inlines: label,
                                });
                            }
                        }
                        // A `div` inside a `p` closes the paragraph.
                        name if is_block_tag(name) => {
                            break;
                        }
                        name => {
                            let mut inner = context.clone();
                            inner.flags = inner.flags.with(name);
                            let contents = self.inlines(&inner, &[name]);
                            out.push(wrap(name, context, contents));
                        }
                    }
                }
            }
        }
        out
    }
}

fn attr_value<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
}

fn align_of(attrs: &[(&str, &str)]) -> Align {
    match attr_value_pairs(attrs, "align") {
        Some("center") => Align::Center,
        Some("right") => Align::Right,
        _ => Align::Left,
    }
}

fn attr_value_pairs<'a>(attrs: &'a [(&'a str, &'a str)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(name, _)| *name == key)
        .map(|(_, value)| *value)
}

fn wrap(tag: &str, context: &InlineContext, contents: Inlines) -> Inline {
    let flags = context.flags.with(tag);
    if flags.strong && !context.flags.strong {
        Inline::Strong(contents)
    } else if flags.emphasis && !context.flags.emphasis {
        Inline::Emph(contents)
    } else if flags.strike && !context.flags.strike {
        Inline::Strike(contents)
    } else if flags.code && !context.flags.code {
        Inline::Code(crate::ui::components::markdown::doc::plain_text(&contents))
    } else if flags.sup && !context.flags.sup {
        Inline::Sup(contents)
    } else if contents.len() == 1 {
        contents.into_iter().next().unwrap()
    } else {
        Inline::Emph(contents)
    }
}

fn is_block_tag(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "main"
            | "aside"
            | "figure"
            | "figcaption"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "ul"
            | "ol"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            | "pre"
            | "blockquote"
            | "hr"
            | "table"
            | "tr"
            | "td"
            | "th"
            | "thead"
            | "tbody"
            | "address"
            | "details"
            | "summary"
    )
}

fn flush_paragraph(pending: &mut Inlines, out: &mut Vec<Block>) {
    if pending.is_empty() {
        return;
    }
    let inlines = std::mem::take(pending);
    // A run of only whitespace, or a lone `<br>`, disappears.
    let text: String = inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text(text) => text.clone(),
            other => other_text(other),
        })
        .collect();
    if text.trim().is_empty() {
        return;
    }
    if let [Inline::Image { url, title, alt }] = inlines.as_slice() {
        out.push(Block::Image {
            url: url.clone(),
            title: title.clone(),
            alt: alt.clone(),
        });
        return;
    }
    out.push(Block::Paragraph { inlines });
}

fn other_text(inline: &Inline) -> String {
    match inline {
        Inline::Text(text) => text.clone(),
        Inline::Code(text) => text.clone(),
        Inline::HardBreak => "\n".into(),
        other => {
            let mut out = Vec::new();
            match other {
                Inline::Emph(inner)
                | Inline::Strong(inner)
                | Inline::Strike(inner)
                | Inline::Sup(inner) => out.extend(inner.iter().cloned()),
                Inline::Link { inlines, .. } => out.extend(inlines.iter().cloned()),
                _ => {}
            }
            crate::ui::components::markdown::doc::plain_text(&out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_table() {
        assert_eq!(unescape_entities("a &amp; b &mdash; c"), "a & b — c");
        assert_eq!(unescape_entities("&#65;&#x42;"), "AB");
        assert_eq!(
            unescape_entities("100 &notarealentity; x"),
            "100 &notarealentity; x"
        );
    }

    #[test]
    fn tokenizer_splits_tags() {
        let tokens = tokenize("a<b>c</b> &amp; d");
        assert!(matches!(tokens[0], Token::Text(ref t) if t == "a"));
        assert!(matches!(tokens[1], Token::Open { ref name, .. } if name == "b"));
        assert!(matches!(tokens[2], Token::Text(ref t) if t == "c"));
        assert!(matches!(tokens[3], Token::Close(ref name) if name == "b"));
    }

    #[test]
    fn a_block_after_a_list_is_not_nested_in_its_last_item() {
        // The Java changelog HTML closes neither its `<li>`s nor its lists. A
        // list item's blocks stopped only on `<li>`, swallowed the `</ul>` as a
        // stray close and then took every following block into the item — so
        // each heading and list came out a little further right than the last.
        let blocks = parse("<ul><li>one<li>two</ul><h2>After</h2><p>tail</p>");
        assert_eq!(blocks.len(), 3, "a list, a heading and a paragraph");
        match &blocks[0] {
            Block::List { items, .. } => assert_eq!(items.len(), 2),
            _ => panic!("the first block is the list"),
        }
        assert!(matches!(blocks[1], Block::Heading { level: 1, .. }));
        assert!(matches!(blocks[2], Block::Paragraph { .. }));
    }

    #[test]
    fn a_nested_list_stays_in_its_item_and_the_outer_list_continues() {
        let blocks = parse("<ul><li>a<ul><li>b</ul><li>c</ul>");
        let Block::List { items, .. } = &blocks[0] else {
            panic!("the first block is the list");
        };
        assert_eq!(items.len(), 2, "the outer list keeps both of its items");
        assert!(
            items[0]
                .blocks
                .iter()
                .any(|block| matches!(block, Block::List { .. })),
            "the nested list is inside the first item"
        );
    }
}
