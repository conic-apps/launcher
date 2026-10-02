// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The display list: geometry, line breaking and the item kinds.
//!
//! These need a font collection, so they take the system's. They do not need
//! *a particular* one: every assertion here is either about how the geometry
//! relates to itself (a line is inside the box, a link's rule is under its text,
//! nothing overlaps) or about a count (three lines, two rows), both of which hold
//! whichever font the machine has.

use markdown::{ColorRole, ItemKind, MdItem, MdStyle, Measurer, Renderer, SourceFormat};

const WIDTH: f32 = 600.0;

/// A renderer with the default style over a document, laid out for [`WIDTH`].
fn render(source: &str) -> Vec<MdItem> {
    render_with(source, MdStyle::default())
}

fn render_with(source: &str, style: MdStyle) -> Vec<MdItem> {
    let mut measurer = Measurer::new();
    let blocks = markdown::parse::parse(source);
    let images = markdown::ImageStore::default();
    markdown::layout::layout(&blocks, &style, WIDTH, &images, &mut measurer).items
}

fn texts(items: &[MdItem]) -> Vec<&str> {
    items
        .iter()
        .filter(|item| item.kind == ItemKind::Text)
        .map(|item| item.text.as_str())
        .collect()
}

fn rects(items: &[MdItem]) -> Vec<&MdItem> {
    items
        .iter()
        .filter(|item| item.kind == ItemKind::Rect)
        .collect()
}

// ---------------------------------------------------------------------------
// paragraphs and line breaking
// ---------------------------------------------------------------------------

#[test]
fn a_short_paragraph_is_one_line_inside_the_width() {
    let items = render("A short sentence.\n");
    assert_eq!(texts(&items), vec!["A short sentence."]);
    let text = &items[0];
    assert!(text.x >= 0.0 && text.x + text.width <= WIDTH, "{text:?}");
    assert!(text.height > 0.0);
}

#[test]
fn a_long_paragraph_wraps_onto_several_lines() {
    let paragraph = "The quick brown fox jumps over the lazy dog. ".repeat(12);
    let items = render(&paragraph);
    let lines = texts(&items);
    assert!(lines.len() > 1, "expected a wrap, got one line");
    for line in &items {
        assert!(
            line.x + line.width <= WIDTH + 1.0,
            "a line overflows the width: {:?}",
            line
        );
    }
    // A line that fits is filled: greedy filling means every line but the last
    // comes close to the width.
    let widest = items
        .iter()
        .map(|item| item.x + item.width)
        .fold(0.0f32, f32::max);
    assert!(
        widest > WIDTH * 0.8,
        "the fill is too loose: {widest} of {WIDTH}"
    );
}

#[test]
fn a_narrower_box_wraps_earlier() {
    let paragraph = "The quick brown fox jumps over the lazy dog. ".repeat(12);
    let wide = render(&paragraph).len();
    let mut measurer = Measurer::new();
    let blocks = markdown::parse::parse(&paragraph);
    let images = markdown::ImageStore::default();
    let narrow =
        markdown::layout::layout(&blocks, &MdStyle::default(), 200.0, &images, &mut measurer)
            .items
            .len();
    assert!(
        narrow > wide,
        "narrower should not use fewer items: {narrow} vs {wide}"
    );
}

#[test]
fn a_word_longer_than_the_line_is_broken() {
    // `word-wrap: break-word`, which is what the launcher's container sets.
    let items = render(&format!("{}end\n", "x".repeat(900)));
    let lines = texts(&items);
    assert!(lines.len() > 1, "an unbreakable word should still be split");
    for item in &items {
        assert!(item.x + item.width <= WIDTH + 1.0, "{item:?}");
    }
}

#[test]
fn a_hard_break_ends_the_line() {
    let items = render("one  \ntwo\n");
    assert_eq!(texts(&items), vec!["one", "two"]);
    assert!(items[1].y >= items[0].y + items[0].height - 0.5);
}

#[test]
fn runs_of_one_line_share_a_baseline() {
    // An inline code span is 85% of the body size, so its box is shorter; the
    // glyphs still have to sit on the same baseline as the text beside them.
    let items = render("a `b` c\n");
    let runs: Vec<&MdItem> = items
        .iter()
        .filter(|item| item.kind == ItemKind::Text)
        .collect();
    assert_eq!(runs.len(), 3, "text, code, text: {runs:?}");
    let baseline = |item: &MdItem| item.y + item.height;
    assert!(
        (baseline(runs[0]) - baseline(runs[1])).abs() < 0.6
            && (baseline(runs[1]) - baseline(runs[2])).abs() < 0.6,
        "baselines should agree: {:?}",
        runs.iter()
            .map(|run| (run.text.as_str(), run.y, run.height))
            .collect::<Vec<_>>()
    );
    // And the code run is narrower than the same text at body size.
    assert!(runs[1].font_size < runs[0].font_size);
}

// ---------------------------------------------------------------------------
// decorations
// ---------------------------------------------------------------------------

#[test]
fn inline_code_gets_a_capsule_behind_it() {
    let items = render("a `b` c\n");
    let capsule = rects(&items)
        .into_iter()
        .find(|item| item.color == ColorRole::CodeBackground)
        .expect("no capsule");
    let glyphs = &items
        .iter()
        .find(|item| item.kind == ItemKind::Text && item.mono)
        .expect("no code run")
        .clone();
    // The capsule covers the glyphs, with the style's padding on each side.
    assert!(capsule.x < glyphs.x);
    assert!(capsule.x + capsule.width > glyphs.x + glyphs.width);
    assert!(capsule.y < glyphs.y && capsule.y + capsule.height > glyphs.y + glyphs.height);
    assert!(capsule.radius > 0.0);
}

#[test]
fn a_struck_run_gets_a_rule_across_it() {
    let items = render("a ~~gone~~ b\n");
    let strike = rects(&items)
        .into_iter()
        .find(|item| item.color == ColorRole::Strike)
        .expect("no strikethrough");
    let run = items
        .iter()
        .find(|item| item.kind == ItemKind::Text && item.text.contains("gone"))
        .expect("no struck run");
    assert!(
        (strike.x - run.x).abs() < 0.6,
        "the rule should start at the text"
    );
    assert!(
        (strike.width - run.width).abs() < 0.6,
        "and be as wide as it"
    );
    assert!(strike.height > 0.0);
}

