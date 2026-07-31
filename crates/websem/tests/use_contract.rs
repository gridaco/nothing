//! The use/defs contract: same-document reference resolution, the
//! expansion's rendering semantics, and the boundary that keeps it honest.
//!
//! `<use>` renders as a container whose children are the cloned instance
//! (SVG2 §5.6), expanded before the one cascade so inheritance flows from
//! the use site and the clone's own presentation attributes stand. Every
//! law here restates a Chromium probe verdict; the pixel claims are baked
//! in `reftest_oracle.rs`. The refusals: author CSS (shadow-scoped
//! selector matching), external references, authored children, expansion
//! overflow — each by name, in both admissions' shapes.

// This binary consumes only the frame half of the shared plumbing.
#[allow(dead_code)]
mod support;

use math2::transform::AffineTransform;
use websem::{CompileError, DegradationAction, InitialViewport, SvgFrameSource};

fn viewport(width: f32, height: f32) -> InitialViewport {
    InitialViewport::new(width, height)
}

/// A 64x64 canvas around the markup under test.
fn document(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">
{body}
</svg>"##
    )
}

/// Strict and best-effort agree and declare nothing.
fn admit_both(source: &str) -> rframe::Frame {
    let strict =
        SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0)).expect("strict admits");
    let best = SvgFrameSource::from_standalone_svg_best_effort(source, viewport(64.0, 64.0))
        .expect("best-effort admits");
    assert!(
        best.degradations().is_empty(),
        "an admitted document declares nothing: {:?}",
        best.degradations()
    );
    let frame = strict.base_frame();
    assert_eq!(frame, best.base_frame(), "admissions are frame-identical");
    frame
}

/// The strict refusal for one document.
fn refusal(source: &str) -> CompileError {
    SvgFrameSource::from_standalone_svg(source, viewport(64.0, 64.0))
        .expect_err("must refuse")
        .clone()
}

/// A use of a defs-held shape resolves to the identical frame as the shape
/// authored in place — the reference is a spelling, not a new semantics.
#[test]
fn a_use_of_a_defs_shape_renders_as_the_shape_in_place() {
    let referenced = admit_both(&document(
        r##"  <defs><rect id="r" x="8" y="8" width="20" height="12" fill="#16a34a"/></defs>
  <use href="#r"/>"##,
    ));
    let inline = admit_both(&document(
        r##"  <rect x="8" y="8" width="20" height="12" fill="#16a34a"/>"##,
    ));
    assert_eq!(
        referenced.nodes, inline.nodes,
        "one geometry, two spellings"
    );
}

/// `x`/`y` are an additional translate appended INSIDE the use's own
/// transform (SVG2 §5.6.2 "appended to the right-side of the
/// transformation list"; measured: scale(2) then x=5 lands at 26, not 21).
#[test]
fn use_x_y_translate_inside_the_uses_transform() {
    let frame = admit_both(&document(
        r##"  <defs><rect id="r" width="4" height="4" fill="#16a34a"/></defs>
  <use href="#r" transform="scale(2)" x="5" y="5"/>"##,
    ));
    assert_eq!(
        frame.nodes[0].transform,
        AffineTransform::from_acebdf(2.0, 0.0, 10.0, 0.0, 2.0, 10.0),
        "scale, then translate in the scaled frame"
    );
}

/// Chained references expand through, cycles render Chromium's silent
/// nothing, an unresolved reference renders nothing, and a reference to a
/// shadow-including ancestor is an invalid circle that renders nothing —
/// all with zero declarations, because in each case the pixels agree.
#[test]
fn chains_cycles_and_unresolved_references_match_chromium() {
    let chain = admit_both(&document(
        r##"  <defs><rect id="leaf" width="8" height="8" fill="#16a34a"/><use id="mid" href="#leaf" x="10"/></defs>
  <use href="#mid" y="10"/>"##,
    ));
    assert_eq!(chain.nodes.len(), 1, "the chain reaches the leaf");
    assert_eq!(
        chain.nodes[0].transform,
        AffineTransform::from_acebdf(1.0, 0.0, 10.0, 0.0, 1.0, 10.0),
        "each hop's x/y composes"
    );

    let cycle = admit_both(&document(
        r##"  <defs><use id="a" href="#b"/><use id="b" href="#a"/></defs>
  <use href="#a"/>
  <rect x="40" y="40" width="8" height="8" fill="#2563eb"/>"##,
    ));
    assert_eq!(
        cycle.nodes.len(),
        1,
        "the cycle renders nothing; the sibling paints"
    );

    let missing = admit_both(&document(
        r##"  <use href="#nope"/>
  <rect x="40" y="40" width="8" height="8" fill="#2563eb"/>"##,
    ));
    assert_eq!(
        missing.nodes.len(),
        1,
        "unresolved renders nothing, silently"
    );

    let ancestor = admit_both(&document(
        r##"  <g id="cy"><rect width="8" height="8" fill="#16a34a"/><use href="#cy" y="20"/></g>"##,
    ));
    assert_eq!(
        ancestor.nodes.len(),
        1,
        "the ancestor reference is an invalid circle: content paints once"
    );
}

