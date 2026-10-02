// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The block tree → a positioned display list.
//!
//! # Why the geometry is all here
//!
//! Slint's `Text` draws one run in one style and reports the box it needs, but it
//! cannot tell a caller *where* a line of mixed styles breaks, and it has no
//! strikethrough, no underline and no text background. So the work a browser's
//! layout engine does — deciding the line breaks, aligning several runs of
//! different sizes on one baseline, hanging a rule under a struck-through run —
//! is done here, and the view is left with nothing but "put these runs at these
//! coordinates".
//!
//! The result is a flat list in paint order, which is also what makes the view
//! cheap: one `for` loop, and a view that only wants the visible part of a long
//! document can skip the rest with a `visible` binding on each item's `y`.
//!
//! # Line breaking
//!
//! Runs are measured one at a time (see [`crate::markdown::measure`]) and the breaks are
//! found here, which is what keeps a measured width and a drawn width the same
//! number. The break opportunities are the ones a browser uses for the scripts
//! this renderer sees: after a space, after a hyphen or a slash, and between two
//! ideographs — with the small sets of "never start a line with this" and "never
//! end one with that" characters that keep CJK punctuation off a line's edge. It
//! is not a full UAX #14 implementation, and it does not try to be: a line that
//! comes out a little different from Chromium's is invisible, whereas a wrong
//! *width* would not be.

use crate::markdown::doc::{Align, Block, Cell, Inline};
use crate::markdown::measure::{Measurer, SpanMetrics, SpanStyle};
use crate::markdown::model::{
    ColorRole, DisplayList, HeadingRule, ImageStore, ItemKind, MdChunk, MdItem, MdSection, MdStyle,
};

/// LINE SEPARATOR, which a hard line break is turned into so that it survives
/// being flattened into a run of text on its way to the layout pass. It is
/// never shaped: the pass splits the run set on it first.
const HARD_BREAK: char = '\u{2028}';
const HARD_BREAK_STRING: &str = "\u{2028}";

/// Lays a document out and returns the display list.
///
/// `avail` is the width the document is laid out for. Anything wider — a table
/// that has been given a minimum width, a code block that does not wrap — is
/// allowed to exceed it, and the excess shows up in
/// [`DisplayList::content_width`].
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
    // In document order, which is the order a view stacks them in. A section is
    // recorded once its content is laid out, and a nested one is finished before
    // the section around it, so the order they come out in is the wrong way round
    // for a nested pair.
    let mut sections = std::mem::take(&mut ctx.sections);
    sections.sort_by(|a, b| a.head_y.total_cmp(&b.head_y));

    let height = round(y);
    let content_width = round(ctx.content_width);
    let chunks = runs(&ctx.items, &sections, height);
    DisplayList {
        items: ctx.items,
        height,
        width: avail,
        content_width,
        chunks,
    }
}