#[test]
fn a_link_gets_a_hit_area_and_a_rule_for_each_of_its_lines() {
    let label = "a link that is long enough to wrap onto a second line in a narrow box";
    let style = MdStyle {
        font_size: 28.0,
        line_height: 1.2,
        ..MdStyle::default()
    };
    let items = render_with(&format!("[{label}](https://example.com)\n"), style);
    let hit: Vec<&MdItem> = rects(&items)
        .into_iter()
        .filter(|item| !item.link_url.is_empty())
        .collect();
    assert!(
        hit.len() >= 2,
        "a wrapped link needs a box per line: {hit:?}"
    );
    // Every piece belongs to the same link, which is what lets a view reveal the
    // rule under all of the link's lines from one piece of pointer state.
    assert!(hit.iter().all(|item| item.link_id == hit[0].link_id));
    assert!(
        hit.iter()
            .all(|item| item.link_url == "https://example.com")
    );
    // Each box carries where its own rule goes, and is never a fill of its own:
    // the glyphs under it are already drawn in the link's colour.
    assert!(hit.iter().all(|item| !item.filled));
    assert!(hit.iter().all(|item| item.rule_offset >= 0.0));
    assert!(hit.iter().all(|item| item.rule_offset <= item.height));
    // They are on different lines, one below the other.
    for pair in hit.windows(2) {
        assert!(pair[1].y >= pair[0].y, "the pieces stack");
    }
}

#[test]
fn every_link_is_its_own_id_and_nothing_else_has_one() {
    let items = render("[one](https://one.example) mid [two](https://two.example)\n");
    let text: Vec<&MdItem> = items
        .iter()
        .filter(|item| item.kind == ItemKind::Text)
        .collect();
    let one = text
        .iter()
        .find(|item| item.text == "one")
        .expect("no first link");
    let two = text
        .iter()
        .find(|item| item.text == "two")
        .expect("no second link");
    let mid = text
        .iter()
        .find(|item| item.text.contains("mid"))
        .expect("no text between the links");
    // The plain run between the links is not part of either of them, so it must
    // carry no id. An id that leaked onto it would light up an underline under
    // text the reader never hovered.
    assert!(mid.link_id.is_empty());
    assert!(mid.link_url.is_empty());
    // The two links are distinguishable, and the ids match their hit boxes: a
    // view decides which rule to draw by comparing the two.
    assert!(!one.link_id.is_empty());
    assert!(!two.link_id.is_empty());
    assert_ne!(one.link_id, two.link_id);
    for item in &items {
        if item.link_id == one.link_id {
            assert_eq!(item.link_url, "https://one.example");
        }
    }
}

#[test]
fn a_link_is_drawn_in_the_link_colour_and_surrounding_text_is_not() {
    let items = render("[label](https://example.com) and more\n");
    let link = items
        .iter()
        .find(|item| item.kind == ItemKind::Text && item.text == "label")
        .expect("no link label");
    let other = items
        .iter()
        .find(|item| item.kind == ItemKind::Text && item.text.contains("more"))
        .expect("no trailing text");
    assert_eq!(link.color, ColorRole::Link);
    assert_eq!(other.color, ColorRole::Text);
}

#[test]
fn strong_and_emphasis_reach_the_item() {
    let items = render("**b** *i* ~~s~~\n");
    let of = |needle: &str| {
        items
            .iter()
            .find(|item| item.text == needle)
            .unwrap_or_else(|| panic!("no run {needle:?} in {:?}", texts(&items)))
    };
    assert!(of("b").font_weight > of("i").font_weight);
    assert!(of("i").font_italic);
    assert!(!of("b").font_italic);
    assert!(
        rects(&items)
            .iter()
            .any(|item| item.color == ColorRole::Strike)
    );
}

// ---------------------------------------------------------------------------
// block structure
// ---------------------------------------------------------------------------

#[test]
fn a_heading_is_larger_and_heavier_than_body_text() {
    let items = render("# Title\n\nbody\n");
    let heading = items.iter().find(|item| item.text == "Title").unwrap();
    let body = items.iter().find(|item| item.text == "body").unwrap();
    assert!(heading.font_size > body.font_size * 1.5);
    assert!(heading.font_weight > body.font_weight);
    assert_eq!(heading.color, ColorRole::Heading);
}

#[test]
fn h1_and_h2_carry_a_rule_underneath_and_h3_does_not() {
    let items = render("# One\n\n## Two\n\n### Three\n");
    let rules = rects(&items)
        .into_iter()
        .filter(|item| item.color == ColorRole::Rule)
        .collect::<Vec<_>>();
    assert_eq!(rules.len(), 2, "only h1 and h2 are underlined");
    // The rule sits below the heading it belongs to and above the next one.
    let three = items.iter().find(|item| item.text == "Three").unwrap();
    let two_rule = rules[1];
    assert!(two_rule.y < three.y);
    assert!(two_rule.y + two_rule.height <= three.y);
}

#[test]
fn a_code_block_is_a_filled_box_around_monospace_lines() {
    let code = "fn main() {\n    println!(\"hi\");\n}\n";
    let items = render(&format!("```rust\n{code}```\n"));
    let box_item = rects(&items)
        .into_iter()
        .find(|item| item.color == ColorRole::CodeBlockBackground)
        .expect("no code block background");
    assert!(box_item.radius > 0.0);
    let lines: Vec<&MdItem> = items
        .iter()
        .filter(|item| item.kind == ItemKind::Text && item.mono)
        .collect();
    assert_eq!(lines.len(), 3, "one item per source line");
    assert!(lines.iter().all(|line| line.mono));
    for line in &lines {
        assert!(line.y > box_item.y && line.y + line.height < box_item.y + box_item.height);
    }
    // The first line starts below the padding, not on the edge of the box.
    assert!(lines[0].y - box_item.y >= 8.0);
}

