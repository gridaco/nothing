//! Computed group ownership and refusal transactions. Pixel meaning is
//! independently guarded by the Chromium cells, not by these frame assertions.
use rframe::{Frame, FrameItem, ScopeBlend, ScopeBlendMode, ScopeEffect};
use websem::{InitialViewport, SvgFrameSource, compile_standalone_svg};

fn svg(body: &str) -> String {
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">{body}</svg>"#)
}
fn frame(body: &str) -> Frame {
    compile_standalone_svg(&svg(body), InitialViewport::new(64.0, 64.0)).unwrap()
}
fn blends(frame: &Frame) -> Vec<ScopeBlend> {
    frame
        .items
        .iter()
        .filter_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match scope.effect {
                ScopeEffect::Blend(blend) => Some(blend),
                _ => None,
            },
            _ => None,
        })
        .collect()
}
const RECT: &str = r#"<rect width="40" height="40" fill="red"/>"#;
const RAMP: &str = "<defs><linearGradient id='r'><stop stop-color='#cd6843'/><stop offset='1' stop-color='#5bace1' stop-opacity='.6'/></linearGradient></defs>";
const RAMP_RECT: &str = "<rect x='8.3' y='12.7' width='38.2' height='28.4' fill='url(#r)'/>";

#[test]
fn paintless_effects_and_disabled_servers_keep_the_source_patrol() {
    let ghost = "<rect x='3.2' y='5.7' width='13.4' height='17.2' fill='transparent'/>";
    let definitions = "<defs><filter id='f'><feOffset dx='8' dy='8'/></filter><clipPath id='c'><rect width='2' height='2'/></clipPath><mask id='m'><rect width='64' height='64' fill='white'/></mask><linearGradient id='s' gradientTransform='scale(0)'><stop stop-color='red'/><stop offset='1' stop-color='blue'/></linearGradient></defs>";
    let mut siblings = Vec::new();
    siblings.push(format!("<defs><pattern id='p' patternUnits='userSpaceOnUse' width='8' height='8'><rect width='8' height='8' fill='lime'/></pattern></defs>{}", ghost.replace("fill='transparent'", "fill='url(#p)' fill-opacity='0'")));
    for filter in [
        "<filter id='hide'/>",
        "<filter id='hide' filterUnits='userSpaceOnUse' width='0' height='64'><feOffset dx='8' dy='8'/></filter>",
    ] {
        for target in [
            ghost.replace("/>", " filter='url(#hide)'/>"),
            format!("<g filter='url(#hide)'>{ghost}</g>"),
            format!("<svg width='64' height='64' filter='url(#hide)'>{ghost}</svg>"),
            format!(
                "<defs>{}</defs><use href='#ghost' filter='url(#hide)'/>",
                ghost.replace("<rect ", "<rect id='ghost' ")
            ),
        ] {
            siblings.push(format!("<defs>{filter}</defs>{target}"));
        }
    }
    for effect in ["filter='url(#f)'", "mask='url(#m)'", "clip-path='url(#c)'"] {
        siblings.push(ghost.replace("/>", &format!(" {effect}/>")));
        siblings.push(format!("<g {effect}>{ghost}</g>"));
    }
    siblings.push(format!("<svg width='2' height='2'>{ghost}</svg>"));
    siblings.push(format!(
        "<svg width='2' height='2' overflow='visible'>{ghost}</svg>"
    ));
    siblings.push(ghost.replace("fill='transparent'", "fill='url(#s)'"));
    siblings.push(format!(
        "<defs><linearGradient id='stopless' gradientTransform='scale(0)'/></defs>{}",
        ghost.replace("fill='transparent'", "fill='url(#stopless)'")
    ));
    // A declared source must also cover every retained painted node, even
    // when its zero-width geometry paints no pixels.
    siblings.push(format!(
        "{}<rect x='-100' y='-100' width='0' height='8' fill='red'/>",
        RAMP_RECT.replace("/>", " stroke='transparent' stroke-width='4'/>")
    ));
    for mode in ["multiply", "screen"] {
        for sibling in &siblings {
            let source = svg(&format!(
                "{RAMP}{definitions}<g style='mix-blend-mode:{mode}'>{RAMP_RECT}{sibling}</g>{RECT}"
            ));
            let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
                .unwrap_err()
                .to_string();
            assert!(
                strict.contains("linear-gradient source-extent"),
                "{strict}: {source}"
            );
            let best = SvgFrameSource::from_standalone_svg_best_effort(
                source.as_str(),
                InitialViewport::new(64.0, 64.0),
            )
            .unwrap();
            assert_eq!(
                best.base_frame().items.len(),
                1,
                "complete group rollback: {source}"
            );
            assert!(
                best.degradations().iter().any(|d| d.path() == "svg/g[1]"
                    && d.reason().contains("linear-gradient source-extent")),
                "{:?}",
                best.degradations()
            );
        }
    }
}

