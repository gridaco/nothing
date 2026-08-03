//! Every fixture in the `unsupported/` corpus departs by name, in both
//! admissions, and the directory holds nothing else.
//!
//! The corpus documents what this slice refuses and why. Before this gate it
//! documented that to *humans* only: thirteen of its twenty files had no reader
//! at all, and the four that did were pinned one `include_str!` at a time, so a
//! new file could be added — or an old one silently start rendering — with
//! nothing to notice. The table below is read against `read_dir`, so adding a
//! fixture without declaring it fails, and deleting one without removing its row
//! fails too.
//!
//! The property being defended is the programme's invariant, stated over a whole
//! directory instead of one construct: **nothing here renders silently.** A
//! fixture either refuses in both admissions, because its construct is a
//! document-level contract no per-element hole can express, or it is
//! attributable and best-effort declares it by name at a stable path. There is
//! no third outcome, and a fixture that started rendering would land in it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use websem::{DegradationAction, InitialViewport, SvgFrameSource};

/// What the two admissions must do with a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Departure {
    /// A document-level contract: the viewport grammar and root sizing decide
    /// the canvas itself, so best-effort cannot soften them into a hole
    /// without inventing pixels. Both admissions refuse.
    BothRefuse,
    /// Attributable to an element or to one stylesheet, so strict refuses and
    /// best-effort renders the rest and declares this by name.
    DeclaredByBestEffort,
}

use Departure::{BothRefuse, DeclaredByBestEffort};

/// The closed enumeration: every file, what it must do, and a fragment of the
/// reason that must name the construct. The fragments are deliberately the
/// construct itself — a refusal that stopped naming what it refused would pass
/// a bare "does it error" check and fail this one.
const CORPUS: &[(&str, Departure, &str)] = &[
    ("svg-clip-path", DeclaredByBestEffort, "<clipPath>"),
    // The CSS `transform` property graduated with the transform rung (its
    // fixture is now a baked cell); the family's still-refused members each
    // hold a row: the individual transform properties, the beyond-2D
    // function forms, and the origin/box knobs.
    ("svg-css-individual-rotate", DeclaredByBestEffort, "rotate"),
    ("svg-css-transform-3d", DeclaredByBestEffort, "translate3d"),
    (
        "svg-css-transform-box",
        DeclaredByBestEffort,
        "transform-box",
    ),
    (
        "svg-css-transform-origin",
        DeclaredByBestEffort,
        "transform-origin",
    ),
    (
        "svg-display-contents",
        DeclaredByBestEffort,
        "display: contents",
    ),
    // `svg-element-opacity` graduated with the group-scope rung (its
    // fixture is now a baked cell); the family's still-refused members
    // each hold a row: the fold over a gradient paint (the paint carries
    // one quantized alpha, and Chromium composites the element opacity
    // after that quantization) and the root, whose opacity composites the
    // whole canvas — inexpressible over this engine's opaque surface.
    (
        "svg-element-opacity-gradient",
        DeclaredByBestEffort,
        "gradient",
    ),
    ("svg-filter", DeclaredByBestEffort, "<filter>"),
    (
        "svg-foreign-object",
        DeclaredByBestEffort,
        "<foreignObject>",
    ),
    ("svg-gradient-focal", DeclaredByBestEffort, "focal"),
    ("svg-gradient-linearrgb", DeclaredByBestEffort, "linearRGB"),
    // Sheet-level: the pinned cascade cannot represent stop-color, so the
    // declaration is named against the sheet and the gradient renders with
    // its attribute colors — a declared divergence (Chromium honors the
    // sheet).
    (
        "svg-gradient-stop-css",
        DeclaredByBestEffort,
        "declares stop-color",
    ),
    (
        "svg-gradient-stop-style-attr",
        DeclaredByBestEffort,
        "stop-color",
    ),
    (
        "svg-gradient-unit-basis",
        DeclaredByBestEffort,
        "unit whose basis",
    ),
    ("svg-image", DeclaredByBestEffort, "<image>"),
    ("svg-mask", DeclaredByBestEffort, "<mask>"),
    (
        "svg-nested-svg",
        DeclaredByBestEffort,
        "unsupported element <svg>",
    ),
    ("svg-path-arc", DeclaredByBestEffort, "path command A"),
    (
        "svg-path-css-d-property",
        DeclaredByBestEffort,
        "declares d",
    ),
    (
        "svg-path-malformed-d",
        DeclaredByBestEffort,
        "invalid at byte 29",
    ),
    // The <defs> half of this fixture stopped declaring when the use/defs
    // rung consumed defs; the marker attribute itself is the named hole.
    ("svg-path-marker-end", DeclaredByBestEffort, "marker-end"),
    (
        "svg-path-no-leading-moveto",
        DeclaredByBestEffort,
        "invalid at byte 0",
    ),
    ("svg-path-pathlength", DeclaredByBestEffort, "pathLength"),
    (
        "svg-path-trailing-dot-number",
        DeclaredByBestEffort,
        "invalid at byte 1",
    ),
    (
        "svg-pattern-paint-server",
        DeclaredByBestEffort,
        "<pattern>",
    ),
    (
        "svg-points-odd-coordinate",
        DeclaredByBestEffort,
        "points on <polygon>",
    ),
    (
        "svg-preserve-aspect-ratio-case-folded",
        BothRefuse,
        "preserveAspectRatio",
    ),
    (
        "svg-preserve-aspect-ratio-defer",
        BothRefuse,
        "preserveAspectRatio",
    ),
    (
        "svg-preserve-aspect-ratio-invalid-align",
        BothRefuse,
        "preserveAspectRatio",
    ),
    ("svg-rect-rounded", DeclaredByBestEffort, "rx"),
    ("svg-root-opacity", BothRefuse, "root <svg>"),
    (
        "svg-smil-animate-transform",
        DeclaredByBestEffort,
        "animation element <animateTransform>",
    ),
    ("svg-smil-retarget-href", BothRefuse, "href"),
    (
        "svg-smil-set-load-active",
        DeclaredByBestEffort,
        "animation element <set>",
    ),
    (
        "svg-stroke-dasharray",
        DeclaredByBestEffort,
        "stroke-dasharray",
    ),
    (
        "svg-stroke-paint-order",
        DeclaredByBestEffort,
        "paint-order",
    ),
    (
        "svg-stroke-sheet-unit-width",
        DeclaredByBestEffort,
        "stroke-width in",
    ),
    (
        "svg-stroke-vector-effect",
        DeclaredByBestEffort,
        "vector-effect",
    ),
    ("svg-switch", DeclaredByBestEffort, "<switch>"),
    (
        "svg-text",
        DeclaredByBestEffort,
        "unsupported element <text>",
    ),
    // The use/defs rung's residue: `<use>` itself graduated (its fixture
    // is a baked cell), and the named refusals that remain each hold a row.
    (
        "svg-use-authored-children",
        DeclaredByBestEffort,
        "authored element children",
    ),
    (
        "svg-use-external",
        DeclaredByBestEffort,
        "same-document fragment",
    ),
    ("svg-use-stylesheet", DeclaredByBestEffort, "author CSS"),
    (
        "svg-use-symbol",
        DeclaredByBestEffort,
        "unsupported element <symbol>",
    ),
    ("svg-viewbox-invalid-token", BothRefuse, "viewBox"),
    ("svg-viewbox-repeated-comma", BothRefuse, "viewBox"),
    ("svg-viewbox-trailing-comma", BothRefuse, "viewBox"),
    ("svg-width-percentage", BothRefuse, "percentage width"),
];

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/web-first/unsupported")
}

