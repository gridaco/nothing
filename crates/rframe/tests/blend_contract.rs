//! Producer-only laws for isolated group blending.
//!
//! A small diagram producer originates these frames from geometry and explicit
//! composition choices. No document parser, authored model, or renderer is
//! involved: the contract must preserve those choices on its own.

use std::sync::Arc;

use cg::{BlendMode, CGColor, LinearGradientPaint, Paint, Paints, RadialGradientPaint, SolidPaint};
use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    ClipGeometry, ClipLayer, ClipPath, Filter, FilterColorSpace, FilterInput, FilterNode,
    FilterPrimitive, FilterProgram, Frame, FrameItem, FrameItems, FrameItemsError, FrameNode,
    Geometry, Identity, MAX_PATTERN_DEPTH, MAX_SCOPE_DEPTH, Mask, MaskMode, PaintAlphaFactor,
    PaintStack, PaintStackError, PatternPaint, PatternPaintError, Provenance, Scope, ScopeBlend,
    ScopeBlendMode, ScopeEffect, ScopeOpacity, VisualRef,
};

const MODES: [ScopeBlendMode; 3] = [
    ScopeBlendMode::Normal,
    ScopeBlendMode::Multiply,
    ScopeBlendMode::Screen,
];

fn owner(id: u64) -> VisualRef {
    VisualRef::new(Identity::new(id), Provenance::new(id + 1000))
}

