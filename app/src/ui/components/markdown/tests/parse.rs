// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The block tree, before any measurement: what the two front ends make of a
//! document. These tests are the ones that can run without a font collection,
//! which is why the layout tests live in `layout.rs` and need one.

use crate::ui::components::markdown::doc::{Align, Block, Inline, Inlines, plain_text};
use crate::ui::components::markdown::{SourceFormat, parse};

fn md(source: &str) -> Vec<Block> {
    parse::parse(source)
}

fn html(source: &str) -> Vec<Block> {
    parse::html::parse(source)
}

fn text_of(inlines: &[Inline]) -> String {
    plain_text(inlines)
}

#[test]
fn headings_keep_their_level_and_content() {
    let blocks = md("# One\n\n## Two\n\nSetext\n===\n");
    assert_eq!(
        blocks,
        vec![
            Block::Heading {
                level: 0,
                inlines: vec![Inline::Text("One".into())]
            },
            Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("Two".into())]
            },
            Block::Heading {
                level: 0,
                inlines: vec![Inline::Text("Setext".into())]
            },
        ]
    );
}

#[test]
fn the_inline_set_round_trips() {
    let blocks = md("a *b* **c** ~~d~~ `e` [f](g)\n");
    let Block::Paragraph { inlines } = &blocks[0] else {
        panic!("expected a paragraph, got {blocks:?}");
    };
    assert_eq!(
        inlines,
        &vec![
            Inline::Text("a ".into()),
            Inline::Emph(vec![Inline::Text("b".into())]),
            Inline::Text(" ".into()),
            Inline::Strong(vec![Inline::Text("c".into())]),
            Inline::Text(" ".into()),
            Inline::Strike(vec![Inline::Text("d".into())]),
            Inline::Text(" ".into()),
            Inline::Code("e".into()),
            Inline::Text(" ".into()),
            Inline::Link {
                url: "g".into(),
                title: String::new(),
                inlines: vec![Inline::Text("f".into())]
            },
        ]
    );
    assert_eq!(text_of(inlines), "a b c d e f");
}

#[test]
fn a_link_nests_its_own_styling() {
    let blocks = md("[**bold link**](https://example.com)\n");
    let Block::Paragraph { inlines } = &blocks[0] else {
        panic!("expected a paragraph");
    };
    assert_eq!(
        inlines,
        &vec![Inline::Link {
            url: "https://example.com".into(),
            title: String::new(),
            inlines: vec![Inline::Strong(vec![Inline::Text("bold link".into())])]
        }]
    );
}

#[test]
fn hard_and_soft_breaks_differ() {
    let blocks = md("one  \ntwo\nthree\nfour\n");
    let Block::Paragraph { inlines } = &blocks[0] else {
        panic!("expected a paragraph");
    };
    // Two trailing spaces is a hard break; a bare newline is a space, which is
    // what GitHub's renderer shows.
    assert!(inlines.contains(&Inline::HardBreak));
    assert_eq!(text_of(inlines), "one two three four");
}

#[test]
fn an_image_alone_is_a_block() {
    let blocks = md("![the logo](https://example.com/logo.png)\n");
    assert_eq!(
        blocks,
        vec![Block::Image {
            url: "https://example.com/logo.png".into(),
            title: String::new(),
            alt: "the logo".into()
        }]
    );
    // Inside a sentence it stays inline, because a paragraph with text in it is
    // a paragraph.
    let blocks = md("see ![the logo](logo.png) here\n");
    assert!(matches!(blocks[0], Block::Paragraph { .. }));
}

#[test]
fn a_table_keeps_its_alignment() {
    let blocks = md("| a | b | c |\n|:--|:-:|--:|\n| 1 | 2 | 3 |\n");
    let Block::Table { head, rows, align } = &blocks[0] else {
        panic!("expected a table, got {blocks:?}");
    };
    assert_eq!(align, &vec![Align::Left, Align::Center, Align::Right]);
    assert_eq!(head.len(), 3);
    assert!(head.iter().all(|cell| cell.head));
    assert_eq!(rows.len(), 1);
    assert_eq!(text_of(&rows[0][2].inlines), "3");
    assert_eq!(rows[0][0].align, Align::Left);
}

#[test]
fn a_list_knows_its_ordinal_and_its_tasks() {
    let blocks = md("3. three\n4. four\n\n- [x] done\n- [ ] todo\n");
    let first = &blocks[0];
    let Block::List {
        ordered,
        start,
        items,
        ..
    } = first
    else {
        panic!("expected a list, got {first:?}");
    };
    assert!(ordered);
    assert_eq!(*start, 3);
    assert_eq!(items.len(), 2);

    let second = &blocks[1];
    let Block::List { ordered, items, .. } = second else {
        panic!("expected a list, got {second:?}");
    };
    assert!(!ordered);
    assert_eq!(items[0].task, Some(true));
    assert_eq!(items[1].task, Some(false));
}

