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
