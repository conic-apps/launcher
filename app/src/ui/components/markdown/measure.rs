// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Text measurement, through the engine the view draws with.
//!
//! The view hands every run of a line to a `Text` element, so the width of that
//! run is the width `Text` will report — and a line is only as wide as the sum
//! of the runs on it. Measuring a run therefore has to use *the same engine, the
//! same font collection and the same family fallback chain* the renderer uses,
//! or the two drift apart and lines overflow or under-fill. That is why this
//! shapes with `parley` against
//! `slint::fontique_011::shared_collection` rather than with a font parser of
//! its own: it is the collection Slint registered its embedded families and its
//! script fallbacks into.
//!
//! A run is shaped on its own, with no wrapping, and the caller breaks the line
//! itself. That is deliberate: a paragraph is drawn as one `Text` per style per
//! line, so shaping a whole paragraph at once and then splitting it would
//! measure ligatures and kerning pairs that straddle a run boundary and that
//! the view, which never draws across a boundary, would never render.

use std::borrow::Cow;

use fontique::{Collection, GenericFamily, SourceCache, SourceCacheOptions};
use parley::style::{
    FontFamily, FontFamilyName, FontStyle, FontWeight, OverflowWrap, TextStyle, WhiteSpaceCollapse,
    WordBreak,
};
use parley::{FontContext, Layout, LayoutContext};

/// parley's brush type parameter. The renderer measures and positions, it never
/// paints, so the brush carries nothing — and `()` satisfies parley's `Brush`
/// blanket impl.
type Brush = ();

/// The generic families Slint appends behind a `Text`'s own family, in the order
/// `i-slint-common::sharedfontique::FALLBACK_FAMILIES` lists them, which puts
/// `SansSerif` first for FemtoVG.
const FALLBACK_FAMILIES: [GenericFamily; 2] = [GenericFamily::SansSerif, GenericFamily::SystemUi];

/// The same, for a run set in the monospace face. CSS resolves
/// `font-family: monospace` to the platform's monospace face, and the launcher's
/// stylesheet asks for exactly that (`.markdown-body code { font-family:
/// monospace }`), so the generic has to be in the chain: without it the name
/// resolves to nothing and the run falls through to the sans-serif face, which
/// renders code in the body typeface and measures it at the wrong advances.
const MONO_FALLBACK_FAMILIES: [GenericFamily; 3] = [
    GenericFamily::Monospace,
    GenericFamily::SansSerif,
    GenericFamily::SystemUi,
];

/// What a run is set in. Only the properties that change a run's metrics are
/// here; colour and decoration live on the display-list item instead, because
/// they do not affect where anything lands.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanStyle {
    /// The family, with the generic fallbacks appended. An empty string shapes
    /// with the fallbacks alone, which is parley's own default.
    pub family: String,
    /// Whether this run is set in the monospace face, which is what
    /// `font-family: monospace` asks for. It changes the *fallback chain* and
    /// nothing else: the family above is still tried first, so a caller that
    /// names a real monospace face gets that face, and a caller that leaves the
    /// generic name — as GitHub's `.markdown-body` does — gets the platform's.
    pub mono: bool,
    /// The font size, in logical pixels.
    pub size: f32,
    /// The `font-weight`, 100 to 900.
    pub weight: u16,
    /// Whether the run is slanted.
    pub italic: bool,
}

/// One shaped cluster: a group of characters that cannot be broken apart, with
/// the space it takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cluster {
    /// Byte range of the cluster within the measured run.
    pub start: usize,
    /// One past the last byte of the cluster.
    pub end: usize,
    /// Its advance, in logical pixels.
    pub advance: f32,
}

/// Everything a line needs to know about one run: where the line breaks can go,
/// how wide the run is, and where its decorations belong.
#[derive(Debug, Clone, Default)]
pub struct SpanMetrics {
    /// The text the shaper actually laid out, which — because every run here
    /// asks the shaper to preserve whitespace — is the text that went in. The
    /// clusters index into this string, and the display list slices it rather
    /// than the input.
    pub text: String,
    /// The clusters, in reading order. Their advances add up to `width`.
    pub clusters: Vec<Cluster>,
    /// The run's total advance.
    pub width: f32,
    /// The run's own ascent, descent and leading, in logical pixels.
    pub ascent: f32,
    /// Typographic descent, in logical pixels.
    pub descent: f32,
    /// The font's own leading, which a run with a taller neighbour does not
    /// contribute to the line box.
    pub leading: f32,
    /// Distance from the top of the run's box to its baseline.
    pub baseline: f32,
    /// Where a strikethrough sits, measured from the baseline and upwards. The
    /// layout draws it as a box, because Slint's `Text` cannot.
    pub strike_offset: f32,
    /// The rule's thickness, in logical pixels.
    pub strike_size: f32,
    /// Where an underline sits, measured from the baseline and downwards.
    pub underline_offset: f32,
    /// The rule's thickness, in logical pixels.
    pub underline_size: f32,
}