#[test]
fn a_tight_list_has_no_paragraphs_but_a_loose_one_does() {
    let tight = md("- one\n- two\n");
    let Block::List { items, tight, .. } = &tight[0] else {
        panic!("expected a list");
    };
    assert!(tight);
    for item in items {
        assert!(matches!(item.blocks[0], Block::Paragraph { .. }));
    }

    let loose = md("- one\n\n- two\n");
    let Block::List { tight, .. } = &loose[0] else {
        panic!("expected a list");
    };
    assert!(!tight);
}

#[test]
fn front_matter_is_dropped_and_raw_html_becomes_its_text() {
    let blocks = md("---\ntitle: x\n---\n\n<div>inside</div>\n");
    assert!(
        !blocks
            .iter()
            .any(|block| matches!(block, Block::RawText { text } if text.contains("title"))),
        "front matter should not be rendered: {blocks:?}"
    );
    assert!(blocks.iter().any(|block| matches!(
        block,
        Block::RawText { text } if text == "inside"
    )));
}

#[test]
fn footnotes_are_hoisted_and_numbered() {
    // comrak numbers a footnote by the order it is *referenced*, which is what
    // GitHub does, so the definitions being written in the other order changes
    // nothing.
    let blocks = md("text[^a] and[^b]\n\n[^b]: second\n[^a]: first\n");
    let footnote_refs: Vec<usize> = blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph { inlines } => Some(inlines),
            _ => None,
        })
        .flatten()
        .filter_map(|inline| match inline {
            Inline::FootnoteRef { number, .. } => Some(*number),
            _ => None,
        })
        .collect();
    assert_eq!(footnote_refs, vec![1, 2], "numbers follow the references");
    let definitions: Vec<&Block> = blocks
        .iter()
        .filter(|block| matches!(block, Block::Footnote { .. }))
        .collect();
    assert_eq!(definitions.len(), 2);
    assert!(matches!(definitions[0], Block::Footnote { name, .. } if name == "1"));
}

#[test]
fn a_script_block_is_its_text_and_never_its_markup() {
    let blocks = md("before\n\n<script>alert(1)</script>\n\nafter\n");
    // The tag itself is escaped away and only its text survives, so nothing the
    // layout pass sees could be mistaken for markup.
    assert!(blocks.iter().all(|block| match block {
        Block::RawText { text } => !text.contains('<'),
        _ => true,
    }));
}

#[test]
fn html_paragraphs_and_headings() {
    let blocks = html("<h2>Title</h2><p>One.</p><p>Two <b>bold</b>.</p>");
    assert_eq!(
        blocks[0],
        Block::Heading {
            level: 1,
            inlines: vec![Inline::Text("Title".into())]
        }
    );
    let Block::Paragraph { inlines } = &blocks[1] else {
        panic!("expected a paragraph, got {blocks:?}");
    };
    assert_eq!(text_of(inlines), "One.");
    let Block::Paragraph { inlines } = &blocks[2] else {
        panic!("expected a paragraph");
    };
    assert_eq!(
        inlines,
        &vec![
            Inline::Text("Two ".into()),
            Inline::Strong(vec![Inline::Text("bold".into())]),
            Inline::Text(".".into()),
        ]
    );
}

#[test]
fn html_inline_styles_nest() {
    let blocks = html("<p><em>a <strong>b</strong> c</em></p>");
    let Block::Paragraph { inlines } = &blocks[0] else {
        panic!("expected a paragraph");
    };
    assert_eq!(
        inlines,
        &vec![Inline::Emph(vec![
            Inline::Text("a ".into()),
            Inline::Strong(vec![Inline::Text("b".into())]),
            Inline::Text(" c".into()),
        ])]
    );
}

#[test]
fn html_lists_and_quotes() {
    let blocks = html("<ul><li>one</li><li>two</li></ul><blockquote><p>quoted</p></blockquote>");
    let Block::List { items, ordered, .. } = &blocks[0] else {
        panic!("expected a list, got {blocks:?}");
    };
    assert!(!ordered);
    assert_eq!(items.len(), 2);
    assert_eq!(
        text_of(match &items[1].blocks[0] {
            Block::Paragraph { inlines } => inlines,
            other => panic!("expected a paragraph, got {other:?}"),
        }),
        "two"
    );
    let Block::Quote { blocks } = &blocks[1] else {
        panic!("expected a quote");
    };
    assert!(matches!(blocks[0], Block::Paragraph { .. }));
}

