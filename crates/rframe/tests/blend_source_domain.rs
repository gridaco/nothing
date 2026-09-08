//! Producer-only laws for complete, already-enclosed blend-source domains.
//!
//! An illustration producer states paint and source enclosure independently.
//! These tests establish representation and refusal laws, not raster results.

use std::sync::Arc;

use cg::{CGColor, GradientStop, LinearGradientPaint, Paint, Paints};
use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    BlendSourceDomain, BlendSourceDomainError, ClipEdgeMode, ClipGeometry, ClipLayer, ClipPath,
    Frame, FrameItem, FrameItems, FrameItemsError, FrameNode, Geometry, Identity,
    MAX_PATTERN_DEPTH, MAX_SCOPE_DEPTH, Mask, MaskMode, PaintAlphaFactor, PaintStack, PatternPaint,
    PatternPaintError, Provenance, Scope, ScopeBlend, ScopeBlendMode, ScopeEffect, ScopeOpacity,
    Stroke, StrokeCap, StrokeJoin, VisualRef,
};

fn owner(id: u64) -> VisualRef {
    VisualRef::new(Identity::new(id), Provenance::new(id + 100))
}

fn rect() -> Rectangle {
    Rectangle::from_xywh(4.0, 6.0, 12.0, 8.0)
}

fn domain() -> BlendSourceDomain {
    BlendSourceDomain::new(
        Rectangle::from_xywh(0.0, 0.0, 24.0, 24.0),
        AffineTransform::identity(),
    )
    .unwrap()
}

fn node(id: u64, paints: PaintStack) -> FrameItem {
    FrameItem::Node(FrameNode {
        owner: owner(id),
        transform: AffineTransform::identity(),
        geometry: Geometry::Rect(rect()),
        bounds: rect(),
        paints,
        stroke: None,
    })
}

fn begin(id: u64, source: BlendSourceDomain) -> FrameItem {
    FrameItem::ScopeBegin(Scope {
        owner: owner(id),
        effect: ScopeEffect::Blend(
            ScopeBlend::new(ScopeBlendMode::Normal, None).with_source_domain(source),
        ),
    })
}