/// Shapes runs and hands back their metrics.
///
/// The context caches shaping results, so a document is measured quickly the
/// second time — which is what makes a window resize cheap.
pub struct Measurer {
    layout_ctx: LayoutContext<Brush>,
    font_ctx: FontContext,
    scale: f32,
}

impl Default for Measurer {
    fn default() -> Self {
        Self::new()
    }
}

impl Measurer {
    /// Creates a measurer over a collection of the system's own fonts.
    ///
    /// Correct for a caller that renders its text with the platform's fonts.
    /// A caller that embeds a family — as every Slint app that ships a typeface
    /// does — must use [`Self::with_collection`] instead, or every advance will
    /// be the wrong width and every line will break in the wrong place.
    pub fn new() -> Self {
        Self::with_collection(crate::ui::components::markdown::fonts::system())
    }

    /// Creates a measurer over the caller's own font collection.
    ///
    /// A Slint app passes the one the renderer draws with:
    /// `slint::fontique_011::shared_collection()`, which holds the families
    /// embedded in the UI *and* the script fallbacks an application registered
    /// with it.
    pub fn with_collection(collection: Collection) -> Self {
        // `shared: true` so a second measurer does not load the same font files
        // twice.
        let source_cache = SourceCache::new(SourceCacheOptions { shared: true });
        Self {
            layout_ctx: LayoutContext::new(),
            font_ctx: FontContext {
                collection,
                source_cache,
            },
            scale: 1.0,
        }
    }

    /// Sets the device scale factor the measurement is taken at.
    ///
    /// Slint shapes at the window's scale factor, and parley rounds hinting
    /// results to it, so a caller that wants the advances to agree with the
    /// renderer to the last sub-pixel should pass `window.scale_factor()` here
    /// and re-layout when it changes. At the default of 1 the difference is well
    /// under a pixel per run.
    pub fn set_scale(&mut self, scale: f32) {
        if scale > 0.0 {
            self.scale = scale;
        }
    }

    /// The scale factor the measurement is taken at.
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Shapes one run.
    pub fn measure(&mut self, text: &str, style: &SpanStyle) -> SpanMetrics {
        if text.is_empty() {
            return SpanMetrics::default();
        }
        let (layout, laid_out) = self.shape(text, style);
        let mut metrics = SpanMetrics {
            text: laid_out,
            width: layout.width(),
            ..Default::default()
        };
        // With no wrapping there is exactly one line, and — a single style, a
        // single script direction — one run per font that had to cover part of
        // it. The clusters of all of them are in visual order, which for the
        // left-to-right documents this renderer takes is also logical order.
        for line in layout.lines() {
            let line_metrics = *line.metrics();
            metrics.ascent = metrics.ascent.max(line_metrics.ascent);
            metrics.descent = metrics.descent.max(line_metrics.descent);
            metrics.leading = metrics.leading.max(line_metrics.leading);
            metrics.baseline = line_metrics.baseline;
            for run in line.runs() {
                let run_metrics = *run.metrics();
                metrics.strike_offset = run_metrics.strikethrough_offset;
                metrics.strike_size = run_metrics.strikethrough_size;
                metrics.underline_offset = run_metrics.underline_offset;
                metrics.underline_size = run_metrics.underline_size;
                for cluster in run.clusters() {
                    // A cluster's range is already absolute — it carries its
                    // run's own start — so it addresses `text` directly.
                    let range = cluster.text_range();
                    metrics.clusters.push(Cluster {
                        start: range.start,
                        end: range.end,
                        advance: cluster.advance(),
                    });
                }
            }
        }
        metrics.clusters.sort_by_key(|cluster| cluster.start);
        metrics
    }