#[test]
fn html_tables_and_code() {
    let blocks = html(
        "<table><tr><th align=\"right\">h</th></tr><tr><td>c</td></tr></table><pre><code>fn a();</code></pre>",
    );
    let Block::Table { head, rows, align } = &blocks[0] else {
        panic!("expected a table, got {blocks:?}");
    };
    assert!(head[0].head);
    assert_eq!(head[0].align, Align::Right);
    assert_eq!(align[0], Align::Right);
    assert_eq!(rows.len(), 1);
    let Block::CodeBlock { code, .. } = &blocks[1] else {
        panic!("expected a code block, got {blocks:?}");
    };
    assert_eq!(code, "fn a();");
}

#[test]
fn html_entities_and_self_closing_tags() {
    let blocks = html("<p>a &amp; b &#8212; c &mdash; d</p><p>one<br/>two</p><p>x<sup>2</sup></p>");
    let text = |index: usize| match &blocks[index] {
        Block::Paragraph { inlines } => text_of(inlines),
        other => panic!("expected a paragraph, got {other:?}"),
    };
    assert_eq!(text(0), "a & b — c — d");
    // A hard break flattens to a space; the layout turns it back into a break.
    assert_eq!(text(1), "one two");
    assert!(matches!(blocks[2], Block::Paragraph { .. }));
}

#[test]
fn html_drops_a_script_with_its_contents() {
    let blocks = html("<p>keep</p><script>drop()</script><style>p{}</style><p>keep too</p>");
    let rendered: Vec<String> = blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph { inlines } => Some(text_of(inlines)),
            _ => None,
        })
        .collect();
    assert_eq!(rendered, vec!["keep", "keep too"]);
}

#[test]
fn html_links_and_images() {
    let blocks = html(
        "<p><a href=\"https://a.example\">label</a></p><p><img src=\"i.png\" alt=\"pic\"></p>",
    );
    let Block::Paragraph { inlines } = &blocks[0] else {
        panic!("expected a paragraph");
    };
    assert_eq!(
        inlines,
        &vec![Inline::Link {
            url: "https://a.example".into(),
            title: String::new(),
            inlines: vec![Inline::Text("label".into())]
        }]
    );
    assert!(matches!(
        blocks[1],
        Block::Image { ref url, .. } if url == "i.png"
    ));
}

#[test]
fn an_image_link_keeps_the_image_inside_the_link() {
    let blocks = html("<p><a href=\"https://a.example\"><img src=\"i.png\" alt=\"pic\"></a></p>");
    let Block::Paragraph { inlines } = &blocks[0] else {
        panic!("expected a paragraph, got {blocks:?}");
    };
    // The link wraps the image, so the alt text the layout draws in its place
    // is both the link's label and clickable — which is the closest a renderer
    // without inline bitmaps can get to GitHub's clickable image.
    assert_eq!(
        inlines,
        &vec![Inline::Link {
            url: "https://a.example".into(),
            title: String::new(),
            inlines: vec![Inline::Image {
                url: "i.png".into(),
                title: String::new(),
                alt: "pic".into()
            }]
        }]
    );
}

#[test]
fn an_unknown_tag_is_transparent() {
    let blocks = html("<p><span>a</span><font color=\"red\">b</font></p><custom><p>c</p></custom>");
    assert_eq!(
        text_of(match &blocks[0] {
            Block::Paragraph { inlines } => inlines,
            other => panic!("expected a paragraph, got {other:?}"),
        }),
        "ab"
    );
    assert_eq!(
        text_of(match &blocks[1] {
            Block::Paragraph { inlines } => inlines,
            other => panic!("expected a paragraph, got {other:?}"),
        }),
        "c"
    );
}