#[test]
fn nonpainted_sibling_membership_is_independent_of_visible_paint() {
    for mode in ["multiply", "screen"] {
        for (attrs, expected) in [
            ("fill='transparent'", Some((3.0, 5.0, 44.0, 37.0))),
            ("fill='red' fill-opacity='0'", Some((3.0, 5.0, 44.0, 37.0))),
            ("fill='url(#empty)'", Some((3.0, 5.0, 44.0, 37.0))),
            (
                "fill='none' stroke='transparent' stroke-width='4'",
                Some((1.0, 3.0, 46.0, 39.0)),
            ),
            (
                "fill='none' stroke='red' stroke-opacity='0' stroke-width='4'",
                Some((1.0, 3.0, 46.0, 39.0)),
            ),
            ("fill='none' stroke='none'", None),
            ("fill='url(#missing)'", None),
        ] {
            let sibling = format!("<rect x='3.2' y='5.7' width='13.4' height='17.2' {attrs}/>");
            let source = svg(&format!(
                "{RAMP}<defs><linearGradient id='empty'/></defs><g style='mix-blend-mode:{mode}'>{RAMP_RECT}{sibling}</g>"
            ));
            let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0)).unwrap();
            let best = SvgFrameSource::from_standalone_svg_best_effort(
                source.as_str(),
                InitialViewport::new(64.0, 64.0),
            )
            .unwrap();
            assert_eq!(strict, best.base_frame());
            assert!(
                !best
                    .degradations()
                    .iter()
                    .any(|d| d.action() == websem::DegradationAction::Skipped)
            );
            let actual = blends(&strict)
                .into_iter()
                .find_map(|b| b.source_domain())
                .map(|d| d.rect());
            assert_eq!(
                actual,
                expected.map(|(x, y, w, h)| math2::Rectangle::from_xywh(x, y, w, h)),
                "{mode}: {attrs}"
            );
            assert_eq!(
                strict.nodes().len(),
                2,
                "retained geometry is not fake paint"
            );
            assert_eq!(
                strict.nodes()[1].geometry,
                rframe::Geometry::Rect(math2::Rectangle::from_xywh(3.2, 5.7, 13.4, 17.2))
            );
            assert!(strict.nodes()[1].paints.is_empty());
            assert!(strict.nodes()[1].stroke.is_none());
        }
    }
}

#[test]
fn absent_paint_sibling_does_not_enlarge_an_existing_declared_source() {
    let omitted = RAMP_RECT.replace("/>", " stroke='transparent' stroke-width='4'/>");
    for attrs in [
        "fill='none'",
        "fill='url(#missing)'",
        "fill='transparent' width='0'",
    ] {
        let sibling = if attrs.contains("width=") {
            format!("<rect x='-100' y='-100' height='8' {attrs}/>")
        } else {
            format!("<rect x='-100' y='-100' width='8' height='8' {attrs}/>")
        };
        let result = frame(&format!(
            "{RAMP}<g style='mix-blend-mode:screen'>{omitted}{sibling}</g>"
        ));
        let domain = blends(&result)
            .into_iter()
            .find_map(|b| b.source_domain())
            .unwrap();
        assert_eq!(
            domain.rect(),
            math2::Rectangle::from_xywh(6.0, 10.0, 43.0, 34.0)
        );
        assert_eq!(result.nodes().len(), 2);
    }
}

#[test]
fn sibling_contributions_do_not_create_domains_without_a_live_linear_source() {
    for content in [
        "<rect width='16' height='16' fill='transparent'/>",
        "<rect width='16' height='16' stroke='transparent' stroke-width='4'/>",
    ] {
        for wrapper in [
            format!("{content}"),
            format!("<g style='mix-blend-mode:multiply'>{content}</g>"),
        ] {
            let result = frame(&wrapper);
            assert!(
                blends(&result)
                    .into_iter()
                    .all(|b| b.source_domain().is_none())
            );
        }
    }
}

