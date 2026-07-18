//! Pure mappings from the shared per-element style record
//! ([`StyledElement`]) to the types the HTML importer's emitters consume.
//!
//! This is the importer's half of the htmlcss seam (gridaco/nothing#30):
//! `crate::htmlcss::collect::styled_of` extracts one element's resolved
//! style into a [`StyledElement`]; the functions here map that record
//! onto v1 node-record fields. Every function is pure and total — no
//! Stylo types, no DOM access.
//!
//! The mappings reproduce the importer's pinned behavior (the
//! `html_import_snapshot` golden corpus) exactly; where the record is
//! richer than what the importer historically consumed (percent radii,
//! `repeating` gradient flags, flex-basis, …), the extra information is
//! deliberately dropped, matching the old `ComputedValues` walk.

use crate::cg::prelude::*;
use crate::node::schema::*;

use crate::htmlcss::style::StyledElement;
use crate::htmlcss::types::AlignItems as CssAlignItems;
use crate::htmlcss::types::{
    CssLength, Display, FlexDirection, FlexWrap, JustifyContent, Position,
};

use super::CSSMargin;

/// CSS `width`/`height`/`min-*`/`max-*` → [`LayoutDimensionStyle`].
///
/// Only definite px lengths are honored; `auto` clears the target
/// dimension (containers default to the factory's 100×100 otherwise)
/// and percentages/calc leave the field untouched — the importer has
/// never resolved them (no containing-block size at import time).
pub(super) fn dimensions_from(el: &StyledElement, dims: &mut LayoutDimensionStyle) {
    match el.width {
        CssLength::Px(v) => dims.layout_target_width = Some(v),
        CssLength::Auto => dims.layout_target_width = None,
        _ => {}
    }
    match el.height {
        CssLength::Px(v) => dims.layout_target_height = Some(v),
        CssLength::Auto => dims.layout_target_height = None,
        _ => {}
    }
    if let CssLength::Px(v) = el.min_width {
        if v > 0.0 {
            dims.layout_min_width = Some(v);
        }
    }
    if let CssLength::Px(v) = el.min_height {
        if v > 0.0 {
            dims.layout_min_height = Some(v);
        }
    }
    if let CssLength::Px(v) = el.max_width {
        dims.layout_max_width = Some(v);
    }
    if let CssLength::Px(v) = el.max_height {
        dims.layout_max_height = Some(v);
    }
}

/// CSS `width`/`height` → a leaf [`Size`] (Rectangle nodes).
///
/// `auto`, percentages, and calc all resolve to `0×0` — unlike the
/// design-tool convention (100×100 default), HTML leaf elements have no
/// intrinsic size.
pub(super) fn size_from(el: &StyledElement) -> Size {
    let px = |v: CssLength| -> f32 {
        match v {
            CssLength::Px(p) => p,
            _ => 0.0,
        }
    };
    Size {
        width: px(el.width),
        height: px(el.height),
    }
}

/// Flex-child properties (`flex-grow`, absolute positioning) →
/// [`LayoutChildStyle`]. `None` when all values are at their defaults
/// (grow = 0, position static/relative).
pub(super) fn layout_child_from(el: &StyledElement) -> Option<LayoutChildStyle> {
    let grow = el.flex_grow;
    // The record maps `position: fixed` to `Position::Absolute` at
    // extraction (Stylo's `is_absolutely_positioned` covers both), so
    // one variant check matches the old ComputedValues read.
    let is_absolute = el.position == Position::Absolute;
    if grow > 0.0 || is_absolute {
        Some(LayoutChildStyle {
            layout_grow: grow,
            layout_positioning: if is_absolute {
                LayoutPositioning::Absolute
            } else {
                LayoutPositioning::Auto
            },
        })
    } else {
        None
    }
}

