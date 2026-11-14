// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Block tree → positioned display list. Slint's `Text` cannot report where a
//! line breaks, mix styles on one baseline or draw decorations, so the geometry
//! is computed here and the view only places runs.

use crate::ui::components::markdown::doc::{Align, Block, Cell, Inline};
use crate::ui::components::markdown::measure::{Measurer, SpanMetrics, SpanStyle};
use crate::ui::components::markdown::model::{
    ColorRole, DisplayList, HeadingRule, ImageStore, ItemKind, MdChunk, MdItem, MdSection, MdStyle,
};

/// Line separator: a hard break that survives being flattened into a run. The
/// pass splits the run set on it before shaping.
const HARD_BREAK: char = '\u{2028}';
const HARD_BREAK_STRING: &str = "\u{2028}";

/// Anything wider than `avail` (a minimum-width table, a non-wrapping code
/// block) overflows into [`DisplayList::content_width`].
pub fn layout(
    blocks: &[Block],
    style: &MdStyle,
    avail: f32,
    images: &ImageStore,
    measurer: &mut Measurer,
) -> DisplayList {
    let avail = if avail.is_finite() && avail > 1.0 {
        avail
    } else {
        1.0
    };
    let mut ctx = Ctx {
        style,
        avail,
        images,
        measurer,
        items: Vec::new(),
        links: 0,
        content_width: 0.0,
        sections: Vec::new(),
        section_seq: 0,
    };
    let mut y = 0.0f32;
    for block in blocks {
        y = ctx.block(block, 0.0, y, avail);
    }
    // A nested section finishes before its parent, so re-sort into paint order.
    let mut sections = std::mem::take(&mut ctx.sections);
    sections.sort_by(|a, b| a.head_y.total_cmp(&b.head_y));

    let height = round(y);
    let content_width = round(ctx.content_width);
    let chunks = runs(&ctx.items, &sections, height);
    // Makes an empty render distinguishable from a short one.
    log::debug!(
        "laid out {} block(s) into {} item(s) at {height}px across {} section(s)",
        blocks.len(),
        ctx.items.len(),
        sections.len()
    );
    DisplayList {
        items: ctx.items,
        height,
        width: avail,
        content_width,
        chunks,
    }
}

/// Splits a document into the runs a view stacks; a document with no sections is
/// one run of everything.
fn runs(items: &[MdItem], sections: &[MdSection], height: f32) -> Vec<MdChunk> {
    let mut out: Vec<MdChunk> = Vec::with_capacity(sections.len() * 2 + 1);
    let mut first = 0usize;
    let mut top = 0.0f32;

    for section in sections {
        // Use the first item that reaches into the section rather than the first
        // that starts at it, so an overlapping line goes with the row.
        let section_first = first_reaching(items, section.head_y, first);
        if section_first > first {
            out.push(MdChunk {
                section: None,
                top,
                height: section.head_y - top,
                items: first..section_first,
            });
        }
        let section_last = first_past(items, section.flow_bottom, section_first);
        out.push(MdChunk {
            section: Some(section.clone()),
            top: section.head_y,
            height: section.flow_bottom - section.head_y,
            items: section_first..section_last,
        });
        first = section_last;
        top = section.flow_bottom;
    }

    if first < items.len() {
        out.push(MdChunk {
            section: None,
            top,
            height: (height - top).max(0.0),
            items: first..items.len(),
        });
    }
    out
}

/// The first item at or after `from` that reaches down to `y`.
fn first_reaching(items: &[MdItem], y: f32, from: usize) -> usize {
    (from..items.len())
        .find(|index| items[*index].y + items[*index].height > y)
        .unwrap_or(items.len())
}

/// The first item at or after `from` that starts at or below `y`, which is where
/// the run before it ends.
fn first_past(items: &[MdItem], y: f32, from: usize) -> usize {
    (from..items.len())
        .find(|index| items[*index].y >= y)
        .unwrap_or(items.len())
}

#[derive(Debug, Clone)]
struct RunStyle {
    size: f32,
    weight: u16,
    italic: bool,
    mono: bool,
    color: ColorRole,
    /// `(id, url)` shared by every line of the same link.
    link: Option<(String, String)>,
    chip: bool,
    strike: bool,
    pad_x: f32,
    code_padding_x: f32,
    code_margin_x: f32,
    code_padding_y: f32,
    code_font_scale: f32,
    /// Space outside the capsule, which has to be room in the line or the
    /// capsule is drawn over the glyphs beside it.
    margin_x: f32,
    pad_y: f32,
    line_factor: f32,
    /// A code block's indentation is the code.
    preserve_whitespace: bool,
}

impl RunStyle {
    fn body(style: &MdStyle) -> Self {
        Self {
            code_padding_x: style.code_padding_x,
            code_margin_x: style.code_margin_x,
            code_padding_y: style.code_padding_y,
            code_font_scale: style.code_font_scale,
            size: style.font_size,
            weight: style.weight_normal,
            italic: false,
            mono: false,
            color: ColorRole::Text,
            link: None,
            chip: false,
            strike: false,
            pad_x: 0.0,
            margin_x: 0.0,
            pad_y: 0.0,
            line_factor: style.line_height,
            preserve_whitespace: false,
        }
    }

    fn heading(style: &MdStyle, level: usize) -> Self {
        Self {
            size: style.font_size * style.heading_sizes[level],
            weight: style.heading_weight,
            color: ColorRole::Heading,
            line_factor: style.heading_line_height,
            ..Self::body(style)
        }
    }

    fn code(style: &MdStyle) -> Self {
        Self {
            size: style.font_size * style.code_font_scale,
            mono: true,
            color: ColorRole::CodeBlockText,
            line_factor: style.code_line_height,
            preserve_whitespace: true,
            ..Self::body(style)
        }
    }

    fn quote(style: &MdStyle) -> Self {
        Self {
            color: ColorRole::QuoteText,
            ..Self::body(style)
        }
    }

    fn details_head(style: &MdStyle) -> Self {
        Self {
            size: style.details_head_font_size,
            weight: style.weight_normal,
            line_factor: style.line_height,
            ..Self::body(style)
        }
    }

    fn span_style(&self, body_family: &str, mono_family: &str) -> SpanStyle {
        SpanStyle {
            family: if self.mono {
                mono_family.to_string()
            } else {
                body_family.to_string()
            },
            mono: self.mono,
            size: self.size,
            weight: self.weight,
            italic: self.italic,
        }
    }
}

#[derive(Debug, Clone)]
struct Run {
    text: String,
    style: RunStyle,
    /// The URL of an inline image. `text` is kept as its alt text, so the same
    /// run reads as alt text until the bitmap arrives and as the picture after.
    image: Option<String>,
}