#[test]
fn linear_source_extent_patrol_is_transactional_and_names_the_owner() {
    for content in [
        format!("<g transform='translate(2.5 3.5)'>{RAMP_RECT}</g>"),
        format!("{RAMP_RECT}<g transform='scale(2)'>{RECT}</g>"),
        format!("<g opacity='.5'>{RAMP_RECT}{RECT}</g>"),
        format!("{RAMP_RECT}<g opacity='0'>{RECT}</g>"),
        format!("{RAMP_RECT}<g style='mix-blend-mode:screen'>{RECT}</g>"),
        format!("<svg x='3' y='5' width='56' height='48'>{RAMP_RECT}</svg>"),
        RAMP_RECT.replace(
            "/>",
            " stroke='transparent' stroke-width='4' vector-effect='non-scaling-stroke'/>",
        ),
        RAMP_RECT.replace("/>", " stroke='transparent' stroke-width='100000000'/>"),
        format!(
            "<defs><linearGradient id='empty'/></defs>{}",
            RAMP_RECT.replace("/>", " stroke='url(#empty)' stroke-width='4'/>")
        ),
        RAMP_RECT.replace("/>", " stroke='url(#missing)' stroke-width='4'/>"),
        RAMP_RECT.replace("/>", " stroke='context-stroke' stroke-width='4'/>"),
        format!(
            "<defs>{}</defs><use href='#ctx' fill='none'/>",
            RAMP_RECT.replace("/>", " id='ctx' stroke='context-fill' stroke-width='4'/>")
        ),
        RAMP_RECT.replace("/>", " stroke='url(#missing) none' stroke-width='4'/>"),
        format!(
            "<defs><rect id='wrong'/></defs>{}",
            RAMP_RECT.replace("/>", " stroke='url(#wrong)' stroke-width='4'/>")
        ),
    ] {
        for style in [
            "mix-blend-mode:multiply",
            "mix-blend-mode:screen",
            "isolation:isolate",
        ] {
            let source = svg(&format!("{RAMP}<g style='{style}'>{content}</g>{RECT}"));
            let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
                .unwrap_err()
                .to_string();
            assert!(
                strict.contains("linear-gradient source-extent"),
                "{strict}: {source}"
            );
            let best = SvgFrameSource::from_standalone_svg_best_effort(
                source.as_str(),
                InitialViewport::new(64.0, 64.0),
            )
            .unwrap();
            assert_eq!(
                best.base_frame().items.len(),
                1,
                "the entire failed group is rolled back: {source}"
            );
            assert!(
                best.degradations().iter().any(|d| d.path() == "svg/g[1]"
                    && d.reason().contains("linear-gradient source-extent")),
                "{:?}",
                best.degradations()
            );
        }
    }
}

#[test]
fn omitted_solid_stroke_changes_only_the_complete_source_domain() {
    use math2::{Rectangle, transform::AffineTransform};
    let source = |attrs: &str| {
        svg(&format!(
            "{RAMP}<g style='mix-blend-mode:multiply'>{}</g>",
            RAMP_RECT.replace("/>", &format!(" {attrs}/>"))
        ))
    };
    let ordinary = compile_standalone_svg(
        &source("stroke='none' stroke-width='4'"),
        InitialViewport::new(64.0, 64.0),
    )
    .unwrap();
    for (attrs, expected) in [
        (
            "stroke='transparent' stroke-width='4'",
            Rectangle::from_xywh(6.0, 10.0, 43.0, 34.0),
        ),
        (
            "stroke='red' stroke-opacity='0' stroke-width='4'",
            Rectangle::from_xywh(6.0, 10.0, 43.0, 34.0),
        ),
        (
            "stroke='transparent' stroke-width='8'",
            Rectangle::from_xywh(4.0, 8.0, 47.0, 38.0),
        ),
    ] {
        let source = source(attrs);
        let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0)).unwrap();
        let best = SvgFrameSource::from_standalone_svg_best_effort(
            source.as_str(),
            InitialViewport::new(64.0, 64.0),
        )
        .unwrap();
        assert_eq!(strict, best.base_frame());
        assert!(
            !best
                .degradations()
                .iter()
                .any(|d| d.action() == websem::DegradationAction::Skipped)
        );
        assert_eq!(strict.owner, ordinary.owner);
        assert_eq!(strict.bounds, ordinary.bounds);
        assert_eq!(
            strict.nodes(),
            ordinary.nodes(),
            "no fake stroke, paint, or geometry bounds"
        );
        assert_ne!(
            strict, ordinary,
            "the former full-Frame collapse is impossible"
        );
        let mut domains = Vec::new();
        let without_domain = strict
            .items
            .iter()
            .cloned()
            .map(|item| match item {
                FrameItem::ScopeBegin(mut scope) => {
                    if let ScopeEffect::Blend(blend) = scope.effect {
                        if let Some(domain) = blend.source_domain() {
                            domains.push(domain);
                        }
                        scope.effect =
                            ScopeEffect::Blend(ScopeBlend::new(blend.mode(), blend.opacity()));
                    }
                    FrameItem::ScopeBegin(scope)
                }
                item => item,
            })
            .collect();
        assert_eq!(
            rframe::FrameItems::try_new(without_domain).unwrap(),
            ordinary.items
        );
        assert_eq!(domains.len(), 1, "root isolation makes no declaration");
        assert_eq!(domains[0].rect(), expected);
        assert_eq!(domains[0].source_to_stream(), AffineTransform::identity());
    }
    let zero = compile_standalone_svg(
        &source("stroke='transparent' stroke-width='0'"),
        InitialViewport::new(64.0, 64.0),
    )
    .unwrap();
    assert_eq!(zero, ordinary, "zero width contributes no source extent");
}