/// The id table is whole-document and first-wins: forward references
/// resolve, and a duplicate id resolves to the first element in tree
/// order (DOM getElementById semantics, measured).
#[test]
fn the_id_table_is_whole_document_and_first_wins() {
    let forward = admit_both(&document(
        r##"  <use href="#fwd"/>
  <defs><rect id="fwd" x="8" y="8" width="20" height="12" fill="#16a34a"/></defs>"##,
    ));
    assert_eq!(forward.nodes.len(), 1);

    let duplicate = admit_both(&document(
        r##"  <defs>
    <rect id="dup" x="8" y="8" width="20" height="12" fill="#16a34a"/>
    <circle id="dup" cx="32" cy="32" r="10" fill="#2563eb"/>
  </defs>
  <use href="#dup"/>"##,
    ));
    assert_eq!(duplicate.nodes.len(), 1);
    assert!(
        matches!(duplicate.nodes[0].geometry, rframe::Geometry::Rect(_)),
        "the first id in tree order is the target"
    );
}

/// A target rendered in place and referenced paints twice; `<defs>`
/// content paints only through references.
#[test]
fn defs_never_paints_in_place_and_a_light_target_paints_twice() {
    let twice = admit_both(&document(
        r##"  <rect id="dup2" x="4" y="4" width="12" height="8" fill="#16a34a"/>
  <use href="#dup2" x="20"/>"##,
    ));
    assert_eq!(twice.nodes.len(), 2, "in place and as an instance");

    let defs_only = admit_both(&document(
        r##"  <defs><rect x="8" y="8" width="20" height="12" fill="#16a34a"/></defs>"##,
    ));
    assert_eq!(defs_only.nodes.len(), 0, "defs content is reference-only");
}

/// Inheritance flows from the use site (measured): a hint on the `<use>`
/// colors a clone that authors no fill, the clone's own attribute beats
/// it, and `currentColor` resolves against the use site's `color` — the
/// hint this rung admitted.
#[test]
fn instance_styling_inherits_from_the_use_site() {
    let inherited = admit_both(&document(
        r##"  <defs><rect id="r" x="8" y="8" width="20" height="12"/></defs>
  <use href="#r" fill="#16a34a"/>"##,
    ));
    let own = admit_both(&document(
        r##"  <defs><rect id="o" x="8" y="8" width="20" height="12" fill="#16a34a"/></defs>
  <use href="#o" fill="#2563eb"/>"##,
    ));
    let current = admit_both(&document(
        r##"  <defs><rect id="c" x="8" y="8" width="20" height="12" fill="currentColor"/></defs>
  <use href="#c" color="#16a34a"/>"##,
    ));
    let reference = admit_both(&document(
        r##"  <rect x="8" y="8" width="20" height="12" fill="#16a34a"/>"##,
    ));
    for (label, frame) in [
        ("inherited from use", inherited),
        ("own attribute wins", own),
        ("currentColor at the use site", current),
    ] {
        assert_eq!(frame.nodes, reference.nodes, "{label}");
    }
}

/// `display: none` cloned onto the instance prunes it (the clone carries
/// the target's attributes), and a hidden use hides its instance through
/// ordinary inheritance — with a `visibility: visible` clone un-hiding,
/// the visibility rung's law restated through the shadow content.
#[test]
fn instance_disposition_follows_the_one_cascade() {
    let none_target = admit_both(&document(
        r##"  <defs><rect id="h" width="20" height="12" fill="#16a34a" display="none"/></defs>
  <use href="#h"/>
  <rect x="40" y="40" width="8" height="8" fill="#2563eb"/>"##,
    ));
    assert_eq!(none_target.nodes.len(), 1, "the instance is pruned");

    let hidden_use = admit_both(&document(
        r##"  <defs><rect id="r" width="20" height="12" fill="#16a34a"/></defs>
  <use href="#r" visibility="hidden"/>
  <rect x="40" y="40" width="8" height="8" fill="#2563eb"/>"##,
    ));
    assert_eq!(
        hidden_use.nodes.len(),
        1,
        "visibility inherits into the clone"
    );

    let unhidden = admit_both(&document(
        r##"  <defs><rect id="u" width="20" height="12" fill="#16a34a" visibility="visible"/></defs>
  <use href="#u" visibility="hidden"/>"##,
    ));
    assert_eq!(unhidden.nodes.len(), 1, "the clone's own visible un-hides");
}

/// `width`/`height` on a use are inert for every admitted target
/// (measured; they size only `<svg>`/`<symbol>` targets, which refuse).
#[test]
fn use_width_height_are_inert_for_admitted_targets() {
    let sized = admit_both(&document(
        r##"  <defs><rect id="r" x="8" y="8" width="20" height="12" fill="#16a34a"/></defs>
  <use href="#r" width="10" height="6"/>"##,
    ));
    let bare = admit_both(&document(
        r##"  <defs><rect id="r" x="8" y="8" width="20" height="12" fill="#16a34a"/></defs>
  <use href="#r"/>"##,
    ));
    assert_eq!(sized, bare);
}