#[test]
fn a_block_quote_is_bordered_on_the_left_and_dimmed() {
    let items = render("> quoted\n");
    let bar = rects(&items)
        .into_iter()
        .find(|item| item.color == ColorRole::QuoteBar)
        .expect("no quote bar");
    let text = items.iter().find(|item| item.text == "quoted").unwrap();
    assert!(text.x >= bar.x + bar.width, "the text is inside the bar");
    assert_eq!(text.color, ColorRole::QuoteText);
    // The bar spans the quoted content.
    assert!(bar.y <= text.y && bar.y + bar.height >= text.y + text.height - 0.5);
}

#[test]
fn a_thematic_break_is_a_thin_wide_box() {
    let items = render("a\n\n---\n\nb\n");
    let rule = rects(&items)
        .into_iter()
        .find(|item| item.color == ColorRole::Rule)
        .expect("no rule");
    assert!(rule.width > WIDTH * 0.9, "a rule spans the column");
    assert!(rule.height > 0.0 && rule.height < 8.0);
}

#[test]
fn a_list_numbers_its_items_and_indents_them() {
    let items = render("3. three\n4. four\n");
    let marker_three = items
        .iter()
        .find(|item| item.text == "3.")
        .expect("no marker");
    let marker_four = items
        .iter()
        .find(|item| item.text == "4.")
        .expect("no marker");
    let body = items.iter().find(|item| item.text == "three").unwrap();
    // The marker sits to the left of the item's own text, and the list starts at
    // the ordinal it was given.
    assert!(marker_three.x + marker_three.width <= body.x);
    assert!(marker_three.y < body.y + body.height);
    assert!(marker_four.y > marker_three.y, "the items are stacked");
}

#[test]
fn a_bullet_list_uses_bullets() {
    let items = render("- one\n- two\n");
    assert!(items.iter().any(|item| item.text == "\u{2022}"));
}

#[test]
fn a_task_list_draws_a_box_per_item_and_a_tick_in_the_checked_ones() {
    let items = render("- [x] done\n- [ ] todo\n");
    let boxes = rects(&items)
        .into_iter()
        .filter(|item| item.border_width > 0.0 && item.color == ColorRole::Marker)
        .collect::<Vec<_>>();
    assert_eq!(boxes.len(), 2, "one checkbox per item");
    assert_eq!(boxes[0].width, boxes[0].height, "a checkbox is square");
    assert!(items.iter().any(|item| item.text == "\u{2713}"), "one tick");
    assert!(items.iter().any(|item| item.text == "done"));
}

#[test]
fn a_nested_list_indents_further() {
    let items = render("- outer\n    - inner\n");
    let outer = items.iter().find(|item| item.text == "outer").unwrap();
    let inner = items.iter().find(|item| item.text == "inner").unwrap();
    assert!(inner.x > outer.x, "the nested item is further right");
}

#[test]
fn a_table_fills_its_header_and_its_body_cells() {
    let items = render("| name | count |\n| --- | ---: |\n| a | 1 |\n| b | 2 |\n");
    let heads = rects(&items)
        .into_iter()
        .filter(|item| item.color == ColorRole::TableHeadBackground)
        .collect::<Vec<_>>();
    assert_eq!(heads.len(), 2, "one header cell per column");
    assert!(heads.iter().all(|cell| cell.border_width > 0.0));
    // A body cell is an outline and nothing else, so it is not filled.
    let body = rects(&items)
        .into_iter()
        .find(|item| !item.filled && item.border_width > 0.0)
        .expect("no body cell");
    assert!(body.height > 0.0);
    // A right-aligned column puts its text against the right edge of the cell.
    let right_column = heads
        .iter()
        .max_by(|a, b| a.x.partial_cmp(&b.x).unwrap())
        .unwrap();
    let count = items.iter().find(|item| item.text == "1").unwrap();
    assert!(
        count.x > right_column.x,
        "the right-aligned cell is indented inside its column"
    );
}

#[test]
fn a_wide_table_stays_inside_the_box() {
    let rows = (0..8)
        .map(|index| format!("| a rather long cell number {index} | b |"))
        .collect::<Vec<_>>()
        .join("\n");
    let items = render(&format!("| head one | head two |\n| --- | --- |\n{rows}\n"));
    for item in &items {
        assert!(
            item.x + item.width <= WIDTH + 1.0,
            "a table that is too wide shares the shortfall instead: {item:?}"
        );
    }
}

/// The empty band between a table and whatever follows it, in a document whose
/// content before the table is `filler` paragraphs long.
fn gap_after_table(filler: usize) -> f32 {
    let mut source = (0..filler)
        .map(|_| "filler")
        .collect::<Vec<_>>()
        .join("\n\n");
    source.push_str("\n\n| head one | head two |\n| --- | --- |\n| a | b |\n\nafter\n");
    let items = render(&source);
    let last_cell = *rects(&items)
        .iter()
        .filter(|item| item.border_width > 0.0)
        .max_by(|a, b| a.y.partial_cmp(&b.y).unwrap())
        .expect("the table's cells");
    let after = items
        .iter()
        .find(|item| item.text == "after")
        .expect("the paragraph");
    after.y - (last_cell.y + last_cell.height)
}

#[test]
fn a_block_after_a_table_follows_the_table() {
    let gap = gap_after_table(3);
    let style = MdStyle::default();
    assert!(
        gap < style.table_margin_bottom + style.font_size,
        "a block after a table sits under its last row, separated by the margin: {gap}"
    );
}

#[test]
fn the_gap_after_a_table_does_not_grow_with_what_comes_before_it() {
    // `table` returns the bottom of its last row as an absolute y, so a block
    // that adds that y a second time shifts everything after the table down by
    // the whole offset, and every later table compounds it. The gap is a
    // property of the table, not of its position in the document.
    let short = gap_after_table(0);
    let long = gap_after_table(20);
    assert!(
        (short - long).abs() < 1.0,
        "the gap is the same wherever the table sits: {short} against {long}"
    );
}

