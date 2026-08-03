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
    FrameItem, FrameItems, FrameItemsError, FrameNode, Geometry, Identity, MAX_SCOPE_DEPTH,
    PaintStack, Provenance, Scope, ScopeEffect, ScopeOpacity, VisualRef,
};

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