fn illustration(source: Option<BlendSourceDomain>) -> Frame {
    let mut blend = ScopeBlend::new(
        ScopeBlendMode::Screen,
        Some(ScopeOpacity::new(0.375).unwrap()),
    );
    if let Some(source) = source {
        blend = blend.with_source_domain(source);
    }
    let ramp =
        PaintStack::try_from_paints(Paints::new([Paint::LinearGradient(LinearGradientPaint {
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
    Frame {
        owner: owner(1),
        bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0),
        items: FrameItems::try_new(vec![
            FrameItem::ScopeBegin(Scope {
                owner: owner(2),
                effect: ScopeEffect::Blend(blend),
            }),
            node(3, ramp),
            node(4, PaintStack::solid(CGColor::from_rgba(40, 80, 160, 128))),
            FrameItem::ScopeEnd,
        ])
        .unwrap(),
    }
}

/// Attaching a domain adds no boundary and changes neither final operation.
#[test]
fn absence_makes_no_completeness_assertion_and_attachment_preserves_blend_and_opacity() {
    for mode in [
        ScopeBlendMode::Normal,
        ScopeBlendMode::Multiply,
        ScopeBlendMode::Screen,
    ] {
        for opacity in [None, Some(ScopeOpacity::new(0.375).unwrap())] {
            let ordinary = ScopeBlend::new(mode, opacity);
            let declared = ordinary.with_source_domain(domain());
            assert_eq!(ordinary.source_domain(), None);
            assert_eq!(declared.source_domain(), Some(domain()));
            assert_eq!(declared.mode(), ordinary.mode());
            assert_eq!(declared.opacity(), ordinary.opacity());
            assert_ne!(declared, ordinary);
        }
    }
}

/// A resolved non-painted extent can distinguish two otherwise identical
/// illustration products without becoming geometry, a paint, or a clip item.
#[test]
fn an_independent_illustration_changes_domain_without_changing_painted_nodes() {
    let ordinary = illustration(None);
    let enclosed = illustration(Some(domain()));
    let wider = illustration(Some(
        BlendSourceDomain::new(
            Rectangle::from_xywh(-2.0, -2.0, 28.0, 28.0),
            AffineTransform::identity(),
        )
        .unwrap(),
    ));
    for frame in [&enclosed, &wider] {
        assert_eq!(frame.owner, ordinary.owner);
        assert_eq!(frame.bounds, ordinary.bounds);
        assert_eq!(frame.nodes(), ordinary.nodes());
        assert_eq!(frame.items.len(), ordinary.items.len());
        assert_eq!(
            frame.nodes().iter().map(|n| n.paints.len()).sum::<usize>(),
            2
        );
        assert!(frame.nodes().iter().all(|n| n.stroke.is_none()));
        assert_ne!(frame, &ordinary);
    }
    assert_ne!(enclosed, wider);
    assert_eq!(enclosed, illustration(Some(domain())));
}

/// Enclosure is already resolved in the producer's units. The contract does
/// not round fractional inputs or canonicalize local placement into the map.
#[test]
fn construction_preserves_the_declared_rectangle_and_map_exactly() {
    let local = Rectangle::from_xywh(-0.0, -0.5, 10.25, 8.75);
    let map = AffineTransform::from_acebdf(-1.0, 0.25, 25.5, 0.125, 2.0, -0.25);
    let source = BlendSourceDomain::new(local, map).unwrap();
    let stored = source.rect();
    assert_eq!(
        [stored.x, stored.y, stored.width, stored.height].map(f32::to_bits),
        [local.x, local.y, local.width, local.height].map(f32::to_bits)
    );
    assert_eq!(
        source
            .source_to_stream()
            .matrix
            .map(|row| row.map(f32::to_bits)),
        map.matrix.map(|row| row.map(f32::to_bits))
    );
}

/// Mapping may hide distinctions in where the source was enclosed. A final
/// enclosing box cannot replace either of the two carried facts.
#[test]
fn equal_final_boxes_do_not_identify_different_local_domains() {
    let first = BlendSourceDomain::new(
        Rectangle::from_xywh(0.0, 0.0, 4.0, 2.0),
        AffineTransform::identity(),
    )
    .unwrap();
    let second = BlendSourceDomain::new(
        Rectangle::from_xywh(0.0, 0.0, 2.0, 4.0),
        AffineTransform::from_acebdf(0.0, 1.0, 0.0, 1.0, 0.0, 0.0),
    )
    .unwrap();
    assert_eq!(
        math2::rect_transform(first.rect(), &first.source_to_stream()),
        math2::rect_transform(second.rect(), &second.source_to_stream())
    );
    assert_ne!(first, second);
}

/// Enclosing two diagonal unit squares before a shear yields a different
/// domain than enclosing them afterwards. Only the former is declared here.
#[test]
fn the_declared_enclosure_precedes_its_map() {
    let shear = AffineTransform::from_acebdf(1.0, -1.0, 0.0, 0.0, 1.0, 0.0);
    let source = BlendSourceDomain::new(Rectangle::from_xywh(0.0, 0.0, 3.0, 3.0), shear).unwrap();
    assert_eq!(source.rect(), Rectangle::from_xywh(0.0, 0.0, 3.0, 3.0));
    let mapped = math2::rect_transform(source.rect(), &source.source_to_stream());
    assert_eq!(mapped, Rectangle::from_xywh(-3.0, 0.0, 6.0, 3.0));
    let individually_enclosed = Rectangle::from_xywh(-1.0, 0.0, 2.0, 3.0);
    assert_ne!(mapped, individually_enclosed);
}

/// An empty rectangle is not an empty-source API. Finite widths also need
/// ordered representable endpoints rather than overflowing or collapsing.
#[test]
fn empty_nonfinite_and_unrepresentable_local_rectangles_are_refused() {
    let map = AffineTransform::identity();
    for rectangle in [
        Rectangle::from_xywh(0.0, 0.0, 0.0, 1.0),
        Rectangle::from_xywh(0.0, 0.0, 1.0, -0.0),
        Rectangle::from_xywh(0.0, 0.0, -1.0, 1.0),
        Rectangle::from_xywh(0.0, 0.0, 1.0, -1.0),
        Rectangle::from_xywh(f32::MAX, 0.0, f32::MAX, 1.0),
        Rectangle::from_xywh(0.0, f32::MAX, 1.0, f32::MAX),
        Rectangle::from_xywh(1.0, 0.0, f32::from_bits(1), 1.0),
        Rectangle::from_xywh(0.0, 1.0, 1.0, f32::from_bits(1)),
    ] {
        assert_eq!(
            BlendSourceDomain::new(rectangle, map),
            Err(BlendSourceDomainError::InvalidRectangle)
        );
    }
    for index in 0..4 {
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut members = [0.0, 0.0, 1.0, 1.0];
            members[index] = value;
            let [x, y, width, height] = members;
            assert_eq!(
                BlendSourceDomain::new(Rectangle::from_xywh(x, y, width, height), map),
                Err(BlendSourceDomainError::InvalidRectangle)
            );
        }
    }
}

/// A finite forward matrix is insufficient when inverse arithmetic is
/// unsupported, overflows, or loses its determinant to non-finite arithmetic.
#[test]
fn unusable_coordinate_maps_are_refused_without_an_identity_fallback() {
    for map in [
        AffineTransform::from_acebdf(1.0, 2.0, 0.0, 2.0, 4.0, 0.0),
        AffineTransform::from_acebdf(f32::EPSILON / 2.0, 0.0, 0.0, 0.0, 1.0, 0.0),
        AffineTransform::from_acebdf(f32::MAX, 0.0, 0.0, 0.0, 2.0, 0.0),
        AffineTransform::from_acebdf(f32::MAX, f32::MAX, 0.0, f32::MAX, f32::MAX, 0.0),
        AffineTransform::from_acebdf(0.5, 0.0, f32::MAX, 0.0, 1.0, 0.0),
    ] {
        assert_eq!(
            BlendSourceDomain::new(rect(), map),
            Err(BlendSourceDomainError::InvalidTransform)
        );
    }
    for row in 0..2 {
        for column in 0..3 {
            for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut map = AffineTransform::identity();
                map.matrix[row][column] = value;
                assert_eq!(
                    BlendSourceDomain::new(rect(), map),
                    Err(BlendSourceDomainError::InvalidTransform)
                );
            }
        }
    }
}

/// Bounding must not hide a non-finite corner, an overflowing span between
/// finite corners, or a positive domain collapsed by finite translation.
#[test]
fn mapped_corners_and_their_enclosing_rectangle_must_remain_representable() {
    for (rectangle, map) in [
        (
            Rectangle::from_xywh(0.0, 0.0, f32::MAX, 1.0),
            AffineTransform::from_acebdf(2.0, 0.0, 0.0, 0.0, 1.0, 0.0),
        ),
        (
            Rectangle::from_xywh(-f32::MAX / 2.0, 0.0, f32::MAX, 1.0),
            AffineTransform::from_acebdf(1.5, 0.0, 0.0, 0.0, 1.0, 0.0),
        ),
        (
            Rectangle::from_xywh(0.0, 0.0, 2.0, 2.0),
            AffineTransform::from_acebdf(f32::MAX, -f32::MAX, 0.0, 1.0, -0.5, 0.0),
        ),
        (
            rect(),
            AffineTransform::from_acebdf(1.0, 0.0, 1e20, 0.0, 1.0, 1e20),
        ),
    ] {
        assert_eq!(
            BlendSourceDomain::new(rectangle, map),
            Err(BlendSourceDomainError::InvalidMappedBounds)
        );
    }
}

/// The numerical boundary imposes neither a minimum pixel nor a maximum
/// allocation size. Neither belongs to source-local units.
#[test]
fn representable_domains_have_no_device_size_limit() {
    for size in [f32::from_bits(1), f32::MIN_POSITIVE, 1.0, f32::MAX] {
        let rectangle = Rectangle::from_xywh(0.0, 0.0, size, size);
        let source = BlendSourceDomain::new(rectangle, AffineTransform::identity()).unwrap();
        assert_eq!(source.rect(), rectangle);
    }
}

/// Extent is not paint and cannot satisfy stream content validation.
#[test]
fn a_declared_domain_creates_neither_paint_nor_an_empty_group_exception() {
    let zero = PaintAlphaFactor::new(0.0).unwrap();
    assert!(
        PaintStack::solid(CGColor::RED)
            .with_alpha_factor(zero)
            .is_empty()
    );
    assert_eq!(
        Stroke::new(
            PaintStack::solid(CGColor::TRANSPARENT),
            8.0,
            StrokeCap::Butt,
            StrokeJoin::Miter,
            4.0,
        ),
        Ok(None)
    );
    assert_eq!(
        FrameItems::try_new(vec![begin(1, domain()), FrameItem::ScopeEnd]),
        Err(FrameItemsError::EmptyScope { index: 0 })
    );
}

/// One stream owns completion and order. The source map does not rebase
/// children, collapse clip scopes into isolation, or combine mask phases.
#[test]
fn nested_domains_preserve_owners_maps_nodes_clips_and_mask_phases() {
    let clip = ClipPath::new_with_edge_mode(
        vec![
            ClipLayer::new(vec![
                ClipGeometry::new(AffineTransform::identity(), Geometry::Rect(domain().rect()))
                    .unwrap(),
            ])
            .unwrap(),
        ],
        ClipEdgeMode::Hard,
    )
    .unwrap();
    let child = BlendSourceDomain::new(
        Rectangle::from_xywh(0.0, 0.0, 20.0, 20.0),
        AffineTransform::from_acebdf(1.0, 0.0, 2.0, 0.0, 1.0, 3.0),
    )
    .unwrap();
    let items = vec![
        begin(1, domain()),
        FrameItem::ScopeBegin(Scope {
            owner: owner(2),
            effect: ScopeEffect::Clip(clip.clone()),
        }),
        FrameItem::MaskBegin(Mask::new(owner(3), MaskMode::Alpha, clip)),
        node(4, PaintStack::solid(CGColor::RED)),
        FrameItem::MaskSource,
        begin(5, child),
        node(6, PaintStack::solid(CGColor::WHITE)),
        FrameItem::ScopeEnd,
        FrameItem::MaskEnd,
        FrameItem::ScopeEnd,
        FrameItem::ScopeEnd,
    ];
    let checked = FrameItems::try_new(items.clone()).unwrap();
    assert_eq!(checked.iter().cloned().collect::<Vec<_>>(), items);
    assert_eq!(
        checked.nodes().map(|n| n.owner).collect::<Vec<_>>(),
        [owner(4), owner(6)]
    );
    assert!(
        checked
            .nodes()
            .all(|n| n.transform == AffineTransform::identity())
    );

    assert_eq!(
        FrameItems::try_new(vec![
            FrameItem::MaskBegin(Mask::new(
                owner(1),
                MaskMode::Alpha,
                ClipPath::new(vec![ClipLayer::new(Vec::new()).unwrap()]).unwrap(),
            )),
            begin(2, child),
            node(3, PaintStack::solid(CGColor::RED)),
            FrameItem::MaskSource,
        ]),
        Err(FrameItemsError::UnexpectedMaskSource { index: 3 })
    );
}

/// The declaration adds no structural nesting and earns no additional depth.
#[test]
fn domains_preserve_the_existing_scope_depth_bound() {
    let begins = || {
        (0..MAX_SCOPE_DEPTH)
            .map(|id| begin(id as u64, domain()))
            .collect::<Vec<_>>()
    };
    let mut maximum = begins();
    maximum.push(node(100, PaintStack::solid(CGColor::RED)));
    maximum.extend(std::iter::repeat_n(FrameItem::ScopeEnd, MAX_SCOPE_DEPTH));
    FrameItems::try_new(maximum).unwrap();
    let mut overflow = begins();
    overflow.push(begin(100, domain()));
    assert_eq!(
        FrameItems::try_new(overflow),
        Err(FrameItemsError::ScopeTooDeep {
            index: MAX_SCOPE_DEPTH
        })
    );
}

fn repeating_illustration(paints: PaintStack) -> Result<PatternPaint, PatternPaintError> {
    let items = Arc::new(
        FrameItems::try_new(vec![
            begin(1, domain()),
            node(2, paints),
            FrameItem::ScopeEnd,
        ])
        .unwrap(),
    );
    let pattern = PatternPaint::new(
        24.0,
        24.0,
        AffineTransform::from_acebdf(2.0, 0.0, 12.0, 0.0, 2.0, 16.0),
        Arc::clone(&items),
        1.0,
    )?;
    assert!(Arc::ptr_eq(pattern.items(), &items));
    assert_eq!(pattern.items().iter().next(), Some(&begin(1, domain())));
    Ok(pattern)
}

/// Program placement never rewrites a contained source-to-tile declaration,
/// and declarations cannot hide recursive programs from the depth check.
#[test]
fn repeating_programs_keep_tile_local_domains_immutable_and_recursion_bounded() {
    let mut pattern = repeating_illustration(PaintStack::solid(CGColor::RED)).unwrap();
    for expected in 2..=MAX_PATTERN_DEPTH {
        pattern = repeating_illustration(PaintStack::from_pattern(pattern)).unwrap();
        assert_eq!(pattern.depth(), expected);
    }
    assert_eq!(
        repeating_illustration(PaintStack::from_pattern(pattern)),
        Err(PatternPaintError::TooDeep)
    );
}
