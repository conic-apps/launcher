# markdown

A Markdown and HTML renderer for Slint, as a layout engine. `comrak` parses,
`parley` measures, and the caller paints.

The split is not a preference. Slint 1.18's `Text` has no strikethrough, no
underline control and no text background, and its `line-height-factor` multiplies
the font's *natural* line height rather than the font size, so it is not the
`line-height` a stylesheet asks for; `StyledText`
underlines every link whether or not the pointer is over it; and `@markdown` is a
compile-time literal the parser reads out of the source, so it cannot take a
document that arrives at runtime. None of what a `.markdown-body` stylesheet asks
for is expressible in Slint's own text elements. So this crate positions every
run itself and hands the view a flat list of boxes — which is also what makes the
geometry testable without a window.

## Using it

The caller compiles `ui/*.slint`. A Slint struct's Rust type belongs to the
compilation that produced it, so a crate that compiled the component and an app
that imported the same file would hold two unrelated types with no way to move an
item between them.

```slint
import { MarkdownView, MdChunk } from "path/to/markdown/ui/markdown-view.slint";

MarkdownView {
    chunks: <[MdChunk]>;            // the runs, in document order
    section-open: <[bool]>;         // index-aligned with `chunks`
    content-height: 100px;          // the engine's open height
    resized(width) => { re-lay-the-document-out-for-this-width(width); }
}
```

```rust
let mut renderer = Renderer::with_collection(slint::fontique_011::shared_collection());
renderer.set_style(MdStyle::default().with_families("Comfortaa Nunito", "monospace"));
renderer.set_source(document, SourceFormat::Markdown);
renderer.set_width(the_view's_width);
let list = renderer.layout();
```

`layout()` returns a `DisplayList` of `MdItem`s. The `MdItem` here and the one in
`markdown-types.slint` are the same fields under the same names, so copying the
values across is a field-for-field transcription. A bitmap is the exception: it
cannot be an `MdItem`, because a `slint::Image` cannot be left empty in a struct
literal, so image items go into the separate `MarkdownImage` list instead. Both
lists share one coordinate space.

