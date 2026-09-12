//! Producer-only laws for ordinary isolated opacity with a complete source.
//!
//! An illustration states its source enclosure separately from its painted
//! nodes. These tests establish contract facts and refusals without a renderer.

use std::sync::Arc;

use cg::{CGColor, GradientStop, Paint, Paints, RadialGradientPaint};
use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    BlendSourceDomain, BlendSourceDomainError, Frame, FrameItem, FrameItems, FrameItemsError,
    FrameNode, Geometry, Identity, IsolatedSourceDomain, IsolatedSourceDomainError,
    MAX_PATTERN_DEPTH, MAX_SCOPE_DEPTH, PaintStack, PatternPaint, PatternPaintError, Provenance,
    Scope, ScopeBlend, ScopeBlendMode, ScopeEffect, ScopeOpacity, ScopeOpacityGroup, VisualRef,
};

fn owner(id: u64) -> VisualRef {
    VisualRef::new(Identity::new(id), Provenance::new(id + 100))
}

fn domain() -> IsolatedSourceDomain {
    IsolatedSourceDomain::new(
        Rectangle::from_xywh(-4.0, -2.0, 48.0, 40.0),
        AffineTransform::from_acebdf(1.0, 0.25, 8.0, 0.0, 1.0, 6.0),
    )
    .unwrap()
}

fn group(source: Option<IsolatedSourceDomain>) -> ScopeOpacityGroup {
    let group = ScopeOpacityGroup::new(ScopeOpacity::new(0.375).unwrap());
    match source {
        Some(source) => group.with_source_domain(source),
        None => group,
    }
}

fn begin(id: u64, group: ScopeOpacityGroup) -> FrameItem {
    FrameItem::ScopeBegin(Scope {
        owner: owner(id),
        effect: ScopeEffect::Opacity(group),
    })
}

fn node(id: u64, paints: PaintStack) -> FrameItem {
    let bounds = Rectangle::from_xywh(4.0, 6.0, 20.0, 16.0);
    FrameItem::Node(FrameNode {
        owner: owner(id),
        transform: AffineTransform::from_acebdf(1.0, 0.0, 12.0, 0.0, 1.0, 9.0),
        geometry: Geometry::Rect(bounds),
        bounds,
        paints,
        stroke: None,
    })
}

fn illustration(source: Option<IsolatedSourceDomain>) -> Frame {
    let ramp =
        PaintStack::try_from_paints(Paints::new([Paint::RadialGradient(RadialGradientPaint {
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: CGColor::RED.into(),
                },
                GradientStop {
                    offset: 1.0,
                    color: CGColor::BLUE.into(),
                },
            ],
            ..Default::default()
        })]))
        .unwrap();
    let mut ellipse = node(3, ramp);
    let FrameItem::Node(ref mut ellipse_node) = ellipse else {
        unreachable!()
    };
    ellipse_node.geometry = Geometry::Ellipse(ellipse_node.bounds);
    Frame {
        owner: owner(1),
        bounds: Rectangle::from_xywh(0.0, 0.0, 80.0, 64.0),
        items: FrameItems::try_new(vec![
            begin(2, group(source)),
            ellipse,
            node(4, PaintStack::solid(CGColor::from_rgba(40, 80, 160, 128))),
            FrameItem::ScopeEnd,
        ])
        .unwrap(),
    }
}

/// The source declaration belongs to the group, while the exact numeric factor
/// remains usable independently by a combined blend operation.
#[test]
fn absence_and_attachment_preserve_the_exact_numeric_factor() {
    for value in [
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        0.375,
        1.0_f32.next_down(),
    ] {
        let factor = ScopeOpacity::new(value).unwrap();
        let ordinary = ScopeOpacityGroup::new(factor);
        let declared = ordinary.with_source_domain(domain());
        assert_eq!(ordinary.source_domain(), None);
        assert_eq!(declared.source_domain(), Some(domain()));
        assert_eq!(ordinary.opacity().get().to_bits(), value.to_bits());
        assert_eq!(declared.opacity().get().to_bits(), value.to_bits());
        assert_ne!(ordinary, declared);
        let blend = ScopeBlend::new(ScopeBlendMode::Multiply, Some(declared.opacity()));
        assert_eq!(blend.opacity(), Some(factor));
        assert_eq!(blend.source_domain(), None);
    }
}