#[test]
fn a_footnote_definition_ends_the_document() {
    let items = render("text[^1]\n\n[^1]: the note\n");
    let note = items.iter().find(|item| item.text.contains("the note"));
    let reference = items.iter().find(|item| item.text == "[1]");
    assert!(note.is_some() && reference.is_some());
    if let (Some(note), Some(reference)) = (note, reference) {
        assert!(note.y > reference.y, "the definition comes after the text");
    }
}

// ---------------------------------------------------------------------------
// the whole list
// ---------------------------------------------------------------------------

#[test]
fn the_list_starts_at_the_top_left_and_never_goes_back_up() {
    let items = render("# Title\n\nA paragraph of a few words.\n\n- one\n- two\n");
    for item in &items {
        assert!(item.x >= -0.01, "{item:?} starts left of the origin");
        assert!(item.height > 0.0, "{item:?} has no height");
    }
    // Two items may share a band vertically as long as they are side by side —
    // a list marker and the item's first line do exactly that — so the
    // invariant is only checked where the two would also collide horizontally.
    for (index, later) in items.iter().enumerate() {
        for earlier in &items[..index] {
            let shares_x = earlier.x < later.x + later.width - 0.6
                && later.x < earlier.x + earlier.width - 0.6;
            if shares_x {
                assert!(
                    later.y >= earlier.y - 0.6,
                    "{later:?} is drawn over {earlier:?}"
                );
            }
        }
    }
}

#[test]
fn every_item_is_inside_the_width_it_was_given() {
    let source = "# A heading with quite a lot of words in it\n\n\
                  A paragraph long enough to wrap more than once, with a `code span` in it, \
                  a [link](https://example.com) and some ~~struck~~ words.\n\n\
                  | a | b |\n| --- | --- |\n| 1 | 2 |\n\n\
                  > a quote\n\n```\nlet x = 1;\n```\n\n\
                  - one\n- two\n  - nested\n\n---\n";
    let items = render(source);
    for item in &items {
        assert!(
            item.x + item.width <= WIDTH + 1.0,
            "{item:?} overflows the width"
        );
        assert!(item.height > 0.0, "{item:?} has no height");
    }
    assert!(
        items.len() > 30,
        "the document should produce plenty of items"
    );
}

#[test]
fn a_document_relaid_out_at_the_same_width_does_the_same_thing() {
    let mut renderer = Renderer::new();
    renderer.set_source("# Title\n\nSome words.\n", SourceFormat::Markdown);
    renderer.set_width(WIDTH);
    let first = renderer.layout().items.clone();
    assert!(!renderer.set_width(WIDTH), "the width did not change");
    assert!(!renderer.is_dirty());
    assert_eq!(renderer.layout().items.len(), first.len());
    assert!(
        !renderer.set_width(WIDTH + 0.1),
        "a tenth of a pixel is not a resize"
    );
    assert!(renderer.set_width(WIDTH + 4.0), "four pixels is");
    assert!(renderer.is_dirty());
}

#[test]
fn the_list_never_renders_before_it_is_asked_for() {
    let mut renderer = Renderer::new();
    renderer.set_source("a", SourceFormat::Markdown);
    renderer.set_width(WIDTH);
    assert!(renderer.is_dirty());
    renderer.layout();
    assert!(!renderer.is_dirty());
    renderer.set_style(MdStyle::default());
    assert!(!renderer.is_dirty(), "the same style is not a change");
    renderer.set_style(MdStyle {
        font_size: 20.0,
        ..MdStyle::default()
    });
    assert!(renderer.is_dirty());
}

#[test]
fn a_table_cells_hold_their_text_inside_them() {
    // A cell's padding moves its content down, so the row's height has to be the
    // content's height plus that padding. A row that forgot leaves the fill short:
    // the last line of a cell hangs off the bottom of it onto the page, and the top
    // of the fill is left as a band of nothing above the first line. The same goes
    // sideways when a cell is squeezed narrower than its own padding: it starts its
    // text outside itself, in whatever width is left over, which is how a word
    // comes to need a line to itself.
    let source = "| Feature | State |\n| --- | :---: |\n\
        | **Markdown** | done |\n| `hover` underline | a longer line of text |\n";
    for font_size in [4.0, 8.0, 14.0, 28.0] {
        for width in [40.0, 60.0, 120.0, 200.0, 400.0] {
            let style = MdStyle {
                font_size,
                ..MdStyle::default()
            };
            let mut measurer = Measurer::new();
            let blocks = markdown::parse::parse(source);
            let images = markdown::ImageStore::default();
            let items =
                markdown::layout::layout(&blocks, &style, width, &images, &mut measurer).items;
            let cells: Vec<&MdItem> = rects(&items)
                .iter()
                .copied()
                .filter(|item| {
                    matches!(
                        item.color,
                        ColorRole::TableBorder | ColorRole::TableHeadBackground
                    )
                })
                .collect();
            assert!(!cells.is_empty(), "no cells at {font_size}pt in {width}px");
            for cell in &cells {
                let inside: Vec<&MdItem> = items
                    .iter()
                    .filter(|item| item.kind == ItemKind::Text)
                    .filter(|item| {
                        item.x >= cell.x
                            && item.x < cell.x + cell.width
                            && item.y >= cell.y
                            && item.y < cell.y + cell.height
                    })
                    .collect();
                let sideways: Vec<&MdItem> = inside
                    .iter()
                    .copied()
                    .filter(|item| item.x + item.width > cell.x + cell.width + 0.01)
                    .collect();
                assert!(
                    sideways.is_empty(),
                    "at {font_size}pt in {width}px, {:?} runs out of the side of the cell that \
                     holds it",
                    sideways.first().map(|item| item.text.as_str())
                );
                let below: Vec<&MdItem> = inside
                    .iter()
                    .copied()
                    .filter(|item| item.y + item.height > cell.y + cell.height + 0.01)
                    .collect();
                assert!(
                    below.is_empty(),
                    "at {font_size}pt in {width}px, {:?} runs out of the bottom of the cell that \
                     holds it",
                    below.first().map(|item| item.text.as_str())
                );
            }
        }
    }
}

