//! The paint-server contract: what `<linearGradient>`/`<radialGradient>`
//! resolve to, what an invalid reference falls back to, and what refuses by
//! name.
//!
//! Every law here is a value-level pin of a Chromium measurement from the
//! gradient rung's probe matrix; the pixel truth lives in the
//! `svg-gradient-*` cells of the primitive suite. The division is
//! deliberate: cells prove bytes, these laws prove the resolved facts a
//! byte can't attribute — which stop list won, which reference died, which
//! spelling entered the cascade.

#[allow(dead_code)]
mod support;

use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn viewport(width: f32, height: f32) -> InitialViewport {
    InitialViewport::new(width, height)
}

fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

/// Strict and best-effort agree and declare nothing static (a style
/// attribute or sheet blocks only the sampling inventory).
fn admit_both(source: &str) -> rframe::Frame {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort admits");
    let static_degradations: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() != DegradationAction::SamplesAsBase)
        .collect();
    assert!(
        static_degradations.is_empty(),
        "an admitted document declares nothing static: {static_degradations:?}"
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

fn refusal(source: &str) -> CompileError {
    SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0))
        .expect_err("must refuse")
        .clone()
}

fn sole_fill(frame: &rframe::Frame) -> &cg::Paint {
    assert_eq!(frame.nodes().len(), 1, "one shape");
    frame.nodes()[0].paints.iter().next().expect("one paint")
}

fn linear_of(paint: &cg::Paint) -> &cg::LinearGradientPaint {
    match paint {
        cg::Paint::LinearGradient(gradient) => gradient,
        other => panic!("expected a linear gradient, got {other:?}"),
    }
}

fn solid_of(paint: &cg::Paint) -> cg::CGColor {
    match paint {
        cg::Paint::Solid(solid) => solid.color,
        other => panic!("expected a solid, got {other:?}"),
    }
}

const RAMP: &str = r##"<stop offset="0" stop-color="red"/><stop offset="1" stop-color="blue"/>"##;
const RECT: &str = r##"<rect x="8" y="8" width="48" height="48" fill="url(#g)"/>"##;

// ─── the one computed transform ──────────────────────────────────────────

/// `gradientTransform` is the transform property's presentation attribute
/// on a gradient element, so an author `transform: none` disarms it with
/// ordinary cascade precedence (measured; the attr and CSS spellings are
/// byte-identical through non-quarter rotations and scales).
#[test]
fn an_author_transform_declaration_beats_gradienttransform() {
    let disarmed = admit_both(&document(&format!(
        r##"  <defs><linearGradient id="g" gradientTransform="rotate(90 0.5 0.5)" style="transform: none">{RAMP}</linearGradient></defs>
  {RECT}"##
    )));
    let plain = admit_both(&document(&format!(
        r##"  <defs><linearGradient id="g">{RAMP}</linearGradient></defs>
  {RECT}"##
    )));
    assert_eq!(
        sole_fill(&disarmed),
        sole_fill(&plain),
        "transform: none leaves the untransformed gradient"
    );
}

/// The plain `transform` attribute is inert on a gradient element
/// (measured: it changes no pixel; only `gradientTransform` and an author
/// `transform` declaration act).
#[test]
fn the_transform_attribute_is_inert_on_gradient_elements() {
    let with_attr = admit_both(&document(&format!(
        r##"  <defs><linearGradient id="g" transform="rotate(90 0.5 0.5)">{RAMP}</linearGradient></defs>
  {RECT}"##
    )));
    let plain = admit_both(&document(&format!(
        r##"  <defs><linearGradient id="g">{RAMP}</linearGradient></defs>
  {RECT}"##
    )));
    assert_eq!(sole_fill(&with_attr), sole_fill(&plain));
}