    /// Shapes a run and hands back both the layout and the text it addressed.
    fn shape(&mut self, text: &str, style: &SpanStyle) -> (Layout<Brush>, String) {
        let family = family_list(&style.family, style.mono);
        let mut text_style = TextStyle {
            font_family: family,
            font_size: style.size,
            font_weight: FontWeight::new(style.weight as f32),
            font_style: if style.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            },
            // `word-wrap: break-word` on the container. The caller does the
            // breaking, but the flag is what makes parley keep a long
            // unbreakable run (a URL, a long hash) from being laid out as one
            // indivisible cluster.
            word_break: WordBreak::Normal,
            overflow_wrap: OverflowWrap::Anywhere,
            ..Default::default()
        };
        // A zero or negative size would make every advance zero and every
        // decoration land on the same pixel.
        if text_style.font_size <= 0.0 || text_style.font_size.is_nan() {
            text_style.font_size = 1.0;
        }
        let mut builder =
            self.layout_ctx
                .tree_builder(&mut self.font_ctx, self.scale, false, &text_style);
        // Whitespace is *never* touched here, and that is not a simplification:
        // every run is shaped on its own, so a shaper that collapsed would trim
        // the space off the end of a run and the two runs of a sentence would
        // come together. Collapsing is the caller's, and the layout pass does it
        // while it flattens, where the run boundaries are.
        builder.set_white_space_mode(WhiteSpaceCollapse::Preserve);
        builder.push_text(text);
        let (mut layout, laid_out) = builder.build();
        // No wrapping: the caller owns the line breaks, and asking parley for
        // them would wrap the run into lines whose advances were then summed
        // back into the single number it already reports.
        layout.break_all_lines(None);
        (layout, laid_out)
    }
}

/// `[family, SansSerif, SystemUi]`, or `[family, Monospace, SansSerif, SystemUi]`
/// for a monospace run; the generics alone when no family was given.
///
/// The name is borrowed rather than owned: parley's family list is a `Cow`, and
/// shaping only ever reads it.
fn family_list<'a>(family: &'a str, mono: bool) -> FontFamily<'a> {
    let generics: &[GenericFamily] = if mono {
        &MONO_FALLBACK_FAMILIES
    } else {
        &FALLBACK_FAMILIES
    };
    let fallback = generics.iter().copied().map(FontFamilyName::Generic);
    if family.is_empty() {
        return FontFamily::List(Cow::Owned(fallback.collect()));
    }
    let mut list = Vec::with_capacity(generics.len() + 1);
    list.push(FontFamilyName::named(family));
    list.extend(fallback);
    FontFamily::List(Cow::Owned(list))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_run_measures_to_nothing() {
        let mut measurer = Measurer::new();
        let metrics = measurer.measure(
            "",
            &SpanStyle {
                family: String::new(),
                mono: false,
                size: 14.0,
                weight: 400,
                italic: false,
            },
        );
        assert_eq!(metrics.width, 0.0);
        assert!(metrics.clusters.is_empty());
    }

    #[test]
    fn clusters_cover_the_whole_run() {
        let mut measurer = Measurer::new();
        let style = SpanStyle {
            family: String::new(),
            mono: false,
            size: 14.0,
            weight: 400,
            italic: false,
        };
        let metrics = measurer.measure("hello world", &style);
        assert!(!metrics.clusters.is_empty());
        assert_eq!(metrics.clusters[0].start, 0);
        assert_eq!(metrics.clusters.last().unwrap().end, "hello world".len());
        let summed: f32 = metrics.clusters.iter().map(|cluster| cluster.advance).sum();
        assert!(
            (summed - metrics.width).abs() < 0.5,
            "cluster advances {summed} should add up to the run width {}",
            metrics.width
        );
    }

    #[test]
    fn whitespace_is_measured_exactly_as_it_was_given() {
        // The shaper must not touch a run's whitespace. Every run is measured on
        // its own, so a collapse would trim the space off its end and the two
        // runs of a sentence would come together; the collapsing is the layout
        // pass's, and it keeps the ends.
        let mut measurer = Measurer::new();
        let style = SpanStyle {
            family: String::new(),
            mono: false,
            size: 14.0,
            weight: 400,
            italic: false,
        };
        for text in ["a    b", "a ", " a", " ", "a\n b", "  "] {
            let metrics = measurer.measure(text, &style);
            assert_eq!(metrics.text, text, "the shaper changed {text:?}");
            assert!(
                metrics
                    .clusters
                    .iter()
                    .all(|cluster| cluster.end <= text.len()),
                "a cluster of {text:?} is past its end"
            );
        }
    }

    #[test]
    fn a_bigger_font_is_wider() {
        let mut measurer = Measurer::new();
        let small = measurer.measure(
            "conic launcher",
            &SpanStyle {
                mono: false,
                family: String::new(),
                size: 12.0,
                weight: 400,
                italic: false,
            },
        );
        let large = measurer.measure(
            "conic launcher",
            &SpanStyle {
                mono: false,
                family: String::new(),
                size: 24.0,
                weight: 400,
                italic: false,
            },
        );
        assert!(large.width > small.width * 1.5);
    }
}