#[test]
fn source_domain_is_complete_order_independent_and_absent_on_elided_isolation() {
    let omitted = RAMP_RECT.replace("/>", " stroke='transparent' stroke-width='4'/>");
    let sibling = "<rect x='3' y='5' width='13' height='17' fill='purple'/>";
    for content in [
        format!("{omitted}{sibling}"),
        format!("{sibling}<g>{omitted}</g>"),
    ] {
        let result = frame(&format!(
            "{RAMP}<g style='mix-blend-mode:screen'>{content}</g>"
        ));
        let domain = blends(&result)
            .into_iter()
            .find_map(|blend| blend.source_domain())
            .unwrap();
        assert_eq!(
            domain.rect(),
            math2::Rectangle::from_xywh(3.0, 5.0, 46.0, 39.0)
        );
    }
    for content in [
        omitted.clone(),
        format!("<g style='isolation:isolate'>{omitted}</g>"),
    ] {
        assert!(blends(&frame(&format!("{RAMP}{content}"))).is_empty());
    }
    let result = frame(&format!(
        "{RAMP}<g opacity='.5'><g style='mix-blend-mode:screen'>{omitted}</g></g>"
    ));
    assert_eq!(
        blends(&result)
            .into_iter()
            .filter(|blend| blend.source_domain().is_some())
            .count(),
        1
    );
}

#[test]
fn an_omitted_stroke_cannot_make_an_unknown_sibling_domain_complete() {
    let source = svg(&format!(
        "{RAMP}<defs><pattern id='p' width='8' height='8' patternUnits='userSpaceOnUse'><rect width='4' height='8'/></pattern></defs><g style='mix-blend-mode:multiply'>{}<rect width='12' height='12' fill='url(#p)'/></g>{RECT}",
        RAMP_RECT.replace("/>", " stroke='transparent' stroke-width='4'/>")
    ));
    let error = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
        .unwrap_err()
        .to_string();
    assert!(error.contains("linear-gradient source-extent"), "{error}");
    let best = SvgFrameSource::from_standalone_svg_best_effort(
        source.as_str(),
        InitialViewport::new(64.0, 64.0),
    )
    .unwrap();
    assert_eq!(best.base_frame().items.len(), 1);
    assert!(best.degradations().iter().any(|d| d.path() == "svg/g[1]" && d.reason().contains("linear-gradient source-extent")));
}

#[test]
fn source_enclosure_rounding_refuses_at_the_svg_owner_before_consumer_preflight() {
    let narrow = RAMP_RECT
        .replace("width='38.2'", "width='.7'")
        .replace("/>", " stroke='transparent' stroke-width='.0000001'/>");
    for mode in ["multiply", "screen"] {
        let source = svg(&format!(
            "{RAMP}<g style='mix-blend-mode:{mode}'>{narrow}</g>{RECT}"
        ));
        let error = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
            .unwrap_err()
            .to_string();
        assert!(error.contains("linear-gradient source-extent"), "{error}");
        let best = SvgFrameSource::from_standalone_svg_best_effort(
            source.as_str(),
            InitialViewport::new(64.0, 64.0),
        )
        .unwrap();
        assert_eq!(best.base_frame().items.len(), 1);
        assert!(best.degradations().iter().any(
            |d| d.path() == "svg/g[1]" && d.reason().contains("linear-gradient source-extent")
        ));
    }
}