/// Transparent extent is independent of geometry and ramp placement. Attaching
/// it preserves one ordinary opacity boundary, its identity, and its children.
#[test]
fn an_independent_illustration_preserves_paint_geometry_owners_and_transforms() {
    let ordinary = illustration(None);
    let declared = illustration(Some(domain()));
    let wider = illustration(Some(
        IsolatedSourceDomain::new(
            Rectangle::from_xywh(-8.0, -6.0, 56.0, 48.0),
            domain().source_to_stream(),
        )
        .unwrap(),
    ));
    for frame in [&declared, &wider] {
        assert_eq!(frame.owner, ordinary.owner);
        assert_eq!(frame.bounds, ordinary.bounds);
        assert_eq!(frame.nodes(), ordinary.nodes());
        assert_eq!(frame.items.len(), 4);
        let FrameItem::ScopeBegin(scope) = frame.items.iter().next().unwrap() else {
            panic!("the ordinary opacity boundary must survive");
        };
        assert_eq!(scope.owner, owner(2));
        let ScopeEffect::Opacity(group) = scope.effect else {
            panic!("a source declaration must not become a different effect");
        };
        assert_eq!(group.opacity(), ScopeOpacity::new(0.375).unwrap());
        assert!(group.source_domain().is_some());
        assert_ne!(frame, &ordinary);
    }
    assert_ne!(declared, wider);
    assert_eq!(declared, illustration(Some(domain())));
}

/// Source-local units and placement are exact resolved facts, including signed
/// zero. Attaching a declaration must not canonicalize either into the other.
#[test]
fn attachment_preserves_rectangle_and_map_bits() {
    let rect = Rectangle::from_xywh(-0.0, -0.5, 10.25, 8.75);
    let map = AffineTransform::from_acebdf(-1.0, 0.25, 25.5, -0.0, 2.0, -0.25);
    let declared = group(Some(IsolatedSourceDomain::new(rect, map).unwrap()));
    let source = declared.source_domain().unwrap();
    let stored = source.rect();
    assert_eq!(
        [stored.x, stored.y, stored.width, stored.height].map(f32::to_bits),
        [rect.x, rect.y, rect.width, rect.height].map(f32::to_bits),
    );
    assert_eq!(
        source
            .source_to_stream()
            .matrix
            .map(|row| row.map(f32::to_bits)),
        map.matrix.map(|row| row.map(f32::to_bits)),
    );
}

/// Equal mapped enclosures cannot erase distinct source-local declarations.
#[test]
fn equal_mapped_boxes_do_not_identify_opacity_groups() {
    let first = IsolatedSourceDomain::new(
        Rectangle::from_xywh(0.0, 0.0, 4.0, 2.0),
        AffineTransform::identity(),
    )
    .unwrap();
    let second = IsolatedSourceDomain::new(
        Rectangle::from_xywh(0.0, 0.0, 2.0, 4.0),
        AffineTransform::from_acebdf(0.0, 1.0, 0.0, 1.0, 0.0, 0.0),
    )
    .unwrap();
    assert_eq!(
        math2::rect_transform(first.rect(), &first.source_to_stream()),
        math2::rect_transform(second.rect(), &second.source_to_stream()),
    );
    assert_ne!(group(Some(first)), group(Some(second)));
}

/// Opacity and blend share one numerical gate. The compatibility names retain
/// the same checked type and typed refusals, not a second validation path.
#[test]
fn opacity_and_blend_share_the_domain_and_its_numerical_refusals() {
    use rframe::BlendSourceDomainError::InvalidRectangle;

    assert_eq!(
        InvalidRectangle,
        IsolatedSourceDomainError::InvalidRectangle
    );
    let legacy: BlendSourceDomain = domain();
    let blend = ScopeBlend::new(ScopeBlendMode::Screen, Some(group(None).opacity()))
        .with_source_domain(legacy);
    assert_eq!(blend.source_domain(), group(Some(domain())).source_domain());
    for (rect, map, error) in [
        (
            Rectangle::from_xywh(0.0, 0.0, 0.0, 1.0),
            AffineTransform::identity(),
            IsolatedSourceDomainError::InvalidRectangle,
        ),
        (
            Rectangle::from_xywh(f32::MAX, 0.0, f32::MAX, 1.0),
            AffineTransform::identity(),
            IsolatedSourceDomainError::InvalidRectangle,
        ),
        (
            domain().rect(),
            AffineTransform::from_acebdf(1.0, 2.0, 0.0, 2.0, 4.0, 0.0),
            IsolatedSourceDomainError::InvalidTransform,
        ),
        (
            domain().rect(),
            AffineTransform::from_acebdf(1.0, 0.0, f32::INFINITY, 0.0, 1.0, 0.0),
            IsolatedSourceDomainError::InvalidTransform,
        ),
        (
            domain().rect(),
            AffineTransform::from_acebdf(1.0, 0.0, 1e20, 0.0, 1.0, 1e20),
            IsolatedSourceDomainError::InvalidMappedBounds,
        ),
    ] {
        let legacy_error: BlendSourceDomainError = error;
        assert_eq!(IsolatedSourceDomain::new(rect, map), Err(error));
        assert_eq!(BlendSourceDomain::new(rect, map), Err(legacy_error));
    }
}