impl Run {
    fn text(text: impl Into<String>, style: RunStyle) -> Self {
        Self {
            text: text.into(),
            style,
            image: None,
        }
    }

    fn is_placed_image(&self, images: &ImageStore) -> bool {
        self.image.as_ref().is_some_and(|url| images.contains(url))
    }
}

/// `links` numbers the links so one id ties a link's pieces together across
/// lines; a run outside a link gets none.
fn flatten(inlines: &[Inline], style: &RunStyle, out: &mut Vec<Run>, links: &mut u32) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => {
                if style.preserve_whitespace {
                    push_run(out, text, style);
                } else {
                    push_run(out, &collapse_whitespace(text), style);
                }
            }
            Inline::Emph(inner) => {
                let mut nested = style.clone();
                nested.italic = true;
                flatten(inner, &nested, out, links);
            }
            Inline::Strong(inner) => {
                let mut nested = style.clone();
                // A document that nests `***x***` should not end up lighter than
                // the body text, so the weight only ever goes up.
                nested.weight = style.weight.max(700);
                flatten(inner, &nested, out, links);
            }
            Inline::Strike(inner) => {
                let mut nested = style.clone();
                nested.strike = true;
                flatten(inner, &nested, out, links);
            }
            Inline::Sup(inner) => {
                // Slint has no baseline shift, so a superscript is just smaller.
                let mut nested = style.clone();
                nested.size = style.size * 0.7;
                flatten(inner, &nested, out, links);
            }
            Inline::Code(text) => {
                if text.is_empty() {
                    continue;
                }
                let mut nested = style.clone();
                nested.mono = true;
                nested.chip = true;
                nested.color = ColorRole::CodeText;
                nested.size = style.size * style.code_font_scale;
                nested.pad_x = nested.size * style.code_padding_x;
                nested.margin_x = nested.size * style.code_margin_x;
                nested.pad_y = nested.size * style.code_padding_y;
                nested.preserve_whitespace = true;
                out.push(Run::text(text.clone(), nested));
            }
            Inline::Link {
                url,
                inlines: label,
                ..
            } => {
                let mut nested = style.clone();
                nested.color = ColorRole::Link;
                // A nested link keeps the outermost one's id, as a browser does.
                if nested.link.is_none() {
                    *links += 1;
                    nested.link = Some((format!("l{links}"), url.clone()));
                }
                flatten(label, &nested, out, links);
            }
            Inline::Image { url, alt, .. } => {
                // The alt text stays as the run's text so an unfetched badge row
                // still reads, rather than showing nothing.
                let mut nested = style.clone();
                nested.color = ColorRole::Placeholder;
                let text = if alt.is_empty() {
                    url.clone()
                } else {
                    alt.clone()
                };
                out.push(Run {
                    text,
                    style: nested,
                    image: Some(url.clone()),
                });
            }
            Inline::HardBreak => {
                out.push(Run::text(HARD_BREAK, style.clone()));
            }
            Inline::FootnoteRef { number, .. } => {
                let mut nested = style.clone();
                nested.color = ColorRole::Link;
                push_run(out, &format!("[{number}]"), &nested);
            }
            Inline::Raw(text) => push_run(out, text, style),
        }
    }
}

fn push_run(out: &mut Vec<Run>, text: &str, style: &RunStyle) {
    let text = collapse_whitespace(text);
    if !text.is_empty() {
        out.push(Run::text(text, style.clone()));
    }
}

/// A run of whitespace is one space, but the space at either end is kept because
/// it separates two runs. Code blocks keep their whitespace instead.
fn collapse_whitespace(text: &str) -> String {
    if !text.contains(char::is_whitespace) || text.chars().all(char::is_whitespace) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            space = true;
        } else {
            if space {
                out.push(' ');
            }
            space = false;
            out.push(ch);
        }
    }
    if space {
        out.push(' ');
    }
    out
}

/// One shaped cluster, placed in the line it belongs to.
#[derive(Debug, Clone, Copy)]
struct Atom {
    run: usize,
    /// Byte range within that run's text.
    start: usize,
    end: usize,
    /// The cluster's advance plus [`Self::leading`], baked in so every reader
    /// agrees without knowing about it.
    advance: f32,
    /// The capsule's margin before the glyphs: part of `advance`, but not ink.
    leading: f32,
    trailing: f32,
    /// A line may start here.
    break_before: bool,
    space: bool,
    hard_after: bool,
}

struct Metrics {
    runs: Vec<Run>,
    measured: Vec<SpanMetrics>,
}

/// `hard_breaks` turns every newline into its own break, which a code block
/// needs.
fn build_atoms(metrics: &Metrics, hard_breaks: bool) -> Vec<Atom> {
    let mut atoms: Vec<Atom> = Vec::new();
    for index in 0..metrics.runs.len() {
        // The run's own ends, not the ends of a line it is later split across.
        let first_of_run = atoms.len();
        let margin_x = metrics.runs[index].style.margin_x;
        // An image has no glyphs, so it is one unbreakable atom.
        if metrics.measured[index].clusters.is_empty() {
            let advance = metrics.measured[index].width;
            if advance > 0.0 {
                atoms.push(Atom {
                    run: index,
                    start: 0,
                    end: 0,
                    advance: advance + margin_x * 2.0,
                    leading: margin_x,
                    trailing: margin_x,
                    break_before: false,
                    space: false,
                    hard_after: false,
                });
            }
            continue;
        }
        let laid_out = &metrics.measured[index].text;
        for cluster in &metrics.measured[index].clusters {
            let raw = &laid_out[cluster.start..cluster.end];
            let hard_after = hard_breaks && raw.contains('\n');
            let text = if hard_after {
                raw.replace('\n', "")
            } else {
                raw.to_string()
            };
            if text.is_empty() {
                if hard_after && let Some(last) = atoms.last_mut() {
                    last.hard_after = true;
                }
                continue;
            }
            atoms.push(Atom {
                run: index,
                start: cluster.start,
                end: cluster.start + text.len(),
                advance: cluster.advance,
                leading: 0.0,
                trailing: 0.0,
                break_before: false,
                space: text.chars().all(char::is_whitespace),
                hard_after,
            });
        }
        let end_of_run = atoms.len();
        if end_of_run > first_of_run {
            atoms[first_of_run].leading += margin_x;
            atoms[first_of_run].advance += margin_x;
            let last = end_of_run - 1;
            atoms[last].trailing += margin_x;
            atoms[last].advance += margin_x;
        }
    }
    mark_breaks(&mut atoms, metrics);
    atoms
}