#[test]
fn root_linear_source_extent_refusal_cannot_silently_fall_back() {
    let base = svg(&format!(
        "{RAMP}{RAMP_RECT}<g style='mix-blend-mode:screen'>{RECT}</g>"
    ));
    for opacity in ["1", ".5", ".999"] {
        let source = base.replace("width=\"64\"", &format!("opacity='{opacity}' width=\"64\""));
        for result in [
            SvgFrameSource::from_standalone_svg(source.as_str(), InitialViewport::new(64.0, 64.0)),
            SvgFrameSource::from_standalone_svg_best_effort(
                source.as_str(),
                InitialViewport::new(64.0, 64.0),
            ),
        ] {
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("linear-gradient source-extent")
            );
        }
    }
}

#[test]
fn root_linear_blend_sources_cannot_borrow_the_child_domain_exemption() {
    for (style, stroke) in [
        ("mix-blend-mode:multiply", "none"),
        ("mix-blend-mode:screen", "none"),
        ("mix-blend-mode:multiply", "transparent"),
        ("mix-blend-mode:screen", "transparent"),
        ("isolation:isolate", "transparent"),
    ] {
        for backdrop in ["", "<rect width='64' height='64' fill='#426589'/>"] {
            let source = svg(&format!(
                "{RAMP}{backdrop}{}",
                RAMP_RECT.replace("/>", &format!(" stroke='{stroke}' stroke-width='4'/>"))
            ))
            .replacen("<svg ", &format!("<svg style='{style}' "), 1);
            for result in [
                SvgFrameSource::from_standalone_svg(
                    source.as_str(),
                    InitialViewport::new(64.0, 64.0),
                ),
                SvgFrameSource::from_standalone_svg_best_effort(
                    source.as_str(),
                    InitialViewport::new(64.0, 64.0),
                ),
            ] {
                let error = result.unwrap_err().to_string();
                assert!(
                    error.contains("linear-gradient source-extent"),
                    "{error}: {source}"
                );
            }
        }
    }
    // An elided root Normal boundary without a missing source contribution
    // keeps its existing route; ordinary ramps acquire no new root refusal.
    frame(&format!("{RAMP}{RAMP_RECT}"));
    compile_standalone_svg(
        &svg(&format!("{RAMP}{RAMP_RECT}")).replacen("<svg ", "<svg style='isolation:isolate' ", 1),
        InitialViewport::new(64.0, 64.0),
    )
    .unwrap();
}

#[test]
fn completed_linear_blend_images_do_not_poison_outer_solid_groups() {
    for content in [
        RAMP_RECT.to_string(),
        format!("<g>{RAMP_RECT}</g>{RECT}"),
        format!("{RAMP_RECT}<rect width='12' height='12' display='none'/>"),
    ] {
        frame(&format!(
            "{RAMP}<g opacity='.5'><g style='mix-blend-mode:multiply' opacity='.6'>{content}</g></g>"
        ));
    }
    // The new patrol is source-specific; ordinary transformed solids remain
    // admitted, and an ordinary ramp without blending keeps its old route.
    frame(&format!(
        "{RAMP}<g style='mix-blend-mode:screen'>{}</g>",
        RAMP_RECT.replace(
            "fill='url(#r)'",
            "fill='transparent' stroke='url(#r)' stroke-width='4'"
        )
    ));
    for attrs in [
        "stroke='none' stroke-width='4'",
        "stroke='transparent' stroke-width='0'",
    ] {
        frame(&format!(
            "{RAMP}<g style='mix-blend-mode:multiply'>{}</g>",
            RAMP_RECT.replace("/>", &format!(" {attrs}/>"))
        ));
    }
    frame(&format!(
        "<g style='mix-blend-mode:screen' transform='rotate(15)'>{RECT}</g>"
    ));
    frame(&format!(
        "{RAMP}<g transform='translate(2.5 3.5)'>{RAMP_RECT}</g>"
    ));
}

#[test]
fn neutral_groups_have_no_scope_and_raw_attribute_lookalikes_are_inert() {
    for attrs in [
        "",
        "style='mix-blend-mode:normal;isolation:auto'",
        "mix-blend-mode='multiply' isolation='isolate'",
    ] {
        let result = frame(&format!("<g {attrs}>{RECT}</g>"));
        assert_eq!(result.items.len(), 1);
    }
}