fn rect() -> Rectangle {
    Rectangle::from_xywh(0.0, 0.0, 16.0, 16.0)
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

fn begin(id: u64, effect: ScopeEffect) -> FrameItem {
    FrameItem::ScopeBegin(Scope {
        owner: owner(id),
        effect,
    })
}

fn blend(id: u64, mode: ScopeBlendMode) -> FrameItem {
    begin(id, ScopeEffect::Blend(ScopeBlend::new(mode, None)))
}

fn clip() -> ClipPath {
    ClipPath::new(vec![
        ClipLayer::new(vec![
            ClipGeometry::new(AffineTransform::identity(), Geometry::Rect(rect())).unwrap(),
        ])
        .unwrap(),
    ])
    .unwrap()
}

fn mask(id: u64, mode: MaskMode) -> FrameItem {
    FrameItem::MaskBegin(Mask::new(owner(id), mode, clip()))
}

fn wrap(effect: ScopeEffect, children: Vec<FrameItem>) -> Vec<FrameItem> {
    let mut items = vec![begin(100, effect)];
    items.extend(children);
    items.push(FrameItem::ScopeEnd);
    items
}

fn checked(items: Vec<FrameItem>) -> FrameItems {
    let expected = items.clone();
    let checked = FrameItems::try_new(items).expect("resolved, balanced program");
    assert_eq!(
        checked.iter().cloned().collect::<Vec<_>>(),
        expected,
        "validation must preserve every fact, owner, and boundary in order"
    );
    checked
}

fn diagram(composition: Option<ScopeBlend>) -> Frame {
    // Two overlapping translucent shapes; the first also has a stroke and a
    // separate paint alpha factor. Group composition cannot rewrite any of them.
    let mut first = node(
        1,
        PaintStack::solid(CGColor::from_rgba(220, 40, 60, 192))
            .with_alpha_factor(PaintAlphaFactor::new(0.75).unwrap()),
    );
    let FrameItem::Node(first_node) = &mut first else {
        unreachable!()
    };
    first_node.stroke = rframe::Stroke::new(
        PaintStack::solid(CGColor::BLUE),
        2.0,
        rframe::StrokeCap::Round,
        rframe::StrokeJoin::Round,
        4.0,
    )
    .unwrap();
    let children = vec![
        first,
        node(2, PaintStack::solid(CGColor::from_rgba(40, 200, 80, 128))),
    ];
    Frame {
        owner: owner(200),
        bounds: rect(),
        items: checked(match composition {
            Some(composition) => wrap(ScopeEffect::Blend(composition), children),
            None => children,
        }),
    }
}

/// The exhaustive match is also an admission lock: growing the vocabulary
/// requires revisiting this producer's contract, not inheriting a shared enum.
#[test]
fn group_blending_admits_exactly_three_named_functions() {
    let names = MODES.map(|mode| match mode {
        ScopeBlendMode::Normal => "normal",
        ScopeBlendMode::Multiply => "multiply",
        ScopeBlendMode::Screen => "screen",
    });
    assert_eq!(names, ["normal", "multiply", "screen"]);
}

/// An explicit isolation boundary remains significant even if its own final
/// operation is neutral; a descendant may blend against that boundary later.
#[test]
fn an_independent_diagram_distinguishes_absence_isolation_and_blend() {
    let direct = diagram(None);
    let grouped = MODES.map(|mode| diagram(Some(ScopeBlend::new(mode, None))));
    assert_eq!(direct.items.len(), 2);
    for (index, frame) in grouped.iter().enumerate() {
        assert_eq!(frame.items.len(), 4);
        assert_eq!(frame.nodes(), direct.nodes());
        assert_ne!(frame, &direct);
        for other in &grouped[..index] {
            assert_ne!(frame, other);
        }
        let FrameItem::ScopeBegin(scope) = frame.items.iter().next().unwrap() else {
            panic!("unit opacity never erases the isolation boundary");
        };
        assert_eq!(scope.owner, owner(100));
        assert_eq!(
            scope.effect,
            ScopeEffect::Blend(ScopeBlend::new(MODES[index], None))
        );
    }
}

/// Optional checked opacity adds the missing unit case without relaxing the
/// existing open interval or smuggling a raw scalar into a checked stream.
#[test]
fn unit_opacity_is_absence_of_attenuation_and_fractional_opacity_stays_exact() {
    for mode in MODES {
        let unit = ScopeBlend::new(mode, None);
        assert_eq!(unit.mode(), mode);
        assert_eq!(unit.opacity(), None);
        for value in [
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            0.375,
            1.0_f32.next_down(),
        ] {
            let opacity = ScopeOpacity::new(value).unwrap();
            let group = ScopeBlend::new(mode, Some(opacity));
            assert_eq!(group.mode(), mode);
            assert_eq!(group.opacity().unwrap().get().to_bits(), value.to_bits());
            assert_ne!(group, unit);
        }
    }
    for value in [
        0.0,
        -0.0,
        1.0,
        -0.5,
        1.5,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        let error = ScopeOpacity::new(value).expect_err("opacity domain stays unchanged");
        assert_eq!(error.value.to_bits(), value.to_bits());
    }
}

/// A combined operation must survive as one scope, because a nested opacity
/// scope makes a different backdrop for its enclosed blend.
#[test]
fn combined_blend_and_opacity_is_distinct_from_two_nested_operations() {
    let opacity = ScopeOpacity::new(0.375).unwrap();
    let combined = diagram(Some(ScopeBlend::new(
        ScopeBlendMode::Multiply,
        Some(opacity),
    )));
    let blended = diagram(Some(ScopeBlend::new(ScopeBlendMode::Multiply, None)));
    let mut nested = blended.clone();
    let mut items = vec![begin(101, ScopeEffect::Opacity(opacity))];
    items.extend(blended.items.iter().cloned());
    items.push(FrameItem::ScopeEnd);
    nested.items = checked(items);

    assert_eq!(combined.items.len(), 4);
    assert_eq!(nested.items.len(), 6);
    assert_eq!(combined.nodes(), nested.nodes());
    assert_ne!(combined, nested);
    assert_ne!(combined, blended);
}

/// Isolation must remain present around a blending descendant, including when
/// the outer span contains only another scope rather than a direct node.
#[test]
fn unit_normal_isolation_retains_a_nested_blending_program() {
    let children = diagram(Some(ScopeBlend::new(ScopeBlendMode::Screen, None)));
    let mut items = vec![blend(101, ScopeBlendMode::Normal)];
    items.extend(children.items.iter().cloned());
    items.push(FrameItem::ScopeEnd);
    let isolated = checked(items);
    assert_eq!(isolated.len(), children.items.len() + 2);
    assert_eq!(isolated.nodes().collect::<Vec<_>>(), children.nodes());
    assert_ne!(isolated, children.items);
}

/// No new effect gets a separate or weaker path through stream validation.
#[test]
fn blend_scopes_reject_empty_unclosed_and_unopened_boundaries() {
    for mode in MODES {
        assert_eq!(
            FrameItems::try_new(vec![blend(1, mode), FrameItem::ScopeEnd]),
            Err(FrameItemsError::EmptyScope { index: 0 })
        );
        assert_eq!(
            FrameItems::try_new(vec![
                blend(1, mode),
                node(2, PaintStack::solid(CGColor::RED))
            ]),
            Err(FrameItemsError::UnclosedScope { index: 0 })
        );
        assert_eq!(
            FrameItems::try_new(vec![
                blend(1, mode),
                node(2, PaintStack::solid(CGColor::RED)),
                FrameItem::ScopeEnd,
                FrameItem::ScopeEnd,
            ]),
            Err(FrameItemsError::UnopenedScopeEnd { index: 3 })
        );
    }
}

/// Both mask phases are independent composites that can contain groups, and a
/// whole masked result can itself belong to an enclosing blend group.
#[test]
fn blending_can_enclose_masks_and_nest_in_either_mask_phase() {
    for mode in [MaskMode::Alpha, MaskMode::Luminance] {
        checked(vec![
            blend(1, ScopeBlendMode::Multiply),
            mask(2, mode),
            blend(3, ScopeBlendMode::Screen),
            node(4, PaintStack::solid(CGColor::RED)),
            FrameItem::ScopeEnd,
            FrameItem::MaskSource,
            blend(5, ScopeBlendMode::Normal),
            node(6, PaintStack::solid(CGColor::WHITE)),
            FrameItem::ScopeEnd,
            FrameItem::MaskEnd,
            FrameItem::ScopeEnd,
        ]);
        checked(vec![
            mask(1, mode),
            blend(2, ScopeBlendMode::Normal),
            node(3, PaintStack::solid(CGColor::RED)),
            FrameItem::ScopeEnd,
            FrameItem::MaskSource,
            FrameItem::MaskEnd,
        ]);
    }
}

/// Mask phase markers cannot close or bypass a blend boundary in either
/// direction, even though all effects use the same bounded stack.
#[test]
fn blend_boundaries_cannot_be_crossed_by_mask_phase_markers() {
    for mode in MODES {
        assert_eq!(
            FrameItems::try_new(vec![
                mask(1, MaskMode::Alpha),
                blend(2, mode),
                node(3, PaintStack::solid(CGColor::RED)),
                FrameItem::MaskSource,
            ]),
            Err(FrameItemsError::UnexpectedMaskSource { index: 3 })
        );
        assert_eq!(
            FrameItems::try_new(vec![
                mask(1, MaskMode::Alpha),
                node(2, PaintStack::solid(CGColor::RED)),
                FrameItem::MaskSource,
                blend(3, mode),
                node(4, PaintStack::solid(CGColor::WHITE)),
                FrameItem::MaskEnd,
            ]),
            Err(FrameItemsError::UnexpectedMaskEnd { index: 5 })
        );
        assert_eq!(
            FrameItems::try_new(vec![
                blend(1, mode),
                mask(2, MaskMode::Alpha),
                node(3, PaintStack::solid(CGColor::RED)),
                FrameItem::ScopeEnd,
            ]),
            Err(FrameItemsError::UnopenedScopeEnd { index: 3 })
        );
    }
}

/// The bound counts all simultaneously open scopes and masks, with the same
/// inclusive limit for old and new effects.
#[test]
fn blends_and_masks_share_the_existing_depth_bound() {
    let begins = || {
        (0..MAX_SCOPE_DEPTH)
            .map(|id| blend(id as u64, MODES[id % 3]))
            .collect::<Vec<_>>()
    };
    let mut maximum = begins();
    maximum.push(node(100, PaintStack::solid(CGColor::RED)));
    maximum.extend(std::iter::repeat_n(FrameItem::ScopeEnd, MAX_SCOPE_DEPTH));
    checked(maximum);

    let mut overflow = begins();
    overflow.push(blend(100, ScopeBlendMode::Normal));
    assert_eq!(
        FrameItems::try_new(overflow),
        Err(FrameItemsError::ScopeTooDeep {
            index: MAX_SCOPE_DEPTH
        })
    );
    let mut overflow = begins();
    overflow.push(mask(100, MaskMode::Alpha));
    assert_eq!(
        FrameItems::try_new(overflow),
        Err(FrameItemsError::MaskTooDeep {
            index: MAX_SCOPE_DEPTH
        })
    );

    let mut masked = vec![mask(100, MaskMode::Alpha)];
    masked.extend(begins());
    assert_eq!(
        FrameItems::try_new(masked),
        Err(FrameItemsError::ScopeTooDeep {
            index: MAX_SCOPE_DEPTH
        })
    );

    let mut maximum = begins();
    maximum.pop();
    maximum.extend([
        mask(100, MaskMode::Alpha),
        node(101, PaintStack::solid(CGColor::RED)),
        FrameItem::MaskSource,
        FrameItem::MaskEnd,
    ]);
    maximum.extend(std::iter::repeat_n(
        FrameItem::ScopeEnd,
        MAX_SCOPE_DEPTH - 1,
    ));
    checked(maximum);
}

fn repeating_masked_group(paints: PaintStack) -> Result<PatternPaint, PatternPaintError> {
    // Put recursion in a mask-source stroke to prove neither mask markers nor
    // group scopes hide nested programs from the existing depth calculation.
    let mut source = node(4, PaintStack::empty());
    let FrameItem::Node(source_node) = &mut source else {
        unreachable!()
    };
    source_node.stroke = rframe::Stroke::new(
        paints,
        1.0,
        rframe::StrokeCap::Butt,
        rframe::StrokeJoin::Round,
        4.0,
    )
    .unwrap();
    let items = checked(vec![
        blend(1, ScopeBlendMode::Normal),
        mask(2, MaskMode::Alpha),
        node(3, PaintStack::solid(CGColor::WHITE)),
        FrameItem::MaskSource,
        blend(5, ScopeBlendMode::Multiply),
        source,
        FrameItem::ScopeEnd,
        FrameItem::MaskEnd,
        FrameItem::ScopeEnd,
    ]);
    let shared = Arc::new(items);
    let pattern = PatternPaint::new(
        16.0,
        16.0,
        AffineTransform::identity(),
        Arc::clone(&shared),
        1.0,
    )?;
    assert!(
        Arc::ptr_eq(pattern.items(), &shared),
        "the checked program stays shared and immutable"
    );
    Ok(pattern)
}

/// Repeating programs admit exactly the same group and mask facts as a frame;
/// wrapping a recursive paint in them cannot reset its depth.
#[test]
fn immutable_repeating_programs_preserve_blends_and_the_recursion_bound() {
    let mut pattern = repeating_masked_group(PaintStack::solid(CGColor::RED)).unwrap();
    assert_eq!(pattern.depth(), 1);
    for depth in 2..=MAX_PATTERN_DEPTH {
        pattern = repeating_masked_group(PaintStack::from_pattern(pattern)).unwrap();
        assert_eq!(pattern.depth(), depth);
    }
    assert_eq!(
        repeating_masked_group(PaintStack::from_pattern(pattern)),
        Err(PatternPaintError::TooDeep)
    );
}

/// Legacy programs remain exact children of a new group. The only admitted
/// empty effect is still a filter that explicitly generates its own source.
#[test]
fn blending_preserves_opacity_clip_filter_and_mask_programs() {
    let program = FilterProgram::new(Arc::from([FilterNode::new(
        Arc::from([FilterInput::Source]),
        rect(),
        FilterColorSpace::Srgb,
        FilterPrimitive::GaussianBlur {
            sigma_x: 1.0,
            sigma_y: 2.0,
        },
    )]))
    .unwrap();
    let filter = Filter::new(AffineTransform::identity(), rect(), program).unwrap();
    let legacy = checked(vec![
        begin(1, ScopeEffect::Opacity(ScopeOpacity::new(0.5).unwrap())),
        begin(2, ScopeEffect::Clip(clip())),
        begin(3, ScopeEffect::Filter(filter)),
        mask(4, MaskMode::Luminance),
        node(5, PaintStack::solid(CGColor::RED)),
        FrameItem::MaskSource,
        node(6, PaintStack::solid(CGColor::WHITE)),
        FrameItem::MaskEnd,
        FrameItem::ScopeEnd,
        FrameItem::ScopeEnd,
        FrameItem::ScopeEnd,
    ]);
    for mode in MODES {
        let enclosed = checked(wrap(
            ScopeEffect::Blend(ScopeBlend::new(mode, None)),
            legacy.iter().cloned().collect(),
        ));
        assert_eq!(
            enclosed
                .iter()
                .skip(1)
                .take(legacy.len())
                .cloned()
                .collect::<Vec<_>>(),
            legacy.iter().cloned().collect::<Vec<_>>()
        );
    }

    let generated = FilterProgram::new(Arc::from([FilterNode::new(
        Arc::from([]),
        rect(),
        FilterColorSpace::Srgb,
        FilterPrimitive::SolidColor {
            color: CGColor::RED.into(),
        },
    )]))
    .unwrap();
    let generated = Filter::new(AffineTransform::identity(), rect(), generated)
        .unwrap()
        .with_transparent_source();
    checked(vec![
        blend(1, ScopeBlendMode::Normal),
        begin(2, ScopeEffect::Filter(generated)),
        FrameItem::ScopeEnd,
        FrameItem::ScopeEnd,
    ]);
}

/// Admitting group functions cannot silently grant the same functions to any
/// visible leaf kind, including gradients used by fill or stroke stacks.
#[test]
fn group_multiply_and_screen_remain_forbidden_on_every_leaf_kind() {
    let stops = || {
        vec![
            cg::GradientStop {
                offset: 0.0,
                color: CGColor::RED.into(),
            },
            cg::GradientStop {
                offset: 1.0,
                color: CGColor::BLUE.into(),
            },
        ]
    };
    for mode in [BlendMode::Multiply, BlendMode::Screen] {
        let mut solid = SolidPaint::new_color(CGColor::RED);
        solid.blend_mode = mode;
        for paint in [
            Paint::Solid(solid),
            Paint::LinearGradient(LinearGradientPaint {
                blend_mode: mode,
                stops: stops(),
                ..Default::default()
            }),
            Paint::RadialGradient(RadialGradientPaint {
                blend_mode: mode,
                stops: stops(),
                ..Default::default()
            }),
        ] {
            assert_eq!(
                PaintStack::try_from_paints(Paints::new([paint])),
                Err(PaintStackError { index: 0 })
            );
        }
    }
}