fn mark_breaks(atoms: &mut [Atom], metrics: &Metrics) {
    for index in 1..atoms.len() {
        // A line may start and end either side of an image, which is what lets a
        // badge row wrap.
        if is_image_atom(metrics, &atoms[index - 1]) || is_image_atom(metrics, &atoms[index]) {
            atoms[index].break_before = true;
            continue;
        }
        if atoms[index - 1].hard_after || atoms[index].space {
            continue;
        }
        if atoms[index - 1].space {
            // Breaking *after* a space is the only opportunity Latin text has.
            atoms[index].break_before = true;
            continue;
        }
        let before = last_char(metrics, &atoms[index - 1]);
        let after = first_char(metrics, &atoms[index]);
        atoms[index].break_before = break_allowed(before, after);
    }
}

fn is_image_atom(metrics: &Metrics, atom: &Atom) -> bool {
    metrics.measured[atom.run].clusters.is_empty() && atom.advance > 0.0
}

fn first_char(metrics: &Metrics, atom: &Atom) -> Option<char> {
    metrics.measured[atom.run].text[atom.start..atom.end]
        .chars()
        .next()
}

fn last_char(metrics: &Metrics, atom: &Atom) -> Option<char> {
    metrics.measured[atom.run].text[atom.start..atom.end]
        .chars()
        .next_back()
}

fn break_allowed(before: Option<char>, after: Option<char>) -> bool {
    let (Some(before), Some(after)) = (before, after) else {
        return false;
    };
    if NO_BREAK_BEFORE.contains(&after) || NO_BREAK_AFTER.contains(&before) {
        return false;
    }
    is_break_after(before) || (is_wide(before) && is_wide(after))
}

/// The characters a line may break after without a space before them.
fn is_break_after(ch: char) -> bool {
    matches!(
        ch,
        '-' | '\u{2010}' | '\u{2013}' | '\u{2014}' | '/' | '\u{2026}'
    )
}

/// The full-width scripts, between any two of which a line may break: there is
/// no space to break at, so every character boundary is an opportunity.
fn is_wide(ch: char) -> bool {
    matches!(ch as u32,
        0x1100..=0x115f      // Hangul Jamo
        | 0x2e80..=0x303e   // CJK radicals … CJK symbols and punctuation
        | 0x3041..=0x33ff   // Kana, Hangul compatibility Jamo, CJK compat
        | 0x3400..=0x4dbf   // CJK extension A
        | 0x4e00..=0x9fff   // CJK unified ideographs
        | 0xa000..=0xa4cf   // Yi
        | 0xac00..=0xd7a3   // Hangul syllables
        | 0xf900..=0xfaff   // CJK compatibility ideographs
        | 0xfe30..=0xfe4f   // CJK compatibility forms
        | 0xff00..=0xff60   // Full-width forms
        | 0xffe0..=0xffe6
        | 0x20000..=0x3fffd // CJK extensions B–F
    )
}

/// Never start a line with these.
const NO_BREAK_BEFORE: &[char] = &[
    ')', ']', '}', '»', '”', '’', '〉', '》', '」', '』', '】', '〕', '｝', '）', '］', '、', '。',
    '，', '．', '・', '：', '；', '！', '？', '…', 'ー', '々', 'ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ', 'っ',
    'ゃ', 'ゅ', 'ょ', 'ゎ', 'ァ', 'ィ', 'ゥ', 'ェ', 'ォ', 'ッ', 'ャ', 'ュ', 'ョ', 'ヮ', '%', ',',
    '.', ':', ';', '?', '!',
];

/// Never end a line with these.
const NO_BREAK_AFTER: &[char] = &[
    '(', '[', '{', '«', '“', '‘', '〈', '《', '「', '『', '【', '〔', '｛', '（', '［',
];

/// Lets a run with no space in it break anywhere: `word-wrap: break-word`, so a
/// URL or a long hash is split rather than allowed to overflow the line.
fn allow_breaks_inside_long_units(atoms: &mut [Atom], avail: f32) {
    if avail <= 0.0 || avail.is_nan() {
        return;
    }
    let mut start = 0usize;
    while start < atoms.len() {
        let mut end = start + 1;
        while end < atoms.len() && !atoms[end].break_before {
            end += 1;
        }
        let width: f32 = atoms[start..end].iter().map(|atom| atom.advance).sum();
        if width > avail {
            for atom in &mut atoms[start + 1..end] {
                atom.break_before = true;
            }
        }
        start = end;
    }
}