fn viewport() -> InitialViewport {
    InitialViewport::new(64.0, 64.0)
}

#[test]
fn the_corpus_on_disk_is_exactly_the_declared_one() {
    let disk: BTreeSet<String> = fs::read_dir(corpus_root())
        .expect("read the unsupported corpus")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".svg"))
        .map(|name| name.trim_end_matches(".svg").to_string())
        .collect();
    let declared: BTreeSet<String> = CORPUS.iter().map(|(id, _, _)| (*id).to_string()).collect();

    assert_eq!(
        disk, declared,
        "a fixture in unsupported/ is undeclared, or a declared one is missing from disk"
    );
}

#[test]
fn every_unsupported_fixture_departs_by_name_in_both_admissions() {
    for (id, departure, named) in CORPUS {
        let source = fs::read_to_string(corpus_root().join(format!("{id}.svg")))
            .unwrap_or_else(|error| panic!("{id}: read: {error}"));

        let strict = SvgFrameSource::from_standalone_svg(source.as_str(), viewport())
            .err()
            .unwrap_or_else(|| panic!("{id}: strict must refuse an unsupported fixture"));
        assert!(
            strict.to_string().contains(named),
            "{id}: the strict refusal must name {named:?}; got {strict}"
        );

        match departure {
            BothRefuse => {
                let best =
                    SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
                        .err()
                        .unwrap_or_else(|| {
                            panic!("{id}: a document-level contract refuses in both admissions")
                        });
                assert!(
                    best.to_string().contains(named),
                    "{id}: the best-effort refusal must name {named:?}; got {best}"
                );
            }
            DeclaredByBestEffort => {
                let best =
                    SvgFrameSource::from_standalone_svg_best_effort(source.as_str(), viewport())
                        .unwrap_or_else(|error| panic!("{id}: best-effort compiles: {error}"));
                let declared: Vec<&websem::Degradation> = best
                    .degradations()
                    .iter()
                    .filter(|d| d.action() != DegradationAction::SamplesAsBase)
                    .collect();
                assert!(
                    !declared.is_empty(),
                    "{id}: best-effort rendered it with nothing declared — a silent hole"
                );
                assert!(
                    declared.iter().any(|d| d.reason().contains(named)),
                    "{id}: a declaration must name {named:?}; got {declared:?}"
                );
                assert!(
                    declared.iter().all(|d| !d.path().is_empty()),
                    "{id}: every declaration carries a structural path"
                );
            }
        }
    }
}
