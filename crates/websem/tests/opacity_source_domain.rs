//! SVG producer laws for ordinary opacity's complete drawable source.
//! Pixel claims live in the externally baked corpus, not these shape checks.

use math2::Rectangle;
use rframe::{FrameItem, ScopeEffect};
use websem::{InitialViewport, SvgFrameSource};

fn svg(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><defs><radialGradient id="r"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></radialGradient></defs>{body}</svg>"##
    )
}

fn source() -> &'static str {
    r##"<circle cx="30" cy="30" r="24" fill="url(#r)"/><rect x="38" y="34" width="18" height="20" fill="blue"/>"##
}

fn both(input: &str) -> rframe::Frame {
    let viewport = InitialViewport::new(64.0, 64.0);
    let strict = SvgFrameSource::from_standalone_svg(input, viewport).unwrap();
    let best = SvgFrameSource::from_standalone_svg_best_effort(input, viewport).unwrap();
    assert!(best.degradations().is_empty());
    assert_eq!(strict.base_frame(), best.base_frame());
    strict.base_frame()
}

fn domains(frame: &rframe::Frame) -> Vec<rframe::IsolatedSourceDomain> {
    frame
        .items
        .iter()
        .filter_map(|item| match item {
            FrameItem::ScopeBegin(scope) => match scope.effect {
                ScopeEffect::Opacity(group) => group.source_domain(),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn opacity_keeps_local_drawable_enclosure_separate_from_mapping_and_geometry() {
    for alpha in [".57", ".998", ".999"] {
        for map in [
            "translate(0 0)",
            "translate(1.25 2.5)",
            "rotate(15 32 32)",
            "matrix(-1 0 0 1 64 0)",
        ] {
            let frame = both(&svg(&format!(
                r##"<g opacity="{alpha}" transform="{map}">{}</g>"##,
                source()
            )));
            let declarations = domains(&frame);
            assert_eq!(declarations.len(), 1);
            assert_eq!(
                declarations[0].rect(),
                Rectangle::from_xywh(6.0, 6.0, 50.0, 48.0)
            );
            for node in frame.nodes() {
                assert_eq!(node.transform, declarations[0].source_to_stream());
            }
            assert_eq!(
                frame.nodes()[0].geometry.local_box(),
                Rectangle::from_xywh(6.0, 6.0, 48.0, 48.0)
            );
        }
    }
}

#[test]
fn plain_solid_and_one_pass_sources_do_not_gain_enclosure_facts() {
    let solid = source().replace("url(#r)", "red");
    assert!(domains(&both(&svg(&format!(r##"<g opacity=".999">{solid}</g>"##)))).is_empty());
    assert!(
        domains(&both(&svg(
            r##"<g opacity=".999"><circle cx="30" cy="30" r="24" fill="url(#r)"/></g>"##
        )))
        .is_empty()
    );
}

#[test]
fn disabled_box_nodes_keep_identity_without_enlarging_the_opacity_source() {
    for disabled in [
        r#"<rect x=".25" y=".25" width="0" height="0" fill="red"/>"#,
        r#"<rect x=".25" y=".25" width="0" height="20" fill="red"/>"#,
        r#"<rect x=".25" y=".25" width="20" height="0" fill="red"/>"#,
        r#"<ellipse cx=".25" cy=".25" rx="0" ry="10" fill="red"/>"#,
        r#"<ellipse cx=".25" cy=".25" rx="10" ry="0" fill="red"/>"#,
        r#"<rect x=".25" y=".25" width="0" height="20" fill="red" stroke="blue" stroke-width="2"/>"#,
        r#"<ellipse cx=".25" cy=".25" rx="0" ry="10" fill="red" stroke="blue" stroke-width="2"/>"#,
    ] {
        let frame = both(&svg(&format!(
            r#"<g opacity=".999">{}{disabled}</g>"#,
            source()
        )));
        assert_eq!(frame.nodes().len(), 3, "disabled node remains: {disabled}");
        assert!(frame.nodes()[2].stroke.is_none());
        assert!(!frame.nodes()[2].paints.is_empty());
        assert_eq!(
            domains(&frame)[0].rect(),
            Rectangle::from_xywh(6.0, 6.0, 50.0, 48.0),
            "{disabled}"
        );
    }
}

#[test]
fn incomplete_sources_skip_the_whole_owner_without_leaking_partial_children() {
    let variants = [
        (
            source().replace("<circle", "<circle transform=\"translate(1 0)\""),
            "independently transformed",
        ),
        (
            source().replace(
                "r=\"24\"",
                "r=\"24\" stroke=\"red\" stroke-width=\"2\" stroke-dasharray=\"4 2\"",
            ),
            "complex-stroke",
        ),
        (
            format!(
                "{}<rect width=\"2\" height=\"2\" fill=\"transparent\"/>",
                source()
            ),
            "completely represented",
        ),
    ];
    for (body, reason) in variants {
        let input = svg(&format!(
            r##"<g opacity=".999">{body}</g><rect x="60" y="60" width="2" height="2" fill="green"/>"##
        ));
        let strict =
            SvgFrameSource::from_standalone_svg(input.as_str(), InitialViewport::new(64.0, 64.0))
                .unwrap_err();
        assert!(strict.to_string().contains(reason), "{strict}");
        let best = SvgFrameSource::from_standalone_svg_best_effort(
            input.as_str(),
            InitialViewport::new(64.0, 64.0),
        )
        .unwrap();
        assert_eq!(best.degradations().len(), 1);
        assert_eq!(best.degradations()[0].path(), "svg/g[1]");
        assert!(best.degradations()[0].reason().contains(reason));
        assert_eq!(best.base_frame().nodes().len(), 1);
        assert_eq!(
            best.base_frame().nodes()[0].geometry.local_box(),
            Rectangle::from_xywh(60.0, 60.0, 2.0, 2.0)
        );
    }
}

#[test]
fn resource_and_marker_sources_rollback_their_complete_client() {
    for (input, path, reason) in [
        (
            include_str!(
                "../../../fixtures/web-first/unsupported/svg-opacity-source-marker-client.svg"
            ),
            "svg/path[1]",
            "marker-client source profile",
        ),
        (
            include_str!(
                "../../../fixtures/web-first/unsupported/svg-opacity-source-mask-program.svg"
            ),
            "svg/rect[2]",
            "source-program profile",
        ),
        (
            include_str!(
                "../../../fixtures/web-first/unsupported/svg-paint-server-source-filter.svg"
            ),
            "svg/g[1]",
            "multi-operation filter or mask profile",
        ),
        (
            include_str!(
                "../../../fixtures/web-first/unsupported/svg-paint-server-source-mask.svg"
            ),
            "svg/g[1]",
            "multi-operation filter or mask profile",
        ),
    ] {
        let viewport = InitialViewport::new(64.0, 64.0);
        let strict = SvgFrameSource::from_standalone_svg(input, viewport).unwrap_err();
        assert!(strict.to_string().contains(reason), "{strict}");
        let best = SvgFrameSource::from_standalone_svg_best_effort(input, viewport).unwrap();
        assert_eq!(best.degradations().len(), 1);
        assert_eq!(best.degradations()[0].path(), path);
        assert!(best.degradations()[0].reason().contains(reason));
        let frame = best.base_frame();
        assert_eq!(
            frame.nodes().len(),
            1,
            "only the unrelated background remains"
        );
        assert_eq!(frame.items.len(), 1, "no failed source scope leaks");
        assert_eq!(
            frame.nodes()[0].geometry.local_box(),
            Rectangle::from_xywh(0.0, 0.0, 64.0, 64.0)
        );
    }
}