#[test]
fn combined_opacity_is_one_blend_operation_not_a_nested_opacity_scope() {
    let result = frame(&format!(
        "<g style='mix-blend-mode:multiply' opacity='.5'>{RECT}{RECT}</g>"
    ));
    let ops = blends(&result);
    assert_eq!(ops.len(), 2); // standalone initial backdrop + completed group
    assert_eq!(ops[0].mode(), ScopeBlendMode::Normal);
    assert_eq!(ops[1].mode(), ScopeBlendMode::Multiply);
    assert_eq!(ops[1].opacity().unwrap().get(), 0.5);
    assert!(!result.items.iter().any(|item|matches!(item,FrameItem::ScopeBegin(scope) if matches!(scope.effect,ScopeEffect::Opacity(_)))));
}

#[test]
fn neutral_isolation_survives_and_consumes_descendant_backdrop_dependency() {
    let result = frame(&format!(
        "<g style='isolation:isolate'><g><g style='mix-blend-mode:screen'>{RECT}</g></g></g>"
    ));
    let ops = blends(&result);
    assert_eq!(ops.len(), 2); // no extra standalone root boundary needed
    assert_eq!(ops[0].mode(), ScopeBlendMode::Normal);
    assert_eq!(ops[1].mode(), ScopeBlendMode::Screen);
}

#[test]
fn redundant_normal_isolation_preserves_the_one_pass_opacity_fold() {
    for body in [
        "<rect width='40' height='40' opacity='.6' style='isolation:isolate'/>",
        "<g opacity='.6' style='isolation:isolate'><rect width='40' height='40'/></g>",
        "<g opacity='.6'><g style='isolation:isolate'><g style='isolation:isolate'><rect width='40' height='40'/></g></g></g>",
    ] {
        let result = frame(body);
        assert_eq!(result.items.len(), 1, "{body}");
        assert!(matches!(
            result.items.iter().next(),
            Some(FrameItem::Node(_))
        ));
    }
    let deep = format!(
        "{}{}{}",
        "<g style='isolation:isolate'>".repeat(48),
        RECT,
        "</g>".repeat(48)
    );
    assert_eq!(frame(&deep).items.len(), 1);
}

#[test]
fn elision_rollback_cannot_erase_a_reused_scope_identity() {
    let source = svg(&format!(
        "<g style='mix-blend-mode:multiply'><g style='isolation:isolate'>{RECT}</g><path d='M0 0L10 20L20 0Z'/></g><g style='mix-blend-mode:screen'><g style='isolation:isolate'><g style='mix-blend-mode:multiply'>{RECT}</g></g></g>"
    ));
    let best = SvgFrameSource::from_standalone_svg_best_effort(
        source.as_str(),
        InitialViewport::new(64.0, 64.0),
    )
    .unwrap();
    assert_eq!(
        best.degradations()
            .iter()
            .filter(|d| d.action() == websem::DegradationAction::Skipped)
            .count(),
        1
    );
    let modes: Vec<_> = blends(&best.base_frame())
        .iter()
        .map(|b| b.mode())
        .collect();
    assert_eq!(
        modes,
        [
            ScopeBlendMode::Normal,
            ScopeBlendMode::Screen,
            ScopeBlendMode::Normal,
            ScopeBlendMode::Multiply
        ]
    );
}

#[test]
fn outer_root_blending_keeps_its_own_transparent_initial_backdrop() {
    for (value, mode) in [
        ("multiply", ScopeBlendMode::Multiply),
        ("screen", ScopeBlendMode::Screen),
    ] {
        let source = svg(RECT).replace(
            "width=\"64\"",
            &format!("style='mix-blend-mode:{value}' width=\"64\""),
        );
        let result = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0)).unwrap();
        let modes: Vec<_> = blends(&result).iter().map(|b| b.mode()).collect();
        assert_eq!(modes, [ScopeBlendMode::Normal, mode]);
        assert!(
            matches!(result.items.iter().next(), Some(FrameItem::ScopeBegin(scope))
            if matches!(scope.effect, ScopeEffect::Blend(b) if b.mode() == ScopeBlendMode::Normal))
        );
    }
}