#[test]
fn a_task_marker_is_an_outline_with_a_tick_in_it() {
    let items = render("- [x] done\n- [ ] not\n");
    let boxes: Vec<&MdItem> = rects(&items)
        .into_iter()
        .filter(|item| item.color == ColorRole::Marker)
        .collect();
    assert_eq!(boxes.len(), 2, "one box per task");
    // An outline, in both states: a filled box in the marker's colour carries a
    // tick in the marker's colour, which is a tick nobody can see.
    for box_item in &boxes {
        assert!(
            !box_item.filled,
            "a task box is filled, so its tick is invisible"
        );
        assert!(box_item.border_width > 0.0, "a task box has no outline");
    }
    // And the checked one has a tick the unchecked one does not, which is the
    // only thing that tells them apart.
    let ticks: Vec<&MdItem> = items
        .iter()
        .filter(|item| item.kind == ItemKind::Text && item.text.contains('\u{2713}'))
        .collect();
    assert_eq!(ticks.len(), 1, "exactly one of the two is ticked");
    let tick = ticks[0];
    let ticked_box = boxes
        .iter()
        .find(|item| {
            tick.x >= item.x
                && tick.x < item.x + item.width
                && tick.y >= item.y
                && tick.y < item.y + item.height
        })
        .expect("the tick is not inside either box");
    assert_eq!(ticked_box.color, tick.color, "the tick matches its box");
}

#[test]
fn a_squeezed_table_never_narrows_a_column_past_its_longest_word() {
    // The share a column gets of a table's shortfall is proportional to what its
    // cells need, so a column of one-word headings next to a column of long text
    // comes out narrower than its own heading and breaks it mid-word. A column
    // that cannot hold its longest word is not a column.
    let source = "| Feature | State | Notes |\n| --- | --- | --- |\n\
        | a cell with a great deal of text in it | done | and more of it |\n";
    let style = MdStyle::default();
    let mut measurer = Measurer::new();
    let blocks = markdown::parse::parse(source);
    let images = markdown::ImageStore::default();
    // A box narrow enough that the table has to be squeezed.
    let items = markdown::layout::layout(&blocks, &style, 240.0, &images, &mut measurer).items;
    // Every word in the head is on one line: a text item is a run, and a broken
    // word is two of them.
    for word in ["Feature", "State", "Notes"] {
        let hits = items
            .iter()
            .filter(|item| item.kind == ItemKind::Text && item.text == word)
            .count();
        assert_eq!(hits, 1, "{word} was broken across lines");
    }
    // And the table still fits the box it was given, which is the whole reason
    // the columns were narrowed at all.
    for item in rects(&items) {
        assert!(
            item.x + item.width <= 240.0 + 0.01,
            "{item:?} is outside the box"
        );
    }
}

#[test]
fn a_link_rule_sits_below_the_text_and_above_the_box_top() {
    // The rule must land below the middle of the glyphs' box. Placing it at
    // `underline_offset - descent` is negative for every font and clamps to the
    // box's top edge — a line through the tops of the letters, not under them.
    let items = render("[label](https://example.com)\n");
    let link = items
        .iter()
        .find(|item| item.kind == ItemKind::Rect && !item.link_url.is_empty())
        .expect("no link box");
    let glyphs = items
        .iter()
        .find(|item| item.kind == ItemKind::Text && item.text == "label")
        .expect("no link label");
    let rule_y = link.y + link.rule_offset;
    let top = glyphs.y;
    let bottom = glyphs.y + glyphs.height;
    // Below the middle of the glyphs' own box. The box is the run's ascent plus
    // its descent, and the baseline sits at its bottom third, so "under the
    // letters" is anywhere below the middle. The bug this is here for put the
    // rule at the *top* of the box, which is above the middle every time.
    assert!(
        rule_y > (top + bottom) / 2.0,
        "the rule is at {rule_y}, in the top half of the box {top}..{bottom}"
    );
    // And inside the link's own hit box, so a view that clips to the item still
    // shows it.
    assert!(rule_y < bottom + 0.5, "the rule fell out of its own box");
    assert!(rule_y >= top, "the rule is above its own box");
}

// ---------------------------------------------------------------------------
// inline images
// ---------------------------------------------------------------------------

/// An image store holding one bitmap of the given size, at `url`.
fn store_with(url: &str, width: u32, height: u32) -> markdown::ImageStore {
    let mut store = markdown::ImageStore::default();
    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(width, height);
    store.insert(
        url,
        markdown::ImageAsset {
            image: slint::Image::from_rgba8(buffer),
            size: (width, height),
        },
    );
    store
}

fn render_with_images(source: &str, images: &markdown::ImageStore) -> Vec<MdItem> {
    let style = MdStyle::default();
    let mut measurer = Measurer::new();
    let blocks = markdown::parse::parse(source);
    markdown::layout::layout(&blocks, &style, WIDTH, images, &mut measurer).items
}

#[test]
fn an_inline_image_with_a_bitmap_is_drawn_as_a_picture_and_not_as_its_alt_text() {
    let url = "https://example.com/badge.png";
    let items = render_with_images(
        "text before [![Badge](https://example.com/badge.png)](https://example.com) text after\n",
        &store_with(url, 120, 40),
    );
    let picture = items
        .iter()
        .find(|item| item.kind == ItemKind::Image)
        .expect("the badge is not drawn as a picture");
    assert_eq!(picture.width, 120.0, "the bitmap's own width");
    assert_eq!(picture.height, 40.0, "and its own height");
    assert!(
        picture.image.is_some(),
        "the picture carries no bitmap, so a view draws nothing"
    );
    // The alt text is what the run falls back to, not what it shows: a README
    // full of badges must not read as a wall of alternative text.
    assert!(
        !texts(&items).iter().any(|text| text.contains("Badge")),
        "the alt text was drawn as well as the picture"
    );
    // And it is clickable, because it is a link.
    assert_eq!(picture.link_url, "https://example.com");
    assert!(!picture.link_id.is_empty());
}

