//! Contract tests for the compositing scope and the checked item stream.
//!
//! A scope states that its span composites as one isolated group; the
//! stream construction proves balance, non-emptiness, and bounded nesting
//! once, so no consumer ever meets a dangling boundary. A scope opacity is
//! a fact only in the open unit interval — 1 is identity and 0 composites
//! nothing, and a producer states those as no scope at all.

use cg::CGColor;
use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    Filter, FilterColorSpace, FilterInput, FilterNode, FilterPrimitive, FilterProgram,
    FilterTurbulenceKind, FrameItem, FrameItems, FrameItemsError, FrameNode, Geometry, Identity,
    MAX_SCOPE_DEPTH, PaintStack, Provenance, Scope, ScopeEffect, ScopeOpacity, VisualRef,
};
use std::sync::Arc;

fn owner(id: u64) -> VisualRef {
    VisualRef::new(Identity::new(id), Provenance::new(id))
}

fn node(id: u64) -> FrameItem {
    let rect = Rectangle::from_xywh(8.0, 8.0, 24.0, 24.0);
    FrameItem::Node(FrameNode {
        owner: owner(id),
        transform: AffineTransform::identity(),
        geometry: Geometry::Rect(rect),
        bounds: rect,
        paints: PaintStack::solid(CGColor::RED),
        stroke: None,
    })
}

fn begin(id: u64) -> FrameItem {
    FrameItem::ScopeBegin(Scope {
        owner: owner(id),
        effect: ScopeEffect::Opacity(ScopeOpacity::new(0.5).expect("0.5 is a scope fact")),
    })
}

fn filter_begin(id: u64, generated: bool, transparent_source: bool) -> FrameItem {
    let region = Rectangle::from_xywh(0.0, 0.0, 40.0, 40.0);
    let primitive = if generated {
        FilterPrimitive::Turbulence {
            kind: FilterTurbulenceKind::Turbulence,
            base_frequency_x: 0.1,
            base_frequency_y: 0.2,
            num_octaves: 2,
            seed: 3.0,
            stitch_tiles: false,
        }
    } else {
        FilterPrimitive::GaussianBlur {
            sigma_x: 2.0,
            sigma_y: 2.0,
        }
    };
    let inputs: Arc<[FilterInput]> = if generated {
        Arc::from([])
    } else {
        Arc::from([FilterInput::Source])
    };
    let program = FilterProgram::new(Arc::from([FilterNode::new(
        inputs,
        region,
        FilterColorSpace::LinearRgb,
        primitive,
    )]))
    .expect("test filter graph is checked");
    let filter = Filter::new(AffineTransform::identity(), region, program)
        .expect("test filter invocation is checked");
    let filter = if transparent_source {
        filter.with_transparent_source()
    } else {
        filter
    };
    FrameItem::ScopeBegin(Scope {
        owner: owner(id),
        effect: ScopeEffect::Filter(filter),
    })
}

#[test]
fn a_balanced_nested_stream_is_admitted() {
    let items = FrameItems::try_new(vec![
        node(1),
        begin(2),
        node(3),
        begin(4),
        node(5),
        FrameItem::ScopeEnd,
        FrameItem::ScopeEnd,
        node(6),
    ])
    .expect("balanced non-empty scopes are the contract");
    assert_eq!(items.len(), 8);
    assert_eq!(
        items.nodes().map(|n| n.owner).collect::<Vec<_>>(),
        [owner(1), owner(3), owner(5), owner(6)],
        "the flat node view keeps painter order across scopes"
    );
}

#[test]
fn an_unopened_end_is_refused_at_its_index() {
    assert_eq!(
        FrameItems::try_new(vec![node(1), FrameItem::ScopeEnd]),
        Err(FrameItemsError::UnopenedScopeEnd { index: 1 })
    );
}

#[test]
fn an_unclosed_begin_is_refused_at_its_index() {
    assert_eq!(
        FrameItems::try_new(vec![begin(1), node(2)]),
        Err(FrameItemsError::UnclosedScope { index: 0 })
    );
}

#[test]
fn an_empty_scope_is_refused_at_its_begin() {
    assert_eq!(
        FrameItems::try_new(vec![node(1), begin(2), FrameItem::ScopeEnd]),
        Err(FrameItemsError::EmptyScope { index: 1 })
    );
}

#[test]
fn a_generated_filter_over_a_declared_transparent_source_is_meaningful_when_empty() {
    FrameItems::try_new(vec![filter_begin(1, true, true), FrameItem::ScopeEnd])
        .expect("the generated filter output is the empty span's visual fact");
}

#[test]
fn an_empty_filter_cannot_claim_meaning_without_both_contract_facts() {
    assert_eq!(
        FrameItems::try_new(vec![filter_begin(1, true, false), FrameItem::ScopeEnd]),
        Err(FrameItemsError::EmptyScope { index: 0 })
    );
    assert_eq!(
        FrameItems::try_new(vec![filter_begin(1, false, true), FrameItem::ScopeEnd]),
        Err(FrameItemsError::EmptyScope { index: 0 })
    );
}

/// A scope holding only another scope is not empty — group-of-group is a
/// resolved fact (a container whose one child is itself a real layer).
#[test]
fn a_scope_of_one_scope_is_admitted() {
    FrameItems::try_new(vec![
        begin(1),
        begin(2),
        node(3),
        FrameItem::ScopeEnd,
        FrameItem::ScopeEnd,
    ])
    .expect("nested-only content is a group fact");
}

#[test]
fn nesting_beyond_the_bound_is_refused() {
    let mut items = Vec::new();
    for id in 0..=MAX_SCOPE_DEPTH as u64 {
        items.push(begin(id));
    }
    assert_eq!(
        FrameItems::try_new(items),
        Err(FrameItemsError::ScopeTooDeep {
            index: MAX_SCOPE_DEPTH
        })
    );
}

/// The open unit interval is the whole opacity contract: identity and
/// nothing are producer resolutions, not scope facts, and a non-finite
/// value is no fact at all.
#[test]
fn scope_opacity_admits_exactly_the_open_unit_interval() {
    assert!(ScopeOpacity::new(f32::MIN_POSITIVE).is_ok());
    assert!(ScopeOpacity::new(0.5).is_ok());
    for refused in [0.0, 1.0, -0.25, 1.5, f32::NAN, f32::INFINITY] {
        assert!(
            ScopeOpacity::new(refused).is_err(),
            "{refused} is not a scope fact"
        );
    }
}