#[test]
fn computed_winners_are_consumed_without_a_second_matcher() {
    for attrs in [
        "style='mix-blend-mode:screen;mix-blend-mode:invalid'",
        "style='--blend:screen;mix-blend-mode:var(--blend)'",
        "class='subject' style='mix-blend-mode:multiply'",
    ] {
        let result = frame(&format!(
            "<style>.subject{{mix-blend-mode:screen!important}}</style><rect width='40' height='40' {attrs}/>"
        ));
        assert_eq!(
            blends(&result).last().unwrap().mode(),
            ScopeBlendMode::Screen
        );
    }
}

#[test]
fn unsupported_mode_skips_one_named_element_and_keeps_its_sibling() {
    let source = svg(&format!(
        "<g style='mix-blend-mode:overlay'>{RECT}</g>{RECT}"
    ));
    let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
        .unwrap_err()
        .to_string();
    assert!(strict.contains("mix-blend-mode Overlay"), "{strict}");
    let best = SvgFrameSource::from_standalone_svg_best_effort(
        source.as_str(),
        InitialViewport::new(64.0, 64.0),
    )
    .unwrap();
    let result = best.base_frame();
    assert_eq!(result.items.len(), 1);
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == websem::DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].path(), "svg/g[1]");
    assert!(skipped[0].reason().contains("mix-blend-mode Overlay"));
}

#[test]
fn authored_clip_isolates_but_nested_viewport_clip_does_not() {
    let child = format!("<g style='mix-blend-mode:multiply'>{RECT}</g>");
    let clipped = frame(&format!(
        "<defs><clipPath id='c'><rect width='64' height='64'/></clipPath></defs><g clip-path='url(#c)'>{child}</g>"
    ));
    // Normal clip boundary + multiply child; no standalone-root layer needed.
    assert_eq!(blends(&clipped).len(), 2);
    let nested = frame(&format!("<svg width='64' height='64'>{child}</svg>"));
    // The normal boundary belongs to the standalone root, outside viewport clip.
    assert!(
        matches!(nested.items.iter().next(),Some(FrameItem::ScopeBegin(scope)) if matches!(scope.effect,ScopeEffect::Blend(_)))
    );
    assert!(
        matches!(clipped.items.iter().next(),Some(FrameItem::ScopeBegin(scope)) if matches!(scope.effect,ScopeEffect::Clip(_)))
    );
}

#[test]
fn image_effect_refusal_is_transactional() {
    let body = format!(
        "<defs><filter id='f'><feOffset dx='0' dy='0'/></filter></defs><g filter='url(#f)'>{RECT}<g style='mix-blend-mode:multiply'>{RECT}</g></g>{RECT}"
    );
    let source = svg(&body);
    let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
        .unwrap_err()
        .to_string();
    assert!(strict.contains("image-effect composition"), "{strict}");
    let best = SvgFrameSource::from_standalone_svg_best_effort(
        source.as_str(),
        InitialViewport::new(64.0, 64.0),
    )
    .unwrap();
    assert_eq!(
        best.base_frame().items.len(),
        1,
        "no partial group source survives"
    );
    assert!(
        best.degradations()
            .iter()
            .any(|d| d.path() == "svg/g[1]" && d.reason().contains("image-effect composition"))
    );
}

#[test]
fn elided_normal_isolation_keeps_ancestor_image_effect_patrols() {
    for (effect, resource) in [
        (
            "filter",
            "<filter id='effect'><feOffset dx='0' dy='0'/></filter>",
        ),
        (
            "mask",
            "<mask id='effect'><rect width='64' height='64' fill='white'/></mask>",
        ),
    ] {
        for opacity in ["1", ".6"] {
            let source = svg(&format!(
                "<defs>{resource}</defs><g {effect}='url(#effect)'><g style='isolation:isolate' opacity='{opacity}'>{RECT}</g></g>{RECT}"
            ));
            let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
                .unwrap_err()
                .to_string();
            assert!(strict.contains("image-effect composition"), "{strict}");
            let best = SvgFrameSource::from_standalone_svg_best_effort(
                source.as_str(),
                InitialViewport::new(64.0, 64.0),
            )
            .unwrap();
            assert_eq!(best.base_frame().items.len(), 1);
            assert!(
                best.degradations()
                    .iter()
                    .any(|d| d.path() == "svg/g[1]"
                        && d.reason().contains("image-effect composition"))
            );
        }
    }
}