/// A non-invertible gradient transform paints nothing at all — a measured
/// correct nothing, admitted in both modes, never a declared hole.
#[test]
fn a_noninvertible_gradient_transform_paints_nothing() {
    let frame = admit_both(&document(&format!(
        r##"  <defs><linearGradient id="g" gradientTransform="scale(0)">{RAMP}</linearGradient></defs>
  {RECT}"##
    )));
    assert!(frame.nodes()[0].paints.is_empty());
}

/// A percentage in a gradient element's computed transform refuses by name:
/// Chromium resolves it against the viewport and then applies the raw
/// number in fraction space (measured) — mismatched spaces this slice will
/// not repeat.
#[test]
fn a_percentage_gradient_transform_refuses_by_name() {
    let error = refusal(&document(&format!(
        r##"  <defs><linearGradient id="g" style="transform: translate(25%, 0px)">{RAMP}</linearGradient></defs>
  {RECT}"##
    )));
    let CompileError::UnsupportedFill(reason) = error else {
        panic!("expected a fill refusal, got {error:?}");
    };
    assert!(reason.contains("percentage"), "{reason}");
}

// ─── the href chain ──────────────────────────────────────────────────────

/// `href` beats `xlink:href` when both are present (measured).
#[test]
fn href_beats_xlink_href() {
    let frame = admit_both(&format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="64" height="64">
  <defs>
    <linearGradient id="p1"><stop offset="0" stop-color="red"/></linearGradient>
    <linearGradient id="p2"><stop offset="0" stop-color="blue"/></linearGradient>
    <linearGradient id="g" href="#p1" xlink:href="#p2"/>
  </defs>
  {RECT}
</svg>"##,
        RECT = RECT
    ));
    let gradient = linear_of(sole_fill(&frame));
    assert!(
        gradient
            .stops
            .iter()
            .all(|stop| stop.color == cg::CGColor::from_rgb(255, 0, 0).into())
    );
}

/// A reference cycle kills only the href edge: the referenced element's own
/// stops still resolve (measured), while a self-cycle with no own stops
/// composes to zero stops and paints nothing — and the authored fallback
/// does **not** fire, because the reference itself is valid.
#[test]
fn a_reference_cycle_kills_only_the_edge() {
    let pair = admit_both(&document(&format!(
        r##"  <defs>
    <linearGradient id="g" href="#b">{RAMP}</linearGradient>
    <linearGradient id="b" href="#g"/>
  </defs>
  {RECT}"##
    )));
    let gradient = linear_of(sole_fill(&pair));
    assert_eq!(gradient.stops.len(), 2, "the element's own stops survive");

    let composed_empty = admit_both(&document(
        r##"  <defs><linearGradient id="g" href="#g"/></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g) red"/>"##,
    ));
    assert!(
        composed_empty.nodes()[0].paints.is_empty(),
        "a valid reference that composes to zero stops paints nothing, fallback unfired"
    );
}

/// Any own `<stop>` suppresses every template stop — there is no merging
/// (measured: one own stop beats a two-stop template).
#[test]
fn own_stops_suppress_the_whole_template_stop_list() {
    let frame = admit_both(&document(&format!(
        r##"  <defs>
    <linearGradient id="a">{RAMP}</linearGradient>
    <linearGradient id="g" href="#a"><stop offset="0" stop-color="lime"/></linearGradient>
  </defs>
  {RECT}"##
    )));
    let gradient = linear_of(sole_fill(&frame));
    assert!(
        gradient
            .stops
            .iter()
            .all(|stop| stop.color == cg::CGColor::from_rgb(0, 255, 0).into())
    );
}

// ─── stops ───────────────────────────────────────────────────────────────

/// One stop is spatially constant but retains gradient rasterization. The
/// duplicated resolved stops preserve that backend route without carrying
/// source syntax across the render contract.
#[test]
fn one_stop_resolves_to_a_constant_gradient() {
    let frame = admit_both(&document(&format!(
        r##"  <defs><linearGradient id="g"><stop offset="0.3" stop-color="lime"/></linearGradient></defs>
  {RECT}"##
    )));
    let gradient = linear_of(sole_fill(&frame));
    assert_eq!(gradient.stops.len(), 2);
    assert!(
        gradient
            .stops
            .iter()
            .all(|stop| stop.color == cg::CGColor::from_rgb(0, 255, 0).into())
    );
}