/// Flex-container properties (`display`, direction, wrap, alignment,
/// gap) → [`LayoutContainerStyle`].
///
/// Non-flex displays (block, grid, table, …) map to a Flex column: the
/// IR's `LayoutMode::Normal` maps to taffy `Display::Block`, which
/// causes children of a flex parent to stretch to 100% width (block
/// intrinsic sizing). Using Flex column instead gives correct sizing
/// when these containers are nested inside flex parents.
pub(super) fn flex_container_from(el: &StyledElement, out: &mut LayoutContainerStyle) {
    out.layout_mode = LayoutMode::Flex;
    if el.display != Display::Flex {
        out.layout_direction = Axis::Vertical;
        return;
    }

    out.layout_direction = match el.flex_direction {
        FlexDirection::Row | FlexDirection::RowReverse => Axis::Horizontal,
        FlexDirection::Column | FlexDirection::ColumnReverse => Axis::Vertical,
    };

    out.layout_wrap = Some(match el.flex_wrap {
        FlexWrap::Nowrap => LayoutWrap::NoWrap,
        FlexWrap::Wrap | FlexWrap::WrapReverse => LayoutWrap::Wrap,
    });

    // align-items → cross axis alignment. `None` is the record's
    // authored-`normal` (and unrecognized-keyword) case — the importer
    // leaves the field unset; `baseline` has no IR equivalent and joins
    // it, exactly like the old AlignFlags fall-through arm.
    out.layout_cross_axis_alignment = match el.align_items {
        Some(CssAlignItems::Center) => Some(CrossAxisAlignment::Center),
        Some(CssAlignItems::Start) => Some(CrossAxisAlignment::Start),
        Some(CssAlignItems::End) => Some(CrossAxisAlignment::End),
        Some(CssAlignItems::Stretch) => Some(CrossAxisAlignment::Stretch),
        Some(CssAlignItems::Baseline) | None => None,
    };

    // justify-content → main axis alignment. `None` (authored `normal`
    // or unrecognized) leaves the field unset, as before.
    out.layout_main_axis_alignment = match el.justify_content {
        Some(JustifyContent::Center) => Some(MainAxisAlignment::Center),
        Some(JustifyContent::Start) => Some(MainAxisAlignment::Start),
        Some(JustifyContent::End) => Some(MainAxisAlignment::End),
        Some(JustifyContent::SpaceBetween) => Some(MainAxisAlignment::SpaceBetween),
        Some(JustifyContent::SpaceAround) => Some(MainAxisAlignment::SpaceAround),
        Some(JustifyContent::SpaceEvenly) => Some(MainAxisAlignment::SpaceEvenly),
        Some(JustifyContent::Stretch) => Some(MainAxisAlignment::Stretch),
        None => None,
    };

    // Gap (flex containers only).
    // CSS column-gap = inline-axis gap, row-gap = block-axis gap.
    // For flex-direction: row, column-gap is the main-axis gap.
    // For flex-direction: column, row-gap is the main-axis gap.
    if el.row_gap != 0.0 || el.column_gap != 0.0 {
        let (main_gap, cross_gap) = match el.flex_direction {
            FlexDirection::Row | FlexDirection::RowReverse => (el.column_gap, el.row_gap),
            FlexDirection::Column | FlexDirection::ColumnReverse => (el.row_gap, el.column_gap),
        };
        out.layout_gap = Some(LayoutGap {
            main_axis_gap: main_gap,
            cross_axis_gap: cross_gap,
        });
    }
}

/// CSS margin → the importer's margin-surgery input. The record stores
/// each side as `Auto` or a resolved px length (percent margins resolve
/// to 0 at extraction, as the old walk did).
pub(super) fn margin_from(el: &StyledElement) -> CSSMargin {
    let side = |v: CssLength| -> (f32, bool) {
        match v {
            CssLength::Auto => (0.0, true),
            CssLength::Px(p) => (p, false),
            _ => (0.0, false),
        }
    };
    let (top, top_auto) = side(el.margin.top);
    let (right, right_auto) = side(el.margin.right);
    let (bottom, bottom_auto) = side(el.margin.bottom);
    let (left, left_auto) = side(el.margin.left);
    CSSMargin {
        top,
        right,
        bottom,
        left,
        top_auto,
        right_auto,
        bottom_auto,
        left_auto,
    }
}