A `<details>` is not a third list: `DisplayList::chunks` is every run in document
order, and a run carries its `section` — an `Option<MdSection>`, `None` for a run
of ordinary blocks. See [Collapsible sections](#collapsible-sections).

## Collapsible sections

A `<details>` is the one piece of GitHub's markup that is a control rather than
text, and it is the one that does not fit a flat display list.

**The engine hands over runs, not a flat list with holes in it.** `DisplayList`
has `chunks`: a document split into the stretches between its sections' own
extents, each carrying `top`, `height` and a contiguous range of `items`. A
document with no `<details>` in it is one run of everything, which is what keeps
the common case on the path it was always on.

That is because **hiding a section means taking its height out of the flow**, and
every line below it has to move up by the same amount. A view that painted a cover
over the content and shrank the number it reported would reclaim no space at all:
the gap would still be there, and everything past it would be scrolled out of
reach. So the view *stacks* the runs in a `VerticalLayout` — a section's run is as
tall as its content allows and as short as its summary row alone, and animating
between the two moves everything below it for free. `SettingCollapse` is the same
idea with a `clip: true` box, and for the same reason. The `clip` is what crops a
closing run's own content against what it is still showing.

**A run's items carry `y` relative to the run**, not to the document. That is the
caller's copy, not the engine's: the document's coordinates stay absolute there,
and everything else in the crate is written against them.

`MdStyle::details_gap` is 8px, not the 1px `SettingCollapse` puts between its rows.
That is a seam *inside* one group, where the rows below carry on; a `<summary>` is
the end of a group and the beginning of another, and at 1px the first line of the
content sits against the row's own edge.

**The caller owns the open/closed state**, as `section-open`, index-aligned with
the runs. A width change re-lays the document out and rebuilds the runs, so state
the view held in a property would be gone the first time the window moved a pixel.
Seed each section from its own `open` — the `<details open>` attribute — so a
README's sections, which have none, start closed as they do on GitHub.

`MdSection::hidden` is the content **and the margin under it**, not the content
alone. The space under a closed section is still its own, so a run that shrank by
the content alone would still be a margin taller than its summary row, and the top
of the first hidden line would sit inside it.

The view draws a row per section: `palette.details-head` and its hover and active
states, and a chevron. The chevron is **the caller's**, handed over as `marker-icon`
— path data, a view box and a stroke width, which is what the launcher's `Icons`
holds and what `AppIcon` draws — because the crate has no icon set and a glyph from
the body family would be a box on `Comfortaa`, which has none of the triangles a
disclosure wants. An empty `marker-icon` draws a built-in path instead, which is
what keeps the crate usable with nothing but a palette.

The icon is `chevron-forward` and turns a **quarter** turn when the section opens.
A quarter turn is only honest because the icon set's `chevron-forward` and
`chevron-down` are the same glyph; hand it a different one and the two states are
two different pictures and the turn lies.

The row takes no `mouse-cursor: pointer`. That is the launcher's house rule rather
than an oversight — a desktop app, a control's own shape is what says it is one —
and a link is the documented exception, in `MdLink`.

`shown-height` is the document's height while a collapse runs, and a caller has to
size its scroll container from that rather than from `content-height` — the
engine's height is the open one.

Two things about the item loop that look like choices and are not: it is one loop
with a conditional per kind, because a run's *paint order* is the engine's and a
chip's background is emitted before the run of text it sits behind; and each item
is drawn straight onto its run, because a box of no size between the run and its
text is harmless until something clips, and then it takes the text with it.

## Three things that will bite you

**The font collection has to be Slint's.** `Renderer::new()` and
`fonts::system()` collect the platform's own fonts, which is right for a caller
that draws with platform fonts and wrong for one that embeds a family — an
embedded family is not in the system collection, so the engine would shape with a
fallback and its line breaks would not match the width the renderer gives the
same string. Pass `slint::fontique_011::shared_collection()`, and name the same
family in `MdStyle::font_family` that the view draws with. Two collections, or
one face measured and another drawn, and every line is the wrong length.

**The width is the view's, not the container's.** The engine lays out content;
any padding around it is the caller's business. Handing it the outer width is how
a paragraph runs off the right edge.

**The monospace family has to be a real family name on both sides.** `font-family`
is not CSS: it takes a name, and the generic names mean nothing to it, so a view
given `"monospace"` draws in whatever the window's default face is. The engine
resolves `GenericFamily::Monospace` and shapes with a real monospace font, so the
two halves measure and draw in different faces — the code comes out in the body
typeface *and* its advances are narrower than the box the engine reserved, which
shows as a band of empty space at the right of every inline-code capsule. Resolve
one name and hand it to both: `set_style(..with_families(body, mono))` and the
view's `mono-family`.

**The view reports its size from `init` *and* from `changed width`, and both are
load-bearing.** A `changed` handler does not fire for a bound property's first
value, only for changes after it — so a view reporting only from `changed` tells
the caller about every resize except the one that decides where every line
breaks, and a document laid out with neither stays unwrapped until the window
happens to be dragged. If you are copying `markdown-view.slint` rather than
importing it, keep both lines.

## What it does not do

- **An inline image needs its bitmap before it is a picture.** The run carries
  both the alternative text and the URL, so before the bitmap arrives it reads as
  the alternative and afterwards it is drawn — the same way a browser behaves, and
  a badge row is the one image a Modrinth README always has.
- **No text selection or copy.** The display list has shaped runs, not source
  spans, so mapping a pointer back to a selection range is not there.
- **No horizontal scrolling.** A table wider than its box is scaled to fit and
  its cells wrap, where a browser would scroll. A column is never narrowed past
  its longest word, so a heading does not break mid-word, and a cell that cannot
  fit its padding gives the padding up rather than its text. A scroll view that
  only ever scrolls one way was the trade.
- **Code blocks clip** by default, for the same reason; `MdStyle::code_block_wrap`
  wraps them instead. Neither scrolls sideways, which is what `pre { overflow:
  auto }` does in a browser.
- **An inline-code capsule keeps a half-space from the text beside it**
  (`MdStyle::code_margin_x`). Padding is *inside* the capsule and this is not, which
  is the whole of it: `code_padding_x` alone leaves the capsule drawn straight over
  the glyphs next to it, because nothing in the line's width was ever asked for the
  room. The margin is a `leading`/`trailing` pair on the run's first and last atom
  and is part of the atom's advance — the line has to make room for it — while
  `advance_of_before` and `trim_segment` both subtract it again, because everything
  that draws a run wants the glyphs' box and not the margin's.
- **A document is laid out as a whole.** Nothing streams. Changing any input
  re-lays it out, and the shaping cache makes the second pass cheap.

## Tests

```
cargo test -p markdown
```

`tests/layout.rs` asserts on geometry without a window: that every item sits
inside the width it was given, that a wrapped link gets a hit box per line
sharing one id, that a link's colour reaches the glyphs but its box is never a
fill, that a table's text stays inside the cell that holds it at every size and
width, and that a task list's tick is visible against its own box.
`tests/parse.rs` covers both parsers, including the HTML one on the awkward input
real descriptions contain.

`MdStyle`'s defaults are GitHub's `.markdown-body` at a 14px body — except the
`<details>` fields, which follow a launcher settings row — and most fields' doc
comments cite the rule they come from.

## Licence

GPL-3.0-only, to match the launcher. `comrak` is BSD-2-Clause, so a repository
that vendors this needs `BSD-2-Clause` in whatever licence allowlist it keeps.
