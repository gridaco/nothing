//! Independent illustration-producer laws for complete opacity sources.
//! These are consumer and reuse laws, not browser conformance measurements.

use cg::{CGColor, GradientStop, Paint, Paints, RadialGradientPaint};
use math2::{transform::AffineTransform, Rectangle};
use n0::{
    glyphless::{self, BuildError},
    paint::{read_pixels, PaintCtx},
};
use rframe::{
    Frame, FrameItem, FrameItems, FrameNode, Geometry, Identity, IsolatedSourceDomain, PaintStack,
    Provenance, Scope, ScopeEffect, ScopeOpacity, ScopeOpacityGroup, VisualRef,
};

fn owner(id: u64) -> VisualRef {
    VisualRef::new(Identity::new(id), Provenance::new(id + 100))
}

fn domain(map: AffineTransform) -> IsolatedSourceDomain {
    IsolatedSourceDomain::new(Rectangle::from_xywh(6.0, 6.0, 50.0, 48.0), map).unwrap()
}

fn illustration(alpha: f32, source: Option<IsolatedSourceDomain>, map: AffineTransform) -> Frame {
    let mut group = ScopeOpacityGroup::new(ScopeOpacity::new(alpha).unwrap());
    if let Some(source) = source {
        group = group.with_source_domain(source);
    }
    let circle = Rectangle::from_xywh(6.0, 6.0, 48.0, 48.0);
    let sibling = Rectangle::from_xywh(38.0, 34.0, 18.0, 20.0);
    let paints =
        PaintStack::try_from_paints(Paints::new([Paint::RadialGradient(RadialGradientPaint {
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: CGColor::from_rgba(215, 104, 67, 255).into(),
                },
                GradientStop {
                    offset: 1.0,
                    color: CGColor::from_rgba(91, 172, 225, 102).into(),
                },
            ],
            ..Default::default()
        })]))
        .unwrap();
    Frame {
        owner: owner(0),
        bounds: Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0),
        items: FrameItems::try_new(vec![
            FrameItem::ScopeBegin(Scope {
                owner: owner(1),
                effect: ScopeEffect::Opacity(group),
            }),
            FrameItem::Node(FrameNode {
                owner: owner(2),
                transform: map,
                geometry: Geometry::Ellipse(circle),
                bounds: math2::rect_transform(circle, &map),
                paints,
                stroke: None,
            }),
            FrameItem::Node(FrameNode {
                owner: owner(3),
                transform: map,
                geometry: Geometry::Rect(sibling),
                bounds: math2::rect_transform(sibling, &map),
                paints: PaintStack::solid(CGColor::from_rgba(91, 172, 225, 153)),
                stroke: None,
            }),
            FrameItem::ScopeEnd,
        ])
        .unwrap(),
    }
}

fn raster(product: &glyphless::FrameProduct, view: AffineTransform) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((64, 64)).unwrap();
    surface
        .canvas()
        .clear(skia_safe::Color::from_rgb(66, 101, 137));
    let saves = surface.canvas().save_count();
    product
        .execute(surface.canvas(), &view, &PaintCtx::new(None))
        .unwrap();
    assert_eq!(surface.canvas().save_count(), saves);
    read_pixels(&mut surface, 64, 64)
}

#[test]
fn the_declared_source_changes_raster_without_changing_painted_geometry() {
    let map = AffineTransform::identity();
    for alpha in [0.998, 0.999] {
        let ordinary = illustration(alpha, None, map);
        let declared = illustration(alpha, Some(domain(map)), map);
        assert_eq!(ordinary.nodes(), declared.nodes());
        let a = glyphless::compile(ordinary).unwrap();
        let b = glyphless::compile(declared).unwrap();
        assert_ne!(raster(&a, map), raster(&b, map));
        let damage = glyphless::diff_frame(&a, &b);
        assert!(damage.changed.contains(&owner(1)));
    }
}

#[test]
fn a_mapped_source_replayed_at_a_changed_view_matches_fresh_with_balanced_saves() {
    let map = AffineTransform::from_acebdf(
        0.9659258,
        -0.25881904,
        9.372583,
        0.25881904,
        0.9659258,
        -7.1918354,
    );
    for alpha in [0.57, 0.998, 0.999] {
        let frame = illustration(alpha, Some(domain(map)), map);
        let retained = glyphless::compile(frame.clone()).unwrap();
        for view in [
            AffineTransform::identity(),
            AffineTransform::from_acebdf(0.75, 0.0, 1.25, 0.0, 1.125, -0.5),
        ] {
            let fresh = glyphless::compile(frame.clone()).unwrap();
            assert_eq!(raster(&retained, view), raster(&fresh, view));
        }
    }
}