#[test]
fn empty_or_pruned_normal_isolation_does_not_poison_an_ancestor_profile() {
    for opacity in ["1", ".6"] {
        for child in [
            "<g style='isolation:isolate'/>",
            "<g style='isolation:isolate;display:none'><rect width='40' height='40'/></g>",
            "<rect width='40' height='40' style='isolation:isolate;visibility:hidden'/>",
        ] {
            let child = child.replacen("style=", &format!("opacity='{opacity}' style="), 1);
            let result = frame(&format!(
                "<defs><filter id='f'><feOffset dx='0' dy='0'/></filter></defs><g filter='url(#f)'>{child}{RECT}</g>"
            ));
            assert!(blends(&result).is_empty());
        }
    }
}

#[test]
fn inline_backdrop_boundary_is_explicit() {
    let body = svg(&format!("<g style='mix-blend-mode:multiply'>{RECT}</g>"));
    let html = format!("<html><body>{body}</body></html>");
    for result in [
        SvgFrameSource::from_html_inline_svg(html.as_str()),
        SvgFrameSource::from_html_inline_svg_best_effort(html.as_str()),
    ] {
        assert!(result.unwrap_err().to_string().contains("host backdrop"));
    }
    let html = html.replace(
        "width=\"64\" height=\"64\"",
        "width=\"64\" height=\"64\" style='isolation:isolate'",
    );
    assert!(SvgFrameSource::from_html_inline_svg(html.as_str()).is_ok());
}

#[test]
fn keyframes_cannot_bypass_the_static_computed_blend_patrol() {
    let source = svg(&format!(
        "<style>@keyframes b{{from,to{{mix-blend-mode:screen}}}} rect{{animation:b 1s paused -1s both}}</style>{RECT}"
    ));
    let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
        .unwrap_err()
        .to_string();
    assert!(strict.contains("animated group-composition"), "{strict}");
    let best = SvgFrameSource::from_standalone_svg_best_effort(
        source.as_str(),
        InitialViewport::new(64.0, 64.0),
    )
    .unwrap();
    assert!(
        best.degradations().iter().any(
            |d| d.path() == "svg/style[1]" && d.reason().contains("animated group-composition")
        )
    );
}

#[test]
fn group_source_precision_refuses_transactionally_and_keeps_named_siblings() {
    for source in [
        "<path d='M8 12C50 2 2 58 54 46Z'/>",
        "<ellipse cx='24' cy='24' rx='12' ry='8'/>",
    ] {
        for style in ["mix-blend-mode:multiply", "isolation:isolate"] {
            let source = svg(&format!("<g style='{style}'>{RECT}{source}</g>{RECT}"));
            let strict = compile_standalone_svg(&source, InitialViewport::new(64.0, 64.0))
                .unwrap_err()
                .to_string();
            assert!(strict.contains("group-source precision"), "{strict}");
            let best = SvgFrameSource::from_standalone_svg_best_effort(
                source.as_str(),
                InitialViewport::new(64.0, 64.0),
            )
            .unwrap();
            assert_eq!(best.base_frame().items.len(), 1);
            assert!(
                best.degradations().iter().any(
                    |d| d.path() == "svg/g[1]" && d.reason().contains("group-source precision")
                )
            );
        }
    }
}

#[test]
fn root_blend_opacity_precision_has_no_best_effort_fallback() {
    let source = svg(RECT).replace(
        "width=\"64\"",
        "style='mix-blend-mode:screen' opacity='.5' width=\"64\"",
    );
    for result in [
        SvgFrameSource::from_standalone_svg(source.as_str(), InitialViewport::new(64.0, 64.0)),
        SvgFrameSource::from_standalone_svg_best_effort(
            source.as_str(),
            InitialViewport::new(64.0, 64.0),
        ),
    ] {
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("root-layer precision")
        );
    }
}

#[test]
fn implicit_standalone_root_isolation_cannot_smuggle_a_sibling_image_effect() {
    let source = svg(&format!(
        "<defs><filter id='f'><feOffset dx='0' dy='0'/></filter></defs><rect width='40' height='40' filter='url(#f)'/><g style='mix-blend-mode:multiply'>{RECT}</g>"
    ));
    for result in [
        SvgFrameSource::from_standalone_svg(source.as_str(), InitialViewport::new(64.0, 64.0)),
        SvgFrameSource::from_standalone_svg_best_effort(
            source.as_str(),
            InitialViewport::new(64.0, 64.0),
        ),
    ] {
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("image-effect composition")
        );
    }
}