#[test]
fn an_inline_image_without_a_bitmap_reads_as_its_alt_text() {
    // Nothing has been fetched yet, so the run is ordinary text and the document
    // is still readable — which is what a browser shows for an image that has not
    // loaded. When the bitmap arrives the same run becomes the picture.
    let items = render("text before [![Badge](https://example.com/badge.png)](x) after\n");
    assert!(
        !items.iter().any(|item| item.kind == ItemKind::Image),
        "an unfetched image was given a box"
    );
    assert!(
        texts(&items).iter().any(|text| text.contains("Badge")),
        "an unfetched image showed nothing at all"
    );
}

#[test]
fn a_linked_image_reveals_no_rule() {
    // `text-decoration` does not reach a replaced element, so a browser
    // underlines nothing under a linked picture. A view that drew the rule anyway
    // would put a line across every badge in a README.
    let url = "https://example.com/badge.png";
    let items = render_with_images(
        "[![Badge](https://example.com/badge.png)](https://example.com)\n",
        &store_with(url, 120, 40),
    );
    let link = items
        .iter()
        .find(|item| item.kind == ItemKind::Rect && !item.link_url.is_empty())
        .expect("the badge is not clickable");
    assert!(!link.rule, "a linked image has a rule under it");
    // A link that is *text* still has one, or the other half of the rule is gone.
    let text_link = render("text [label](https://example.com)\n");
    let text_link = text_link
        .iter()
        .find(|item| item.kind == ItemKind::Rect && !item.link_url.is_empty())
        .expect("no text link");
    assert!(text_link.rule, "a text link has no rule");
}