#[test]
fn invalid_view_refuses_before_mutating_the_destination() {
    let map = AffineTransform::identity();
    let product = glyphless::compile(illustration(0.999, Some(domain(map)), map)).unwrap();
    let mut surface = skia_safe::surfaces::raster_n32_premul((64, 64)).unwrap();
    surface.canvas().clear(skia_safe::Color::MAGENTA);
    let before = read_pixels(&mut surface, 64, 64);
    let saves = surface.canvas().save_count();
    let bad = AffineTransform::from_acebdf(0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    assert!(product
        .execute(surface.canvas(), &bad, &PaintCtx::new(None))
        .is_err());
    assert_eq!(surface.canvas().save_count(), saves);
    assert_eq!(read_pixels(&mut surface, 64, 64), before);
}

#[test]
fn zero_area_unstroked_boxes_do_not_require_phantom_source_area() {
    let map = AffineTransform::identity();
    for alpha in [0.998, 0.999] {
        let base = illustration(alpha, Some(domain(map)), map);
        let expected = raster(&glyphless::compile(base.clone()).unwrap(), map);
        for (width, height) in [(0.0, 0.0), (0.0, 20.0), (20.0, 0.0)] {
            let bounds = Rectangle::from_xywh(0.25, 0.25, width, height);
            for geometry in [Geometry::Rect(bounds), Geometry::Ellipse(bounds)] {
                let mut disabled = base.nodes()[1].clone();
                disabled.owner = owner(4);
                disabled.geometry = geometry;
                disabled.bounds = bounds;
                let mut items: Vec<_> = base.items.iter().cloned().collect();
                items.insert(3, FrameItem::Node(disabled.clone()));
                let frame = Frame {
                    items: FrameItems::try_new(items).unwrap(),
                    ..base.clone()
                };
                assert_eq!(frame.nodes()[2], &disabled);
                assert_eq!(raster(&glyphless::compile(frame).unwrap(), map), expected);
            }
        }
    }
}

#[test]
fn incomplete_and_unconsumable_declarations_refuse_by_owner() {
    let map = AffineTransform::identity();
    let small = IsolatedSourceDomain::new(Rectangle::from_xywh(8.0, 8.0, 40.0, 40.0), map).unwrap();
    let moved = domain(AffineTransform::from_acebdf(1.0, 0.0, 1.0, 0.0, 1.0, 0.0));
    for source in [small, moved] {
        let result = glyphless::compile(illustration(0.999, Some(source), map));
        assert!(
            matches!(result, Err(BuildError::SourceDomain { owner: found, .. }) if found == owner(1))
        );
    }
    let frame = illustration(0.998, Some(domain(map)), map);
    let mut items: Vec<_> = frame.items.iter().cloned().collect();
    items.insert(
        1,
        FrameItem::ScopeBegin(Scope {
            owner: owner(4),
            effect: ScopeEffect::Opacity(ScopeOpacityGroup::new(ScopeOpacity::new(0.5).unwrap())),
        }),
    );
    items.insert(3, FrameItem::ScopeEnd);
    let frame = Frame {
        items: FrameItems::try_new(items).unwrap(),
        ..frame
    };
    assert!(
        matches!(glyphless::compile(frame), Err(BuildError::SourceDomain { owner: found, .. }) if found == owner(1))
    );
}

#[test]
fn image_effect_and_repeating_program_placement_refuse_by_owner() {
    use rframe::{
        ClipGeometry, ClipLayer, ClipPath, Filter, FilterColorSpace, FilterInput, FilterNode,
        FilterPrimitive, FilterProgram, Mask, MaskMode, PatternPaint,
    };
    use std::sync::Arc;
    let map = AffineTransform::identity();
    let frame = illustration(0.999, Some(domain(map)), map);
    let region = Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0);
    let program = FilterProgram::new(Arc::from([FilterNode::new(
        Arc::from([FilterInput::Source]),
        region,
        FilterColorSpace::Srgb,
        FilterPrimitive::GaussianBlur {
            sigma_x: 4.0,
            sigma_y: 4.0,
        },
    )]))
    .unwrap();
    let filter = Filter::new(map, region, program).unwrap();
    let mut filtered: Vec<_> = frame.items.iter().cloned().collect();
    filtered.insert(
        0,
        FrameItem::ScopeBegin(Scope {
            owner: owner(5),
            effect: ScopeEffect::Filter(filter),
        }),
    );
    filtered.push(FrameItem::ScopeEnd);
    let filtered = Frame {
        items: FrameItems::try_new(filtered).unwrap(),
        ..frame.clone()
    };
    assert!(
        matches!(glyphless::compile(filtered), Err(BuildError::SourceDomain { owner: found, .. }) if found == owner(1))
    );

    let clip = ClipPath::new(vec![ClipLayer::new(vec![ClipGeometry::new(
        map,
        Geometry::Rect(region),
    )
    .unwrap()])
    .unwrap()])
    .unwrap();
    let mask = FrameItem::MaskBegin(Mask::new(owner(5), MaskMode::Alpha, clip));
    let mut source = frame.nodes()[1].clone();
    source.owner = owner(6);
    for nested in [false, true] {
        let mut masked: Vec<_> = frame.items.iter().cloned().collect();
        if nested {
            masked.insert(1, mask.clone());
            masked.insert(3, FrameItem::MaskSource);
            masked.insert(4, FrameItem::Node(source.clone()));
            masked.insert(5, FrameItem::MaskEnd);
        } else {
            masked.insert(0, mask.clone());
            masked.extend([
                FrameItem::MaskSource,
                FrameItem::Node(source.clone()),
                FrameItem::MaskEnd,
            ]);
        }
        let masked = Frame {
            items: FrameItems::try_new(masked).unwrap(),
            ..frame.clone()
        };
        assert!(
            matches!(glyphless::compile(masked), Err(BuildError::SourceDomain { owner: found, .. }) if found == owner(1))
        );
    }
    let pattern = PatternPaint::new(64.0, 64.0, map, Arc::new(frame.items.clone()), 1.0).unwrap();
    let mut target = source;
    target.owner = owner(10);
    target.paints = PaintStack::from_pattern(pattern);
    let repeated = Frame {
        items: FrameItems::try_new(vec![FrameItem::Node(target)]).unwrap(),
        ..frame
    };
    assert!(
        matches!(glyphless::compile(repeated), Err(BuildError::Paint { owner: found, reason }) if found == owner(10) && reason.contains("source domain inside a repeating program"))
    );
}