/// Each boundary owns its completed source. Mixed nesting cannot transfer a
/// domain to its parent, add a synthetic opacity scope, or rebase child nodes.
#[test]
fn nested_opacity_and_blend_sources_remain_independent_in_stream_order() {
    let outer = group(Some(domain()));
    let inner_domain = IsolatedSourceDomain::new(
        Rectangle::from_xywh(0.0, 0.0, 16.0, 12.0),
        AffineTransform::from_acebdf(2.0, 0.0, 16.0, 0.0, 2.0, 12.0),
    )
    .unwrap();
    for inner in [
        ScopeEffect::Opacity(group(Some(inner_domain))),
        ScopeEffect::Blend(
            ScopeBlend::new(ScopeBlendMode::Multiply, Some(outer.opacity()))
                .with_source_domain(inner_domain),
        ),
    ] {
        let items = vec![
            begin(1, outer),
            FrameItem::ScopeBegin(Scope {
                owner: owner(2),
                effect: inner,
            }),
            node(3, PaintStack::solid(CGColor::RED)),
            FrameItem::ScopeEnd,
            FrameItem::ScopeEnd,
        ];
        let checked = FrameItems::try_new(items.clone()).unwrap();
        assert_eq!(checked.iter().cloned().collect::<Vec<_>>(), items);
        assert_eq!(checked.len(), 5);
        assert_eq!(checked.nodes().count(), 1);
    }
}

/// A domain supplies no content and no extra structural depth allowance.
#[test]
fn source_declarations_preserve_scope_content_balance_and_depth_refusals() {
    let declared = group(Some(domain()));
    assert_eq!(
        FrameItems::try_new(vec![begin(1, declared), FrameItem::ScopeEnd]),
        Err(FrameItemsError::EmptyScope { index: 0 }),
    );
    assert_eq!(
        FrameItems::try_new(vec![
            begin(1, declared),
            node(2, PaintStack::solid(CGColor::RED))
        ]),
        Err(FrameItemsError::UnclosedScope { index: 0 }),
    );
    let mut maximum = (0..MAX_SCOPE_DEPTH)
        .map(|id| begin(id as u64, declared))
        .collect::<Vec<_>>();
    let mut overflow = maximum.clone();
    overflow.push(begin(100, declared));
    assert_eq!(
        FrameItems::try_new(overflow),
        Err(FrameItemsError::ScopeTooDeep {
            index: MAX_SCOPE_DEPTH,
        })
    );
    maximum.push(node(100, PaintStack::solid(CGColor::RED)));
    maximum.extend(std::iter::repeat_n(FrameItem::ScopeEnd, MAX_SCOPE_DEPTH));
    FrameItems::try_new(maximum).unwrap();
}

fn repeating(paints: PaintStack) -> Result<PatternPaint, PatternPaintError> {
    let declared = group(Some(domain()));
    let items = Arc::new(
        FrameItems::try_new(vec![
            begin(1, declared),
            node(2, paints),
            FrameItem::ScopeEnd,
        ])
        .unwrap(),
    );
    let pattern = PatternPaint::new(
        64.0,
        64.0,
        AffineTransform::from_acebdf(3.0, 0.0, 12.0, 0.0, 2.0, 16.0),
        Arc::clone(&items),
        1.0,
    )?;
    assert!(Arc::ptr_eq(pattern.items(), &items));
    assert_eq!(pattern.items().iter().next(), Some(&begin(1, declared)));
    Ok(pattern)
}

/// A repeating program's placement must not rewrite its source-to-tile map.
/// Opacity declarations also cannot hide recursive paint programs from checks.
#[test]
fn repeating_programs_preserve_tile_local_sources_and_the_recursion_bound() {
    let mut pattern = repeating(PaintStack::solid(CGColor::RED)).unwrap();
    for expected in 2..=MAX_PATTERN_DEPTH {
        pattern = repeating(PaintStack::from_pattern(pattern)).unwrap();
        assert_eq!(pattern.depth(), expected);
    }
    assert_eq!(
        repeating(PaintStack::from_pattern(pattern)),
        Err(PatternPaintError::TooDeep)
    );
}