/// Linear endpoints at or below the backend's degenerate threshold resolve
/// the measured meaning before the backend can substitute its own: the last
/// stop under `pad`, the ramp's integral average under `repeat`/`reflect`.
#[test]
fn a_degenerate_linear_resolves_to_the_measured_solid() {
    let pad = admit_both(&document(&format!(
        r##"  <defs><linearGradient id="g" x1="0.5" y1="0" x2="0.500001" y2="0">{RAMP}</linearGradient></defs>
  {RECT}"##
    )));
    assert_eq!(
        solid_of(sole_fill(&pad)),
        cg::CGColor::from_rgb(0, 0, 255),
        "pad: the last stop"
    );

    let repeat = admit_both(&document(&format!(
        r##"  <defs><linearGradient id="g" x1="0.5" y1="0" x2="0.5" y2="0" spreadMethod="repeat"><stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#020000"/></linearGradient></defs>
  {RECT}"##,
        RECT = RECT,
    )));
    assert_eq!(
        solid_of(sole_fill(&repeat)),
        cg::CGColor::from_rgba(1, 0, 0, 255),
        "repeat: the ramp's integral average"
    );
}

/// A zero or negative-radius radial takes the same tile-specific backend
/// degeneracy as a collapsed linear ramp: the last stop for pad, the integral
/// ramp average for repeat and reflect.
#[test]
fn a_nonpositive_radius_radial_uses_the_tile_specific_degenerate_color() {
    let radial = |radius: &str, spread: &str| {
        admit_both(&document(&format!(
            r##"  <defs><radialGradient id="g" r="{radius}" spreadMethod="{spread}"><stop offset="0" stop-color="#000000"/><stop offset="1" stop-color="#020000"/></radialGradient></defs>
  {RECT}"##,
            RECT = RECT,
        )))
    };

    for radius in ["0", "-1"] {
        assert_eq!(
            solid_of(sole_fill(&radial(radius, "pad"))),
            cg::CGColor::from_rgb(2, 0, 0),
            "radius {radius}, pad: the last stop"
        );
        for spread in ["repeat", "reflect"] {
            assert_eq!(
                solid_of(sole_fill(&radial(radius, spread))),
                cg::CGColor::from_rgb(1, 0, 0),
                "radius {radius}, {spread}: the ramp's integral average"
            );
        }
    }
}

/// An RGBA8-exact `stop-opacity` survives in the stop color, while the
/// consumer's `fill-opacity` rides the gradient paint's float opacity
/// (measured through the `svg-gradient-fill-opacity` cell). Fractional stop
/// alpha that the frame cannot preserve refuses separately below.
#[test]
fn exact_stop_opacity_and_fill_opacity_keep_separate_facts() {
    let frame = admit_both(&document(
        r##"  <defs><linearGradient id="g"><stop offset="0" stop-color="red"/><stop offset="1" stop-color="red" stop-opacity="0.5019607843137255"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g)" fill-opacity="0.25"/>"##,
    ));
    let gradient = linear_of(sole_fill(&frame));
    assert_eq!(gradient.stops[0].color.a().to_bits(), 1.0f32.to_bits());
    assert_eq!(
        gradient.stops[1].color.a().to_bits(),
        (128.0f32 / 255.0).to_bits()
    );
    assert_eq!(gradient.opacity.to_bits(), 0.25f32.to_bits());
}