/// A beyond-slice construct inside the instance is a declared hole at the
/// clone's real path — the instance is walked like any subtree, not
/// refused wholesale.
#[test]
fn a_beyond_slice_clone_is_its_own_declared_hole() {
    let source = document(
        r##"  <defs><g id="grp"><rect width="8" height="8" fill="#16a34a"/><text x="0" y="30">hi</text></g></defs>
  <use href="#grp"/>"##,
    );
    let best =
        SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
            .expect("best-effort renders the admitted clone");
    assert_eq!(best.base_frame().nodes.len(), 1, "the rect instance paints");
    let skipped: Vec<_> = best
        .degradations()
        .iter()
        .filter(|d| d.action() == DegradationAction::Skipped)
        .collect();
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].path(), "svg/use[1]/g[1]/text[1]");
    assert!(
        skipped[0].reason().contains("<text>"),
        "{}",
        skipped[0].reason()
    );
}

/// The four named refusals, each in both admissions' shapes: author CSS
/// (the measured shadow boundary scopes selectors to the cloned subtree,
/// which the flattened tree cannot express), an external reference, a
/// symbol target (the clone surfaces `<symbol>`, an unsupported element),
/// and authored element children.
#[test]
fn the_use_refusals_name_their_boundary() {
    for (label, body, named) in [
        (
            "author CSS",
            r##"  <style>rect { fill: #2563eb; }</style>
  <defs><rect id="r" width="8" height="8"/></defs>
  <use href="#r"/>
  <rect x="40" y="40" width="8" height="8" fill="#16a34a"/>"##,
            "author CSS",
        ),
        (
            "external reference",
            r##"  <use href="icons.svg#glyph"/>
  <rect x="40" y="40" width="8" height="8" fill="#16a34a"/>"##,
            "same-document fragment",
        ),
        (
            "authored children",
            r##"  <defs><rect id="r" width="8" height="8" fill="#16a34a"/></defs>
  <use href="#r"><rect width="4" height="4" fill="#2563eb"/></use>
  <rect x="40" y="40" width="8" height="8" fill="#16a34a"/>"##,
            "authored element children",
        ),
    ] {
        let source = document(body);
        let error = refusal(&source);
        assert!(
            error.to_string().contains(named),
            "{label}: the refusal names it: {error}"
        );
        let best =
            SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport(64.0, 64.0))
                .expect("best-effort renders the rest");
        assert_eq!(
            best.base_frame().nodes.len(),
            1,
            "{label}: the sibling paints"
        );
        let skipped: Vec<_> = best
            .degradations()
            .iter()
            .filter(|d| d.action() == DegradationAction::Skipped)
            .collect();
        assert_eq!(skipped.len(), 1, "{label}");
        assert_eq!(skipped[0].path(), "svg/use[1]", "{label}");
        assert!(skipped[0].reason().contains(named), "{label}");
    }

    // The symbol target declares at the clone's own path — the walk finds
    // the unsupported element, not a bespoke use refusal.
    let source = document(
        r##"  <symbol id="s"><rect width="8" height="8" fill="#16a34a"/></symbol>
  <use href="#s"/>"##,
    );
    let error = refusal(&source);
    assert!(error.to_string().contains("<symbol>"), "{error}");
}

/// A reference chain deeper than the expansion budget refuses by name
/// instead of recursing — the loud edge of the indirect-cycle family.
#[test]
fn a_chain_beyond_the_budget_refuses_by_name() {
    // The referencing use comes FIRST, so its expansion must chase the
    // whole unexpanded chain from scratch; a leaf-first document order
    // would pre-expand each hop shallowly and render fine, as Chromium
    // does either way — the budget is the loud edge, not the common case.
    let mut defs = String::from(r##"    <rect id="d0" width="4" height="4" fill="#16a34a"/>"##);
    for i in 1..=48 {
        defs.push_str(&format!("\n    <use id=\"d{i}\" href=\"#d{}\"/>", i - 1));
    }
    let source = document(&format!(
        "  <use href=\"#d48\"/>\n  <defs>\n{defs}\n  </defs>"
    ));
    let error = refusal(&source);
    assert!(
        error.to_string().contains("expansion overflows"),
        "the chain budget names itself: {error}"
    );
}

/// The legacy `xlink:href` spelling resolves, and the plain `href` beats
/// it when both are present (measured; the deprecated spelling is
/// ignored, not merged).
#[test]
fn xlink_href_resolves_and_loses_to_the_plain_spelling() {
    let source = r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="64" height="64">
  <defs><rect id="r" x="8" y="8" width="20" height="12" fill="#16a34a"/></defs>
  <use xlink:href="#r"/>
</svg>"##;
    let legacy = admit_both(source);
    assert_eq!(legacy.nodes.len(), 1);

    let both = admit_both(
        r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="64" height="64">
  <defs><rect id="r" x="8" y="8" width="20" height="12" fill="#16a34a"/></defs>
  <use href="#r" xlink:href="#missing"/>
</svg>"##,
    );
    assert_eq!(both.nodes.len(), 1, "the plain spelling wins");
}