#[test]
fn a_row_of_badges_wraps_like_a_row_of_words() {
    // The reason a line may start and end either side of an image: a badge row is
    // the single most common image in a Modrinth README, and a row that cannot
    // wrap runs off the edge of the panel.
    let mut store = markdown::ImageStore::default();
    let source = (0..8)
        .map(|index| {
            let url = format!("https://example.com/badge{index}.png");
            store.insert(
                &url,
                markdown::ImageAsset {
                    image: slint::Image::from_rgba8(
                        slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(120, 40),
                    ),
                    size: (120, 40),
                },
            );
            format!("[![b]({url})](x)")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let items = render_with_images(&format!("{source}\n"), &store);
    let pictures = items
        .iter()
        .filter(|item| item.kind == ItemKind::Image)
        .count();
    assert_eq!(pictures, 8, "every badge is drawn");
    // Every badge is inside the box it was given, which can only hold if the row
    // wrapped.
    for item in items.iter().filter(|item| item.kind == ItemKind::Image) {
        assert!(
            item.x + item.width <= WIDTH + 0.01,
            "a badge runs off the right edge"
        );
    }
}

#[test]
fn an_image_has_the_same_below_it_as_a_paragraph_does() {
    // A browser needs no rule for this: a lone `img` sits in a paragraph, and the
    // paragraph's `margin-bottom` is the gap. Here the image *is* the block, so
    // without a margin of its own the next line starts against the picture.
    let style = MdStyle::default();
    let mut measurer = Measurer::new();
    let blocks = markdown::parse::parse("![a](https://example.com/i.png)\n\nafter\n");
    let tight = {
        let mut style = style.clone();
        style.image_margin_bottom = 0.0;
        markdown::layout::layout(
            &blocks,
            &style,
            WIDTH,
            &markdown::ImageStore::default(),
            &mut measurer,
        )
        .height
    };
    let loose = {
        let mut measurer = Measurer::new();
        markdown::layout::layout(
            &blocks,
            &style,
            WIDTH,
            &markdown::ImageStore::default(),
            &mut measurer,
        )
        .height
    };
    assert!(
        loose - tight >= style.paragraph_margin_bottom - 0.5,
        "the gap below the image is {} and the margin is {}",
        loose - tight,
        style.paragraph_margin_bottom
    );
}

// ---------------------------------------------------------------------------
// <details>
// ---------------------------------------------------------------------------

/// A renderer over a document, keeping the sections as well as the items.
fn display(source: &str) -> markdown::DisplayList {
    let mut measurer = Measurer::new();
    let blocks = markdown::parse::parse(source);
    let images = markdown::ImageStore::default();
    markdown::layout::layout(&blocks, &MdStyle::default(), WIDTH, &images, &mut measurer)
}

/// The `<details>` sections of a laid-out document, in document order.
///
/// A section is reached through the run that holds it, because the run is what a
/// view stacks and what actually moves; this is the same list with the runs taken
/// out, for the tests that are about the section rather than about the flow.
fn sections(list: &markdown::DisplayList) -> Vec<&markdown::model::MdSection> {
    list.chunks
        .iter()
        .filter_map(|chunk| chunk.section.as_ref())
        .collect()
}

/// The runs, in order, as `(top, height, is_section, first_item, item_count)`.
fn runs(list: &markdown::DisplayList) -> Vec<(f32, f32, bool, usize, usize)> {
    list.chunks
        .iter()
        .map(|chunk| {
            (
                chunk.top,
                chunk.height,
                chunk.section.is_some(),
                chunk.items.start,
                chunk.items.len(),
            )
        })
        .collect()
}

const ONE_SECTION: &str =
    "<details>\n<summary>General Settings</summary>\n\nA note.\n\n</details>\n";

#[test]
fn a_details_lays_its_content_out_as_if_it_were_open() {
    // The engine never re-measures to close a section: a view that had to ask
    // would get a jump instead of an animation, and one that hid the content by
    // re-laying out would watch every line below it shift. So the document is
    // laid out open and the view covers what is hidden — which is only possible
    // if the height the engine reports is the open one.
    let list = display(ONE_SECTION);
    let found = sections(&list);
    assert_eq!(found.len(), 1);
    let section = found[0];
    let note = list
        .items
        .iter()
        .find(|item| item.text == "A note.")
        .expect("the content is laid out");
    assert!(
        note.y >= section.content_y,
        "the note is inside the section"
    );
    assert!(
        list.height >= section.content_y + section.content_height,
        "the document is as tall as the open section: {} against {}",
        list.height,
        section.content_y + section.content_height
    );
}

#[test]
fn a_details_with_no_summary_is_not_a_section() {
    // Nothing to click means nothing to collapse: the content is laid out where
    // it stands rather than behind a row nobody can operate.
    let list = display("<details>\n\nA note.\n\n</details>\n");
    assert!(sections(&list).is_empty(), "{:?}", list.chunks);
    assert!(
        list.items.iter().any(|item| item.text == "A note."),
        "and the content is still there"
    );
}

#[test]
fn a_summary_row_is_sized_like_a_settings_row() {
    let style = MdStyle::default();
    let list = display(ONE_SECTION);
    let found = sections(&list);
    let section = found[0];
    // The title's own line box between the row's padding, which is what a
    // settings row with a title and no description measures.
    let expected =
        style.details_head_font_size * style.line_height + 2.0 * style.details_head_padding_y;
    assert!(
        (section.head_height - expected).abs() < 0.01,
        "{} against {expected}",
        section.head_height
    );
    assert_eq!(section.head_radius, style.details_head_radius);
    assert_eq!(section.head_width, WIDTH, "the row is the document's width");
    // The chevron is centred in the row, which is what makes it read as the row's
    // control rather than as text that happens to be beside a triangle.
    let centred = section.head_y + (section.head_height - section.marker_size) / 2.0;
    assert!(
        (section.marker_y - centred).abs() < 0.01,
        "{}",
        section.marker_y
    );
}

#[test]
fn a_details_summary_is_set_in_like_a_row_title_and_indented_past_the_chevron() {
    let style = MdStyle::default();
    let list = display(ONE_SECTION);
    let found = sections(&list);
    let section = found[0];
    let title = list
        .items
        .iter()
        .find(|item| item.text == "General Settings")
        .expect("the summary is drawn");
    assert_eq!(title.font_size, style.details_head_font_size);
    assert_eq!(title.font_weight, style.weight_normal);
    let wanted = section.marker_x + style.details_marker_size + style.details_marker_gap;
    assert!(
        (title.x - wanted).abs() < 0.01,
        "the title starts past the chevron: {} against {wanted}",
        title.x
    );
}

#[test]
fn a_details_content_is_indented_under_the_summary_title() {
    let style = MdStyle::default();
    let list = display(ONE_SECTION);
    let found = sections(&list);
    let section = found[0];
    let note = list
        .items
        .iter()
        .find(|item| item.text == "A note.")
        .unwrap();
    assert!(
        (section.content_x - style.details_content_inset).abs() < 0.01,
        "{}",
        section.content_x
    );
    assert_eq!(
        note.x, section.content_x,
        "and the content is laid out there"
    );
    assert!(
        section.content_height > 0.0,
        "a section that hides something says how tall it is"
    );
}

#[test]
fn an_open_details_is_reported_open() {
    // The attribute is the only thing that opens one, and the view seeds its own
    // state from this — a document that says nothing is a closed section.
    let closed = display(ONE_SECTION);
    let open = display("<details open>\n<summary>a</summary>\n\nb\n\n</details>\n");
    assert!(!sections(&closed)[0].open);
    assert!(sections(&open)[0].open);
}

#[test]
fn every_section_has_its_own_id() {
    // A view holds its open/closed state against the id, so two sections that
    // share one would toggle together.
    let list = display(
        "<details>\n<summary>a</summary>\n\nx\n\n</details>\n\n<details>\n<summary>b</summary>\n\ny\n\n</details>\n",
    );
    let found = sections(&list);
    assert_eq!(found.len(), 2);
    assert_ne!(found[0].id, found[1].id);
    // And they are in document order, which is the order a view stacks them in.
    assert!(found[0].head_y < found[1].head_y);
}

#[test]
fn a_nested_details_gets_its_own_section_inside_its_parent() {
    let list = display(
        "<details>\n<summary>outer</summary>\n\n<details>\n<summary>inner</summary>\n\nx\n\n</details>\n\n</details>\n",
    );
    let found = sections(&list);
    assert_eq!(found.len(), 2);
    let (outer, inner) = (found[0], found[1]);
    assert!(
        inner.head_y > outer.head_y,
        "the inner one is below the outer"
    );
    assert!(
        inner.head_y < outer.content_y + outer.content_height,
        "and inside it, which is what lets a view hide the outer without losing track of the inner"
    );
}

#[test]
fn the_runs_tile_the_document() {
    // Every item is in exactly one run, and the runs are contiguous. A gap would
    // drop a line; an overlap would draw one twice and leave a hole where the
    // stack should be.
    for source in [
        "# Title\n\ntext\n",
        ONE_SECTION,
        "<details>\n<summary>a</summary>\n\nx\n\n</details>\n\nbetween\n\n<details>\n<summary>b</summary>\n\ny\n\n</details>\n\nafter\n",
        "<details>\n<summary>outer</summary>\n\n<details>\n<summary>inner</summary>\n\nz\n\n</details>\n\n</details>\n",
    ] {
        let list = display(source);
        let mut next = 0usize;
        for chunk in &list.chunks {
            assert_eq!(chunk.items.start, next, "runs are contiguous: {source:?}");
            next = chunk.items.end;
        }
        assert_eq!(next, list.items.len(), "every item is in a run: {source:?}");
        assert!(!list.chunks.is_empty(), "a document is at least one run");
    }
}

#[test]
fn a_document_with_no_details_is_one_run() {
    // The common case — a long README with no `<details>` — has to stay on the
    // path it was on, and a run that is one thing is what keeps the view's inner
    // loop worth having.
    let list = display("# Title\n\ntext\n\nmore\n");
    assert_eq!(list.chunks.len(), 1);
    assert!(list.chunks[0].section.is_none());
    assert_eq!(list.chunks[0].items, 0..list.items.len());
    assert_eq!(list.chunks[0].top, 0.0);
}

#[test]
fn a_run_ends_where_the_next_one_begins() {
    // The runs are stacked by height, so a run that does not add up to where the
    // next one starts leaves a gap the view cannot close: a section's own margin
    // has to be inside the run above it, not between the two.
    // Something before the section, so there is a run on either side of it.
    let list = display(&format!("intro\n\n{ONE_SECTION}"));
    let [first, second] = &list.chunks[..] else {
        panic!(
            "a document with a section has a run for it and one either side: {:?}",
            runs(&list)
        );
    };
    assert!(first.section.is_none() && second.section.is_some());
    assert_eq!(
        first.top + first.height,
        second.top,
        "the run before the section ends where it starts"
    );
    // The document's height is rounded and the run's is not, so this one is a
    // comparison rather than an identity.
    assert!(
        (second.top + second.height - list.height).abs() < 0.01,
        "and the section's own margin is inside its run: {} against {}",
        second.top + second.height,
        list.height
    );
}

#[test]
fn a_run_hides_its_section_whole() {
    // What a closed run takes away is its content *and* the margin under it. A run
    // that shrank by the content alone would still be a margin taller than its
    // summary row, and the top of the first hidden line would sit inside it.
    let style = MdStyle::default();
    let list = display(ONE_SECTION);
    let section = sections(&list)[0];
    let run = list
        .chunks
        .iter()
        .find(|chunk| chunk.section.is_some())
        .unwrap();
    assert_eq!(
        section.hidden,
        section.content_height + style.details_margin_bottom,
        "the margin under a closed section is still there"
    );
    // Which leaves the row and the gap, and nothing else.
    assert_eq!(
        run.height - section.hidden,
        section.head_height + style.details_gap,
        "a closed run is its row and the gap under it"
    );
    // And the content starts below that, so nothing of it is left showing.
    assert!(section.content_y - run.top >= run.height - section.hidden);
}

#[test]
fn a_sections_items_are_all_in_its_own_run() {
    // The summary's text and everything under it move as one, so they have to be
    // in one run: a run that ended between them would leave the content in a run
    // that a closed section no longer moved.
    let list = display(
        "<details>\n<summary>label</summary>\n\nfirst\n\n| a |\n| --- |\n| 1 |\n\nlast\n\n</details>\n",
    );
    let run = list.chunks.iter().find(|c| c.section.is_some()).unwrap();
    let texts: Vec<&str> = list.items[run.items.clone()]
        .iter()
        .map(|item| item.text.as_str())
        .collect();
    assert!(texts.contains(&"label"), "{texts:?}");
    assert!(texts.contains(&"first"), "{texts:?}");
    assert!(texts.contains(&"last"), "{texts:?}");
    assert!(
        list.items[run.items.clone()]
            .iter()
            .any(|item| item.kind == ItemKind::Rect),
        "and the table's cells with them: {texts:?}"
    );
}

#[test]
fn an_inline_code_capsule_keeps_a_gap_from_the_text_beside_it() {
    // The capsule's padding is inside it and its margin is not, so a run of code
    // with text hard against it is a capsule drawn over the glyphs next to it: the
    // padding never claimed any room in the line, and nothing else did either.
    let style = MdStyle::default();
    let gap = style.font_size * style.code_font_scale * style.code_margin_x;
    let items = render("plain `code` plain");
    let code = items
        .iter()
        .find(|item| item.text == "code")
        .expect("the code");
    let after = items
        .iter()
        .find(|item| item.text == " plain")
        .expect("the text after it");
    assert!(
        after.x - (code.x + code.width) >= gap - 0.5,
        "half a space between the capsule and the text: {} against {gap}",
        after.x - (code.x + code.width)
    );
    let before = items
        .iter()
        .find(|item| item.text == "plain ")
        .expect("the text before");
    assert!(
        code.x - (before.x + before.width) >= gap - 0.5,
        "and on the other side: {} against {gap}",
        code.x - (before.x + before.width)
    );
}

#[test]
fn the_capsule_is_the_code_alone_and_not_its_margin() {
    // The margin is the gap the text beside it gets, and drawing the capsule over
    // its own margin would eat exactly that.
    let style = MdStyle::default();
    let pad = style.font_size * style.code_font_scale * style.code_padding_x;
    let items = render("plain `code` plain");
    let code = items
        .iter()
        .find(|item| item.text == "code")
        .expect("the code");
    let capsule = rects(&items)
        .into_iter()
        .find(|item| item.color == ColorRole::CodeBackground)
        .expect("the capsule");
    let wanted = code.width + pad * 2.0;
    assert!(
        (capsule.width - wanted).abs() < 0.5,
        "the capsule is the code and its padding, {} against {wanted}",
        capsule.width
    );
}

#[test]
fn the_line_itself_is_wider_by_the_capsules_margins() {
    // The margin has to be room in the *line*, not just in the drawing, or a
    // paragraph laid out to exactly the box's width overflows by however many
    // capsules it holds — and overflows it by however much the last one's margin
    // happens to be, which is the part nobody would notice.
    let style = MdStyle::default();
    let margins = style.font_size * style.code_font_scale * style.code_margin_x;
    let bare = render("`a`");
    let boxed = render("`a`");
    let width_of = |items: &[MdItem]| {
        items
            .iter()
            .filter(|item| item.kind == ItemKind::Text)
            .map(|item| item.x + item.width)
            .fold(0.0f32, f32::max)
    };
    assert!(
        width_of(&bare) - width_of(&boxed) < margins,
        "one capsule either way, so the boxes agree: {} against {}",
        width_of(&bare),
        width_of(&boxed)
    );
    // Two capsules on one line, side by side, are `2 * margin` further right than
    // the second one's glyphs would put it.
    let two = render("`a` `b`");
    let second = two
        .iter()
        .filter(|item| item.kind == ItemKind::Text)
        .find(|item| item.text == "b")
        .expect("the second capsule");
    let first = two
        .iter()
        .filter(|item| item.kind == ItemKind::Text)
        .find(|item| item.text == "a")
        .expect("the first");
    let first_margin = style.font_size * style.code_font_scale * style.code_margin_x;
    assert!(
        second.x - (first.x + first.width) >= first_margin + margins - 0.5,
        "the first capsule's trailing margin and the second's leading one: {} against {}",
        second.x - (first.x + first.width),
        first_margin + margins
    );
}
