// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! Text measurement through the same engine, font collection and fallbacks the
//! view draws with, or measured and drawn widths drift apart.
//!
//! A run is shaped on its own with no wrapping; the caller owns the breaks.

use std::borrow::Cow;

use fontique::{Collection, GenericFamily, SourceCache, SourceCacheOptions};
use parley::style::{
    FontFamily, FontFamilyName, FontStyle, FontWeight, OverflowWrap, TextStyle, WhiteSpaceCollapse,
    WordBreak,
};
use parley::{FontContext, Layout, LayoutContext};

/// This renderer positions but never paints, so the brush carries nothing.
type Brush = ();

/// The generic families Slint appends behind a `Text`'s family, in Slint's
/// order, which puts `SansSerif` first for FemtoVG.
const FALLBACK_FAMILIES: [GenericFamily; 2] = [GenericFamily::SansSerif, GenericFamily::SystemUi];

/// The same for monospace. Without the generic monospace face in the chain an
/// unresolved name falls through to the sans-serif face and code is measured at
/// the wrong advances.
const MONO_FALLBACK_FAMILIES: [GenericFamily; 3] = [
    GenericFamily::Monospace,
    GenericFamily::SansSerif,
    GenericFamily::SystemUi,
];

/// What a run is set in. Only properties that change metrics are here; colour
/// and decoration live on the display-list item.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanStyle {
    /// The family, with generic fallbacks appended; empty shapes with the
    /// fallbacks alone.
    pub family: String,
    /// Changes only the fallback chain, so the family above is still tried
    /// first.
    pub mono: bool,
    pub size: f32,
    pub weight: u16,
    pub italic: bool,
}

/// A group of characters that cannot be broken apart, with its advance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cluster {
    /// Byte range within the measured run.
    pub start: usize,
    pub end: usize,
    pub advance: f32,
}

/// Everything a line needs to know about one run.
#[derive(Debug, Clone, Default)]
pub struct SpanMetrics {
    /// The text the shaper laid out; clusters index into it.
    pub text: String,
    /// In reading order; the advances add up to `width`.
    pub clusters: Vec<Cluster>,
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub leading: f32,
    /// Distance from the top of the run's box to its baseline.
    pub baseline: f32,
    /// Measured upwards from the baseline; drawn as a box because Slint's
    /// `Text` has no strikethrough.
    pub strike_offset: f32,
    pub strike_size: f32,
    /// Measured downwards from the baseline.
    pub underline_offset: f32,
    pub underline_size: f32,
}

/// The shaping context caches results, which makes a resize cheap.
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
    /// A measurer over the system's fonts, for a caller that renders with them.
    /// A caller that embeds a family must use [`Self::with_collection`], or
    /// every advance and line break will be wrong.
    pub fn new() -> Self {
        Self::with_collection(crate::ui::components::markdown::fonts::system())
    }

    /// A measurer over the caller's own font collection, which for a Slint app
    /// is `slint::fontique_011::shared_collection()`.
    pub fn with_collection(collection: Collection) -> Self {
        // `shared: true` avoids loading the same font files twice.
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

    /// Pass `window.scale_factor()` to agree with Slint's hinting to the last
    /// sub-pixel; at 1 the difference is well under a pixel per run.
    pub fn set_scale(&mut self, scale: f32) {
        if scale > 0.0 {
            self.scale = scale;
        }
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

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
        // No wrapping and one style, so this is one line; the clusters are
        // already in visual order.
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
            // The caller does the breaking; the flag keeps a long URL or hash
            // from becoming one indivisible cluster.
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
        // Never collapse here: every run is shaped on its own, so trimming the
        // end would glue two runs of a sentence together. The caller collapses.
        builder.set_white_space_mode(WhiteSpaceCollapse::Preserve);
        builder.push_text(text);
        let (mut layout, laid_out) = builder.build();
        layout.break_all_lines(None);
        (layout, laid_out)
    }
}

/// `[family, SansSerif, SystemUi]`, with `Monospace` added for a monospace run
/// and the generics alone when no family was given.
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
