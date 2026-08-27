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
    ("svg-clip-path-animation", DeclaredByBestEffort, "animation"),
    (
        "svg-clip-path-basic-shape",
        DeclaredByBestEffort,
        "basic-shape",
    ),
    (
        "svg-clip-path-cycle",
        DeclaredByBestEffort,
        "cyclic clip-path chain",
    ),
    ("svg-clip-path-external", DeclaredByBestEffort, "external"),
    (
        "svg-clip-path-geometry-box",
        DeclaredByBestEffort,
        "geometry-box",
    ),
    (
        "svg-clip-path-raster-strategy",
        DeclaredByBestEffort,
        "raster-mask strategy",
    ),
    ("svg-clip-path-root", BothRefuse, "root <svg>"),
    (
        "svg-clip-rule-css-property",
        DeclaredByBestEffort,
        "clip-rule",
    ),
    (
        "svg-clip-rule-raw-syntax",
        DeclaredByBestEffort,
        "clip-rule presentation attribute",
    ),
    (
        "svg-context-paint-fallback-extension",
        DeclaredByBestEffort,
        "non-standard fallback",
    ),
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
    (
        "svg-filter-blend-clip-precision",
        DeclaredByBestEffort,
        "filtered clip-path precision boundary",
    ),
    (
        "svg-filter-blend-transform-precision",
        DeclaredByBestEffort,
        "blend-filter transform precision boundary",
    ),
    (
        "svg-filter-color-css",
        DeclaredByBestEffort,
        "CSS color-interpolation-filters",
    ),
    (
        "svg-filter-color-matrix-source-layer-precision",
        DeclaredByBestEffort,
        "color-matrix source-layer precision boundary",
    ),
    (
        "svg-filter-color-matrix-spatial-precision",
        DeclaredByBestEffort,
        "composed-operation precision boundary",
    ),
    (
        "svg-filter-color-matrix-transform-precision",
        DeclaredByBestEffort,
        "color-matrix transform precision boundary",
    ),
    (
        "svg-filter-component-transfer-source-layer-precision",
        DeclaredByBestEffort,
        "table-filter paint-server precision boundary",
    ),
    (
        "svg-filter-component-transfer-transform-precision",
        DeclaredByBestEffort,
        "table-filter transform precision boundary",
    ),
    (
        "svg-filter-convolve-arithmetic-range",
        DeclaredByBestEffort,
        "finite native-convolution arithmetic boundary",
    ),
    (
        "svg-filter-convolve-paint-server-precision",
        DeclaredByBestEffort,
        "convolution-filter paint-server precision boundary",
    ),
    (
        "svg-filter-convolve-transform-precision",
        DeclaredByBestEffort,
        "convolution-filter transform precision boundary",
    ),
    (
        "svg-filter-color-raw-syntax",
        DeclaredByBestEffort,
        "contains a CSS comment",
    ),
    (
        "svg-filter-css-functions",
        DeclaredByBestEffort,
        "CSS filter functions",
    ),
    (
        "svg-filter-css-property",
        DeclaredByBestEffort,
        "declares filter",
    ),
    (
        "svg-filter-drop-shadow-color-precision",
        DeclaredByBestEffort,
        "native-shadow color-conversion precision boundary",
    ),
    (
        "svg-filter-drop-shadow-range",
        DeclaredByBestEffort,
        "admitted native-shadow range",
    ),
    (
        "svg-filter-drop-shadow-source-layer-precision",
        DeclaredByBestEffort,
        "native-shadow source-layer precision boundary",
    ),
    (
        "svg-filter-drop-shadow-transform-precision",
        DeclaredByBestEffort,
        "native-shadow transform precision boundary",
    ),
    (
        "svg-filter-displacement-clip-precision",
        DeclaredByBestEffort,
        "filtered clip-path precision boundary",
    ),
    (
        "svg-filter-displacement-transform-precision",
        DeclaredByBestEffort,
        "displacement-filter transform precision boundary",
    ),
    (
        "svg-filter-diffuse-composition-precision",
        DeclaredByBestEffort,
        "lighting-composition precision boundary",
    ),
    (
        "svg-filter-diffuse-transform-precision",
        DeclaredByBestEffort,
        "lighting-filter transform precision boundary",
    ),
    (
        "svg-filter-effect-stack-precision",
        DeclaredByBestEffort,
        "effect-stack precision boundary",
    ),
    ("svg-filter-external", DeclaredByBestEffort, "external"),
    (
        "svg-filter-flood-color-syntax",
        DeclaredByBestEffort,
        "outside the admitted color slice",
    ),
    (
        "svg-filter-flood-css-property",
        DeclaredByBestEffort,
        "stylesheet declares flood-color",
    ),
    (
        "svg-filter-flood-inherit",
        DeclaredByBestEffort,
        "flood-color uses inherit",
    ),
    (
        "svg-filter-flood-opacity-calc",
        DeclaredByBestEffort,
        "CSS function",
    ),
    (
        "svg-filter-flood-var",
        DeclaredByBestEffort,
        "flood-color resolves through var()",
    ),
    (
        "svg-filter-blur-sigma-precision",
        DeclaredByBestEffort,
        "small-kernel precision boundary",
    ),
    ("svg-filter-href", DeclaredByBestEffort, "href inheritance"),
    (
        "svg-filter-lighting-color-css",
        DeclaredByBestEffort,
        "CSS lighting-color",
    ),
    (
        "svg-filter-lighting-color-inherit",
        DeclaredByBestEffort,
        "lighting-color uses inherit",
    ),
    (
        "svg-filter-lighting-color-syntax",
        DeclaredByBestEffort,
        "lighting-color is outside the admitted color slice",
    ),
    (
        "svg-filter-lighting-color-var",
        DeclaredByBestEffort,
        "lighting-color resolves through var()",
    ),
    (
        "svg-filter-list",
        DeclaredByBestEffort,
        "multiple filter operations",
    ),
    (
        "svg-filter-list-quoted",
        DeclaredByBestEffort,
        "multiple filter operations",
    ),
    (
        "svg-filter-morphology-filled-ellipse-precision",
        DeclaredByBestEffort,
        "retained filled-ellipse coverage boundary",
    ),
    (
        "svg-filter-morphology-paint-server-precision",
        DeclaredByBestEffort,
        "morphology paint-server precision boundary",
    ),
    (
        "svg-filter-morphology-transform-precision",
        DeclaredByBestEffort,
        "morphology transform precision boundary",
    ),
    (
        "svg-filter-offset-blur-precision",
        DeclaredByBestEffort,
        "combines feOffset with Gaussian blur",
    ),
    (
        "svg-filter-offset-fractional-precision",
        DeclaredByBestEffort,
        "fractional displacement",
    ),
    (
        "svg-filter-offset-transform-precision",
        DeclaredByBestEffort,
        "fractional device-space displacement",
    ),
    (
        "svg-filter-pattern-source-coverage-precision",
        DeclaredByBestEffort,
        "filtered-pattern coverage precision boundary",
    ),
    (
        "svg-filter-primitive",
        DeclaredByBestEffort,
        "unsupported primitive",
    ),
    (
        "svg-filter-primitive-empty-region",
        DeclaredByBestEffort,
        "transparent graph result",
    ),
    (
        "svg-filter-region-calc",
        DeclaredByBestEffort,
        "uses calc()",
    ),
    (
        "svg-filter-region-range",
        DeclaredByBestEffort,
        "crosses the unimplemented Web used-length range",
    ),
    ("svg-filter-region-unit", DeclaredByBestEffort, "em unit"),
    ("svg-filter-region-var", DeclaredByBestEffort, "uses var()"),
    ("svg-filter-root", BothRefuse, "root <svg>"),
    (
        "svg-filter-translucent-source-composition-precision",
        DeclaredByBestEffort,
        "translucent-source composition precision boundary",
    ),
    (
        "svg-filter-turbulence-transform-precision",
        DeclaredByBestEffort,
        "procedural-filter transform precision boundary",
    ),
    (
        "svg-foreign-object",
        DeclaredByBestEffort,
        "<foreignObject>",
    ),
    (
        "svg-geometry-calc-values",
        DeclaredByBestEffort,
        "attribute cx",
    ),
    (
        "svg-geometry-css-comments",
        DeclaredByBestEffort,
        "attribute cx",
    ),
    (
        "svg-geometry-css-properties",
        DeclaredByBestEffort,
        "stylesheet declares cx",
    ),
    (
        "svg-geometry-css-wide-keywords",
        DeclaredByBestEffort,
        "attribute cx",
    ),
    (
        "svg-geometry-numeric-precision-alias",
        DeclaredByBestEffort,
        "cx numeric precision alias",
    ),
    (
        "svg-geometry-percentage-overflow",
        DeclaredByBestEffort,
        "cx resolves outside the finite frame range",
    ),
    (
        "svg-geometry-unit-values",
        DeclaredByBestEffort,
        "attribute cx",
    ),
    (
        "svg-geometry-used-range",
        DeclaredByBestEffort,
        "cx exceeds the admitted Web used-value range",
    ),
    (
        "svg-geometry-var-values",
        DeclaredByBestEffort,
        "attribute cx",
    ),
    (
        "svg-geometry-xywh-calc-values",
        DeclaredByBestEffort,
        "attribute x",
    ),
    (
        "svg-geometry-xywh-css-comments",
        DeclaredByBestEffort,
        "attribute x",
    ),
    (
        "svg-geometry-xywh-css-properties",
        DeclaredByBestEffort,
        "stylesheet declares x",
    ),
    (
        "svg-geometry-xywh-css-wide-keywords",
        DeclaredByBestEffort,
        "attribute x",
    ),
    (
        "svg-geometry-xywh-numeric-precision-alias",
        DeclaredByBestEffort,
        "x numeric precision alias",
    ),
    (
        "svg-geometry-xywh-percentage-overflow",
        DeclaredByBestEffort,
        "x resolves outside the finite frame range",
    ),
    (
        "svg-geometry-xywh-rect-auto",
        DeclaredByBestEffort,
        "attribute width",
    ),
    (
        "svg-geometry-xywh-unit-values",
        DeclaredByBestEffort,
        "attribute x",
    ),
    (
        "svg-geometry-xywh-used-range",
        DeclaredByBestEffort,
        "x exceeds the admitted Web used-value range",
    ),
    (
        "svg-geometry-xywh-var-values",
        DeclaredByBestEffort,
        "attribute x",
    ),
    ("svg-gradient-focal", DeclaredByBestEffort, "focal"),
    ("svg-gradient-linearrgb", DeclaredByBestEffort, "linearRGB"),
    // Sheet-level: the pinned cascade cannot represent stop-color, so the
    // declaration is named against the sheet and the gradient renders with
    // its attribute colors — a declared divergence (Chromium honors the
    // sheet).
    (
        "svg-gradient-degenerate-precision",
        DeclaredByBestEffort,
        "a degenerate paint server",
    ),
    (
        "svg-gradient-stop-css",
        DeclaredByBestEffort,
        "declares stop-color",
    ),
    (
        "svg-gradient-stop-function",
        DeclaredByBestEffort,
        "cannot evaluate without a computation context",
    ),
    (
        "svg-gradient-stop-inherit",
        DeclaredByBestEffort,
        "is inherit, which needs a cascaded longhand",
    ),
    (
        "svg-gradient-stop-nonlegacy-color",
        DeclaredByBestEffort,
        "non-legacy sRGB colour",
    ),
    (
        "svg-gradient-stop-style-attr",
        DeclaredByBestEffort,
        "stop-color",
    ),
    (
        "svg-gradient-stop-var",
        DeclaredByBestEffort,
        "resolves through var()",
    ),
    (
        "svg-gradient-unit-basis",
        DeclaredByBestEffort,
        "unit whose basis",
    ),
    ("svg-image", DeclaredByBestEffort, "<image>"),
    (
        "svg-mask-css-properties",
        DeclaredByBestEffort,
        "declares mask-image",
    ),
    (
        "svg-mask-cycle",
        DeclaredByBestEffort,
        "cyclic nested mask chain",
    ),
    ("svg-mask-external", DeclaredByBestEffort, "external"),
    (
        "svg-mask-full-shorthand",
        DeclaredByBestEffort,
        "full shorthand",
    ),
    (
        "svg-mask-region-calc",
        DeclaredByBestEffort,
        "mask region x uses calc()",
    ),
    (
        "svg-mask-region-unit",
        DeclaredByBestEffort,
        "mask region x uses the em unit",
    ),
    (
        "svg-mask-region-used-range",
        DeclaredByBestEffort,
        "mask region x crosses the unimplemented Web used-length range",
    ),
    (
        "svg-mask-region-var",
        DeclaredByBestEffort,
        "mask region x uses var()",
    ),
    (
        "svg-mask-resource-style-inheritance",
        DeclaredByBestEffort,
        "source-side cascade effect is not represented",
    ),
    ("svg-mask-root", BothRefuse, "root <svg>"),
    (
        "svg-mask-transform-precision",
        DeclaredByBestEffort,
        "translation/positive-downscale precision envelope",
    ),
    ("svg-mask-type-css", DeclaredByBestEffort, "CSS mask-type"),
    (
        "svg-mask-type-inherit",
        DeclaredByBestEffort,
        "mask-type presentation attribute uses inherit",
    ),
    (
        "svg-mask-type-var",
        DeclaredByBestEffort,
        "mask-type presentation attribute uses var()",
    ),
    (
        "svg-mask-var",
        DeclaredByBestEffort,
        "mask presentation attribute uses var()",
    ),
    (
        "svg-nested-svg",
        DeclaredByBestEffort,
        "unsupported element <svg>",
    ),
    (
        "svg-path-css-d-property",
        DeclaredByBestEffort,
        "declares d",
    ),
    // The <defs> half of this fixture stopped declaring when the use/defs
    // rung consumed defs; the marker attribute itself is the named hole.
    ("svg-path-marker-end", DeclaredByBestEffort, "marker-end"),
    (
        "svg-pattern-affine-precision",
        DeclaredByBestEffort,
        "picture-shader affine precision boundary",
    ),
    (
        "svg-pattern-css-transform-percentage",
        DeclaredByBestEffort,
        "pattern transform percentage has no proved reference-box basis",
    ),
    (
        "svg-pattern-external",
        DeclaredByBestEffort,
        "external template",
    ),
    (
        "svg-pattern-length-calc",
        DeclaredByBestEffort,
        "pattern width uses a CSS function",
    ),
    (
        "svg-pattern-length-css-comments",
        DeclaredByBestEffort,
        "pattern width contains a CSS comment",
    ),
    (
        "svg-pattern-length-css-wide",
        DeclaredByBestEffort,
        "pattern width uses the CSS-wide value",
    ),
    (
        "svg-pattern-length-unit",
        DeclaredByBestEffort,
        "length unit whose basis this slice does not consume",
    ),
    (
        "svg-pattern-length-used-range",
        DeclaredByBestEffort,
        "pattern x exceeds the admitted Web used-value range",
    ),
    (
        "svg-pattern-length-var",
        DeclaredByBestEffort,
        "pattern width resolves through var()",
    ),
    (
        "svg-pattern-nested-composition-precision",
        DeclaredByBestEffort,
        "picture-shader composition precision boundary",
    ),
    (
        "svg-pattern-nesting-too-deep",
        DeclaredByBestEffort,
        "nested pattern paint chain exceeds the resolved 8-program limit",
    ),
    (
        "svg-pattern-number-precision-alias",
        DeclaredByBestEffort,
        "numeric precision alias",
    ),
    (
        "svg-pattern-source-clip-precision",
        DeclaredByBestEffort,
        "picture-shader source-effect precision boundary",
    ),
    (
        "svg-pattern-source-coverage-precision",
        DeclaredByBestEffort,
        "picture-shader source-coverage precision boundary",
    ),
    (
        "svg-pattern-source-effect-precision",
        DeclaredByBestEffort,
        "picture-shader source-effect precision boundary",
    ),
    (
        "svg-pattern-source-unsupported",
        DeclaredByBestEffort,
        "source cannot compile completely",
    ),
    (
        "svg-pattern-tile-sampling-precision",
        DeclaredByBestEffort,
        "picture-shader sampling precision boundary",
    ),
    (
        "svg-pattern-transform-none-provenance",
        DeclaredByBestEffort,
        "transform:none on a derived pattern",
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
        "svg-stroke-dasharray-escape",
        DeclaredByBestEffort,
        "CSS escape",
    ),
    (
        "svg-stroke-dasharray-font-basis",
        DeclaredByBestEffort,
        "font-size",
    ),
    (
        "svg-stroke-dasharray-sheet-unit",
        DeclaredByBestEffort,
        "stroke-dasharray in",
    ),
    ("svg-stroke-dasharray-var", DeclaredByBestEffort, "var()"),
    (
        "svg-stroke-dashoffset-escape",
        DeclaredByBestEffort,
        "escape",
    ),
    (
        "svg-stroke-dashoffset-font-basis",
        DeclaredByBestEffort,
        "font-size",
    ),
    (
        "svg-stroke-dashoffset-percentage-precision-alias",
        DeclaredByBestEffort,
        "stroke-dashoffset percentage precision alias",
    ),
    (
        "svg-stroke-dashoffset-sheet-unit",
        DeclaredByBestEffort,
        "stroke-dashoffset in",
    ),
    ("svg-stroke-dashoffset-var", DeclaredByBestEffort, "var()"),
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
    (
        "svg-stroke-width-calc-mixed",
        DeclaredByBestEffort,
        "mixing lengths and percentages",
    ),
    (
        "svg-stroke-width-font-basis",
        DeclaredByBestEffort,
        "font-size",
    ),
    (
        "svg-stroke-width-percentage-precision-alias",
        DeclaredByBestEffort,
        "stroke-width percentage precision alias",
    ),
    ("svg-stroke-width-var", DeclaredByBestEffort, "var()"),
    ("svg-switch", DeclaredByBestEffort, "<switch>"),
    // The text rung's residue: `<text>` itself graduated (its fixtures are
    // baked cells in `fixtures/web-first/text/`), and what remains refused
    // holds a row. The hermetic default is the load-bearing one — a run
    // whose family is not in the declared font environment refuses by name
    // rather than reaching for an ambient face, so there is no tofu and no
    // machine-local pixel anywhere on this path.
    (
        "svg-text-undeclared-font",
        DeclaredByBestEffort,
        "not in the declared environment",
    ),
    ("svg-text-tspan", DeclaredByBestEffort, "tspan"),
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