/// Greedy line filling by unit (everything between two break opportunities): a
/// line takes a unit only if the whole of it fits. `wrap` false honours only
/// hard breaks, which is what a non-wrapping code block wants.
fn break_lines(atoms: &[Atom], avail: f32, wrap: bool) -> Vec<Vec<usize>> {
    // The width is the widest word plus padding added and subtracted again, so
    // f32 rounding can make a word miss by a few thousandths and break mid-word
    // for no visible reason.
    const FITS: f32 = 0.05;
    let limit = avail + FITS;
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    // Includes the last unit's hanging space; only that space may pass the edge.
    let mut line_pen = 0.0f32;
    let mut unit: Vec<usize> = Vec::new();
    let mut unit_text = 0.0f32;
    let mut unit_space = 0.0f32;

    for (index, atom) in atoms.iter().enumerate() {
        let starts_unit =
            unit.is_empty() || atom.break_before || (index > 0 && atoms[index - 1].hard_after);
        if starts_unit && !unit.is_empty() {
            if wrap && !current.is_empty() && line_pen + unit_text > limit {
                lines.push(std::mem::take(&mut current));
                line_pen = 0.0;
            }
            current.append(&mut unit);
            line_pen += unit_text + unit_space;
            unit_text = 0.0;
            unit_space = 0.0;
        }
        unit.push(index);
        if atom.space {
            // Only the space that *ends* a unit may hang off the line.
            unit_space = atom.advance;
        } else {
            unit_text += atom.advance;
        }
        if atom.hard_after {
            current.append(&mut unit);
            lines.push(std::mem::take(&mut current));
            line_pen = 0.0;
            unit_text = 0.0;
            unit_space = 0.0;
        }
    }
    if !unit.is_empty() {
        if wrap && !current.is_empty() && line_pen + unit_text > limit {
            lines.push(std::mem::take(&mut current));
        }
        current.append(&mut unit);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// A line's box: the tallest run's ascent and descent, with the box's own
/// line-height distributed around them the way CSS's half-leading does.
#[derive(Debug, Clone, Copy)]
struct LineInfo {
    height: f32,
    /// From the top of the box to its baseline.
    baseline: f32,
}

fn line_metrics(metrics: &Metrics, atoms: &[Atom], line: &[usize]) -> LineInfo {
    let mut ascent: f32 = 0.0;
    let mut descent: f32 = 0.0;
    let mut content: f32 = 0.0;
    for index in line {
        let run = &metrics.runs[atoms[*index].run];
        let measured = &metrics.measured[atoms[*index].run];
        ascent = ascent.max(measured.ascent);
        descent = descent.max(measured.descent);
        content = content.max(run.style.line_factor * run.style.size);
    }
    if ascent + descent <= 0.0 {
        // A line of nothing but a zero-width glyph, or a font with no metrics.
        let size = line
            .first()
            .map(|index| metrics.runs[atoms[*index].run].style.size)
            .unwrap_or(14.0);
        ascent = size * 0.8;
        descent = size * 0.2;
        content = size;
    }
    let height = content.max(ascent + descent);
    LineInfo {
        height,
        baseline: (height - (ascent + descent)) / 2.0 + ascent,
    }
}

fn segment_of(atoms: &[Atom], line: &[usize], run: usize) -> Option<Vec<usize>> {
    let indices: Vec<usize> = line
        .iter()
        .copied()
        .filter(|index| atoms[*index].run == run)
        .collect();
    (!indices.is_empty()).then_some(indices)
}

/// The text of a run's slice of one line, and its width. A slice that hangs at
/// the line's end drops trailing whitespace; one in the middle keeps it, because
/// it separates the runs.
fn trim_segment(
    metrics: &Metrics,
    atoms: &[Atom],
    segment: &[usize],
    hangs: bool,
) -> (String, f32) {
    let first = &atoms[segment[0]];
    let last = &atoms[*segment.last().unwrap()];
    let full = &metrics.measured[first.run].text[first.start..last.end];
    let trimmed = if hangs { full.trim_end() } else { full };
    let keep = first.start + trimmed.len();
    let width: f32 = segment
        .iter()
        .map(|index| {
            let atom = &atoms[*index];
            if atom.start < keep { atom.advance } else { 0.0 }
        })
        .sum::<f32>()
        - first.leading
        - last.trailing;
    (trimmed.to_string(), width.max(0.0))
}

/// A run set measured, broken into lines and with its height computed. A hard
/// break ends a plan part, so a paragraph with two breaks has three parts.
struct Plan {
    parts: Vec<Part>,
    height: f32,
    widest: f32,
}

struct Part {
    metrics: Metrics,
    atoms: Vec<Atom>,
    lines: Vec<Vec<usize>>,
}

/// A table cell measured once, because both the natural-width pass and the
/// wrapping pass need it.
struct CellPlan {
    metrics: Metrics,
    /// The width the content wants, as `width: max-content` asks for it.
    natural: f32,
    align: Align,
    head: bool,
}

struct Ctx<'a> {
    style: &'a MdStyle,
    #[allow(dead_code)]
    avail: f32,
    images: &'a ImageStore,
    measurer: &'a mut Measurer,
    items: Vec<MdItem>,
    links: u32,
    /// The widest thing drawn, for a shrink-to-fit view.
    content_width: f32,
    sections: Vec<MdSection>,
    /// Section ids restart with every layout, so they are stable for the
    /// document's lifetime, which is all a view's open/closed state needs.
    section_seq: u32,
}

impl Ctx<'_> {
    fn push(&mut self, mut item: MdItem) {
        if item.width > 0.0 {
            let right = item.x + item.width;
            if right > self.content_width {
                self.content_width = right;
            }
        }
        item.x = round(item.x);
        item.y = round(item.y);
        item.width = round(item.width);
        item.height = round(item.height);
        self.items.push(item);
    }

    fn block(&mut self, block: &Block, x: f32, y: f32, avail: f32) -> f32 {
        match block {
            Block::Paragraph { inlines } => {
                if inlines.is_empty() {
                    return y;
                }
                let base = RunStyle::body(self.style);
                let height = self.text(&base, inlines, x, y, avail, false);
                y + height + self.style.paragraph_margin_bottom
            }

            Block::Heading { level, inlines } => {
                let level = (*level as usize).min(5);
                let base = RunStyle::heading(self.style, level);
                let height = self.text(&base, inlines, x, y, avail, false);
                let mut below = y + height;
                if let Some(HeadingRule { width, padding }) = self.style.heading_rule[level] {
                    // `padding-bottom: 0.3em` sits between the text and the
                    // rule, resolved against the heading's own size.
                    below += padding * base.size;
                    let mut rule = MdItem::rect(x, below, avail, width);
                    rule.color = ColorRole::Rule;
                    self.push(rule);
                    below += width;
                }
                below + self.style.heading_margin_bottom
            }

            Block::CodeBlock { code, .. } => {
                let height = self.code_block(code, x, y, avail);
                y + height + self.style.code_block_margin_bottom
            }

            Block::Quote { blocks } => {
                // `blockquote { padding: 0 1em; border-left: 0.25em }`: the bar
                // and its padding together move the content right.
                let inset = self.style.quote_bar_width + self.style.quote_padding_x;
                let top = y;
                let mut below = y;
                for block in blocks {
                    below = self.quote_block(block, x + inset, below, (avail - inset).max(1.0));
                }
                if below > top {
                    let mut bar = MdItem::rect(x, top, self.style.quote_bar_width, below - top);
                    bar.color = ColorRole::QuoteBar;
                    self.push(bar);
                }
                below + self.style.quote_margin_bottom
            }

            Block::List {
                ordered,
                start,
                tight,
                items,
            } => self.list(*ordered, *start, *tight, items, x, y, avail),

            Block::Rule => {
                let margin = self.style.rule_margin;
                let mut rule = MdItem::rect(x, y + margin, avail, self.style.rule_thickness);
                rule.color = ColorRole::Rule;
                self.push(rule);
                y + margin * 2.0 + self.style.rule_thickness
            }

            Block::Table { head, rows, .. } => {
                // `table` returns an absolute bottom, so adding `y` again would
                // push everything after it down, compounding per table.
                self.table(head, rows, x, y, avail) + self.style.table_margin_bottom
            }

            Block::Image { url, alt, .. } => {
                y + self.image(url, alt, x, y, avail) + self.style.image_margin_bottom
            }

            Block::Details {
                open,
                summary,
                blocks,
            } => self.details(*open, summary, blocks, x, y, avail),

            Block::RawText { text } => {
                let inlines: Vec<Inline> = text
                    .split('\n')
                    .flat_map(|line| [Inline::Text(line.to_string()), Inline::HardBreak])
                    .collect();
                let base = RunStyle::body(self.style);
                let height = self.text(&base, &inlines, x, y, avail, false);
                y + height + self.style.paragraph_margin_bottom
            }

            Block::Footnote { number, blocks, .. } => {
                // GitHub separates the definitions with a rule and numbers them
                // as a list.
                let margin = self.style.footnote_rule_margin;
                let mut rule = MdItem::rect(x, y + margin, avail, self.style.rule_thickness);
                rule.color = ColorRole::Rule;
                self.push(rule);
                let indent = self.style.list_indent;
                let y = y + margin * 2.0 + self.style.rule_thickness + self.style.list_item_spacing;
                self.list_marker(true, *number as u64, x, y, x + indent);
                let mut below = y;
                for block in blocks {
                    below =
                        self.item_block(block, x + indent, below, (avail - indent).max(1.0), false);
                }
                below
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn list(
        &mut self,
        ordered: bool,
        start: u64,
        tight: bool,
        items: &[crate::ui::components::markdown::doc::ListItem],
        x: f32,
        y: f32,
        avail: f32,
    ) -> f32 {
        let indent = self.style.list_indent;
        let content_x = x + indent;
        let content_avail = (avail - indent).max(1.0);
        let mut y = y;
        for (index, item) in items.iter().enumerate() {
            y += self.style.list_item_spacing;
            match item.task {
                Some(checked) => {
                    self.task_marker(checked, x, y, content_x);
                }
                None => {
                    self.list_marker(ordered, start + index as u64, x, y, content_x);
                }
            }
            for block in &item.blocks {
                y = self.item_block(block, content_x, y, content_avail, tight);
            }
        }
        y + self.style.list_margin_bottom
    }

    fn item_block(&mut self, block: &Block, x: f32, y: f32, avail: f32, tight: bool) -> f32 {
        if tight && let Block::Paragraph { inlines } = block {
            if inlines.is_empty() {
                return y;
            }
            let base = RunStyle::body(self.style);
            let height = self.text(&base, inlines, x, y, avail, false);
            return y + height;
        }
        self.block(block, x, y, avail)
    }

    fn quote_block(&mut self, block: &Block, x: f32, y: f32, avail: f32) -> f32 {
        if let Block::Paragraph { inlines } = block {
            let base = RunStyle::quote(self.style);
            let height = self.text(&base, inlines, x, y, avail, false);
            return y + height + self.style.paragraph_margin_bottom;
        }
        self.block(block, x, y, avail)
    }

    /// A `<details>` is laid out **open** either way, so a view can animate the
    /// reveal without re-measuring and shifting every line below it.
    fn details(
        &mut self,
        open: bool,
        summary: &[Inline],
        blocks: &[Block],
        x: f32,
        y: f32,
        avail: f32,
    ) -> f32 {
        let style = self.style;
        if summary.is_empty() {
            // No summary means no control to click, so there is no section.
            let mut inner = y;
            for block in blocks {
                inner = self.block(block, x, inner, avail);
            }
            return inner;
        }

        let id = self.section_seq;
        self.section_seq += 1;

        let head = RunStyle::details_head(style);
        let marker_size = style.details_marker_size;
        let head_height = head.size * style.line_height + 2.0 * style.details_head_padding_y;
        let text_x = x + style.details_head_padding_x + marker_size + style.details_marker_gap;
        self.text(
            &head,
            summary,
            text_x,
            y + style.details_head_padding_y,
            (avail - (text_x - x)).max(1.0),
            false,
        );

        let content_x = x + style.details_content_inset;
        let content_y = y + head_height + style.details_gap;
        let mut inner = content_y;
        for block in blocks {
            inner = self.block(
                block,
                content_x,
                inner,
                (avail - style.details_content_inset).max(1.0),
            );
        }

        let flow_bottom = inner + style.details_margin_bottom;
        self.sections.push(MdSection {
            id,
            flow_bottom,
            head_x: x,
            head_y: y,
            head_width: avail,
            head_height,
            head_radius: style.details_head_radius,
            marker_x: x + style.details_head_padding_x,
            marker_y: y + (head_height - marker_size) / 2.0,
            marker_size,
            content_x,
            content_y,
            content_height: (inner - content_y).max(0.0),
            hidden: (inner - content_y).max(0.0) + style.details_margin_bottom,
            open,
        });
        flow_bottom
    }

    fn text(
        &mut self,
        base: &RunStyle,
        inlines: &[Inline],
        x: f32,
        y: f32,
        avail: f32,
        hard_breaks: bool,
    ) -> f32 {
        let mut runs = Vec::new();
        flatten(inlines, base, &mut runs, &mut self.links);
        let plan = self.plan(runs, avail, true, hard_breaks);
        let height = plan.height;
        self.draw(&plan, x, y);
        height
    }

    /// `hard_breaks` is a code block's own newlines; a paragraph's hard breaks
    /// arrive as [`HARD_BREAK`] runs and split the plan into parts.
    fn plan(&mut self, runs: Vec<Run>, avail: f32, wrap: bool, hard_breaks: bool) -> Plan {
        let mut parts: Vec<Part> = Vec::new();
        let mut part: Vec<Run> = Vec::new();
        for run in runs {
            if run.text == HARD_BREAK_STRING {
                parts.push(self.part(std::mem::take(&mut part), avail, wrap, hard_breaks));
            } else {
                part.push(run);
            }
        }
        parts.push(self.part(part, avail, wrap, hard_breaks));
        let mut height = 0.0f32;
        let mut widest = 0.0f32;
        for part in &parts {
            for line in &part.lines {
                height += line_metrics(&part.metrics, &part.atoms, line).height;
                widest = widest.max(line_advance(&part.atoms, line));
            }
        }
        Plan {
            parts,
            height: round(height),
            widest: round(widest),
        }
    }

    fn part(&mut self, runs: Vec<Run>, avail: f32, wrap: bool, hard_breaks: bool) -> Part {
        let mut measured = Vec::with_capacity(runs.len());
        for run in &runs {
            if run.is_placed_image(self.images) {
                measured.push(self.measure_placed_image(run, avail));
                continue;
            }
            let span = run
                .style
                .span_style(&self.style.font_family, &self.style.mono_family);
            measured.push(self.measurer.measure(&run.text, &span));
        }
        let metrics = Metrics { runs, measured };
        let mut atoms = build_atoms(&metrics, hard_breaks);
        if wrap {
            allow_breaks_inside_long_units(&mut atoms, avail);
        }
        let lines = break_lines(&atoms, avail, wrap);
        Part {
            metrics,
            atoms,
            lines,
        }
    }

    /// An inline image's metrics in place of its alt text's: its box is the
    /// bitmap capped at `max-width: 100%`. A bitmap with no size stays alt text.
    fn measure_placed_image(&mut self, run: &Run, avail: f32) -> SpanMetrics {
        let url = run.image.as_deref().unwrap_or_default();
        let Some(asset) = self.images.get(url) else {
            let span = run
                .style
                .span_style(&self.style.font_family, &self.style.mono_family);
            return self.measurer.measure(&run.text, &span);
        };
        let (natural_width, natural_height) = asset.size;
        if natural_width == 0 || natural_height == 0 {
            let span = run
                .style
                .span_style(&self.style.font_family, &self.style.mono_family);
            return self.measurer.measure(&run.text, &span);
        }
        let ratio = natural_width as f32 / natural_height as f32;
        let width = (natural_width as f32).min(avail);
        let height = width / ratio;
        SpanMetrics {
            text: String::new(),
            clusters: Vec::new(),
            width: round(width),
            ascent: round(height),
            descent: 0.0,
            leading: 0.0,
            baseline: round(height),
            strike_offset: 0.0,
            strike_size: 0.0,
            underline_offset: 0.0,
            underline_size: 0.0,
        }
    }

    /// Items come out in paint order: capsules, glyphs, decorations, then the
    /// link boxes that take the presses.
    fn draw(&mut self, plan: &Plan, x: f32, y: f32) {
        let mut y = y;
        for part in &plan.parts {
            for line in &part.lines {
                y = self.draw_line(part, line, x, y);
            }
        }
    }

    fn draw_line(&mut self, part: &Part, line: &[usize], x: f32, y: f32) -> f32 {
        {
            let plan = part;
            let info = line_metrics(&plan.metrics, &plan.atoms, line);
            let baseline = y + info.baseline;
            // A run whose slice reaches the end of the line is the one that may
            // hang its whitespace off it.
            let hangs = |atom: usize| line.last() == Some(&atom);

            for (index, run) in plan.metrics.runs.iter().enumerate() {
                if !run.style.chip {
                    continue;
                }
                let Some(segment) = segment_of(&plan.atoms, line, index) else {
                    continue;
                };
                let last_of_run = *segment.last().unwrap();
                let (_, advance) =
                    trim_segment(&plan.metrics, &plan.atoms, &segment, hangs(last_of_run));
                if advance <= 0.0 {
                    continue;
                }
                let left = x + advance_of_before(&plan.atoms, line, index);
                let measured = &plan.metrics.measured[index];
                let ascent = measured.ascent;
                let height = measured.ascent + measured.descent;
                let pad_y = run.style.pad_y;
                let mut chip = MdItem::rect(
                    left - run.style.pad_x,
                    baseline - ascent - pad_y,
                    advance + run.style.pad_x * 2.0,
                    height + pad_y * 2.0,
                );
                chip.color = ColorRole::CodeBackground;
                chip.radius = self.style.code_radius;
                self.push(chip);
            }

            for (index, run) in plan.metrics.runs.iter().enumerate() {
                if !run.is_placed_image(self.images) {
                    continue;
                }
                let url = run.image.as_deref().unwrap_or_default();
                let measured = &plan.metrics.measured[index];
                if measured.width <= 0.0 {
                    continue;
                }
                if segment_of(&plan.atoms, line, index).is_none() {
                    continue;
                }
                let left = x + advance_of_before(&plan.atoms, line, index);
                let mut item = MdItem::rect(
                    left,
                    baseline - measured.ascent,
                    measured.width,
                    measured.ascent + measured.descent,
                );
                item.kind = ItemKind::Image;
                item.radius = self.style.image_radius;
                item.text = run.text.clone();
                if let Some(asset) = self.images.get(url) {
                    item.image = Some(asset.image.clone());
                    item.image_size = Some(asset.size);
                }
                if let Some((id, url)) = &run.style.link {
                    item.link_id = id.clone();
                    item.link_url = url.clone();
                }
                self.push(item);
            }

            for (index, run) in plan.metrics.runs.iter().enumerate() {
                if run.is_placed_image(self.images) {
                    continue;
                }
                let Some(segment) = segment_of(&plan.atoms, line, index) else {
                    continue;
                };
                let last_of_run = *segment.last().unwrap();
                let (text, advance) =
                    trim_segment(&plan.metrics, &plan.atoms, &segment, hangs(last_of_run));
                if text.is_empty() || advance <= 0.0 {
                    continue;
                }
                let measured = &plan.metrics.measured[index];
                let left = x + advance_of_before(&plan.atoms, line, index);
                let mut item = MdItem::text(
                    left,
                    baseline - measured.ascent,
                    advance,
                    measured.ascent + measured.descent,
                    text,
                );
                item.font_size = run.style.size;
                item.font_weight = run.style.weight;
                item.font_italic = run.style.italic;
                item.mono = run.style.mono;
                item.color = run.style.color;
                if let Some((id, url)) = &run.style.link {
                    item.link_id = id.clone();
                    item.link_url = url.clone();
                }
                self.push(item);
            }

            for (index, run) in plan.metrics.runs.iter().enumerate() {
                if !run.style.strike {
                    continue;
                }
                let Some(segment) = segment_of(&plan.atoms, line, index) else {
                    continue;
                };
                let last_of_run = *segment.last().unwrap();
                let (text, advance) =
                    trim_segment(&plan.metrics, &plan.atoms, &segment, hangs(last_of_run));
                if text.is_empty() || advance <= 0.0 {
                    continue;
                }
                let measured = &plan.metrics.measured[index];
                let left = x + advance_of_before(&plan.atoms, line, index);
                let mut rule = MdItem::rect(
                    left,
                    baseline - measured.strike_offset,
                    advance,
                    measured.strike_size.max(self.style.decoration_width),
                );
                rule.color = ColorRole::Strike;
                self.push(rule);
            }

            for (index, run) in plan.metrics.runs.iter().enumerate() {
                let Some((id, url)) = run.style.link.clone() else {
                    continue;
                };
                if url.is_empty() {
                    continue;
                }
                let Some(segment) = segment_of(&plan.atoms, line, index) else {
                    continue;
                };
                let last_of_run = *segment.last().unwrap();
                let (text, trimmed) =
                    trim_segment(&plan.metrics, &plan.atoms, &segment, hangs(last_of_run));
                // A linked image has no clusters, so its measured width is the
                // advance; without a box of its own a badge would not be clickable.
                let is_image = run.is_placed_image(self.images);
                let advance = if is_image {
                    plan.metrics.measured[index].width
                } else {
                    trimmed
                };
                if advance <= 0.0 {
                    continue;
                }
                if text.is_empty() && !is_image {
                    continue;
                }
                let measured = &plan.metrics.measured[index];
                let left = x + advance_of_before(&plan.atoms, line, index);
                let top = baseline - measured.ascent;
                let box_height = measured.ascent + measured.descent;
                let mut hit = MdItem::rect(left, top, advance, box_height);
                hit.color = ColorRole::Link;
                // Never a fill: the glyphs are already drawn in the link colour.
                hit.filled = false;
                hit.link_id = id;
                hit.link_url = url;
                // `underline_offset` is measured downwards from the baseline, so
                // from the box top it is the ascent plus the offset. It is zero
                // for a font with no underline metrics; fall back to a small
                // fraction of the em rather than draw on the baseline.
                let offset = if measured.underline_offset > 0.0 {
                    measured.underline_offset
                } else {
                    run.style.size * 0.06
                };
                hit.rule_offset =
                    (measured.ascent + offset - self.style.decoration_width / 2.0).max(0.0);
                hit.rule = !is_image;
                self.push(hit);
            }

            y + info.height
        }
    }

    fn code_block(&mut self, code: &str, x: f32, y: f32, avail: f32) -> f32 {
        let padding = self.style.code_block_padding;
        let wrap = self.style.code_block_wrap;
        let runs = vec![Run {
            image: None,
            text: code.to_string(),
            style: RunStyle::code(self.style),
        }];
        let plan = self.plan(runs, (avail - padding * 2.0).max(1.0), wrap, true);
        let height = plan.height + padding * 2.0;
        let mut background = MdItem::rect(x, y, avail, height);
        background.color = ColorRole::CodeBlockBackground;
        background.radius = self.style.code_block_radius;
        self.push(background);
        self.draw(&plan, x + padding, y + padding);
        height
    }

    /// Column widths come from `width: max-content`; a table still too wide
    /// shares the shortfall between its columns and wraps inside them.
    fn table(&mut self, head: &[Cell], rows: &[Vec<Cell>], x: f32, y: f32, avail: f32) -> f32 {
        let style = self.style;
        let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
        if columns == 0 {
            return 0.0;
        }

        // Every cell is measured once and kept for both width passes.
        let source: Vec<&[Cell]> = std::iter::once(head)
            .chain(rows.iter().map(Vec::as_slice))
            .collect();
        let mut plans: Vec<Vec<CellPlan>> = Vec::with_capacity(source.len());
        for cells in source {
            let mut row = Vec::with_capacity(cells.len());
            for cell in cells {
                let base = if cell.head {
                    RunStyle {
                        weight: style.heading_weight,
                        color: ColorRole::TableHeadText,
                        ..RunStyle::body(style)
                    }
                } else {
                    RunStyle::body(style)
                };
                let mut runs = Vec::new();
                flatten(&cell.inlines, &base, &mut runs, &mut self.links);
                let mut measured = Vec::with_capacity(runs.len());
                for run in &runs {
                    if run.is_placed_image(self.images) {
                        measured.push(self.measure_placed_image(run, avail));
                        continue;
                    }
                    let span = run.style.span_style(&style.font_family, &style.mono_family);
                    measured.push(self.measurer.measure(&run.text, &span));
                }
                let natural = natural_width(&Metrics {
                    runs: runs.clone(),
                    measured: measured.clone(),
                });
                row.push(CellPlan {
                    natural,
                    metrics: Metrics { runs, measured },
                    align: cell.align,
                    head: cell.head,
                });
            }
            plans.push(row);
        }

        let chrome = style.cell_padding_x * 2.0 + style.cell_border * 2.0;
        let mut column_widths = vec![0.0f32; columns];
        for row in &plans {
            for (index, cell) in row.iter().enumerate() {
                if let Some(width) = column_widths.get_mut(index) {
                    *width = width.max(cell.natural);
                }
            }
        }
        for width in &mut column_widths {
            *width += chrome;
        }
        let total: f32 = column_widths.iter().sum();
        if total > avail {
            // Every column keeps a share of what its cells need, and the cells
            // wrap inside it, because a view cannot scroll sideways.
            let factor = avail / total;
            let floor = style.font_size * 2.0;
            for width in &mut column_widths {
                *width = (*width * factor).max(floor);
            }
            // A column cannot usefully be narrower than its longest word plus
            // chrome, so raise each to that and let the columns with slack pay.
            let mut minimums = vec![0.0f32; columns];
            for row in &plans {
                for (index, cell) in row.iter().enumerate() {
                    if let Some(minimum) = minimums.get_mut(index) {
                        *minimum = minimum.max(min_content_width(&cell.metrics) + chrome);
                    }
                }
            }
            let wanted: f32 = minimums
                .iter()
                .zip(&column_widths)
                .map(|(min, got)| (min - got).max(0.0))
                .sum();
            if wanted > 0.0 {
                let slack: f32 = column_widths
                    .iter()
                    .zip(&minimums)
                    .map(|(got, min)| (got - min).max(0.0))
                    .sum();
                if slack > 0.0 {
                    for (index, width) in column_widths.iter_mut().enumerate() {
                        let minimum = minimums[index];
                        if *width < minimum {
                            *width = minimum;
                        } else {
                            *width -= (*width - minimum) / slack * wanted;
                        }
                    }
                }
            }
        }

        // The text keeps at least a character's width; the padding takes what
        // is left, because a cell with no padding still reads and one with no
        // text does not.
        let column_chrome: Vec<f32> = column_widths
            .iter()
            .map(|width| chrome.min((*width - style.font_size).max(0.0)))
            .collect();

        let mut y = y;
        for row in plans.iter_mut() {
            let mut laid_out: Vec<(f32, Plan)> = Vec::with_capacity(row.len());
            // The row height is the tallest content plus the cell padding.
            let mut content_height = 0.0f32;
            for (index, cell) in row.iter_mut().enumerate() {
                let column = column_widths.get(index).copied().unwrap_or(0.0);
                if column <= 0.0 {
                    laid_out.push((0.0, empty_plan()));
                    continue;
                }
                let cell_chrome = column_chrome.get(index).copied().unwrap_or(chrome);
                let runs = std::mem::take(&mut cell.metrics.runs);
                let plan = self.plan(runs, (column - cell_chrome).max(1.0), true, false);
                content_height = content_height.max(plan.height);
                laid_out.push((column, plan));
            }
            let row_height = content_height + 2.0 * (style.cell_border + style.cell_padding_y);

            let mut cell_x = x;
            for (index, (column, plan)) in laid_out.iter().enumerate() {
                if *column <= 0.0 {
                    continue;
                }
                let cell = &row[index];
                let mut background = MdItem::rect(cell_x, y, *column, row_height);
                background.border_width = style.cell_border;
                background.border_color = ColorRole::TableBorder;
                if cell.head {
                    background.color = ColorRole::TableHeadBackground;
                } else {
                    background.color = ColorRole::TableBorder;
                    background.filled = false;
                }
                self.push(background);
                let cell_chrome = column_chrome.get(index).copied().unwrap_or(chrome);
                let slack = (*column - cell_chrome - plan.widest).max(0.0);
                let offset = cell.align.align_factor() * slack;
                let inset = (cell_chrome * 0.5).max(0.0);
                self.draw(
                    plan,
                    cell_x + inset + offset,
                    y + style.cell_border + style.cell_padding_y,
                );
                cell_x += *column;
            }
            y += row_height;
        }
        y
    }

    fn image(&mut self, url: &str, alt: &str, x: f32, y: f32, avail: f32) -> f32 {
        let Some(asset) = self.images.get(url) else {
            if alt.is_empty() {
                return 0.0;
            }
            let base = RunStyle::body(self.style);
            let inlines = vec![Inline::Text(alt.to_string())];
            return self.text(&base, &inlines, x, y, avail, false);
        };
        let (natural_width, natural_height) = asset.size;
        if natural_width == 0 || natural_height == 0 {
            return 0.0;
        }
        let ratio = natural_width as f32 / natural_height as f32;
        // `max-width: 100%`, so a wide image scales down and keeps its ratio.
        let width = (natural_width as f32).min(avail);
        let height = width / ratio;
        let mut item = MdItem::rect(x, y, width, height);
        item.kind = ItemKind::Image;
        item.radius = self.style.image_radius;
        item.image = Some(asset.image.clone());
        item.image_size = Some(asset.size);
        item.text = alt.to_string();
        self.push(item);
        height
    }

    /// The marker a user agent would place left of the content, centred on the
    /// item's first line.
    fn list_marker(&mut self, ordered: bool, number: u64, x: f32, y: f32, content_x: f32) {
        let text = if ordered {
            format!("{number}.")
        } else {
            "\u{2022}".to_string()
        };
        let size = self.style.font_size;
        let metrics = self.measurer.measure(
            &text,
            &SpanStyle {
                family: self.style.font_family.clone(),
                // A list marker inherits the body face: GitHub's `.markdown-body`
                // gives `code` a monospace family and nothing else one.
                mono: false,
                size,
                weight: self.style.weight_normal,
                italic: false,
            },
        );
        let natural = metrics.ascent + metrics.descent;
        let line = size * self.style.line_height;
        let top = y + (line - natural).max(0.0) / 2.0;
        let right = content_x - self.style.list_marker_gap;
        let mut item = MdItem::text(
            (right - metrics.width).max(x),
            top,
            metrics.width,
            natural,
            text,
        );
        item.font_size = size;
        item.font_weight = self.style.weight_normal;
        item.color = ColorRole::Text;
        self.push(item);
    }

    fn task_marker(&mut self, checked: bool, x: f32, y: f32, content_x: f32) {
        let size = self.style.task_marker_size;
        let line = self.style.font_size * self.style.line_height;
        let top = y + (line - size) / 2.0;
        let left = (content_x - self.style.task_marker_gap - size).max(x);
        let mut box_item = MdItem::rect(left, top, size, size);
        box_item.radius = 3.0;
        box_item.border_width = 1.0;
        box_item.color = ColorRole::Marker;
        // An outline, never a fill: a tick in the marker's colour on a fill of
        // the same colour would be invisible.
        box_item.filled = false;
        box_item.border_color = ColorRole::Marker;
        self.push(box_item);
        if checked {
            let mark = "\u{2713}";
            let mark_size = size * 0.9;
            let metrics = self.measurer.measure(
                mark,
                &SpanStyle {
                    family: self.style.font_family.clone(),
                    mono: false,
                    size: mark_size,
                    weight: self.style.weight_bold,
                    italic: false,
                },
            );
            let natural = (metrics.ascent + metrics.descent).min(size);
            let mut tick = MdItem::text(
                left + (size - metrics.width) / 2.0,
                top + (size - natural) / 2.0,
                metrics.width,
                natural,
                mark,
            );
            tick.font_size = mark_size;
            tick.font_weight = self.style.weight_bold;
            tick.color = ColorRole::Marker;
            self.push(tick);
        }
    }
}

fn empty_plan() -> Plan {
    Plan {
        parts: Vec::new(),
        height: 0.0,
        widest: 0.0,
    }
}

/// Where the run's glyphs start: the atoms before it plus its leading margin,
/// because that margin is before the first glyph rather than inside it.
fn advance_of_before(atoms: &[Atom], line: &[usize], run: usize) -> f32 {
    let mut x = 0.0;
    for index in line {
        let atom = &atoms[*index];
        if atom.run == run {
            return x + atom.leading;
        }
        x += atom.advance;
    }
    x
}

fn line_advance(atoms: &[Atom], line: &[usize]) -> f32 {
    line.iter().map(|index| atoms[*index].advance).sum()
}

fn natural_width(metrics: &Metrics) -> f32 {
    let mut width = 0.0;
    for (index, run) in metrics.runs.iter().enumerate() {
        width += metrics.measured[index].width;
        if run.style.chip {
            width += (run.style.pad_x + run.style.margin_x) * 2.0;
        }
    }
    width
}

/// The widest unbreakable word, which is a column's min-content width.
fn min_content_width(metrics: &Metrics) -> f32 {
    let mut widest = 0.0f32;
    for (index, run) in metrics.runs.iter().enumerate() {
        let measured = &metrics.measured[index];
        // The chip's own padding is decoration and goes with it when the column
        // is at its narrowest, the same way a cell's padding does above.
        let pad = if run.style.chip {
            (run.style.pad_x + run.style.margin_x) * 2.0
        } else {
            0.0
        };
        let mut word = 0.0f32;
        for cluster in &measured.clusters {
            // A space is a break, so a word ends before it rather than after.
            let is_space = measured.text[cluster.start..cluster.end]
                .chars()
                .all(char::is_whitespace);
            if is_space {
                widest = widest.max(word + pad);
                word = 0.0;
            } else {
                word += cluster.advance;
            }
        }
        widest = widest.max(word + pad);
    }
    widest
}

/// Snaps to 1/64th of a pixel, enough to keep a run on the box behind it.
fn round(value: f32) -> f32 {
    (value * 64.0).round() / 64.0
}