/// Splits a laid-out document into the runs a view stacks.
///
/// The document is one flow, so the runs are the stretches between the sections'
/// own extents: a section's items, then whatever came between it and the next
/// section, then that section, and so on. A document with no sections in it is a
/// single run of everything, which is what keeps the common case — a long README
/// with no `<details>` — on exactly the path it was on.
fn runs(items: &[MdItem], sections: &[MdSection], height: f32) -> Vec<MdChunk> {
    let mut out: Vec<MdChunk> = Vec::with_capacity(sections.len() * 2 + 1);
    let mut first = 0usize;
    let mut top = 0.0f32;

    for section in sections {
        // A run of ordinary blocks, if there is one between here and the section.
        // The boundary is the first item that reaches *into* the section rather
        // than the first that starts at it, so a line whose box overlaps the
        // summary row goes with the row instead of being cropped above it.
        let section_first = first_reaching(items, section.head_y, first);
        if section_first > first {
            out.push(MdChunk {
                section: None,
                top,
                height: section.head_y - top,
                items: first..section_first,
            });
        }
        // And the section itself: everything from its first item up to wherever
        // its own flow ends, which is its content plus the margin under it.
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

    // Whatever is left over, which for a document with no sections is all of it.
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

// ---------------------------------------------------------------------------
// inline flattening
// ---------------------------------------------------------------------------

/// The style of one run, as far as layout is concerned.
#[derive(Debug, Clone)]
struct RunStyle {
    size: f32,
    weight: u16,
    italic: bool,
    mono: bool,
    color: ColorRole,
    /// The link this run belongs to, as `(id, url)`, and nothing else is ever a
    /// link. A link's id is shared by every line it appears on, which is how a
    /// view reveals the rule under all of them from one piece of pointer state.
    link: Option<(String, String)>,
    /// Draw the inline-code capsule behind this run.
    chip: bool,
    strike: bool,
    /// The capsule's padding.
    pad_x: f32,
    /// The capsule's own padding and the space outside it, relative to the code
    /// font's size, carried here so an inline code span can be built without the
    /// whole `MdStyle` in reach — `flatten` sees one run's style and nothing else.
    code_padding_x: f32,
    code_margin_x: f32,
    code_padding_y: f32,
    code_font_scale: f32,
    /// Space *outside* the capsule, between it and the text beside it.
    ///
    /// Padding is inside the capsule and a margin is not, and the difference is
    /// the whole of it: `pad_x` belongs to the code run alone, while this has to
    /// be room in the line, or the capsule is drawn straight over the glyphs next
    /// to it. Half a space, which is what reads as "this is code" without opening
    /// a hole in the paragraph.
    margin_x: f32,
    pad_y: f32,
    /// The line-height multiplier of the box this run is in, so a heading's runs
    /// and a paragraph's differ even at the same size.
    line_factor: f32,
    /// Keep every space. A code block's indentation is the code.
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

    /// A `<summary>`'s own text, which is a settings row's title: the body's size
    /// is `font_size` and a row's is `details_head_font_size`, and the row is the
    /// thing the reader is being asked to click.
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
    /// The URL of an inline image this run stands for, when the caller has
    /// handed over a bitmap for it.
    ///
    /// `text` is kept alongside it rather than replaced by it, and that is what
    /// makes an inline image need no separate code path: before the bitmap
    /// arrives the run is ordinary text and reads as its alternative, and once it
    /// arrives the same run becomes a picture in the same place. A badge row that
    /// loads late therefore reads as its alt text first and fills in after, which
    /// is what a browser does.
    image: Option<String>,
}

impl Run {
    /// A run of text.
    fn text(text: impl Into<String>, style: RunStyle) -> Self {
        Self {
            text: text.into(),
            style,
            image: None,
        }
    }

    /// Whether this run is an inline image whose bitmap is on hand.
    fn is_placed_image(&self, images: &ImageStore) -> bool {
        self.image.as_ref().is_some_and(|url| images.contains(url))
    }
}

/// Flattens the inline tree into the runs a line is made of.
///
/// `links` counts the links met on the way, so that each gets an id. The id is
/// what ties one link's pieces together across its lines, and a run that is *not*
/// inside a link gets none — a box full of text that happens to sit next to a
/// link is not part of it.
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
                // Slint has no baseline shift, so a superscript is set smaller;
                // the line box is the paragraph's either way, which is what
                // `vertical-align: super` amounts to at these sizes.
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
                // An inline span sits on the paragraph's baseline, so it keeps the
                // paragraph's line height and only the glyphs shrink.
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
                // A link nested inside a link is not something CommonMark allows
                // and a browser flattens to the outermost one, which is what
                // keeping the outer id does.
                if nested.link.is_none() {
                    *links += 1;
                    nested.link = Some((format!("l{links}"), url.clone()));
                }
                flatten(label, &nested, out, links);
            }
            Inline::Image { url, alt, .. } => {
                // The alternative text is kept as the run's text *and* the URL is
                // kept alongside it, so the run reads as the alternative until the
                // bitmap arrives and as the picture afterwards. A badge row is
                // almost always a row of images inside links, and rendering those
                // as a wall of alt text is the single most visible way a Markdown
                // body can read as broken.
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

/// A run of whitespace is one space, as in HTML — but the space at either end of
/// a run is kept, because it is the only thing between two runs.
///
/// A code block asks for its whitespace back (`preserve_whitespace` on the run's
/// style), where the indentation and the runs of spaces *are* the code.
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
    // The run's own trailing space, if it had one.
    if space {
        out.push(' ');
    }
    out
}

// ---------------------------------------------------------------------------
// line breaking
// ---------------------------------------------------------------------------

/// One shaped cluster, placed in the line it belongs to.
#[derive(Debug, Clone, Copy)]
struct Atom {
    /// Which run it belongs to.
    run: usize,
    /// Byte range within that run's text.
    start: usize,
    end: usize,
    /// The cluster's own advance, plus [`Self::leading`]. Baked in rather than
    /// added by every reader, so the line breaker, the line's width and the
    /// drawing all agree without each of them knowing about it.
    advance: f32,
    /// Space this atom claims *before* its glyphs, and which is not ink: an
    /// inline-code capsule's margin, on the run's first cluster. It is part of
    /// `advance` — the line has to make room for it — but it is not part of the
    /// box anything draws, so a run's glyphs start after it.
    leading: f32,
    /// The same, *after* the glyphs, on the run's last cluster.
    trailing: f32,
    /// A line may start here.
    break_before: bool,
    /// The cluster is nothing but whitespace.
    space: bool,
    /// A hard break follows this cluster.
    hard_after: bool,
}

struct Metrics {
    runs: Vec<Run>,
    measured: Vec<SpanMetrics>,
}

/// Builds the atom stream of a run set.
///
/// `hard_breaks` turns every newline into a break of its own, which is what a
/// code block needs: its source lines are lines, not a paragraph's soft breaks.
fn build_atoms(metrics: &Metrics, hard_breaks: bool) -> Vec<Atom> {
    let mut atoms: Vec<Atom> = Vec::new();
    for index in 0..metrics.runs.len() {
        // Which of this run's atoms are the ends, so the capsule's margin lands on
        // them and nowhere else. Taken before the atoms are pushed, because it is
        // the run's *own* ends that matter, not the ends of a line it is later
        // split across.
        let first_of_run = atoms.len();
        let margin_x = metrics.runs[index].style.margin_x;
        // An inline image has no glyphs, so the shaper gave it none, and it needs
        // exactly one atom of its own: the whole picture, `max-width: 100%` wide,
        // is one unbreakable thing on the line.
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
        // The shaper's text, not the run's: parley collapsed the whitespace, and
        // the cluster ranges index into what it produced.
        let laid_out = &metrics.measured[index].text;
        for cluster in &metrics.measured[index].clusters {
            let raw = &laid_out[cluster.start..cluster.end];
            // A code block's own newlines are its line breaks, and they are kept
            // by the shaper because a code run asks for its whitespace to be
            // preserved.
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
        // The margin goes on this run's two ends now that they are known. A run
        // the shaper gave nothing is already whole above.
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

/// Decides where a line may start.
fn mark_breaks(atoms: &mut [Atom], metrics: &Metrics) {
    for index in 1..atoms.len() {
        // An image is a replaced element with no letters around it, so the rules
        // that read the neighbouring characters have nothing to read. A line may
        // start and end either side of one: that is what lets a row of badges
        // wrap the way a browser wraps it. It is a little generous in the one case
        // where an image sits flush against a word with no space, which in a real
        // document does not happen.
        if is_image_atom(metrics, &atoms[index - 1]) || is_image_atom(metrics, &atoms[index]) {
            atoms[index].break_before = true;
            continue;
        }
        if atoms[index - 1].hard_after || atoms[index].space {
            // A hard break is not a choice, and a line never starts with the
            // space that follows one.
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

/// Whether an atom is an inline image rather than a piece of text.
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

/// Whether a line may start between `before` and `after`.
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

/// Greedy line filling.
///
/// The decision is made a *unit* at a time, not a cluster at a time: a unit is
/// everything between two break opportunities (a word and the space after it, a
/// run of ideographs, a hyphenated piece), and a line takes a unit only if the
/// whole of it fits. Deciding per cluster instead would let a line run over by
/// up to a word, which is the one thing a line breaker must not do.
///
/// `wrap` false keeps everything on one line and honours only the hard breaks,
/// which is what a non-wrapping code block wants.
fn break_lines(atoms: &[Atom], avail: f32, wrap: bool) -> Vec<Vec<usize>> {
    // A word that is exactly as wide as the line it is on fits on it. Without
    // this, it does not, and the failure is invisible in the source: a table
    // column's width is its widest word plus the cell's padding, and the padding
    // is added and then subtracted again, so the round trip through `f32` can land
    // a few thousandths short. "State" then breaks to "Stat" / "e" in the middle of
    // a row of one-word headings, for a margin no eye could ever see. A twentieth
    // of a pixel is far below anything an eye could see, so it changes nothing
    // that is drawn and everything about whether a word breaks.
    const FITS: f32 = 0.05;
    let limit = avail + FITS;
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    // The width of the line so far, the hanging space of its last unit
    // included: the space between two words is part of the line, and only the
    // last one may hang past the edge — which is what a browser does, and the
    // reason a line can end a hair over the width.
    let mut line_pen = 0.0f32;
    // The unit being gathered, its width, and the space that ends it. A unit is
    // committed to the line only once the whole of it is known to fit.
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

/// The atoms of one run that fall on one line.
fn segment_of(atoms: &[Atom], line: &[usize], run: usize) -> Option<Vec<usize>> {
    let indices: Vec<usize> = line
        .iter()
        .copied()
        .filter(|index| atoms[*index].run == run)
        .collect();
    (!indices.is_empty()).then_some(indices)
}

/// The text of a run's slice of one line, and its width.
///
/// `hangs` is whether this run's slice reaches the end of the line. Such a run
/// may drop the whitespace at its end, because a browser hangs that whitespace
/// past the line and a rule drawn under it would be a rule under nothing. A run
/// in the *middle* of a line may not: `A **bold** word` is three runs, and the
/// spaces that separate them are the only thing between the words.
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
    // The margins are room in the line, not in the box: every caller of this wants
    // the glyphs' own width, and the line's width is the atoms' business.
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

/// A run set that has been measured, broken into lines and had its height
/// computed — everything needed to draw it, so a caller can size a box around it
/// first.
///
/// A hard line break ends one of these, so a plan is a *list* of them: a
/// paragraph with two breaks has three, and the atoms of one do not address the
/// runs of another. Splitting here rather than inside the atom stream is also
/// what keeps a break out of the shaper's hands — a paragraph is shaped with
/// whitespace collapsed, and a break is not whitespace the shaper may touch.
struct Plan {
    parts: Vec<Part>,
    height: f32,
    widest: f32,
}

/// One stretch of a plan between two hard line breaks.
struct Part {
    metrics: Metrics,
    atoms: Vec<Atom>,
    lines: Vec<Vec<usize>>,
}

/// A table cell that has been measured once and kept, because both the natural
/// width pass and the wrapping pass need it.
struct CellPlan {
    metrics: Metrics,
    /// The width the cell's content wants, the way `width: max-content` asks
    /// for it. The column is as wide as the widest cell in it.
    natural: f32,
    /// `text-align` of the column this cell is in.
    align: Align,
    head: bool,
}

struct Ctx<'a> {
    style: &'a MdStyle,
    /// The width the document is laid out for. Blocks are told their own `avail`
    /// as they are walked, because a list, a quote or a table narrows it, so
    /// this is only what a top-level block gets.
    #[allow(dead_code)]
    avail: f32,
    images: &'a ImageStore,
    measurer: &'a mut Measurer,
    items: Vec<MdItem>,
    /// How many links the document has, which is what a link's id is made of.
    links: u32,
    /// The widest thing drawn, which is what a shrink-to-fit view wants.
    content_width: f32,
    /// The `<details>` sections, in paint order.
    sections: Vec<MdSection>,
    /// How many sections have been numbered so far, which is what a section's id
    /// is counted off. It restarts with every layout, so an id is stable for as
    /// long as the document is — which is all a view holding open/closed state
    /// against it needs, since a different document resets that state anyway.
    section_seq: u32,
}

impl Ctx<'_> {
    /// Records an item and keeps the document's width up to date.
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

    // ----- blocks -----

    /// Lays a block out and returns the y below it, margins included.
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
                // `table` walks the rows from the `y` it is given and returns
                // the bottom of the last one, so what comes back is already an
                // absolute y. Adding `y` to it a second time pushes everything
                // after the table down by a whole table's worth of offset, and
                // the error compounds with every later table.
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
                // Markup the tree does not model, as the text of it, one source
                // line per output line.
                let inlines: Vec<Inline> = text
                    .split('\n')
                    .flat_map(|line| [Inline::Text(line.to_string()), Inline::HardBreak])
                    .collect();
                let base = RunStyle::body(self.style);
                let height = self.text(&base, &inlines, x, y, avail, false);
                y + height + self.style.paragraph_margin_bottom
            }

            Block::Footnote { number, blocks, .. } => {
                // GitHub separates the collected definitions with a rule of its
                // own and numbers them as a list.
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
        items: &[crate::markdown::doc::ListItem],
        x: f32,
        y: f32,
        avail: f32,
    ) -> f32 {
        let indent = self.style.list_indent;
        let content_x = x + indent;
        let content_avail = (avail - indent).max(1.0);
        let mut y = y;
        for (index, item) in items.iter().enumerate() {
            // `li { margin-top: 0.25em }` applies to the first item too, which is
            // what the launcher's stylesheet says.
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

    /// A block inside a list item: a tight list's paragraph is not a paragraph.
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

    /// A block inside a block quote. The quote's own padding is a left inset the
    /// caller already applied; a nested quote stacks another one on top of it.
    fn quote_block(&mut self, block: &Block, x: f32, y: f32, avail: f32) -> f32 {
        if let Block::Paragraph { inlines } = block {
            let base = RunStyle::quote(self.style);
            let height = self.text(&base, inlines, x, y, avail, false);
            return y + height + self.style.paragraph_margin_bottom;
        }
        self.block(block, x, y, avail)
    }

    /// A `<details>`: a summary row that is always drawn, and the blocks under
    /// it that a view hides until the row is clicked.
    ///
    /// The content is laid out either way, and what comes back is the height the
    /// section takes **open**. A view that wanted the document re-measured to
    /// close would get a jump rather than an animation, and one that hid the
    /// content by re-laying out would watch every line below it shift; so the
    /// engine reports the geometry and the view does the hiding. See `MdSection`.
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
            // No summary means no control to click, and a control with no label is
            // not one: GitHub draws the content and nothing else, so there is no
            // section here for a view to hide anything behind.
            let mut inner = y;
            for block in blocks {
                inner = self.block(block, x, inner, avail);
            }
            return inner;
        }

        let id = self.section_seq;
        self.section_seq += 1;

        // A settings row with a title and no description: the title's own line box
        // between two paddings. The chevron is centred in what is left.
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

        // The content is indented from the section's left edge by
        // `details_content_inset`, which defaults to the row's own padding.
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

    // ----- text -----
    /// Lays inlines out as wrapped lines and draws them. Returns the height.
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

    /// Measures a run set, finds its breaks and works out how tall it is.
    ///
    /// `hard_breaks` is the code block's own newlines. A paragraph's hard line
    /// breaks arrive as [`HARD_BREAK`] runs and split the plan into parts.
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

    /// Measures one stretch between two hard line breaks.
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

    /// An inline image's own metrics, in place of its alt text's.
    ///
    /// A replaced element has no glyphs to shape: its box is the bitmap's, capped
    /// the way `.markdown-body img { max-width: 100% }` caps one, and it sits on
    /// the baseline with nothing below it — which is what makes a line of badges
    /// as tall as the badges rather than as tall as the text.
    ///
    /// A bitmap with no size is left to the alt text: a caller that hands over an
    /// image with no dimensions has not really handed over an image, and guessing
    /// a box for it would push everything after it around.
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

    /// Draws a plan at `x`, `y`. Items come out in paint order: the capsules
    /// first, then the glyphs on them, then the decorations, then the link boxes
    /// that take the presses.
    fn draw(&mut self, plan: &Plan, x: f32, y: f32) {
        let mut y = y;
        for part in &plan.parts {
            for line in &part.lines {
                y = self.draw_line(part, line, x, y);
            }
        }
    }

    /// Draws one line, and returns the y below it.
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
                // The padding is symmetric, but it may not push the capsule past
                // the glyphs' own ink: a `0.2em` of vertical padding on a
                // 12px code run is 2.4px, and a tall glyph has room for it.
                let pad_y = run.style.pad_y;
                // `left` is the glyphs' start and `advance` their width, the
                // margin being neither — so the capsule is the code and its own
                // padding, and the gap the text beside it gets is still there.
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

            // The pictures of the line, in the same pass as the glyphs so that a
            // badge between two words lands between them rather than in a layer
            // of its own.
            for (index, run) in plan.metrics.runs.iter().enumerate() {
                if !run.is_placed_image(self.images) {
                    continue;
                }
                let url = run.image.as_deref().unwrap_or_default();
                let measured = &plan.metrics.measured[index];
                if measured.width <= 0.0 {
                    continue;
                }
                // The guard, not the segment: a run that did not land on this line
                // has no position here, and `advance_of_before` would answer zero
                // for it and put the picture at the start of the line.
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
                // A linked image has no text to trim and no clusters to add up, so
                // `trim_segment` reports nothing for it — and without a box of its
                // own a badge would draw and not be clickable. Its advance is its
                // measured width, which is the bitmap's.
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
                // One item: the transparent box that takes the press, carrying
                // where its rule belongs so that a view does not have to guess.
                // The rule sits under the descender, which is where a browser
                // puts `text-decoration: underline`, and `underline_offset` is
                // the font's own answer to that question.
                let mut hit = MdItem::rect(left, top, advance, box_height);
                hit.color = ColorRole::Link;
                // Never a fill: this is a hit area and the rule's anchor, and the
                // glyphs under it are already drawn in the link colour.
                hit.filled = false;
                hit.link_id = id;
                hit.link_url = url;
                // The rule sits under the descender, which is where a browser puts
                // `text-decoration: underline`, and `underline_offset` — measured
                // from the baseline *downwards* — is the font's own answer to
                // where that is. The box's top is the baseline less the ascent, so
                // the distance down from the box's top is the ascent plus the
                // offset. `underline_offset - descent` would be negative for every
                // font and clamp to zero, landing the rule on the box's top edge —
                // a line through the tops of the letters rather than under them.
                //
                // A font with no underline position in its metrics reports zero,
                // and a rule *on* the baseline touches the letters instead of
                // sitting under them, so a zero falls back to what a browser uses
                // in the same case: a small fraction of the em.
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

    /// The `pre` box: the code in the code font, on the code block's background
    /// fill, inside `code_block_padding` of padding.
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

    /// A table. Column widths come from the cells' own widths, the way
    /// `width: max-content` does; a table that is still too wide shares the
    /// shortfall between its columns and wraps inside them.
    fn table(&mut self, head: &[Cell], rows: &[Vec<Cell>], x: f32, y: f32, avail: f32) -> f32 {
        let style = self.style;
        let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
        if columns == 0 {
            return 0.0;
        }

        // Measure every cell once and keep it: the natural-width pass and the
        // wrapping pass both need it, and a cell's runs are what the view ends
        // up drawing.
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
            // wrap inside it. A view that cannot scroll sideways gets this
            // instead; see the crate's README for why.
            let factor = avail / total;
            let floor = style.font_size * 2.0;
            for width in &mut column_widths {
                *width = (*width * factor).max(floor);
            }
            // Sharing the shortfall in proportion is how a *header* gets broken:
            // a column sized for its share of a long cell comes out narrower than
            // its own heading, and "State" wraps to "Stat" / "e" in the middle of
            // a row of one-word headings. A column cannot usefully be narrower
            // than its longest word, so each is raised to that, and the columns
            // with slack above their own longest word give up the difference. Only
            // a table whose every column is one long word comes out still too
            // wide, and then the words break, which is unavoidable.
            // The chrome counts: a column has to hold its longest word *and* the
            // padding around it, or the word breaks to make room for the padding.
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
                            // The columns that are already wider than their own
                            // longest word pay for the ones that are not, each by
                            // the same fraction of the slack it holds.
                            *width -= (*width - minimum) / slack * wanted;
                        }
                    }
                }
            }
        }

        // A column the fit above squeezed down can end up narrower than the
        // cell's own padding, and padding the text out of existence wraps it one
        // character to a line: a five-word cell becomes fifty lines tall. The
        // text is the point of a cell and the padding is decoration, so the text
        // keeps a character's width and the padding takes whatever is left of
        // the column. A column narrow enough to lose its padding entirely still
        // reads; one narrow enough to lose the text does not.
        let column_chrome: Vec<f32> = column_widths
            .iter()
            .map(|width| chrome.min((*width - style.font_size).max(0.0)))
            .collect();

        let mut y = y;
        for row in plans.iter_mut() {
            // The text is planned before anything is drawn, because the row's
            // height comes from its tallest cell and the fills go around it.
            let mut laid_out: Vec<(f32, Plan)> = Vec::with_capacity(row.len());
            // The content's height, before the cell's own padding. The row is
            // this plus that padding, so a cell's text is inside its cell: the
            // padding moves the content down, and a height that ignored it would
            // put the text half a line below the fill that is meant to be
            // behind it.
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
                // The cell's fill and border. With `border-collapse: collapse`
                // neighbours share an edge, and they share it exactly here: two
                // cells of the same width meet on the same pixel, so the border
                // is one pixel and not two.
                let mut background = MdItem::rect(cell_x, y, *column, row_height);
                background.border_width = style.cell_border;
                background.border_color = ColorRole::TableBorder;
                if cell.head {
                    background.color = ColorRole::TableHeadBackground;
                } else {
                    // A body cell has no background of its own: GitHub paints the
                    // `tr`, and the row here sits on the container, which has
                    // none. So a body cell is its outline and nothing else.
                    background.color = ColorRole::TableBorder;
                    background.filled = false;
                }
                self.push(background);
                // The column's alignment is the slack between the cell's box and
                // its content, so a right-aligned column reads as right-aligned
                // rather than as text that happens to be wide.
                let cell_chrome = column_chrome.get(index).copied().unwrap_or(chrome);
                let slack = (*column - cell_chrome - plan.widest).max(0.0);
                let offset = cell.align.align_factor() * slack;
                // The same chrome the plan was given, so a cell that gave up its
                // padding keeps its text at the inset it planned for.
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

    // ----- images and markers -----

    /// A block image: its own size, capped to the width of the line it is on.
    fn image(&mut self, url: &str, alt: &str, x: f32, y: f32, avail: f32) -> f32 {
        let Some(asset) = self.images.get(url) else {
            // Nothing fetched, or nothing fetchable: the alt text, which is what
            // a browser shows for an image that did not load.
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
        // `img { max-width: 100% }` and no height constraint, so a wide image
        // comes down to the line's width and keeps its proportions.
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

    /// `•` or `1.`, where a user agent's list marker would sit: a gap of
    /// `0.5em` to the left of the content, right-aligned against it, and
    /// centred on the first line of the item.
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

    /// A task list's checkbox: a box, and a tick in it when it is checked.
    fn task_marker(&mut self, checked: bool, x: f32, y: f32, content_x: f32) {
        let size = self.style.task_marker_size;
        let line = self.style.font_size * self.style.line_height;
        let top = y + (line - size) / 2.0;
        let left = (content_x - self.style.task_marker_gap - size).max(x);
        let mut box_item = MdItem::rect(left, top, size, size);
        box_item.radius = 3.0;
        box_item.border_width = 1.0;
        box_item.color = ColorRole::Marker;
        // An outline, never a fill. A filled box in the marker's colour with a
        // tick in the marker's colour is a tick you cannot see, which is what a
        // checked task looked like: two identical solid squares. A browser's
        // checkbox is an outline with a tick in it, and that is also the one shape
        // that reads in a theme of any colours.
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

/// The x offset of a run's slice of a line: the advance of everything before it.
/// Where the given run's glyphs start on this line.
///
/// The sum of the atoms before it, plus its own first atom's leading margin: that
/// margin is room in the line, so it is part of the advance, but it is *before* the
/// first glyph rather than inside it, and everything that draws a run wants the
/// glyphs' position rather than the margin's.
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

/// The natural width of a run set: every run's advance, plus the padding and
/// margin of the inline-code capsules.
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

/// The width of the widest single word in a cell, which is how narrow a column
/// can usefully be.
///
/// A browser lays a table's cells out at their *min-content* width when the
/// table has to be squeezed, and min-content is the widest unbreakable run. This
/// is that, summed over the runs' clusters: a word that the shaper split across
/// clusters still adds up, because a cluster boundary is not a place a line may
/// break, and one that falls inside a word is not a place at all.
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

/// Snaps to a 1/64th of a pixel: a hundredth of a pixel of drift is enough to
/// put a run a hair away from the box drawn behind it.
fn round(value: f32) -> f32 {
    (value * 64.0).round() / 64.0
}