/// Chromium retains `stop-opacity` as a float through gradient shading, while
/// the current resolved paint leaf stores stop colors as RGBA8. A fractional
/// component that does not round-trip through that leaf is an own-row named
/// refusal, never a neighboring-alpha pixel.
#[test]
fn non_rgba8_gradient_stop_precision_refuses_by_name() {
    for stop in [
        r##"<stop offset="0" stop-color="#16a34a" stop-opacity=".3"/>"##,
        r##"<stop offset="0" stop-color="rgb(22 163 74 / .3)" stop-opacity=".7"/>"##,
    ] {
        let error = refusal(&document(&format!(
            r##"  <defs><linearGradient id="g">{stop}<stop offset="1" stop-color="#2563eb"/></linearGradient></defs>
  {RECT}"##
        )));
        let CompileError::UnsupportedFill(reason) = error else {
            panic!("expected a fill refusal, got {error:?}");
        };
        assert!(
            reason.contains("stop alpha loses float precision"),
            "{reason}"
        );
    }

    let staged = refusal(&document(
        r##"  <defs><linearGradient id="g" x1="0" x2="0"><stop stop-color="#16a34a"/><stop offset="1" stop-color="#2563eb" stop-opacity="0.30196078431372547"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g)" fill-opacity=".7"/>"##,
    ));
    let CompileError::UnsupportedFill(reason) = staged else {
        panic!("expected a fill refusal, got {staged:?}");
    };
    assert!(
        reason.contains("stop alpha loses staged precision"),
        "{reason}"
    );

    for spread in ["repeat", "reflect"] {
        let averaged = refusal(&document(&format!(
            r##"  <defs><linearGradient id="g" x1="0" x2="0" spreadMethod="{spread}"><stop stop-color="#16a34a" stop-opacity="0.30196078431372547"/><stop offset="1" stop-color="#2563eb" stop-opacity="0.5019607843137255"/></linearGradient></defs>
  {RECT}"##
        )));
        let CompileError::UnsupportedFill(reason) = averaged else {
            panic!("expected a fill refusal, got {averaged:?}");
        };
        assert!(
            reason.contains("stop alpha loses float precision"),
            "{reason}"
        );
    }

    let averaged_rgb = refusal(&document(
        r##"  <defs><linearGradient id="g" x1="0" x2="0" spreadMethod="repeat"><stop stop-color="#000000"/><stop offset="1" stop-color="#010000"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g)" opacity=".6"/>"##,
    ));
    let CompileError::UnsupportedFill(reason) = averaged_rgb else {
        panic!("expected a fill refusal, got {averaged_rgb:?}");
    };
    assert!(
        reason.contains("stop color loses float precision"),
        "{reason}"
    );

    let averaged_translucent_rgb = refusal(&document(
        r##"  <defs><linearGradient id="g" x1="0" x2="0" spreadMethod="repeat"><stop stop-color="#00000080"/><stop offset="1" stop-color="#01000080"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g)"/>"##,
    ));
    let CompileError::UnsupportedFill(reason) = averaged_translucent_rgb else {
        panic!("expected a fill refusal, got {averaged_translucent_rgb:?}");
    };
    assert!(
        reason.contains("stop color loses float precision"),
        "{reason}"
    );
}

/// An invalid `offset` is 0, and offsets clamp against the running maximum
/// (measured: lists are never sorted).
#[test]
fn stop_offsets_clamp_against_the_running_maximum() {
    let frame = admit_both(&document(
        r##"  <defs><linearGradient id="g"><stop offset="abc" stop-color="lime"/><stop offset="0.8" stop-color="red"/><stop offset="0.2" stop-color="blue"/></linearGradient></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#g)"/>"##,
    ));
    let gradient = linear_of(sole_fill(&frame));
    let offsets: Vec<f32> = gradient.stops.iter().map(|stop| stop.offset).collect();
    assert_eq!(offsets, [0.0, 0.8, 0.8]);
}

// ─── references and fallbacks ────────────────────────────────────────────