#[test]
fn both_front_ends_produce_the_same_shape() {
    let from_markdown = md("- one\n- two\n");
    let from_html = html("<ul><li>one</li><li>two</li></ul>");
    let shape = |blocks: &[Block]| {
        blocks
            .iter()
            .map(|block| match block {
                Block::List { items, .. } => format!("list:{}", items.len()),
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(shape(&from_markdown), shape(&from_html));
}

#[test]
fn an_empty_document_is_no_blocks() {
    assert!(md("").is_empty());
    assert!(html("").is_empty());
    assert!(html("<!-- just a comment -->").is_empty());
}

#[test]
fn a_source_format_round_trips_through_the_renderer() {
    let mut renderer = crate::ui::components::markdown::Renderer::new();
    renderer.set_source("**x**", SourceFormat::Markdown);
    assert_eq!(renderer.blocks().len(), 1);
    renderer.set_source("<b>y</b>", SourceFormat::Html);
    assert_eq!(renderer.blocks().len(), 1);
    let inlines: &Inlines = match &renderer.blocks()[0] {
        Block::Paragraph { inlines } => inlines,
        other => panic!("expected a paragraph, got {other:?}"),
    };
    assert_eq!(text_of(inlines), "y");
}

#[test]
fn a_details_keeps_its_summary_and_the_markdown_under_it() {
    // The shape a README actually writes: the tags on their own lines, the
    // content between them separated by blank lines, so comrak ends a raw HTML
    // block at each blank line and the tags arrive as siblings of the content.
    let blocks = md(
        "<details>\n<summary>General Settings</summary>\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n\nA note.\n\n</details>\n",
    );
    let Block::Details {
        open,
        summary,
        blocks: inner,
    } = &blocks[0]
    else {
        panic!("expected a details block, got {blocks:?}");
    };
    assert!(!open, "a details with no open attribute is closed");
    assert_eq!(text_of(summary), "General Settings");
    // The content is comrak's, not the HTML parser's: a GFM table inside a
    // details is still a table.
    assert!(matches!(inner[0], Block::Table { .. }), "{inner:?}");
    assert!(matches!(inner[1], Block::Paragraph { .. }), "{inner:?}");
}

#[test]
fn the_open_attribute_is_what_opens_a_details() {
    let closed = md("<details>\n<summary>a</summary>\n\nb\n\n</details>\n");
    let open = md("<details open>\n<summary>a</summary>\n\nb\n\n</details>\n");
    let flagged = md("<details open=\"open\">\n<summary>a</summary>\n\nb\n\n</details>\n");
    for blocks in [&closed, &open, &flagged] {
        assert!(matches!(blocks[0], Block::Details { .. }), "{blocks:?}");
    }
    let is_open = |blocks: &Vec<Block>| match &blocks[0] {
        Block::Details { open, .. } => *open,
        other => panic!("expected a details block, got {other:?}"),
    };
    assert!(!is_open(&closed));
    assert!(is_open(&open));
    assert!(is_open(&flagged), "`open=\"open\"` is the attribute too");
}

#[test]
fn a_details_without_a_summary_keeps_its_content_and_no_label() {
    // There is no label to click, so there is no control. The block still comes
    // through as a `Details` — the source did say `<details>` — but with an empty
    // summary, which is what tells the layout not to make a section of it. See
    // `a_details_with_no_summary_is_not_a_section` in the layout tests.
    let blocks = md("<details>\n\nA note.\n\n</details>\n");
    let Block::Details {
        summary, blocks, ..
    } = &blocks[0]
    else {
        panic!("expected a details block, got {blocks:?}");
    };
    assert!(summary.is_empty(), "nothing to label the control with");
    assert!(matches!(blocks[0], Block::Paragraph { .. }), "{blocks:?}");
}

#[test]
fn a_nested_details_closes_into_its_parent() {
    let blocks = md(
        "<details>\n<summary>outer</summary>\n\ntext\n\n<details>\n<summary>inner</summary>\n\nmore\n\n</details>\n\nafter\n\n</details>\n",
    );
    let Block::Details {
        summary,
        blocks: outer,
        ..
    } = &blocks[0]
    else {
        panic!("expected a details block, got {blocks:?}");
    };
    assert_eq!(text_of(summary), "outer");
    let inner_block = outer
        .iter()
        .find(|b| matches!(b, Block::Details { .. }))
        .expect("the nested section");
    let Block::Details {
        summary, blocks, ..
    } = inner_block
    else {
        panic!("expected a nested details block");
    };
    assert_eq!(text_of(summary), "inner");
    // The text after the inner close belongs to the outer section, not to the
    // inner one: that is what a stack of them has to get right.
    assert!(
        outer.iter().any(|b| matches!(b, Block::Paragraph { .. })),
        "{outer:?}"
    );
    assert_eq!(blocks.len(), 1, "the inner section holds only its own text");
}

#[test]
fn a_details_in_markdown_and_in_html_agree() {
    // The same document through the two front ends: the HTML one has no comrak
    // to split it at blank lines, but the section it produces has to be the same
    // section, or a caller that renders either gets a different page.
    let from_markdown = md("<details>\n<summary>label</summary>\n\nbody\n\n</details>\n");
    let from_html = html("<details><summary>label</summary><p>body</p></details>");
    let shape = |blocks: &[Block]| match &blocks[0] {
        Block::Details { open, summary, .. } => (*open, text_of(summary)),
        other => panic!("expected a details block, got {other:?}"),
    };
    assert_eq!(shape(&from_markdown), shape(&from_html));
}
