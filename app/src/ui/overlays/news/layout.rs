// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

//! The news flow's geometry.
//!
//! Pure arithmetic, apart from Slint, so it can be tested without a window. The
//! card heights and their stacked positions are computed here rather than
//! measured by the flow's layout: the flow positions its cards absolutely (so it
//! can cull them), and a Slint layout cannot be asked which part of a
//! four-hundred-entry list a viewport is over.

/// The gap between two cards, and the flow's top and bottom padding. The
/// `.slint` flow draws the same numbers — Rust stacks the cards, so the two
/// have to agree.
pub(crate) const FLOW_GAP: f32 = 12.0;
pub(crate) const FLOW_PAD_TOP: f32 = 16.0;
pub(crate) const FLOW_PAD_BOTTOM: f32 = 32.0;
/// A changelog row's height: the square image plus nothing — the copy is centred
/// beside it.
pub(crate) const CHANGELOG_CARD_HEIGHT: f32 = 96.0;
/// A news entry's banner height. Fixed, not the image's own ratio: every banner
/// in the flow is the same height, so the column reads as a column and a wide
/// window does not turn each card into a billboard. `news-card.slint` draws the
/// box and lets the picture cover it.
pub(crate) const NEWS_MEDIA_HEIGHT: f32 = 200.0;
/// A banner card's copy block, below the image. `news-card.slint` lays out the
/// same 110px (12 + 22 + 4 + 16 + 4 + 40 + 12).
pub(crate) const BANNER_INFO_HEIGHT: f32 = 110.0;
/// The search panel's 24px padding on each side, and the chip row's 52px label
/// plus its 10px gap.
const SEARCH_PANEL_PAD: f32 = 48.0;
const CHIP_LABEL: f32 = 62.0;
pub(crate) const CHIP_HEIGHT: f32 = 20.0;
pub(crate) const CHIP_GAP: f32 = 6.0;

/// A banner card's height: the fixed banner over the copy block.
pub(crate) fn banner_height() -> f32 {
    NEWS_MEDIA_HEIGHT + BANNER_INFO_HEIGHT
}

/// A changelog card's height is fixed; only the square image's width depends on
/// the ratio, and that is the image's own business.
pub(crate) fn changelog_height() -> f32 {
    CHANGELOG_CARD_HEIGHT
}

/// Each card's top edge inside the flow, in order. The flow positions its
/// cards absolutely (so it can cull them), which is why Rust stacks them.
pub(crate) fn positions(heights: &[f32], gap: f32, top: f32) -> Vec<f32> {
    let mut y = top;
    heights
        .iter()
        .map(|height| {
            let at = y;
            y += height + gap;
            at
        })
        .collect()
}

/// The flow's total height: the top padding, the cards and their gaps, and the
/// bottom padding. An empty flow keeps just the padding.
pub(crate) fn total_height(heights: &[f32], gap: f32, top: f32, bottom: f32) -> f32 {
    if heights.is_empty() {
        return top + bottom;
    }
    let cards: f32 = heights.iter().sum();
    top + cards + gap * (heights.len() - 1) as f32 + bottom
}

/// The cards a viewport touches, as an index range. `offset` is the distance
/// from the flow's top to the top of the viewport and `viewport` its height; a
/// margin widens the window so a card is fetched before it is scrolled to.
///
/// Returns an empty range when nothing is visible — which is not the same as
/// the whole list.
pub(crate) fn visible_range(
    positions: &[f32],
    heights: &[f32],
    offset: f32,
    viewport: f32,
    margin: f32,
) -> std::ops::Range<usize> {
    let top = offset - margin;
    let bottom = offset + viewport + margin;
    let mut start = positions.len();
    let mut end = 0;
    for (index, (&y, &height)) in positions.iter().zip(heights).enumerate() {
        if y + height >= top && y <= bottom {
            start = start.min(index);
            end = index + 1;
        }
    }
    start..end.max(start)
}

/// The width a filter row's chips wrap inside: the panel less the search
/// panel's padding and the row's label column.
pub(crate) fn filter_available(list_width: f32) -> f32 {
    (list_width - SEARCH_PANEL_PAD - CHIP_LABEL).max(0.0)
}

/// A wrapping chip row's height: 20px per line with the 6px gap between them.
/// The chips report their widths and Rust wraps them here, because a wrapping
/// `FlexboxLayout` measures itself at its own "roughly square" preferred width
/// rather than the width it is given.
pub(crate) fn filter_row_height(available: f32, widths: impl Iterator<Item = f32>) -> f32 {
    let mut lines = 1.0f32;
    let mut x = 0.0f32;
    for width in widths {
        if x > 0.0 && x + CHIP_GAP + width > available {
            lines += 1.0;
            x = width;
        } else {
            x += if x > 0.0 { CHIP_GAP } else { 0.0 } + width;
        }
    }
    lines * CHIP_HEIGHT + (lines - 1.0) * CHIP_GAP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_banner_is_a_fixed_height_over_the_copy() {
        assert_eq!(banner_height(), NEWS_MEDIA_HEIGHT + BANNER_INFO_HEIGHT);
        assert_eq!(banner_height(), 310.0);
    }

    #[test]
    fn a_narrower_row_takes_more_lines() {
        let chips = [60.0, 60.0, 60.0, 60.0, 60.0];
        // 5 x 60 + 4 x 6 = 324, so 330 fits and 320 does not.
        assert_eq!(filter_row_height(330.0, chips.into_iter()), 20.0);
        assert_eq!(
            filter_row_height(320.0, chips.into_iter()),
            CHIP_HEIGHT * 2.0 + CHIP_GAP
        );
    }

    #[test]
    fn cards_are_stacked_with_the_gap_between_them() {
        let positions = positions(&[100.0, 50.0, 200.0], 12.0, 16.0);
        assert_eq!(positions, [16.0, 128.0, 190.0]);
        assert_eq!(total_height(&[100.0, 50.0, 200.0], 12.0, 16.0, 32.0), 422.0);
    }

    #[test]
    fn an_empty_flow_keeps_only_its_padding() {
        assert_eq!(positions(&[], 12.0, 16.0), Vec::<f32>::new());
        assert_eq!(total_height(&[], 12.0, 16.0, 32.0), 48.0);
    }

    #[test]
    fn the_viewport_range_covers_what_it_touches() {
        let heights = [100.0; 10];
        let positions = positions(&heights, 0.0, 0.0);
        // A 250px viewport at the top touches cards 0, 1 and 2; no margin.
        assert_eq!(visible_range(&positions, &heights, 0.0, 250.0, 0.0), 0..3);
        // Scrolled to 500 it covers y in 500..750, which cards 4 to 7 touch.
        assert_eq!(visible_range(&positions, &heights, 500.0, 250.0, 0.0), 4..8);
        // A 100px margin widens the window to 400..850: cards 3 to 8.
        assert_eq!(
            visible_range(&positions, &heights, 500.0, 250.0, 100.0),
            3..9
        );
    }

    #[test]
    fn nothing_visible_is_an_empty_range_not_the_whole_list() {
        let heights = [100.0; 4];
        let positions = positions(&heights, 0.0, 0.0);
        assert_eq!(
            visible_range(&positions, &heights, 1000.0, 100.0, 0.0),
            4..4
        );
    }
}