/// A same-document reference to an element that is not a gradient is an
/// invalid reference: the authored fallback fires (measured).
#[test]
fn a_wrong_type_target_is_an_invalid_reference() {
    let frame = admit_both(&document(
        r##"  <defs><rect id="notag" width="4" height="4"/></defs>
  <rect x="8" y="8" width="48" height="48" fill="url(#notag) lime"/>"##,
    ));
    assert_eq!(
        solid_of(sole_fill(&frame)),
        cg::CGColor::from_rgb(0, 255, 0)
    );
}

/// An external reference refuses — external resources are not resolved,
/// and painting the fallback instead of refusing would silently diverge
/// from a Chromium that fetched it. An absolute URL refuses as
/// not-same-document; a relative one cannot even resolve against this
/// engine's non-hierarchical document base and refuses as an invalid URL.
#[test]
fn an_external_reference_refuses_by_name() {
    let absolute = refusal(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="url(https://example.com/g.svg#g) red"/>"##,
    ));
    let CompileError::UnsupportedFill(reason) = absolute else {
        panic!("expected a fill refusal, got {absolute:?}");
    };
    assert!(reason.contains("same-document"), "{reason}");

    let relative = refusal(&document(
        r##"  <rect x="8" y="8" width="48" height="48" fill="url(other.svg#g) red"/>"##,
    ));
    let CompileError::UnsupportedFill(reason) = relative else {
        panic!("expected a fill refusal, got {relative:?}");
    };
    assert!(reason.contains("url("), "{reason}");
}

// ─── the named refusals ──────────────────────────────────────────────────

/// The refusal boundary, each by name: a focal radial (the shared radial
/// leaf is concentric), a focal radius, `color-interpolation: linearRGB`,
/// author CSS on a stop's style attribute, and a geometry unit whose basis
/// this slice does not consume.
#[test]
fn the_beyond_slice_gradient_family_refuses_by_name() {
    for (body, needle) in [
        (
            format!(
                r##"  <defs><radialGradient id="g" fx="0.2" fy="0.2">{RAMP}</radialGradient></defs>
  {RECT}"##
            ),
            "focal",
        ),
        (
            format!(
                r##"  <defs><radialGradient id="g" fr="0.25">{RAMP}</radialGradient></defs>
  {RECT}"##
            ),
            "focal",
        ),
        (
            format!(
                r##"  <defs><linearGradient id="g" color-interpolation="linearRGB">{RAMP}</linearGradient></defs>
  {RECT}"##
            ),
            "linearRGB",
        ),
        (
            format!(
                r##"  <defs><linearGradient id="g"><stop offset="0" style="stop-color: red"/><stop offset="1" stop-color="blue"/></linearGradient></defs>
  {RECT}"##
            ),
            "stop-color",
        ),
        (
            format!(
                r##"  <defs><linearGradient id="g" gradientUnits="userSpaceOnUse" x1="0" x2="4em">{RAMP}</linearGradient></defs>
  {RECT}"##
            ),
            "unit whose basis",
        ),
    ] {
        let error = refusal(&document(&body));
        let CompileError::UnsupportedFill(reason) = error else {
            panic!("expected a fill refusal, got {error:?}");
        };
        assert!(reason.contains(needle), "{reason} must name {needle}");
    }
}

/// A template's focal attribute makes the referencing gradient focal too:
/// the refusal runs on the resolved attribute set, because an inherited
/// `fx` does not re-default from an overridden `cx` (measured).
#[test]
fn an_inherited_focal_point_still_refuses() {
    let error = refusal(&document(&format!(
        r##"  <defs>
    <radialGradient id="t" fx="0.15" fy="0.5">{RAMP}</radialGradient>
    <radialGradient id="g" href="#t" cx="0.8" cy="0.5" r="0.45"/>
  </defs>
  {RECT}"##
    )));
    let CompileError::UnsupportedFill(reason) = error else {
        panic!("expected a fill refusal, got {error:?}");
    };
    assert!(reason.contains("focal"), "{reason}");
}
