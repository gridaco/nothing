//! The SVG semantic compiler — the shared machinery both grammar entries use.
//!
//! One namespace-aware document (`csscascade::DemoDom`) and one browser-grade
//! cascade (`csscascade::CascadeDriver`, Stylo) resolve the source. Two
//! grammar entries reach that shared machinery: inline SVG inside an HTML
//! document (html5ever, compiled in place from the one document) and
//! conforming standalone SVG/XML (xml5ever in csscascade — namespace-aware,
//! case-preserving, recorded recoveries refused). This compiler then reads
//! *resolved* facts — geometry from presentation attributes, paint as typed
//! computed SVG values from the one cascade — and emits the source-neutral
//! [`rframe::Frame`]. It never touches the legacy SVG-only matcher, never
//! serializes-and-reparses inline SVG, and never paints.
//!
//! The admitted surface is a slice, and it is enumerated rather than implied.
//! Shapes: `<rect>`, `<circle>`, `<ellipse>`, `<path>`, `<line>`, `<polygon>`
//! and `<polyline>`, each with a solid or gradient `fill` and `stroke`
//! (`stroke-width`, `-linecap`, `-linejoin`, `-miterlimit`, and `fill-rule`
//! on a path). Containers: `<g>`, `<a>`, `<use>`/`<defs>` and the whole
//! `transform` grammar, flattened into a per-node affine rather than
//! represented — except element `opacity` (the group-scope rung), which
//! folds into a lone draw's paint or emits a real [`rframe::Scope`] by the
//! measured fold rule. Root sizing follows SVG2 §8.2: explicit
//! `width`/`height` win; a missing dimension is `auto` and resolves to 100%
//! of the host-established [`InitialViewport`] (standalone entry only — the
//! inline HTML entry refuses until CSS replaced-element sizing is
//! implemented); `viewBox` maps user units into the viewport under the full
//! `preserveAspectRatio` grammar. Time: one retained exact-time
//! `<animate attributeName="x">` on a top-level `<rect>`.
//!
//! Everything outside that list departs by name. [`CompileError`] makes the
//! static rejections explicit and [`crate::AnimationError`] closes the sampled
//! standalone dynamic inventory. Inline HTML remains Base-only until its
//! document-wide inventory is closed. This is a bounded slice with a stated
//! edge, not an exhaustive SVG-surface validator and not a capability claim.
//!
//! Two admission modes share this one compiler. Strict refuses on the first
//! beyond-slice construct — the dev harness. Best-effort (the product
//! default at the CLI) compiles the admitted subset and declares every
//! beyond-slice construct as a [`Degradation`]: subtree constructs are
//! skipped by name, a blocked dynamic surface resolves sample requests to
//! the Base view. Neither mode ever guesses pixels, and document-level
//! contracts (well-formed XML, the script-free standalone parse, the `svg`
//! root, the outer viewport sizing/mapping, the root patrols) refuse
//! identically in both.
//!
//! The admitted surface is patrolled per attribute and per cascaded
//! property, not just per element: known rendering-relevant SVG attributes
//! the slice does not consume refuse or skip by name
//! (`RENDERING_ATTRIBUTES_NOT_CONSUMED`), while attributes outside the
//! SVG rendering vocabulary stay ignored exactly as Chromium ignores them;
//! the cascaded surface is patrolled for the enumerated properties
//! `display: contents` and `stroke-dashoffset`, beside the consumed reads —
//! typed `fill`/`fill-opacity`, the stroke family (including resolved dash
//! intervals and their authored-unit patrol), the visibility
//! rung's `display: none`/`visibility` disposition, and the group-scope
//! rung's `opacity`. Cascaded properties beyond that
//! enumeration remain a **named open boundary** of the slice — not a
//! coverage claim.
//!
//! ## Where things are
//!
//! This file is large because the compiler is one algorithm, not six: a rung
//! that admits a construct touches the error type, a patrol table, a shape
//! compiler and a paint read together, and splitting those apart would scatter
//! every future rung's diff. So the map is prose rather than directories:
//!
//! | region | what lives there |
//! | --- | --- |
//! | entries and session | `SourceEntry`, `CompileMode`, [`SvgFrameSource`], the two `compile_*` functions, the child walk |
//! | departures | [`CompileError`], [`Degradation`], and every `patrol_*` — the attribute tables, the cascaded-property reads, the stylesheet scans, the unit patrol |
//! | shapes | `compile_rect`/`_circle`/`_ellipse`/`_path`/`_line` and `shape_node` |
//! | paint | `resolve_fill`, `resolve_stroke`, `resolve_fill_rule`, and the admitted colour surface |
//! | viewport | [`InitialViewport`], `parse_viewbox`, the `preserveAspectRatio` grammar and its viewport mapping |
//!
//! Two conversions *are* separate files, because they are value-in/value-out
//! and owe the compiler nothing: `svg_path` for the `d` grammar and
//! `svg_transform` for the computed `transform` operation list (the
//! *attribute* grammar lives in csscascade, which rewrites it into the one
//! CSS property at presentation-hint level).
//!
//! ## SVG paint boundary
//! Paint is consumed from the one Stylo cascade as typed values:
//! `resolve_fill` reads the computed SVG `fill` longhand, which
//! presentation hints (admitted set: `fill`), stylesheet rules, and inline
//! style attributes all feed with SVG2 precedence — csscascade owns every
//! ingress. `currentColor` resolves against the cascaded `color`; invalid
//! authored values fall back exactly as invalid CSS declarations. The
//! admitted value surface is sRGB solid colors, opaque or translucent —
//! the colour's alpha, the paint-level opacity (`fill-opacity`,
//! `stroke-opacity`), and a folded element `opacity` multiply in float and
//! quantize once, exactly what the Chromium-baked primitive suite gates
//! pixel-exactly — plus same-document linear and radial gradient paint
//! servers (the gradient rung), plus standard `context-fill` / `context-stroke`
//! relationships under expanded `<use>` instances. Context relationships
//! resolve completely here — including recursive selection, currentColor and
//! gradient reference spaces — and never cross `rframe`. Everything else
//! refuses explicitly: Stylo's non-standard context-paint fallback extension,
//! context-valued opacities, non-sRGB color spaces, and `<pattern>`
//! (`tests/typed_fill.rs` and the translucency contract pin each).
//!
//! ## Document lifetime
//! Each retained source owns one [`csscascade::adapter::DocumentSession`].
//! Stylo handles are tied to that session, so independent documents can
//! remain live and resolve colliding arena-local node identifiers without
//! ambient state. Every frame request — Base at construction, each
//! Sample(time) afterward — compiles the same retained document through the
//! same compiler; a request differs only in the effective-value view its
//! attribute reads resolve through.

use std::collections::HashMap;

use csscascade::adapter::{DocumentSession, HtmlElement};
use csscascade::cascade::CascadeDriver;
use csscascade::dom::{DemoDom, DemoNodeData, NodeId};

use style::color::{AbsoluteColor, ColorSpace};
use style::computed_values::stroke_linecap::T as StyloLinecap;
use style::computed_values::stroke_linejoin::T as StyloLinejoin;
use style::computed_values::visibility::T as Visibility;
use style::dom::TElement;

use crate::svg_paint_server::{GradientBases, PaintServers, ResolvedPaintServer};
use crate::svg_transform::{TransformRefusal, computed_transform_to_affine};
use style::properties::ComputedValues;
use style::thread_state::{self, ThreadState};
use style::values::computed::{Length, SVGOpacity, SVGPaint, Size};
use style::values::generics::basic_shape::FillRule as StyloFillRule;
use style::values::generics::svg::{SVGLength, SVGPaintKind, SVGStrokeDashArray};

use cg::CGColor;
use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    FillRule, Frame, FrameItem, FrameItems, FrameNode, Geometry, Identity, PaintStack, PathData,
    Provenance, Scope, ScopeEffect, ScopeOpacity, Stroke, StrokeCap, StrokeDashIntervals,
    StrokeDashIntervalsError, StrokeJoin, VisualRef,
};
use std::sync::Arc;

use crate::effective_values::EffectiveValues;
use crate::svg_animation::{AnimationError, AnimationInventory, is_animation_element};

/// Which grammar entry retained the source.
///
/// The entry selects the parser and the closed dynamic-inventory scope; both
/// entries produce the same semantic document shape and compile through the
/// same SVG compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceEntry {
    /// Conforming standalone SVG/XML through csscascade's namespace-aware,
    /// case-preserving XML grammar. Recorded XML5 recoveries are refused.
    StandaloneSvg,
    /// Inline SVG inside an HTML document (html5ever; the HTML document's
    /// own leniency rules apply).
    InlineHtml,
}

/// How beyond-slice constructs are handled: refused (the dev harness) or
/// declared and degraded (the product default).
///
/// Document-level contracts are identical in both modes: no `<svg>` root,
/// malformed standalone XML, and the outer viewport sizing/mapping checks
/// refuse either way — best-effort degrades subtree content, it never
/// invents the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompileMode {
    /// Refuse on the first beyond-slice construct.
    Strict,
    /// Compile the admitted subset; record every beyond-slice construct as a
    /// [`Degradation`] — dropped or resolved to Base, never guessed pixels.
    BestEffort,
}

/// How the best-effort mode handles one beyond-slice construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationAction {
    /// The element (and its subtree) was left out of the frame entirely.
    Skipped,
    /// A standing policy, not a past event: every sample request of this
    /// source resolves to the Base view, because the declared construct is
    /// outside the closed sampling inventory. Present from construction
    /// even if the consumer only ever requests Base — a Base request is not
    /// degraded by it.
    SamplesAsBase,
    /// The element rendered, but a declaration that would have changed its
    /// pixels was not honored as authored — the cascade cannot represent it,
    /// or represents it but resolves it against a basis this build lacks —
    /// and it is not attributable to one element without selector matching.
    /// Nothing was left out of the frame, so this is neither [`Self::Skipped`]
    /// nor a sampling policy.
    DeclarationIgnored,
}

/// One declared best-effort degradation: a construct the strict admission
/// would refuse — at compile time for subtree constructs ([`Skipped`]), at
/// sample time for the dynamic surface ([`SamplesAsBase`]) — handled by
/// dropping it or resolving it to Base instead, with the construct named,
/// never silently.
///
/// The set is a property of the retained source, fixed at construction: Base
/// and every Sample request share it. `path` follows the same stable
/// structural convention as [`crate::AnimationError`] (e.g. `svg/circle[1]`);
/// it is not an XML line or column.
///
/// Order is by *contract level*, then document order within a level:
/// document-level findings (a stylesheet the cascade cannot represent) come
/// first because they are established before the element walk, then the
/// walk's skips in document order, then the sampling policy entries.
///
/// [`Skipped`]: DegradationAction::Skipped
/// [`SamplesAsBase`]: DegradationAction::SamplesAsBase
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Degradation {
    path: String,
    action: DegradationAction,
    reason: String,
}

impl Degradation {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub const fn action(&self) -> DegradationAction {
        self.action
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for Degradation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.action {
            DegradationAction::Skipped => {
                write!(formatter, "skipped {}: {}", self.path, self.reason)
            }
            DegradationAction::SamplesAsBase => {
                write!(
                    formatter,
                    "samples as base ({}): {}",
                    self.path, self.reason
                )
            }
            DegradationAction::DeclarationIgnored => {
                write!(
                    formatter,
                    "declaration ignored at {}: {}",
                    self.path, self.reason
                )
            }
        }
    }
}

/// The initial viewport (SVG2 §8.2) the embedding environment establishes
/// for a standalone SVG document — the n0 host's requested raster size, the
/// oracle harness's declared fixture dimensions. A missing root
/// `width`/`height` is `auto`, and `auto` resolves to 100% of this viewport,
/// exactly as Chromium sizes a standalone SVG document to its window
/// (Blink: `core/layout/svg/layout_svg_root.cc`). Authored dimensions always
/// win over it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InitialViewport {
    width: f32,
    height: f32,
}

impl InitialViewport {
    /// The host contract requires finite, strictly positive dimensions —
    /// hosts validate their own size input (the CLI parses `WxH` as positive
    /// integers) before establishing a viewport, so a violation is a host
    /// bug, not a document refusal.
    ///
    /// # Panics
    /// If either dimension is non-finite or not strictly positive.
    #[must_use]
    pub fn new(width: f32, height: f32) -> Self {
        assert!(
            width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0,
            "initial viewport must be finite and strictly positive, got {width}x{height}"
        );
        Self { width, height }
    }

    #[must_use]
    pub const fn width(self) -> f32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> f32 {
        self.height
    }
}

/// An explicit failure in the proving shell's enumerated grammar checks.
///
/// This list is not yet exhaustive over SVG attributes or computed style; the
/// closed primitive suite defines the shell's positive coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    /// No `<svg>` element was found in the document. For the standalone
    /// entry this includes a root element outside the SVG namespace or with
    /// a local name other than exactly `svg` — XML is case-sensitive and
    /// namespaces come from authored `xmlns` declarations.
    NoSvgRoot,
    /// The standalone source is not well-formed XML: the XML5 grammar
    /// recorded a recovery, and recovered-from input is refused rather than
    /// silently accepted. (Recovery classes XML5 deliberately leaves
    /// unrecorded — e.g. unquoted attribute values — are a named open
    /// boundary pinned in csscascade's entry laws, not a claim here.)
    MalformedXml(String),
    /// An element the slice does not support. The admitted set is the shape
    /// and container dispatch in [`compile_shape`] and the child walk; the
    /// statement of record is the host README.
    UnsupportedElement(String),
    /// A `fill` value the slice cannot resolve.
    UnsupportedFill(String),
    /// A stroke value the slice cannot resolve: an untrustworthy length basis
    /// or spelling, dash phase/path calibration, an unsupported paint
    /// resource, or a resolved magnitude the frame contract cannot represent.
    UnsupportedStroke(String),
    /// A numeric attribute failed to parse.
    BadNumber { attr: String, value: String },
    /// Viewport sizing needs a default/CSS sizing path this slice lacks.
    UnsupportedSizing(String),
    /// A viewport dimension is syntactically numeric but invalid.
    InvalidDimension { attr: String, value: String },
    /// A `viewBox` whose four-number grammar or positive extent is invalid.
    BadViewBox(String),
    /// A `preserveAspectRatio` the SVG2 grammar Chromium implements does
    /// not parse: an unknown or case-folded alignment keyword, a bad
    /// `meet`/`slice` token, or the dropped SVG 1.1 `defer` prefix.
    /// Chromium silently falls back to the default `xMidYMid meet` for
    /// these; the slice refuses by name instead of silently defaulting.
    BadPreserveAspectRatio(String),
    /// A `points` list outside the SVG2 §10.4 grammar. Chromium renders the
    /// valid coordinate-pair prefix and drops the rest; this slice refuses
    /// the whole element by name instead — the same declared divergence as
    /// [`Self::BadPathData`], so an odd trailing coordinate is one named
    /// hole, never a silently different shape.
    BadPoints {
        element: String,
        /// Byte offset where the value stopped being a valid points list.
        offset: usize,
        excerpt: String,
    },
    /// A `d` value outside the SVG2 §9.3 path-data grammar. Chromium renders
    /// the value's valid prefix and drops the rest; this slice refuses the
    /// whole path by name instead of shipping an unbaked partial geometry, so
    /// the shape becomes one declared hole. Where the prefix is empty the two
    /// agree exactly — both paint nothing.
    BadPathData {
        element: String,
        /// Byte offset where the value stopped being valid path data.
        offset: usize,
        /// The authored text from that offset, clipped — a `d` value can be
        /// kilobytes long and an error is not a place to reprint one.
        excerpt: String,
    },
    /// A composed transform that overflowed to a non-finite matrix. Every
    /// computed component is finite, but composition can overflow, and the
    /// downstream contract refuses a non-finite frame transform with no
    /// element named — which would turn one bad list into a blank render.
    /// The refusal lands here, where the element is known.
    NonFiniteTransform { element: String },
    /// A `<use>` this slice must refuse by name rather than walk: an
    /// external reference (the engine is declared resource-free, and
    /// Chromium with a network would render the target), authored element
    /// children (Chromium renders the shadow content in their place), an
    /// expansion overflow (an indirect reference cycle or pathological
    /// fan-out beyond the measured shapes), or a document carrying author
    /// CSS (the measured shadow boundary scopes selector matching to the
    /// cloned subtree alone, which the one flattened tree cannot express —
    /// the shadow-matching rung's earned work).
    UnsupportedUse(String),
    /// Container nesting deeper than the compiler descends. A recursive
    /// walk cannot honor unbounded depth, so the limit is explicit rather
    /// than a stack overflow.
    ContainerTooDeep(usize),
    /// An element carried no computed style (cascade did not reach it).
    MissingComputedStyle,
    /// A known rendering-relevant SVG attribute the slice does not consume.
    /// Attributes outside the SVG rendering vocabulary are ignored exactly
    /// as Chromium ignores them; this variant fires only for attributes
    /// that would change Chromium's pixels.
    UnsupportedAttribute { element: String, attr: String },
    /// A cascaded computed value the painter would otherwise silently
    /// ignore (e.g. a stylesheet-set `opacity` or `stroke`).
    UnsupportedStyle(String),
    /// `<script>` suspends xml5ever's standalone XML parse and everything
    /// after it is silently absent from the document, so script-bearing
    /// standalone documents refuse in both admissions.
    ScriptSuspendsParse,
    /// `<script>` inside the compiled inline-SVG subtree of an HTML
    /// document: a load-time script can rewrite the authored state that the
    /// Base view renders, at any nesting depth, so it refuses in both
    /// admissions rather than rendering a possibly-stale authored state
    /// silently. (Scripts elsewhere on the page stay under the pinned
    /// first-SVG-only entry contract and the closed sampling inventory.)
    ScriptInCompiledSvg,
    /// An animation element outside the closed sampling inventory. SMIL's
    /// default `begin` is offset `0s`, so such an element is active the
    /// moment Chromium loads the document: rendering its target's authored
    /// state would be a wrong pixel, not a sampling gap. Strict refuses at
    /// construction; best-effort skips the target and declares it. One that
    /// cannot be attributed to a skippable element — an `href` retarget, a
    /// root-`<svg>` target — refuses in both admissions, like `<script>`.
    UnsupportedAnimation(AnimationError),
}

/// One retained, styled Web SVG source.
///
/// The source text and its single owned cascade session live for the full
/// lifetime of this value. Base and exact-time samples are immutable frame
/// products with stable visual identity; sampling never writes values back
/// into the DOM.
pub struct SvgFrameSource {
    source: Arc<str>,
    session: DocumentSession,
    svg_root: NodeId,
    base: Frame,
    animation: AnimationInventory,
    mode: CompileMode,
    degradations: Vec<Degradation>,
    /// The host-established initial viewport, `Some` for the standalone
    /// entry only. Fixed at construction so every sample recompile resolves
    /// root sizing identically to Base.
    initial_viewport: Option<InitialViewport>,
    /// Targets of load-active authored-state overrides, by node, with the
    /// declared reason. Best-effort only, fixed at construction: Base and
    /// every sample recompile leave these elements out identically, so a
    /// skip is a property of the retained source, never of one view.
    override_skips: HashMap<NodeId, String>,
    /// The declared font environment, fixed at construction: Base and every
    /// sample recompile resolve text against exactly the same fonts, so a
    /// run's geometry is a property of the retained source, never of one
    /// view.
    fonts: textlayout::Environment,
}

impl std::fmt::Debug for SvgFrameSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SvgFrameSource")
            .field("source_len", &self.source.len())
            .field("svg_root", &self.svg_root)
            .field("base", &self.base)
            .field("has_animation_elements", &self.has_animation_elements())
            .finish_non_exhaustive()
    }
}

impl SvgFrameSource {
    /// Retain an HTML document and compile its first inline SVG in place under
    /// the document's one Stylo cascade. Strict: the first beyond-slice
    /// construct refuses.
    pub fn from_html_inline_svg(source: impl Into<Arc<str>>) -> Result<Self, CompileError> {
        Self::from_source(
            source.into(),
            SourceEntry::InlineHtml,
            CompileMode::Strict,
            None,
        )
    }

    /// The best-effort variant of [`Self::from_html_inline_svg`]: beyond-slice
    /// constructs inside the first inline SVG are declared in
    /// [`Self::degradations`] instead of refusing. Document-level contracts
    /// (no inline `<svg>`, the outer viewport checks) still refuse.
    pub fn from_html_inline_svg_best_effort(
        source: impl Into<Arc<str>>,
    ) -> Result<Self, CompileError> {
        Self::from_source(
            source.into(),
            SourceEntry::InlineHtml,
            CompileMode::BestEffort,
            None,
        )
    }

    /// Retain one conforming standalone SVG/XML document under the
    /// host-established initial viewport.
    ///
    /// The source parses through csscascade's namespace-aware, case-preserving
    /// XML grammar into the same semantic document shape the HTML entry
    /// produces. Recorded XML recoveries are refused explicitly, and the root
    /// element must be exactly `svg` in the SVG namespace (from an authored
    /// `xmlns` declaration). These enumerated refusals track Chromium's
    /// treatment of standalone SVG documents; the recovery classes XML5
    /// leaves unrecorded (see [`CompileError::MalformedXml`]) remain a named
    /// leniency boundary, not a universal Chromium-alignment claim.
    ///
    /// `initial_viewport` is what a browser window is to a standalone SVG
    /// document: a missing root `width`/`height` is `auto` and resolves to
    /// 100% of it; explicit dimensions win over it.
    pub fn from_standalone_svg(
        source: impl Into<Arc<str>>,
        initial_viewport: InitialViewport,
    ) -> Result<Self, CompileError> {
        Self::from_source(
            source.into(),
            SourceEntry::StandaloneSvg,
            CompileMode::Strict,
            Some(initial_viewport),
        )
    }

    /// The best-effort variant of [`Self::from_standalone_svg`]: beyond-slice
    /// constructs are declared in [`Self::degradations`] instead of refusing.
    /// Document-level contracts (well-formed XML, the `svg` root, the outer
    /// viewport sizing/mapping checks) still refuse.
    pub fn from_standalone_svg_best_effort(
        source: impl Into<Arc<str>>,
        initial_viewport: InitialViewport,
    ) -> Result<Self, CompileError> {
        Self::from_source(
            source.into(),
            SourceEntry::StandaloneSvg,
            CompileMode::BestEffort,
            Some(initial_viewport),
        )
    }

    /// Retain a standalone SVG document together with the fonts a `<text>`
    /// run may resolve against. The environment is a manifest of exact bytes
    /// the host has already verified against their declared digests — this
    /// crate reads no font file and consults no ambient font database.
    pub fn from_standalone_svg_with_fonts(
        source: impl Into<Arc<str>>,
        initial_viewport: InitialViewport,
        fonts: textlayout::Environment,
    ) -> Result<Self, CompileError> {
        Self::from_source_with_fonts(
            source.into(),
            SourceEntry::StandaloneSvg,
            CompileMode::Strict,
            Some(initial_viewport),
            fonts,
        )
    }

    /// The best-effort variant of [`Self::from_standalone_svg_with_fonts`].
    pub fn from_standalone_svg_best_effort_with_fonts(
        source: impl Into<Arc<str>>,
        initial_viewport: InitialViewport,
        fonts: textlayout::Environment,
    ) -> Result<Self, CompileError> {
        Self::from_source_with_fonts(
            source.into(),
            SourceEntry::StandaloneSvg,
            CompileMode::BestEffort,
            Some(initial_viewport),
            fonts,
        )
    }

    fn from_source(
        source: Arc<str>,
        entry: SourceEntry,
        mode: CompileMode,
        initial_viewport: Option<InitialViewport>,
    ) -> Result<Self, CompileError> {
        // No declared fonts: a `<text>` run refuses by name rather than
        // reaching for an ambient face. That is the hermetic default the
        // text-oracle method ratified, not an omission.
        Self::from_source_with_fonts(
            source,
            entry,
            mode,
            initial_viewport,
            textlayout::Environment::default(),
        )
    }

    fn from_source_with_fonts(
        source: Arc<str>,
        entry: SourceEntry,
        mode: CompileMode,
        initial_viewport: Option<InitialViewport>,
        fonts: textlayout::Environment,
    ) -> Result<Self, CompileError> {
        // Idempotent for the same state; safe to call per retained source.
        thread_state::initialize(ThreadState::LAYOUT);

        let dom = match entry {
            SourceEntry::StandaloneSvg => DemoDom::parse_xml_from_bytes(source.as_bytes()),
            SourceEntry::InlineHtml => DemoDom::parse_from_bytes(source.as_bytes()),
        }
        .expect("parse document");
        if entry == SourceEntry::StandaloneSvg && !dom.errors.is_empty() {
            return Err(CompileError::MalformedXml(dom.errors.join("; ")));
        }
        let mut session = DocumentSession::new(dom);
        CascadeDriver::new(&mut session).style_document();

        let mut degradations = Vec::new();
        let (svg_root, compilation, animation, override_skips) = {
            let document = session.document();
            let root = document.root_element().ok_or(CompileError::NoSvgRoot)?;
            // xml5ever suspends its tokenizer at any <script> and the
            // blocking parse never resumes: everything after the element is
            // silently absent from the DOM with no recorded recovery. That
            // would be an undeclared hole, so the standalone entry refuses
            // script-bearing documents in both admissions.
            if entry == SourceEntry::StandaloneSvg && subtree_contains_script(root) {
                return Err(CompileError::ScriptSuspendsParse);
            }
            // A stylesheet declaration this compiler cannot honor as authored
            // — dropped by the cascade, or kept but resolved against a basis
            // this build lacks — changes pixels wherever its selector matches,
            // and it is not attributable to one element without selector
            // matching. So it is document-level: strict refuses, and
            // best-effort declares it once against the sheet and renders — a
            // named departure, never a silent one. (An *attribute* the slice
            // cannot consume is attributable, so that stays a per-element
            // hole. The asymmetry is the cost of not running selector
            // matching.) The scan starts at the document root: the HTML
            // entry's stylesheet commonly lives in <head>, outside the
            // compiled SVG subtree.
            for (reason, path) in stylesheet_findings(root) {
                match mode {
                    CompileMode::Strict => return Err(CompileError::UnsupportedStyle(reason)),
                    CompileMode::BestEffort => degradations.push(Degradation {
                        path,
                        action: DegradationAction::DeclarationIgnored,
                        reason,
                    }),
                }
            }
            let svg = match entry {
                SourceEntry::StandaloneSvg => (root.is_svg_element()
                    && root.local_name_string() == "svg")
                    .then_some(root)
                    .ok_or(CompileError::NoSvgRoot)?,
                SourceEntry::InlineHtml => find_svg(root).ok_or(CompileError::NoSvgRoot)?,
            };
            // A script anywhere inside the compiled subtree — nested in an
            // admitted shape included — can rewrite the authored state at
            // load; rendering that state silently would be wrong pixels
            // versus Chromium's static page, so it refuses in both modes.
            if entry == SourceEntry::InlineHtml && subtree_contains_script(svg) {
                return Err(CompileError::ScriptInCompiledSvg);
            }
            let mut walk_degradations = Vec::new();
            let compilation = compile_svg_element(
                svg,
                &EffectiveValues::base(),
                mode,
                &mut walk_degradations,
                initial_viewport,
                &HashMap::new(),
                &fonts,
            )?;
            let animation = AnimationInventory::inspect(svg, &compilation.top_level_shapes, entry);
            // A beyond-inventory animation element is active at document
            // load (SMIL defaults `begin` to offset 0s): Chromium paints
            // the overridden value, so the target's authored state cannot
            // render as the Base view. Strict refuses on the first, like
            // any beyond-slice construct. One that cannot be attributed to
            // a skippable element — an `href` retarget, a root-`<svg>`
            // target — is document-level and refuses in both admissions,
            // exactly as `<script>` does. Best-effort recompiles with the
            // targets left out, so each becomes a declared hole in every
            // view rather than a wrong pixel in any.
            let (compilation, walk_degradations, override_skips) =
                if let Some(first) = animation.overrides().first() {
                    if mode == CompileMode::Strict {
                        return Err(CompileError::UnsupportedAnimation(first.error().clone()));
                    }
                    if let Some(document_level) = animation
                        .overrides()
                        .iter()
                        .find(|the_override| the_override.document_level())
                    {
                        return Err(CompileError::UnsupportedAnimation(
                            document_level.error().clone(),
                        ));
                    }
                    let override_skips: HashMap<NodeId, String> = animation
                        .overrides()
                        .iter()
                        .map(|the_override| {
                            (
                                the_override.target(),
                                format!(
                                    "its authored state is overridden at document load by \
                                     the unsupported animation at {}: {}",
                                    the_override.error().path(),
                                    the_override.error().reason()
                                ),
                            )
                        })
                        .collect();
                    let mut declared = Vec::new();
                    let compilation = compile_svg_element(
                        svg,
                        &EffectiveValues::base(),
                        mode,
                        &mut declared,
                        initial_viewport,
                        &override_skips,
                        &fonts,
                    )
                    .expect(
                        "narrowing the walk with declared skips cannot change \
                         document-level compilability",
                    );
                    (compilation, declared, override_skips)
                } else {
                    (compilation, walk_degradations, HashMap::new())
                };
            degradations.extend(walk_degradations);
            (svg.node_id(), compilation, animation, override_skips)
        };
        if mode == CompileMode::BestEffort {
            for blocker in animation.blockers() {
                degradations.push(Degradation {
                    path: blocker.path().to_string(),
                    action: DegradationAction::SamplesAsBase,
                    reason: blocker.reason().to_string(),
                });
            }
        }

        Ok(Self {
            source,
            session,
            svg_root,
            base: compilation.frame,
            animation,
            mode,
            degradations,
            initial_viewport,
            override_skips,
            fonts,
        })
    }

    /// The exact retained UTF-8 source snapshot.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Whether the selected SVG subtree contains an SVG animation element,
    /// admitted or unsupported.
    pub const fn has_animation_elements(&self) -> bool {
        self.animation.has_animation_elements()
    }

    /// The authored Base frame. Animation contributes no values.
    pub fn base_frame(&self) -> Frame {
        self.base.clone()
    }

    /// Every declared best-effort degradation of this retained source: the
    /// [`DegradationAction::Skipped`] entries in document order, then every
    /// [`DegradationAction::SamplesAsBase`] entry in inspection order.
    /// Always empty for a strict source — strict handles the same
    /// constructs by refusing instead: at construction for subtree
    /// constructs, at sample time for the dynamic surface. Skips affect
    /// Base and Sample alike; a `SamplesAsBase` entry describes sample
    /// requests only.
    pub fn degradations(&self) -> &[Degradation] {
        &self.degradations
    }

    /// Produce one immutable frame at the caller-supplied exact time.
    ///
    /// The request resolves to the Web-owned effective-value view, then the
    /// retained document recompiles through the same compiler Base used —
    /// time changes effective values, never which compiler runs. No compiled
    /// frame is ever mutated afterward.
    ///
    /// The first slice closes the dynamic inventory only for the standalone
    /// SVG entry. For a strict source, inline HTML sampling and every other
    /// beyond-inventory dynamic surface fail explicitly (with the first
    /// recorded reason); a best-effort source resolves such requests to the
    /// Base view instead, with every reason declared in
    /// [`Self::degradations`] as [`DegradationAction::SamplesAsBase`].
    pub fn sample_frame(
        &self,
        time: animation_sampling::SampleTime,
    ) -> Result<Frame, crate::svg_animation::AnimationError> {
        let values = match self.animation.effective_values(time) {
            Ok(values) => values,
            Err(error) => match self.mode {
                CompileMode::Strict => return Err(error),
                CompileMode::BestEffort => EffectiveValues::base(),
            },
        };

        // Same idempotent per-thread state the construction compile used.
        thread_state::initialize(ThreadState::LAYOUT);
        let document = self.session.document();
        let root = document
            .root_element()
            .expect("retained document keeps its root element");
        let svg = find_svg(root).expect("retained document keeps its <svg> root");
        assert_eq!(
            svg.node_id(),
            self.svg_root,
            "retained document structure is immutable"
        );
        // The degradation set is a property of the retained source, declared
        // once at construction; the sample recompile reproduces the same
        // skips — the walk's and the authored-state overrides' alike —
        // deterministically, and its sink is discarded.
        let compilation = compile_svg_element(
            svg,
            &values,
            self.mode,
            &mut Vec::new(),
            self.initial_viewport,
            &self.override_skips,
            &self.fonts,
        )
        .expect("time changes effective values, not compilability of the retained source");
        Ok(compilation.frame)
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::NoSvgRoot => write!(
                f,
                "no <svg> element in document (the standalone entry requires a root `svg` \
                 element in the SVG namespace, from an authored xmlns declaration)"
            ),
            CompileError::MalformedXml(errors) => {
                write!(f, "standalone SVG is not well-formed XML: {errors}")
            }
            CompileError::UnsupportedElement(t) => write!(f, "unsupported element <{t}>"),
            CompileError::UnsupportedFill(v) => write!(f, "unsupported fill value {v:?}"),
            CompileError::UnsupportedStroke(v) => write!(f, "unsupported stroke value {v:?}"),
            CompileError::BadNumber { attr, value } => {
                write!(f, "attribute {attr}={value:?} is not a number")
            }
            CompileError::UnsupportedSizing(reason) => {
                write!(f, "unsupported SVG viewport sizing: {reason}")
            }
            CompileError::InvalidDimension { attr, value } => {
                write!(f, "invalid SVG viewport dimension {attr}={value:?}")
            }
            CompileError::BadViewBox(v) => write!(f, "viewBox {v:?} is invalid"),
            CompileError::BadPreserveAspectRatio(v) => {
                write!(f, "preserveAspectRatio {v:?} is invalid")
            }
            CompileError::BadPoints {
                element,
                offset,
                excerpt,
            } => write!(
                f,
                "points on <{element}> is invalid at byte {offset} (near {excerpt:?})"
            ),
            CompileError::BadPathData {
                element,
                offset,
                excerpt,
            } => write!(
                f,
                "path data on <{element}> is invalid at byte {offset} (near {excerpt:?})"
            ),
            CompileError::NonFiniteTransform { element } => {
                write!(f, "the composed transform on <{element}> is not finite")
            }
            CompileError::UnsupportedUse(reason) => {
                write!(f, "unsupported <use>: {reason}")
            }
            CompileError::ContainerTooDeep(limit) => {
                write!(f, "container nesting deeper than {limit} is not compiled")
            }
            CompileError::MissingComputedStyle => write!(f, "element has no computed style"),
            CompileError::UnsupportedAttribute { element, attr } => write!(
                f,
                "unsupported rendering attribute {attr} on <{element}> (not yet consumed)"
            ),
            CompileError::UnsupportedStyle(reason) => {
                write!(f, "unsupported computed style: {reason}")
            }
            CompileError::ScriptSuspendsParse => write!(
                f,
                "<script> suspends the standalone XML parse; content after it would be \
                 silently lost, so script-bearing standalone documents are refused"
            ),
            CompileError::ScriptInCompiledSvg => write!(
                f,
                "<script> inside the compiled inline SVG can rewrite the authored state \
                 the Base view renders, so it is refused in both admissions"
            ),
            CompileError::UnsupportedAnimation(error) => write!(
                f,
                "{error}; it is active at document load, so the authored state it \
                 overrides cannot render as the Base view"
            ),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile an HTML document containing inline `<svg>` into an SVG-local
/// [`Frame`]. The inline SVG's descendant style comes from the surrounding
/// HTML cascade (e.g. `color` from a `<style>` rule), never a nested renderer.
pub fn compile_html_inline_svg(html: &str) -> Result<Frame, CompileError> {
    SvgFrameSource::from_html_inline_svg(html).map(|source| source.base_frame())
}

/// Compile one conforming standalone SVG/XML document into an SVG-local
/// [`Frame`], through csscascade's namespace-aware XML grammar and the same
/// compiler as the inline entry, under the host-established initial
/// viewport.
pub fn compile_standalone_svg(
    svg: &str,
    initial_viewport: InitialViewport,
) -> Result<Frame, CompileError> {
    SvgFrameSource::from_standalone_svg(svg, initial_viewport).map(|source| source.base_frame())
}

/// Whether any element in the subtree is a `<script>` (exact local name —
/// XML is case-sensitive, and only the exact tag suspends xml5ever).
fn subtree_contains_script(el: HtmlElement<'_>) -> bool {
    // Iterative: this runs before the compiler's own bounded descent, so a
    // deep document must not exhaust the stack here instead of reaching the
    // compiler's explicit refusal.
    let mut stack = vec![el];
    while let Some(element) = stack.pop() {
        if element.local_name_string() == "script" {
            return true;
        }
        let mut child = element.first_element_child();
        while let Some(c) = child {
            stack.push(c);
            child = c.next_element_sibling();
        }
    }
    false
}

/// Whether the whole document carries any author stylesheet. Scanned from
/// the outermost element — the HTML entry's `<style>` commonly lives in
/// `<head>`, outside the compiled SVG subtree — and iterative for the same
/// stack-safety reason as [`subtree_contains_script`]. This is the
/// `<use>` patrol's document fact: the measured shadow boundary scopes
/// selector matching to the cloned subtree alone, and the one flattened
/// tree cannot express that scoping, so author CSS and `<use>` refuse
/// together until the shadow-matching rung.
/// The outermost element of the document `el` belongs to.
fn document_root(el: HtmlElement<'_>) -> HtmlElement<'_> {
    let mut top = el;
    while let Some(parent) = top.traversal_parent() {
        top = parent;
    }
    top
}

fn document_has_author_css(el: HtmlElement<'_>) -> bool {
    let top = document_root(el);
    let mut stack = vec![top];
    while let Some(element) = stack.pop() {
        if element.local_name_string() == "style" {
            return true;
        }
        let mut child = element.first_element_child();
        while let Some(c) = child {
            stack.push(c);
            child = c.next_element_sibling();
        }
    }
    false
}

/// Known rendering-relevant SVG attributes the slice does not consume. An
/// authored attribute from this set changes Chromium's pixels, so an
/// admitted element carrying one refuses (strict) or skips-and-declares
/// (best-effort) instead of painting wrong pixels. Attributes outside the
/// SVG rendering vocabulary — `data-*`, case-folded junk like `viewbox`,
/// foreign names — stay ignored exactly as Chromium ignores them (pinned by
/// the standalone entry laws). `preserveAspectRatio` is absent here because
/// the viewport mapping consumes it.
const RENDERING_ATTRIBUTES_NOT_CONSUMED: &[&str] = &[
    "transform-origin",
    // `opacity` is absent here since the group-scope rung: it is an
    // admitted presentation hint (csscascade), read as a computed value
    // and consumed as a fold or a compositing scope. `display` and
    // `visibility` are absent for the same reason (the visibility rung).
    "overflow",
    "clip",
    "clip-path",
    "clip-rule",
    "mask",
    "filter",
    // `color` is absent here since the use/defs rung: it is an admitted
    // presentation hint (csscascade), the currentColor basis the paint
    // resolvers already read from the cascade.
    "paint-order",
    "stroke-dashoffset",
    // Markers apply to `<path>`, `<line>`, `<polyline>` and `<polygon>`, and
    // this slice admits the first two. Nothing else "reads" a marker
    // property — the property *is* the paint trigger — so unlike `pathLength`
    // this patrol is load-bearing today: it is what keeps Chromium's arrowhead
    // from becoming a silent hole.
    "marker-start",
    "marker-mid",
    "marker-end",
    "shape-rendering",
    "image-rendering",
    "color-rendering",
    "color-interpolation",
    "vector-effect",
    "requiredFeatures",
    "requiredExtensions",
    "systemLanguage",
];

/// Rendering attributes additionally rejected on `<text>`.
///
/// The text slice resolves one run at one position: every attribute here
/// moves, re-measures, or re-splits that run in Chromium, so an authored one
/// refuses rather than painting a run that ignores it. `x`/`y` are consumed —
/// but only in their single-number form; the SVG list spellings that
/// position glyphs individually are a different construct, refused by the
/// number grammar itself.
const TEXT_RENDERING_ATTRIBUTES_NOT_CONSUMED: &[&str] = &[
    "dx",
    "dy",
    "rotate",
    "textLength",
    "lengthAdjust",
    "xml:space",
    "writing-mode",
    "direction",
    "unicode-bidi",
    "letter-spacing",
    "word-spacing",
    "text-decoration",
    "dominant-baseline",
    "alignment-baseline",
    "baseline-shift",
    "font-weight",
    "font-style",
    "font-stretch",
    "font-variant",
    "font",
];

/// Rendering attributes additionally rejected on every admitted geometry
/// element (`rect`, `circle`, `ellipse`, `path`, `line`, `polygon`, and
/// `polyline`).
///
/// `pathLength` scales user-space distance for dashing. That interaction was
/// measured in Chromium on `path`, `rect`, `circle`, and `ellipse`; the patrol
/// covers all seven admitted SVG geometry elements because `pathLength` is an
/// SVGGeometryElement attribute. It also affects dash offset, markers, and text
/// on a path. Dashing is consumed now, so this patrol is load-bearing: dropping
/// it would feed uncalibrated intervals to the resolved contract and paint a
/// silently different cycle.
const GEOMETRY_RENDERING_ATTRIBUTES_NOT_CONSUMED: &[&str] = &["pathLength"];

/// Rendering attributes additionally rejected on the root `<svg>`.
///
/// `transform` is admitted on containers and shapes, where it maps user
/// space. On the *outermost* `<svg>` it means something else — Chromium
/// applies it to the element's CSS box, outside the viewBox mapping, so
/// composing it like a container's would place the content wrongly. Until
/// that rung, the root's own transform refuses by name rather than being
/// silently dropped.
const ROOT_RENDERING_ATTRIBUTES_NOT_CONSUMED: &[&str] = &["transform"];

/// CSS property names that move, clip, or recolor Chromium's pixels and that no
/// computed-value read here would catch. Three mechanisms put a name in this
/// list, and the distinction is worth keeping straight because it decides
/// whether a future rung could read the property instead:
///
/// - **Not represented: engine-gated.** `d`, `vector-effect`,
///   `shape-rendering`, `clip-rule`, `paint-order`, `transform-box`,
///   `marker-start`/`-mid`/`-end` and `offset-distance`/`offset-rotate` are
///   `engine = "gecko"` in the pinned Stylo `longhands.toml` (`marker` and
///   `offset` likewise in `shorthands.toml`), so the cascade drops the
///   declaration at parse and there is no computed value to read. (`d` is the
///   sharpest of these: Chromium renders a stylesheet's `d: path(…)` in place
///   of the attribute — measured — so a document could otherwise paint one
///   geometry here and another in the browser.)
/// - **Not represented: pref-gated.** `backdrop-filter`, `mask-image`,
///   `mask` and `offset-path` carry `servo_pref = "layout.unimplemented"`,
///   which this build pins off, so they are dropped exactly as the
///   engine-gated names are.
/// - **Represented but unconsumed.** `translate`, `rotate`, `scale`,
///   `transform-origin`, `clip-path`, `filter`, `mix-blend-mode` and
///   `isolation` and `stroke-dashoffset` *do* compute in this build; this
///   compiler simply does not read them, and reading them would mean
///   implementing them. (`transform` was the
///   first name to leave this list: the transform rung reads its computed
///   value in [`compose_element_transform`], so a cascaded declaration is
///   consumed, not smuggled. The individual `translate`/`rotate`/`scale`
///   properties stay — Chromium composes them *with* `transform`, so reading
///   one without the others would compose a different matrix.)
///
/// Either way a cascaded declaration would paint in Chromium and not here, so
/// the scan below reads the authored CSS text — the only ingresses this
/// document model has are `<style>` elements and `style` attributes, and both
/// are scanned.
///
/// Two kinds of entry are **not** longhands and must not be dropped as
/// redundant. A **shorthand** changes the listed properties without naming any
/// of them (`all` resets every one; `mask`, `marker` and `offset` expand into
/// listed longhands), so it needs its own entry. And a **vendor alias** is the
/// same property under another spelling — those are handled by prefix
/// stripping in [`unrepresented_property`] rather than by listing each one.
///
/// The attribute spellings are refused separately by
/// [`RENDERING_ATTRIBUTES_NOT_CONSUMED`]. This is a closed list of
/// known-dangerous names, not a claim about every property: the remainder
/// stays the named open boundary the module doc declares.
const CASCADE_PROPERTIES_NOT_REPRESENTED: &[&str] = &[
    // The shorthand that resets everything, including every name below it.
    // Measured: `all: initial` and `all: unset` make Chromium paint nothing
    // where this compiler would still paint attribute geometry.
    "all",
    "d",
    // Consumed as attributes by the conic rung, and Chromium honors the CSS
    // spelling over the attribute (measured) — but the pinned cascade cannot
    // represent these longhands, so a stylesheet declaring one would paint
    // attribute geometry here and property geometry there.
    "rx",
    "ry",
    "vector-effect",
    "marker",
    "marker-start",
    "marker-mid",
    "marker-end",
    "shape-rendering",
    "transform-origin",
    "transform-box",
    "translate",
    "rotate",
    "scale",
    "clip-path",
    "clip-rule",
    "filter",
    "backdrop-filter",
    "mask",
    "mask-image",
    "mix-blend-mode",
    "isolation",
    "paint-order",
    // Represented by the pinned cascade but deliberately not consumed by the
    // zero-phase dash contract. Scanning authored CSS is load-bearing: a unit
    // whose basis this build lacks can compute to zero while Chromium moves
    // the pattern, so checking only the computed nonzero case would leak.
    "stroke-dashoffset",
    // The gradient rung's sheet-level patrol: Chromium consumes these from
    // the cascade (measured: `stop { stop-color: red }` beats the
    // attribute), but the pinned servo cascade has no such longhands — a
    // sheet declaring one was a silent drop before this row.
    "stop-color",
    "stop-opacity",
    "color-interpolation",
    // The text rung's row, the same shape one rung later: Chromium consumes
    // `text-anchor` from the cascade (measured: a rule anchors an
    // attribute-free `<text>`, and `text-anchor: end` beats
    // `text-anchor="middle"`), but the property is `engine = "gecko"` at the
    // Stylo pin, so the servo build has no such longhand and a sheet
    // declaring one is a silent drop. The *attribute* spelling is admitted
    // and read directly by the text compiler.
    "text-anchor",
    // CSS motion path: measured to translate and rotate an SVG shape — and a
    // whole `<g>` subtree — off its authored position.
    "offset",
    "offset-path",
    "offset-distance",
    "offset-rotate",
    "offset-anchor",
    "offset-position",
];

/// Vendor prefixes stripped before a scanned property name is compared against
/// [`CASCADE_PROPERTIES_NOT_REPRESENTED`].
///
/// Chromium honors `-webkit-clip-path`, `-webkit-transform` and
/// `-webkit-filter` on an SVG shape with full effect (measured), and the
/// unprefixed names of the *refused* pair are on the list — so the alias was
/// never a new capability question, only a spelling the scan missed. Stripping
/// is the safer shape than enumerating aliases: it can only ever *add* a
/// refusal, and the alias family keeps growing. A name that graduates off the
/// list takes its aliases with it, and that is checked, not assumed:
/// `-webkit-transform` passes this scan *and* computes — the pinned Stylo
/// implements the alias (a precedence row pins it), so the spelling Chromium
/// applies is the spelling the cascade carries.
const VENDOR_PREFIXES: &[&str] = &["-webkit-", "-moz-", "-ms-", "-o-"];

/// The first [`CASCADE_PROPERTIES_NOT_REPRESENTED`] property name declared
/// in a CSS fragment — a `style` attribute's declaration block or a
/// `<style>` element's whole text.
///
/// This reads the authored text rather than the cascade because the cascade is
/// exactly what discards these declarations.
///
/// **It is not a CSS tokenizer, and the gap is bounded deliberately.** A scan
/// over `{`/`}`/`;`-delimited chunks sees a *different string* than Chromium's
/// tokenizer does wherever CSS syntax lets a name be spelled indirectly, and
/// each such spelling was a measured leak: `d/**/: path(…)` and `/**/d: path(…)`
/// both render in Chromium, and so does the ident escape `\000064: path(…)`.
/// So the scan now
///
/// - strips `/* … */` comments first, which is what makes the name contiguous;
/// - strips a [vendor prefix](VENDOR_PREFIXES) before the lookup;
/// - and refuses *any* declaration whose property name carries a backslash,
///   without decoding it — an escape can spell any name at all, and this
///   compiler consumes exactly two CSS properties (`fill`, `fill-rule`, plus
///   the enumerated patrols), so a document that escapes a property name is one
///   this slice should not be quietly rendering either way.
///
/// The honest fix is to tokenize with the CSS parser the document already
/// carries; until then the direction of every rule here is the same — it can
/// only ever *add* a refusal, never admit one.
fn unrepresented_property(css: &str) -> Option<String> {
    let css = strip_css_comments(css);
    for chunk in css.split([';', '{', '}']) {
        let Some((name, _)) = chunk.split_once(':') else {
            continue;
        };
        let name = trim_svg_whitespace(name).to_ascii_lowercase();
        if name.contains('\\') {
            return Some(name);
        }
        let unprefixed = VENDOR_PREFIXES
            .iter()
            .find_map(|prefix| name.strip_prefix(prefix))
            .unwrap_or(name.as_str());
        if CASCADE_PROPERTIES_NOT_REPRESENTED.contains(&unprefixed) {
            return Some(name);
        }
    }
    None
}

/// The first basis-less length unit declared for `stroke-width` in a CSS
/// fragment, or `None`.
///
/// The unit list and the token test are [`has_unit_without_a_basis`]'s, so a
/// sheet refuses on exactly the units the two attribute ingresses refuse on —
/// one rule, three spellings. Only the declaration's value is scanned, not the
/// whole fragment, because a sheet carries every rule in the document and
/// scanning it whole would refuse on a `1ex` belonging to some other property.
///
/// It shares [`unrepresented_property`]'s chunking and its bounded gap: this is
/// not a CSS tokenizer. An escaped property name is already refused wholesale by
/// that scan, so `\000073troke-width` needs no handling here.
fn stylesheet_stroke_length_unit(css: &str, property: &str) -> Option<&'static str> {
    let css = strip_css_comments(css);
    css.split([';', '{', '}'])
        .filter_map(|chunk| chunk.split_once(':'))
        .filter(|(name, _)| trim_svg_whitespace(name).eq_ignore_ascii_case(property))
        .find_map(|(_, value)| has_unit_without_a_basis(value))
}

fn stylesheet_stroke_width_unit(css: &str) -> Option<&'static str> {
    stylesheet_stroke_length_unit(css, "stroke-width")
}

fn stylesheet_stroke_dasharray_unit(css: &str) -> Option<&'static str> {
    stylesheet_stroke_length_unit(css, "stroke-dasharray")
}

/// Whether a sheet declares a `stroke-width` through `var()` — an indirection
/// the unit scan above cannot follow (measured: `--w: 1vw` substituted to a
/// silent 12.8 where Chromium paints 0.64).
fn stylesheet_stroke_length_var(css: &str, property: &str) -> bool {
    let css = strip_css_comments(css);
    css.split([';', '{', '}'])
        .filter_map(|chunk| chunk.split_once(':'))
        .filter(|(name, _)| trim_svg_whitespace(name).eq_ignore_ascii_case(property))
        .any(|(_, value)| value.to_ascii_lowercase().contains("var("))
}

fn stylesheet_stroke_width_var(css: &str) -> bool {
    stylesheet_stroke_length_var(css, "stroke-width")
}

fn stylesheet_stroke_dasharray_var(css: &str) -> bool {
    stylesheet_stroke_length_var(css, "stroke-dasharray")
}

/// Whether a sheet declares a `stroke-width` in `em`/`rem` — admitted units
/// whose basis is the cascaded `font-size`, which must then be trustworthy
/// everywhere (see [`poisons_font_basis`]).
fn stylesheet_stroke_length_font_relative(css: &str, property: &str) -> Option<&'static str> {
    let css = strip_css_comments(css);
    css.split([';', '{', '}'])
        .filter_map(|chunk| chunk.split_once(':'))
        .filter(|(name, _)| trim_svg_whitespace(name).eq_ignore_ascii_case(property))
        .find_map(|(_, value)| has_font_relative_unit(value))
}

fn stylesheet_stroke_width_font_relative(css: &str) -> Option<&'static str> {
    stylesheet_stroke_length_font_relative(css, "stroke-width")
}

fn stylesheet_stroke_dasharray_font_relative(css: &str) -> Option<&'static str> {
    stylesheet_stroke_length_font_relative(css, "stroke-dasharray")
}

/// Whether a sheet's `font-size` (or `font` shorthand) declaration would set
/// an `em` basis this build cannot reproduce.
fn stylesheet_font_size_poison(css: &str) -> Option<&'static str> {
    let css = strip_css_comments(css);
    css.split([';', '{', '}'])
        .filter_map(|chunk| chunk.split_once(':'))
        .filter(|(name, _)| {
            let name = trim_svg_whitespace(name);
            name.eq_ignore_ascii_case("font-size") || name.eq_ignore_ascii_case("font")
        })
        .find_map(|(_, value)| poisons_font_basis(value))
}

/// Remove `/* … */` comments so a property name split by one becomes
/// contiguous, exactly as it is to the CSS tokenizer. An unterminated comment
/// runs to the end of the fragment, which is also what CSS says.
fn strip_css_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Whether a CSS declaration fragment names one exact property after comments
/// are removed. Property names are ASCII-case-insensitive; escaped names are
/// refused earlier by [`unrepresented_property`] rather than decoded here.
fn css_declares_property(css: &str, property: &str) -> bool {
    let css = strip_css_comments(css);
    css.split([';', '{', '}'])
        .filter_map(|chunk| chunk.split_once(':'))
        .any(|(name, _)| trim_svg_whitespace(name).eq_ignore_ascii_case(property))
}

/// Patrol an element's authored `style` attribute for a declaration the
/// cascade drops but Chromium paints with.
fn patrol_style_attribute(
    element: HtmlElement<'_>,
    element_name: &str,
) -> Result<(), CompileError> {
    if let Some(style) = get_attr(element, "style")
        && let Some(property) = unrepresented_property(&style)
    {
        return Err(CompileError::UnsupportedStyle(format!(
            "style attribute on <{element_name}> declares {property}, which this cascade \
             does not represent"
        )));
    }
    Ok(())
}

/// Every reason a `<style>` element in the document forces a departure, each
/// with the sheet's structural path.
///
/// Two scans, because a sheet can go wrong in two ways. It can declare a
/// property the cascade drops but Chromium paints with, and it can declare one
/// the cascade *keeps* but resolves against a basis this build lacks — a
/// `stroke-width` in `ex` or `vw`. The second was the leak this scan was added
/// for: [`patrol_stroke_width_units`] catches those units on the presentation
/// attribute and the `style` attribute, but a sheet is not attributable to an
/// element, so the same declaration rendered at a silently wrong width from a
/// sheet while both attribute spellings refused by name.
///
/// A stylesheet is not attributable to one element without running selector
/// matching, so the caller treats each finding as document-level: strict
/// refuses on the first, best-effort declares them all and renders. All of
/// them, not just the first — a second sheet declaring a different property
/// would otherwise render as-absent with nothing said.
///
/// The walk is iterative. A recursive one would be a second descent over the
/// same tree the compiler bounds, and it runs *before* the compiler, so a
/// deep document would exhaust the stack here instead of reaching the
/// compiler's explicit refusal.
///
/// The sheet text is **concatenated across every text child**, which is what
/// the cascade compiles (`csscascade::cascade::collect_author_styles`) and what
/// a browser's `textContent` yields. Scanning each text node separately let a
/// comment node inside `<style>` split a declaration across two nodes — the
/// concatenated sheet was valid to Chromium while neither fragment named a
/// listed property (measured). Whatever is in force is what must be patrolled.
fn stylesheet_findings(root: HtmlElement<'_>) -> Vec<(String, String)> {
    let mut found = Vec::new();
    // A sheet can declare a stroke-width in `em`/`rem` — an admitted unit whose
    // font-size basis may be poisoned anywhere: in the same sheet, another
    // sheet, or an element's own attributes. The walk visits all of them, so
    // both halves are collected and judged after it.
    let mut sheet_width_font_relative: Option<(&'static str, String)> = None;
    let mut sheet_dasharray_font_relative: Option<(&'static str, String)> = None;
    let mut font_poison: Option<&'static str> = None;
    let mut stack = vec![(root, root.local_name_string())];
    while let Some((element, path)) = stack.pop() {
        if element.local_name_string() == "style" {
            let mut sheet = String::new();
            for child_id in &element.dom_node().children {
                if let DemoNodeData::Text(text) = &element.dom().node(*child_id).data {
                    sheet.push_str(text);
                }
            }
            if sheet.contains('\\') {
                // An escape can hide a property name or a unit from every scan
                // below (`1\76 w` is `1vw` to the tokenizer) — measured painting
                // a silent 12.8 before this finding existed.
                found.push((
                    "a stylesheet carries a CSS escape these patrols cannot read; its \
                     declarations cannot be checked by name"
                        .to_string(),
                    path.clone(),
                ));
            }
            if let Some(property) = unrepresented_property(&sheet) {
                found.push((
                    format!(
                        "a stylesheet declares {property}, which this cascade does not \
                         represent; elements it matches render without it"
                    ),
                    path.clone(),
                ));
            }
            if let Some(unit) = stylesheet_stroke_width_unit(&sheet) {
                found.push((
                    format!(
                        "a stylesheet declares a stroke-width in {unit}, which needs a basis \
                         this cascade does not have; elements it matches render at the wrong \
                         width"
                    ),
                    path.clone(),
                ));
            }
            if stylesheet_stroke_width_var(&sheet) {
                found.push((
                    "a stylesheet declares a stroke-width through var(), an indirection this \
                     patrol cannot follow; elements it matches may render at the wrong width"
                        .to_string(),
                    path.clone(),
                ));
            }
            if let Some(unit) = stylesheet_stroke_dasharray_unit(&sheet) {
                found.push((
                    format!(
                        "a stylesheet declares a stroke-dasharray in {unit}, which needs a \
                         basis this cascade does not have; elements it matches render the \
                         wrong dash cycle"
                    ),
                    path.clone(),
                ));
            }
            if stylesheet_stroke_dasharray_var(&sheet) {
                found.push((
                    "a stylesheet declares a stroke-dasharray through var(), an indirection \
                     this patrol cannot follow; elements it matches may render the wrong dash \
                     cycle"
                        .to_string(),
                    path.clone(),
                ));
            }
            if sheet_width_font_relative.is_none()
                && let Some(unit) = stylesheet_stroke_width_font_relative(&sheet)
            {
                sheet_width_font_relative = Some((unit, path.clone()));
            }
            if sheet_dasharray_font_relative.is_none()
                && let Some(unit) = stylesheet_stroke_dasharray_font_relative(&sheet)
            {
                sheet_dasharray_font_relative = Some((unit, path.clone()));
            }
            if font_poison.is_none() {
                font_poison = stylesheet_font_size_poison(&sheet);
            }
        }
        if font_poison.is_none() {
            for text in [
                get_attr(element, "font-size"),
                get_attr(element, "style")
                    .filter(|style| style.to_ascii_lowercase().contains("font")),
            ]
            .into_iter()
            .flatten()
            {
                font_poison = poisons_font_basis(&text);
                if font_poison.is_some() {
                    break;
                }
            }
        }
        let mut ordinals = HashMap::<String, usize>::new();
        let mut children = Vec::new();
        let mut child = element.first_element_child();
        while let Some(c) = child {
            let tag = c.local_name_string();
            let ordinal = ordinals.entry(tag.clone()).or_default();
            *ordinal += 1;
            children.push((c, format!("{path}/{tag}[{ordinal}]")));
            child = c.next_element_sibling();
        }
        // Depth-first in document order: the stack pops in reverse.
        stack.extend(children.into_iter().rev());
    }
    if let (Some((unit, path)), Some(poison)) = (sheet_width_font_relative, font_poison) {
        found.push((
            format!(
                "a stylesheet declares a stroke-width in {unit} while an authored font-size \
                 carries {poison}; the em basis cannot be trusted and elements it matches may \
                 render at the wrong width"
            ),
            path,
        ));
    }
    if let (Some((unit, path)), Some(poison)) = (sheet_dasharray_font_relative, font_poison) {
        found.push((
            format!(
                "a stylesheet declares a stroke-dasharray in {unit} while an authored \
                 font-size carries {poison}; the em basis cannot be trusted and elements it \
                 matches may render the wrong dash cycle"
            ),
            path,
        ));
    }
    found
}

fn patrol_rendering_attributes(
    element: HtmlElement<'_>,
    element_name: &str,
    extra: &[&str],
) -> Result<(), CompileError> {
    if let DemoNodeData::Element(e) = &element.dom_node().data {
        for a in &e.attrs {
            let local = a.name.local.as_ref();
            if RENDERING_ATTRIBUTES_NOT_CONSUMED.contains(&local) || extra.contains(&local) {
                return Err(CompileError::UnsupportedAttribute {
                    element: element_name.to_string(),
                    attr: local.to_string(),
                });
            }
        }
    }
    Ok(())
}

/// Patrol the cascaded properties a stylesheet or `style` attribute could
/// smuggle past the attribute patrol and the painter would otherwise
/// silently ignore. The patrolled set is enumerated — `opacity`,
/// `display: none`, `visibility`, and (for shapes) `stroke` paint, beside
/// the typed `fill`/`fill-opacity` reads in [`resolve_fill`]. Cascaded
/// properties beyond this enumeration remain a named open boundary of the
/// slice, not a covered claim.
///
/// `include_css_sizing` patrols cascaded `width`/`height` only where SVG2's
/// geometry-property applicability table makes them geometry — of the
/// admitted elements, the root `<svg>` and `<rect>`; on
/// `<circle>`/`<ellipse>` both properties are
/// inert in Chromium, so a cascaded value there is not a smuggled size and
/// must not over-refuse. The geometry properties that *do* apply to those
/// elements (`cx`/`cy`/`r`/`rx`/`ry`) do not exist as longhands in the
/// pinned servo-mode Stylo build (`engine = "gecko"`-gated, like the bare
/// `x`/`y` longhands) — they stay inside the named open boundary above.
/// What the cascaded values decide about an element's participation in the
/// frame — the visibility rung's consumed pair, beside the patrols that
/// still refuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderDisposition {
    /// The element participates normally.
    Renders,
    /// `visibility: hidden | collapse` (identical for shapes — measured):
    /// the element's own paint is off, and that is the whole effect.
    /// `visibility` inherits, so a descendant whose computed value is
    /// `visible` un-hides itself (measured) — the walk therefore still
    /// descends; each element's *own* computed value decides its node.
    HiddenPaint,
    /// `display: none`: the element generates no box, so the whole subtree
    /// is pruned — a `visibility: visible` descendant stays gone
    /// (measured). Nothing else about the element matters.
    PrunedSubtree,
}

/// What the disposition-and-opacity patrol reads for one element: whether
/// it renders at all, and its own computed `opacity` — clamped to [0, 1]
/// exactly as Chromium uses it (measured: `1.5` paints opaque, `-0.5`
/// paints nothing). The caller decides what the opacity means at its
/// level: a shape or container folds or scopes it (the group-scope rung),
/// the root refuses a non-1 value by name.
struct ComputedPatrol {
    disposition: RenderDisposition,
    opacity: f32,
}

#[derive(Debug, Clone, Copy)]
enum MeasuredGeometry {
    /// An admitted subtree whose geometry is known to be empty.
    Empty,
    /// A complete geometry box.
    Rect(Rectangle),
    /// A subtree containing geometry this prepass cannot measure. This is
    /// deliberately distinct from `Empty`: using the admitted siblings'
    /// partial union would silently move a context gradient.
    Unknown,
}

impl MeasuredGeometry {
    fn include(&mut self, next: Self) {
        *self = match (*self, next) {
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::Empty, other) | (other, Self::Empty) => other,
            (Self::Rect(current), Self::Rect(next)) => Self::Rect(math2::union(&[current, next])),
        };
    }

    fn transformed(self, transform: AffineTransform) -> Self {
        match self {
            Self::Empty => Self::Empty,
            Self::Rect(rect) => Self::Rect(measured_geometry_box(rect, transform)),
            Self::Unknown => Self::Unknown,
        }
    }

    fn reference_box(self) -> Option<Rectangle> {
        match self {
            Self::Empty => Some(Rectangle::empty()),
            Self::Rect(rect) => Some(rect),
            Self::Unknown => None,
        }
    }
}

/// Chromium defines a context element's object box from each descendant's
/// transformed *local AABB*, including for rotated/skewed curves. That is not
/// the tight affine bounds of the rendered curve; the capability probe's
/// tight-bounds controls discriminate this rule.
fn measured_geometry_box(geometry: Rectangle, transform: AffineTransform) -> Rectangle {
    math2::rect_transform(geometry, &transform)
}

/// Paint-independent geometry inventory for context paint. Each `<use>` box
/// is the union of its expanded descendants in that use's own user space.
/// Visibility and opacity do not remove geometry; `display:none` prunes it.
/// Measuring all use boxes before emitting a node prevents paint order from
/// choosing a partial objectBoundingBox.
fn measure_use_boxes(
    svg: HtmlElement<'_>,
    values: &EffectiveValues,
    bases: PercentBases,
    fonts: &textlayout::Environment,
) -> Result<HashMap<NodeId, Option<Rectangle>>, CompileError> {
    let mut boxes = HashMap::new();
    measure_use_boxes_in_subtree(svg, values, bases, fonts, &mut boxes, 0)?;
    Ok(boxes)
}

fn measure_use_boxes_in_subtree(
    parent: HtmlElement<'_>,
    values: &EffectiveValues,
    bases: PercentBases,
    fonts: &textlayout::Environment,
    boxes: &mut HashMap<NodeId, Option<Rectangle>>,
    depth: usize,
) -> Result<(), CompileError> {
    if depth >= MAX_CONTAINER_DEPTH {
        return Err(CompileError::ContainerTooDeep(MAX_CONTAINER_DEPTH));
    }
    let mut child = parent.first_element_child();
    while let Some(el) = child {
        let tag = el.local_name_string();
        if is_non_rendering_element(&tag)
            || is_animation_element(&tag)
            || tag == "defs"
            || tag == "linearGradient"
            || tag == "radialGradient"
            || tag == "pattern"
        {
            child = el.next_element_sibling();
            continue;
        }
        let Some(data) = el.borrow_data() else {
            return Err(CompileError::MissingComputedStyle);
        };
        let display_none = data.styles.primary().clone_display().is_none();
        drop(data);
        if display_none {
            child = el.next_element_sibling();
            continue;
        }
        if tag == "use" {
            let measured = match measure_subtree_geometry(
                el,
                values,
                bases,
                fonts,
                boxes,
                AffineTransform::identity(),
                depth + 1,
            ) {
                Ok(measured) => measured.reference_box(),
                Err(_) => None,
            };
            boxes.insert(el.node_id(), measured);
        } else {
            // A nested use is indexed independently. An unrelated malformed
            // or unsupported subtree cannot make a context-free document
            // fail merely because the prepass saw it.
            let _ = measure_use_boxes_in_subtree(el, values, bases, fonts, boxes, depth + 1);
        }
        child = el.next_element_sibling();
    }
    Ok(())
}

fn measure_subtree_geometry(
    parent: HtmlElement<'_>,
    values: &EffectiveValues,
    bases: PercentBases,
    fonts: &textlayout::Environment,
    boxes: &mut HashMap<NodeId, Option<Rectangle>>,
    transform: AffineTransform,
    depth: usize,
) -> Result<MeasuredGeometry, CompileError> {
    if depth >= MAX_CONTAINER_DEPTH {
        return Err(CompileError::ContainerTooDeep(MAX_CONTAINER_DEPTH));
    }
    let mut union = MeasuredGeometry::Empty;
    let mut child = parent.first_element_child();
    while let Some(el) = child {
        let tag = el.local_name_string();
        if is_non_rendering_element(&tag)
            || is_animation_element(&tag)
            || tag == "defs"
            || tag == "linearGradient"
            || tag == "radialGradient"
            || tag == "pattern"
        {
            child = el.next_element_sibling();
            continue;
        }
        let Some(data) = el.borrow_data() else {
            return Err(CompileError::MissingComputedStyle);
        };
        let style: &ComputedValues = data.styles.primary();
        if style.clone_display().is_none() {
            child = el.next_element_sibling();
            continue;
        }
        drop(data);
        let next = if tag == "g" || tag == "a" {
            let own = compose_element_transform(el, transform, &tag, bases)?;
            measure_subtree_geometry(el, values, bases, fonts, boxes, own, depth + 1)?
        } else if tag == "use" {
            let use_space = compose_element_transform(el, transform, "use", bases)?;
            let x = geometry_attr_f32(el, "x", values, bases)?.unwrap_or(0.0);
            let y = geometry_attr_f32(el, "y", values, bases)?.unwrap_or(0.0);
            let child_space = AffineTransform::from_acebdf(1.0, 0.0, x, 0.0, 1.0, y);
            // A use owns its descendants' box *before its own x/y*. The x/y
            // belongs to the consumption chain and is applied when this use
            // contributes to an outer owner's union. Keeping that convention
            // prevents an immediate inner URL owner from counting its x/y in
            // both its stored box and the clone's paint translation.
            let local = measure_subtree_geometry(
                el,
                values,
                bases,
                fonts,
                boxes,
                AffineTransform::identity(),
                depth + 1,
            )?;
            boxes.insert(el.node_id(), local.reference_box());
            local.transformed(use_space.compose(&child_space))
        } else {
            let own = compose_element_transform(el, transform, &tag, bases)?;
            measure_leaf_geometry(el, values, bases, fonts)?.transformed(own)
        };
        union.include(next);
        child = el.next_element_sibling();
    }
    Ok(union)
}

fn measure_leaf_geometry(
    el: HtmlElement<'_>,
    values: &EffectiveValues,
    bases: PercentBases,
    fonts: &textlayout::Environment,
) -> Result<MeasuredGeometry, CompileError> {
    let rect = match el.local_name_string().as_str() {
        "rect" => {
            let x = geometry_attr_f32(el, "x", values, bases)?.unwrap_or(0.0);
            let y = geometry_attr_f32(el, "y", values, bases)?.unwrap_or(0.0);
            let w = box_extent(geometry_attr_f32(el, "width", values, bases)?.unwrap_or(0.0));
            let h = box_extent(geometry_attr_f32(el, "height", values, bases)?.unwrap_or(0.0));
            Rectangle::from_xywh(x, y, w, h)
        }
        "circle" => {
            let cx = geometry_attr_f32(el, "cx", values, bases)?.unwrap_or(0.0);
            let cy = geometry_attr_f32(el, "cy", values, bases)?.unwrap_or(0.0);
            let r = geometry_attr_f32(el, "r", values, bases)?
                .unwrap_or(0.0)
                .max(0.0);
            Rectangle::from_xywh(cx - r, cy - r, r * 2.0, r * 2.0)
        }
        "ellipse" => {
            let cx = geometry_attr_f32(el, "cx", values, bases)?.unwrap_or(0.0);
            let cy = geometry_attr_f32(el, "cy", values, bases)?.unwrap_or(0.0);
            let rx = ellipse_radius(el, "rx", values, bases)?;
            let ry = ellipse_radius(el, "ry", values, bases)?;
            let (rx, ry) = match (rx, ry) {
                (Some(rx), Some(ry)) => (rx, ry),
                (Some(rx), None) => (rx, rx),
                (None, Some(ry)) => (ry, ry),
                (None, None) => (0.0, 0.0),
            };
            Rectangle::from_xywh(cx - rx, cy - ry, rx * 2.0, ry * 2.0)
        }
        "path" => {
            let Some(value) = get_attr(el, "d") else {
                return Ok(MeasuredGeometry::Empty);
            };
            let commands = crate::svg_path::parse_path_data(&value).map_err(
                |crate::svg_path::PathDataError::Syntax { offset }| CompileError::BadPathData {
                    element: "path".to_string(),
                    offset,
                    excerpt: excerpt_at(&value, offset),
                },
            )?;
            if commands.is_empty() {
                return Ok(MeasuredGeometry::Empty);
            }
            PathData::new(commands, resolve_fill_rule(el)?)
                .map_err(|error| CompileError::BadPathData {
                    element: "path".to_string(),
                    offset: 0,
                    excerpt: error.to_string(),
                })?
                .local_bounds()
        }
        "line" => {
            let x1 = geometry_attr_f32(el, "x1", values, bases)?.unwrap_or(0.0);
            let y1 = geometry_attr_f32(el, "y1", values, bases)?.unwrap_or(0.0);
            let x2 = geometry_attr_f32(el, "x2", values, bases)?.unwrap_or(0.0);
            let y2 = geometry_attr_f32(el, "y2", values, bases)?.unwrap_or(0.0);
            Rectangle::from_points(&[[x1, y1], [x2, y2]])
        }
        "polygon" | "polyline" => {
            let Some(value) = get_attr(el, "points") else {
                return Ok(MeasuredGeometry::Empty);
            };
            let points = crate::svg_path::parse_points(&value).map_err(
                |crate::svg_path::PathDataError::Syntax { offset }| CompileError::BadPoints {
                    element: el.local_name_string(),
                    offset,
                    excerpt: excerpt_at(&value, offset),
                },
            )?;
            if points.is_empty() {
                return Ok(MeasuredGeometry::Empty);
            }
            let points: Vec<[f32; 2]> = points.into_iter().map(|(x, y)| [x, y]).collect();
            Rectangle::from_points(&points)
        }
        // Text geometry depends on the declared oracle; re-use it only when
        // it can be resolved without changing any paint fact.
        "text" => {
            let Some(data) = el.borrow_data() else {
                return Err(CompileError::MissingComputedStyle);
            };
            let style: &ComputedValues = data.styles.primary();
            let font_size = style.clone_font_size().used_size().px();
            let family = match style.clone_font_family().families.iter().next() {
                Some(style::values::computed::font::SingleFontFamily::FamilyName(name)) => {
                    name.name.to_string()
                }
                _ => return Ok(MeasuredGeometry::Unknown),
            };
            drop(data);
            let mut raw = String::new();
            for child_id in &el.dom_node().children {
                match &el.dom().node(*child_id).data {
                    DemoNodeData::Text(text) => raw.push_str(text),
                    DemoNodeData::Element(_) => return Ok(MeasuredGeometry::Unknown),
                    _ => {}
                }
            }
            let content = crate::svg_text::collapse_whitespace(&raw);
            let x = geometry_attr_f32(el, "x", values, bases)?.unwrap_or(0.0);
            let y = geometry_attr_f32(el, "y", values, bases)?.unwrap_or(0.0);
            let anchor = get_attr(el, "text-anchor")
                .and_then(|value| crate::svg_text::Anchor::parse(&value))
                .unwrap_or(crate::svg_text::Anchor::Start);
            let Some(path) = crate::svg_text::resolve_text_path(
                &content, &family, font_size, x, y, anchor, fonts,
            )
            .map_err(|error| CompileError::UnsupportedStyle(error.to_string()))?
            else {
                return Ok(MeasuredGeometry::Empty);
            };
            path.local_bounds()
        }
        _ => return Ok(MeasuredGeometry::Unknown),
    };
    Ok(MeasuredGeometry::Rect(rect))
}

fn patrol_computed_style(
    element: HtmlElement<'_>,
    include_css_sizing: bool,
) -> Result<ComputedPatrol, CompileError> {
    let data = element
        .borrow_data()
        .ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();
    // Order is load-bearing: a pruned or hidden element paints nothing in
    // Chromium regardless of its other properties, so those dispositions
    // are decided before the refusing patrols — an unconsumed property on
    // an element that paints nothing must not turn a correct nothing into
    // a refusal. (`include_css_sizing` still runs below for the root: the
    // canvas contract is sizing's, not visibility's.)
    let display = style.clone_display();
    // `display: contents` generates no box but paints its children in the
    // parent's place: a container loses its transform and a shape never
    // paints. Rendering it as an ordinary element would diverge silently,
    // and pruning it would drop children Chromium paints — refuse by name.
    if display.is_contents() {
        return Err(CompileError::UnsupportedStyle(
            "display: contents is not yet consumed".to_string(),
        ));
    }
    let disposition = if display.is_none() {
        RenderDisposition::PrunedSubtree
    } else if !matches!(style.clone_visibility(), Visibility::Visible) {
        RenderDisposition::HiddenPaint
    } else {
        RenderDisposition::Renders
    };
    let opacity = style.clone_opacity().clamp(0.0, 1.0);
    if disposition == RenderDisposition::PrunedSubtree && !include_css_sizing {
        return Ok(ComputedPatrol {
            disposition,
            opacity,
        });
    }
    // SVG2 makes width/height geometry properties where they apply: a
    // cascaded (stylesheet or style-attribute) value beats both the
    // authored attribute and the auto default in Chromium, while this
    // compiler reads geometry from attributes only. Only `fill` enters the
    // cascade as a presentation hint (csscascade's admitted set), so a
    // non-auto computed width or height here can only be a smuggled CSS
    // value — refuse it by name rather than paint at the attribute-derived
    // size. (The bare SVG geometry longhands `x`/`y`/`rx`/`ry` — and the
    // `cx`/`cy`/`r` family — do not exist in this Stylo build, so they
    // stay inside the named open boundary above.)
    if include_css_sizing {
        let width = style.clone_width();
        if !matches!(width, Size::Auto) {
            return Err(CompileError::UnsupportedStyle(format!(
                "CSS width ({width:?}) is not yet consumed"
            )));
        }
        let height = style.clone_height();
        if !matches!(height, Size::Auto) {
            return Err(CompileError::UnsupportedStyle(format!(
                "CSS height ({height:?}) is not yet consumed"
            )));
        }
    }
    Ok(ComputedPatrol {
        disposition,
        opacity,
    })
}

/// The percentage bases of the one viewport (SVG2 §7.10): a shape
/// geometry percentage resolves against the viewport's user-unit extent —
/// the `viewBox` when one maps the viewport, the root's own extent
/// otherwise — with x-axis lengths against its width, y-axis lengths
/// against its height, and the "other" lengths (a radius, a stroke width)
/// against the normalized diagonal `sqrt(w² + h²)/√2` (measured: `10%` on
/// a 64x64 viewport paints 6.4 units). Root sizing percentages stay a
/// document-level refusal: their basis is the host window itself, a cell
/// the element-capture baker cannot express, so they graduate only with a
/// host-level oracle.
#[derive(Debug, Clone, Copy)]
struct PercentBases {
    width: f32,
    height: f32,
}

impl PercentBases {
    fn diagonal(self) -> f32 {
        (self.width * self.width + self.height * self.height).sqrt() / std::f32::consts::SQRT_2
    }

    fn axis(self, attr: &str) -> f32 {
        match attr {
            "x" | "x1" | "x2" | "cx" | "rx" | "width" => self.width,
            "y" | "y1" | "y2" | "cy" | "ry" | "height" => self.height,
            _ => self.diagonal(),
        }
    }
}

/// First `<svg>` element in document order.
fn find_svg<'session>(el: HtmlElement<'session>) -> Option<HtmlElement<'session>> {
    if el.local_name_string().eq_ignore_ascii_case("svg") {
        return Some(el);
    }
    let mut child = el.first_element_child();
    while let Some(c) = child {
        if let Some(found) = find_svg(c) {
            return Some(found);
        }
        child = c.next_element_sibling();
    }
    None
}

/// Compile an `<svg>` element and its children into an SVG-local frame.
struct FrameCompilation {
    frame: Frame,
    /// The materialized nodes that are direct children of the root `<svg>`
    /// — the animation inventory's candidate target set, which it narrows
    /// further to `<rect>`. A shape inside a `<g>` is deliberately not a
    /// candidate: admitting overrides there would widen the sampling slice
    /// past what the animation corpus bakes.
    top_level_shapes: Vec<NodeId>,
}

/// The one SVG compiler. Base and Sample(time) both enter here; the only
/// difference between them is the [`EffectiveValues`] view the attribute
/// reads resolve through. `mode` decides what a beyond-slice child does:
/// strict returns its error, best-effort records it in `degradations` and
/// leaves the child out of the frame. The checks up to and including the
/// outer viewport mapping are document-level and refuse in both modes.
#[allow(clippy::too_many_arguments)]
fn compile_svg_element(
    svg: HtmlElement<'_>,
    values: &EffectiveValues,
    mode: CompileMode,
    degradations: &mut Vec<Degradation>,
    initial_viewport: Option<InitialViewport>,
    override_skips: &HashMap<NodeId, String>,
    fonts: &textlayout::Environment,
) -> Result<FrameCompilation, CompileError> {
    // The outer <svg> is the canvas contract: a rendering attribute or a
    // cascaded value the slice cannot honor here would wrong every pixel,
    // so the root patrols are document-level in both modes.
    patrol_rendering_attributes(svg, "svg", ROOT_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(svg, "svg")?;
    // A root `display: none` splits by entry, and the split is measured:
    // a *standalone* document's outermost `<svg>` ignores it — the baked
    // `svg-display-none-root` oracle paints normally — while an *embedded*
    // (inline-HTML) root generates no box, so its subtree contributes the
    // empty canvas. The entry is legible here as the initial viewport: the
    // standalone entry always carries one. A *hidden* root is inert in
    // both entries: the root paints nothing itself, and each descendant's
    // own computed (inherited) visibility decides its node.
    let root_patrol = patrol_computed_style(svg, true)?;
    let root_disposition = root_patrol.disposition;
    // The root's opacity composites the *whole canvas* — measured: the
    // captured SVG-local raster carries the multiplied alpha, identically
    // in the standalone and inline-HTML entries. This engine's raster
    // entry composites over an opaque surface, which cannot carry a
    // translucent frame, so the root's opacity refuses by name in both
    // admissions until a translucent-surface entry exists. (Element and
    // container opacity are consumed — this is the root alone, like its
    // transform.)
    if root_patrol.opacity != 1.0 {
        return Err(CompileError::UnsupportedStyle(format!(
            "opacity {} on the root <svg> is not yet consumed (it composites the whole \
             canvas, which needs a translucent surface entry)",
            root_patrol.opacity
        )));
    }
    // The root's transform applies to its CSS box *outside* the viewBox
    // mapping (the reason its attribute spelling is a root refusal), and
    // since the transform rung both spellings meet in the computed value —
    // the attribute enters as a presentation hint and a stylesheet can
    // reach the root directly. Composing it like a container's would place
    // every pixel wrongly, so a non-`none` computed transform on the root
    // refuses by name in both admissions. The attribute patrol above still
    // fires first for the attribute spelling, keeping its message.
    {
        let data = svg
            .borrow_data()
            .ok_or(CompileError::MissingComputedStyle)?;
        if !data.styles.primary().clone_transform().0.is_empty() {
            return Err(CompileError::UnsupportedStyle(
                "transform on the root <svg> is not yet consumed (it applies to the CSS box \
                 outside the viewBox mapping)"
                    .to_string(),
            ));
        }
    }
    reject_percentage_dimension(svg, "width")?;
    reject_percentage_dimension(svg, "height")?;
    let width_attr = root_dimension_f32(svg, "width", values)?;
    let height_attr = root_dimension_f32(svg, "height", values)?;
    // A missing root width/height is `auto` and resolves to 100% of the
    // initial viewport the embedding environment establishes (SVG2 §8.2) —
    // the standalone entry carries the host's viewport, exactly as Chromium
    // sizes a standalone SVG document to its window. The inline HTML entry
    // has no initial-viewport semantics until CSS replaced-element sizing
    // (auto -> 300x150 and the aspect-ratio rules) is implemented, so a
    // missing dimension there stays a named document-level refusal.
    let (width, height) = match initial_viewport {
        Some(viewport) => (
            width_attr.unwrap_or_else(|| viewport.width()),
            height_attr.unwrap_or_else(|| viewport.height()),
        ),
        None => (
            width_attr.ok_or_else(|| {
                CompileError::UnsupportedSizing(
                    "missing width on the inline HTML entry; CSS replaced-element sizing \
                     (auto -> 300x150 and the aspect-ratio rules) is not yet implemented"
                        .to_string(),
                )
            })?,
            height_attr.ok_or_else(|| {
                CompileError::UnsupportedSizing(
                    "missing height on the inline HTML entry; CSS replaced-element sizing \
                     (auto -> 300x150 and the aspect-ratio rules) is not yet implemented"
                        .to_string(),
                )
            })?,
        ),
    };
    reject_negative_dimension("width", width)?;
    reject_negative_dimension("height", height)?;
    // width="0" / height="0" is admitted and disables rendering (SVG2 §8.2):
    // the frame keeps a zero-extent viewport clip and every pixel stays
    // transparent — an honest nothing, not a refusal.
    let par = match get_attr(svg, "preserveAspectRatio") {
        // Parsed before the viewBox and even without one: a malformed value
        // refuses regardless, and a valid value without a viewBox is inert,
        // as in Chromium.
        Some(value) => parse_preserve_aspect_ratio(&value)?,
        None => PreserveAspectRatio::default(),
    };
    let viewbox = match get_attr(svg, "viewBox") {
        Some(v) => Some(parse_viewbox(&v)?),
        None => None,
    };
    let viewport = match viewbox {
        Some(viewbox) => viewbox_to_viewport_transform((width, height), viewbox, par),
        None => AffineTransform::identity(),
    };
    let frame_bounds = Rectangle::from_xywh(0.0, 0.0, width, height);

    // The percentage bases are the viewport's user-unit extent: the
    // viewBox when one maps the viewport, the root's own extent otherwise.
    let bases = match viewbox {
        Some((_, _, vb_width, vb_height)) => PercentBases {
            width: vb_width,
            height: vb_height,
        },
        None => PercentBases { width, height },
    };
    // The paint-resource id table: whole-document, document-ordered,
    // first-id-wins, shadow-content excluded. It classifies every id so a
    // pattern can never masquerade as a missing gradient; `url(#id)` resolves
    // against the document, and a gradient outside this compiled subtree
    // refuses rather than paints.
    let servers = PaintServers::collect(document_root(svg), svg);
    let use_boxes = measure_use_boxes(svg, values, bases, fonts)?;
    let mut walk = ChildWalk {
        values,
        bases,
        mode,
        degradations,
        override_skips,
        has_author_css: document_has_author_css(svg),
        servers: &servers,
        use_boxes,
        paint_contexts: Vec::new(),
        context_paint_transform: viewport,
        fonts,
        items: Vec::new(),
        top_level_shapes: Vec::new(),
        next_id: 0,
    };
    if root_disposition != RenderDisposition::PrunedSubtree || initial_viewport.is_some() {
        walk.compile_children(svg, viewport, "svg", 0, 1.0)?;
    }
    let ChildWalk {
        items,
        top_level_shapes,
        ..
    } = walk;

    Ok(FrameCompilation {
        frame: Frame {
            owner: VisualRef::new(Identity::new(0), Provenance::new(0)),
            bounds: frame_bounds,
            // The recursive walk opens and closes scopes structurally and
            // never emits an empty one, so a construction failure here is
            // this compiler's own defect, not the document's.
            items: FrameItems::try_new(items).expect("the walk emits balanced non-empty scopes"),
        },
        top_level_shapes,
    })
}

/// How deep a container subtree this compiler descends before refusing.
///
/// A document may nest `<g>` arbitrarily and this walk is recursive, so an
/// adversarial or generated file could otherwise exhaust the stack. The
/// bound is generous against real documents and the refusal is explicit
/// rather than a crash. Every other walk over the same tree
/// ([`subtree_contains_script`], [`unrepresented_stylesheet_properties`],
/// and the animation inventory's own inspection) is iterative or bounded
/// for the same reason — a bound here alone would not prevent the crash it
/// exists to prevent.
pub(crate) const MAX_CONTAINER_DEPTH: usize = 64;

/// What one compiled span (a child, or a whole subtree) contributes to its
/// enclosing container's opacity decision — the facts the measured fold
/// rule reads. A "draw" is one paint pass (a fill or a stroke) not
/// enclosed in a nested scope.
#[derive(Debug, Clone, Copy, Default)]
struct SpanFacts {
    /// Bare paint passes in the span (nested scopes' contents excluded).
    draws: usize,
    /// The span contains a compositing scope.
    has_scope: bool,
    /// An element-opacity fold already landed on a draw in the span.
    folded: bool,
    /// A non-`none` computed transform sits on an element in the span —
    /// which breaks an enclosing scope's fold (measured: an intermediate
    /// transformed container, or a transformed draw, forces the layer; the
    /// scope element's own transform does not).
    transformed: bool,
}

impl SpanFacts {
    fn absorb(&mut self, other: SpanFacts) {
        self.draws += other.draws;
        self.has_scope |= other.has_scope;
        self.folded |= other.folded;
        self.transformed |= other.transformed;
    }
}

/// The recursive descent that materializes shapes in painter order.
///
/// Containers are **flattened** wherever flattening is exact: a `<g>`
/// contributes its transform and its place in paint order, both of which
/// compose into the per-node affine and the ordered item stream. The
/// group-scope rung added the one construct that breaks flattening —
/// element `opacity` — consumed by the measured fold rule: an opacity
/// whose content is a single un-transformed, un-folded draw **folds** into
/// that draw's paint (one float product, quantized once — byte-identical
/// in Chromium), and every other opacity emits a real [`rframe::Scope`]
/// (an isolated layer). Chromium's fold and layer routes differ by one
/// code value, so the split is measured meaning, not an optimization.
/// `clip-path`, `mask`, `filter`, `mix-blend-mode`, and `isolation` are
/// still refused by the patrols; each grows the scope's effect vocabulary
/// with its own rung.
struct ChildWalk<'a> {
    values: &'a EffectiveValues,
    /// The one viewport's percentage bases (SVG2 §7.10), fixed at the
    /// root: no nested viewport is admitted, so every shape shares them.
    bases: PercentBases,
    mode: CompileMode,
    degradations: &'a mut Vec<Degradation>,
    /// Targets of load-active authored-state overrides, best-effort only —
    /// non-empty exactly when the construction pass found attributable
    /// overrides. The walk leaves each out and declares it here, where the
    /// stable path and document order are known.
    override_skips: &'a HashMap<NodeId, String>,
    /// Whether the document carries any author stylesheet — the `<use>`
    /// patrol's document fact (see [`document_has_author_css`]).
    has_author_css: bool,
    /// The document's paint-resource id table: built once before the walk,
    /// first-id-wins, shadow-content excluded.
    servers: &'a PaintServers<'a>,
    /// Complete geometry boxes of expanded `<use>` instances, in each use
    /// element's own user space. They are measured before paint resolution so
    /// a context URL never learns its reference box from whichever leaf
    /// happened to compile first.
    use_boxes: HashMap<NodeId, Option<Rectangle>>,
    /// Nearest use last. A context keyword selects from this stack and a
    /// selected context keyword continues at the next outer entry.
    paint_contexts: Vec<PaintContext<'a>>,
    /// Cumulative frame mapping for context-paint coordinates. It includes
    /// the viewport and every computed `transform`, but deliberately omits
    /// every `<use x/y>` translation: Chromium applies those translations to
    /// the resolved paint along with the consuming clone instead of cancelling
    /// them during owner-to-leaf rebasing. The capability probe discriminates
    /// nested and immediate-owner cases from doubled and cancelled controls.
    context_paint_transform: AffineTransform,
    /// The declared font environment (the text rung). Empty unless the host
    /// declared fonts, so a `<text>` in an undeclared document refuses by
    /// name instead of reaching for an ambient face.
    fonts: &'a textlayout::Environment,
    items: Vec<FrameItem>,
    /// The materialized nodes that are direct children of the root `<svg>`
    /// — the animation inventory's candidate targets, which it narrows
    /// further to `<rect>`.
    top_level_shapes: Vec<NodeId>,
    next_id: u64,
}

#[derive(Clone, Copy)]
struct PaintContext<'d> {
    element: HtmlElement<'d>,
    /// `None` means an unsupported descendant made the box unknowable. A
    /// context solid remains usable, but a context paint server refuses by
    /// name instead of silently using a partial box.
    reference_box: Option<Rectangle>,
    /// Mapping from the context element's paint-coordinate space into frame
    /// space. Computed transforms participate; `<use x/y>` translations do
    /// not, so every translation in a nested consumption chain accumulates.
    to_frame: AffineTransform,
}

impl<'a> ChildWalk<'a> {
    /// Compile a parent's children in painter order, accumulating the span
    /// facts the parent's own opacity decision reads. `fold_opacity` is an
    /// enclosing scope's fold factor mid-replay (see
    /// [`ChildWalk::compile_container`]); it is `1.0` on the first pass.
    fn compile_children(
        &mut self,
        parent: HtmlElement<'a>,
        transform: AffineTransform,
        parent_path: &str,
        depth: usize,
        fold_opacity: f32,
    ) -> Result<SpanFacts, CompileError> {
        let mut facts = SpanFacts::default();
        let mut ordinals = HashMap::<String, usize>::new();
        let mut child = parent.first_element_child();
        while let Some(c) = child {
            let tag = c.local_name_string();
            let ordinal = {
                let ordinal = ordinals.entry(tag.clone()).or_default();
                *ordinal += 1;
                *ordinal
            };
            let path = format!("{parent_path}/{tag}[{ordinal}]");
            // The target of a load-active authored-state override: left out
            // of the frame and declared here, where the walk knows its
            // stable path — so override skips keep document order with
            // every other skip. Strict never reaches this (it refuses the
            // override at construction), and the reason names the animation
            // element the inventory found.
            if let Some(reason) = self.override_skips.get(&c.node_id()) {
                self.degradations.push(Degradation {
                    path,
                    action: DegradationAction::Skipped,
                    reason: reason.clone(),
                });
                child = c.next_element_sibling();
                continue;
            }
            // Non-rendering elements contribute no geometry and no hole:
            // `<style>`'s CSS enters the one cascade (csscascade collects
            // it), and `<title>`/`<desc>`/`<metadata>` are descriptive text
            // Chromium never paints. An animation element paints nothing
            // *itself* either — but it is never silently inert: the
            // animation inventory owns it, admitting the one sampled
            // `<animate>` and turning every other one into a construction
            // refusal (strict) or a declared skip of its target
            // (best-effort, the override map above).
            if is_non_rendering_element(&tag) || is_animation_element(&tag) {
                child = c.next_element_sibling();
                continue;
            }
            // `<defs>` is reference-only since the use/defs rung: its
            // contents never paint in place (SVG2's UA sheet makes it
            // `display: none !important`) and the effect it exists for —
            // changing what referencing elements paint — is consumed by
            // the use expansion, which indexed it before this walk. Its
            // own computed display is deliberately not consulted: the
            // pinned cascade's UA sheet carries no defs rule.
            if tag == "defs" {
                child = c.next_element_sibling();
                continue;
            }
            // A gradient element is reference-only wherever it appears —
            // in `<defs>` or in the open (measured: it paints nothing in
            // place either way). Its effect — what a referencing `url(#…)`
            // paints — is consumed through the paint-server table, which
            // indexed the whole document before this walk, so the walk
            // skips the subtree (its `<stop>` children are the table's
            // material, never paintable content).
            if tag == "linearGradient" || tag == "radialGradient" {
                child = c.next_element_sibling();
                continue;
            }
            // `<a>` renders as a container exactly like `<g>` (SVG2 §16.2:
            // its `href` is interaction, not paint), so the two share the
            // one container compiler and its patrols. `<use>` is a
            // container whose children are its expanded shadow content.
            let result = if tag == "g" || tag == "a" {
                self.compile_container(c, transform, &path, depth, &tag, fold_opacity)
            } else if tag == "use" {
                self.compile_use(c, transform, &path, depth, fold_opacity)
            } else {
                self.compile_leaf(c, transform, depth == 0, fold_opacity)
            };
            match result {
                Ok(child_facts) => facts.absorb(child_facts),
                Err(error) => match self.mode {
                    CompileMode::Strict => return Err(error),
                    CompileMode::BestEffort => self.degradations.push(Degradation {
                        path,
                        action: DegradationAction::Skipped,
                        reason: error.to_string(),
                    }),
                },
            }
            child = c.next_element_sibling();
        }
        Ok(facts)
    }

    /// A container element: patrolled like any admitted element, then
    /// descended with its own transform composed onto the inherited one.
    ///
    /// A failure *on the container itself* — an unconsumed attribute, a
    /// scope-bearing cascaded property, a malformed transform — fails the
    /// whole subtree, because nothing inside it can be placed or composited
    /// correctly without it. A failure on one *descendant* is that
    /// descendant's own hole: its siblings still paint, each skip named at
    /// its nested path. That keeps best-effort's "render what is admitted"
    /// promise inside groups instead of dropping a whole illustration for
    /// one unsupported child.
    fn compile_container(
        &mut self,
        el: HtmlElement<'a>,
        transform: AffineTransform,
        path: &str,
        depth: usize,
        element: &str,
        fold_opacity: f32,
    ) -> Result<SpanFacts, CompileError> {
        if depth >= MAX_CONTAINER_DEPTH {
            return Err(CompileError::ContainerTooDeep(MAX_CONTAINER_DEPTH));
        }
        patrol_rendering_attributes(el, element, &[])?;
        patrol_style_attribute(el, element)?;
        let patrol = patrol_computed_style(el, false)?;
        match patrol.disposition {
            // `display: none` generates no box: the subtree is pruned —
            // Chromium's correct nothing, not a hole to declare. A *hidden*
            // container still descends: `visibility` inherits and a
            // descendant whose computed value is `visible` un-hides itself,
            // while the container itself never painted anything to omit.
            // Its opacity still composites what *does* paint below it.
            RenderDisposition::PrunedSubtree => return Ok(SpanFacts::default()),
            RenderDisposition::Renders | RenderDisposition::HiddenPaint => {}
        }
        // `opacity: 0` composites nothing for the whole subtree — an
        // admitted nothing (measured: siblings paint, contents never do),
        // and unlike a hidden container nothing below can undo it.
        if patrol.opacity == 0.0 {
            return Ok(SpanFacts::default());
        }
        let own_transformed = element_has_computed_transform(el)?;
        let transform = compose_element_transform(el, transform, element, self.bases)?;
        let previous_context_paint_transform = self.context_paint_transform;
        self.context_paint_transform =
            compose_element_transform(el, previous_context_paint_transform, element, self.bases)?;
        let facts = self.compile_span_with_opacity(
            el,
            transform,
            path,
            depth,
            patrol.opacity,
            fold_opacity,
        );
        self.context_paint_transform = previous_context_paint_transform;
        let facts = facts?;
        Ok(SpanFacts {
            transformed: facts.transformed || own_transformed,
            ..facts
        })
    }

    /// Compile a container's subtree and apply its element opacity by the
    /// measured fold rule: **fold** — rewind the span and replay it with
    /// the factor threaded to its one draw's paint resolve, so the alpha
    /// product still quantizes once — when the span is exactly one draw,
    /// un-folded, un-scoped, and un-transformed below this element;
    /// otherwise wrap the span in a real scope (an isolated layer).
    /// Chromium's two routes differ by one code value, so both branches
    /// are oracle-pinned meaning.
    fn compile_span_with_opacity(
        &mut self,
        el: HtmlElement<'a>,
        transform: AffineTransform,
        path: &str,
        depth: usize,
        own_opacity: f32,
        fold_opacity: f32,
    ) -> Result<SpanFacts, CompileError> {
        let checkpoint = (self.items.len(), self.next_id, self.degradations.len());
        let mut facts = self.compile_children(el, transform, path, depth + 1, fold_opacity)?;
        if own_opacity < 1.0 {
            if facts.draws == 1 && !facts.has_scope && !facts.folded && !facts.transformed {
                // Replay the span with the accumulated factor. The one draw
                // may still refuse the fold by name (a gradient paint), in
                // which case the replay records that refusal exactly where
                // the draw was.
                self.items.truncate(checkpoint.0);
                self.next_id = checkpoint.1;
                self.degradations.truncate(checkpoint.2);
                facts = self.compile_children(
                    el,
                    transform,
                    path,
                    depth + 1,
                    fold_opacity * own_opacity,
                )?;
                facts.folded = true;
            } else if facts.draws > 0 || facts.has_scope {
                let scope = scope_item(&mut self.next_id, own_opacity);
                self.items.insert(checkpoint.0, scope);
                self.items.push(FrameItem::ScopeEnd);
                facts = SpanFacts {
                    draws: 0,
                    has_scope: true,
                    folded: false,
                    transformed: facts.transformed,
                };
            }
            // A span with no draws and no scope composites nothing: the
            // opacity states nothing, exactly as Chromium paints it.
        }
        Ok(facts)
    }

    /// A `<use>` element: SVG2 renders it "as if the host element was a
    /// container and the shadow content was its descendents", and the
    /// expansion (csscascade) has already made that literal — the children
    /// are the cloned instance, styled by the one cascade with inheritance
    /// from this element. What remains here is the container walk plus the
    /// use-specific mapping and patrols, each Chromium-measured:
    ///
    /// - `x`/`y` append a translate *inside* the element's own transform
    ///   (SVG2 §5.6.2 — "appended to the right-side of the transformation
    ///   list"); `width`/`height` are inert for every admitted target
    ///   (they only size `<svg>`/`<symbol>` targets, which refuse as
    ///   unsupported elements when the clone surfaces them).
    /// - An unresolved or cyclic reference expanded to nothing, and this
    ///   walk paints the same nothing Chromium paints — no declaration,
    ///   because nothing degrades.
    /// - The refusals by name: the expansion's own flags (external
    ///   reference, authored children, overflow) and the author-CSS
    ///   boundary — the measured shadow scope admits only selectors
    ///   satisfiable inside the cloned subtree, which the one flattened
    ///   tree cannot express, so a document with any author stylesheet
    ///   refuses every `<use>` until the shadow-matching rung.
    fn compile_use(
        &mut self,
        el: HtmlElement<'a>,
        transform: AffineTransform,
        path: &str,
        depth: usize,
        fold_opacity: f32,
    ) -> Result<SpanFacts, CompileError> {
        if depth >= MAX_CONTAINER_DEPTH {
            return Err(CompileError::ContainerTooDeep(MAX_CONTAINER_DEPTH));
        }
        patrol_rendering_attributes(el, "use", &[])?;
        patrol_style_attribute(el, "use")?;
        if let DemoNodeData::Element(element) = &el.dom_node().data
            && let Some(refusal) = element.svg_use_refusal
        {
            use csscascade::svg_use::SvgUseRefusal;
            return Err(CompileError::UnsupportedUse(match refusal {
                SvgUseRefusal::ExternalReference => {
                    "its reference is not a same-document fragment, and external \
                     resources are not resolved"
                        .to_string()
                }
                SvgUseRefusal::AuthoredChildren => {
                    "it has authored element children, which Chromium replaces with \
                     the shadow content"
                        .to_string()
                }
                SvgUseRefusal::ExpansionOverflow => {
                    "its expansion overflows the reference-chain budget — an \
                     indirect cycle or pathological fan-out"
                        .to_string()
                }
            }));
        }
        if self.has_author_css {
            return Err(CompileError::UnsupportedUse(
                "the document carries author CSS, and shadow-scoped selector \
                 matching is not yet consumed (selectors must match inside the \
                 cloned subtree alone — measured)"
                    .to_string(),
            ));
        }
        let patrol = patrol_computed_style(el, false)?;
        match patrol.disposition {
            RenderDisposition::PrunedSubtree => return Ok(SpanFacts::default()),
            RenderDisposition::Renders | RenderDisposition::HiddenPaint => {}
        }
        if patrol.opacity == 0.0 {
            return Ok(SpanFacts::default());
        }
        let own_transformed = element_has_computed_transform(el)?;
        let context_transform = compose_element_transform(el, transform, "use", self.bases)?;
        let previous_context_paint_transform = self.context_paint_transform;
        let context_paint_transform =
            compose_element_transform(el, previous_context_paint_transform, "use", self.bases)?;
        let x = geometry_attr_f32(el, "x", self.values, self.bases)?.unwrap_or(0.0);
        let y = geometry_attr_f32(el, "y", self.values, self.bases)?.unwrap_or(0.0);
        let transform =
            context_transform.compose(&AffineTransform::from_acebdf(1.0, 0.0, x, 0.0, 1.0, y));
        let reference_box = self
            .use_boxes
            .get(&el.node_id())
            .copied()
            // A genuinely empty use was indexed as `Some(empty)`. Absence
            // means an earlier measurement error prevented this nested use
            // from being indexed; treating that as empty would silently turn
            // its context gradient into no paint.
            .unwrap_or(None);
        self.paint_contexts.push(PaintContext {
            element: el,
            reference_box,
            to_frame: context_paint_transform,
        });
        self.context_paint_transform = context_paint_transform;
        let facts = self.compile_span_with_opacity(
            el,
            transform,
            path,
            depth,
            patrol.opacity,
            fold_opacity,
        );
        self.context_paint_transform = previous_context_paint_transform;
        self.paint_contexts.pop();
        let facts = facts?;
        // The `x`/`y` translate is part of the use's own transform (SVG2
        // §5.6.2), so like the transform property it stays *on* this
        // element — an enclosing scope's fold is broken only by a transform
        // strictly below it, and this one is not below.
        Ok(SpanFacts {
            transformed: facts.transformed || own_transformed || x != 0.0 || y != 0.0,
            ..facts
        })
    }

    fn compile_leaf(
        &mut self,
        el: HtmlElement<'a>,
        transform: AffineTransform,
        top_level: bool,
        fold_opacity: f32,
    ) -> Result<SpanFacts, CompileError> {
        // An admitted shape may resolve to no visual fact at all — a `<path>`
        // whose `d` draws nothing. That is not a hole: the element is
        // admitted, it is simply not a node.
        let mut facts = SpanFacts::default();
        if let Some(outcome) = compile_shape(
            el,
            transform,
            self.context_paint_transform,
            &mut self.next_id,
            self.values,
            self.servers,
            &self.paint_contexts,
            self.bases,
            fold_opacity,
            self.fonts,
        )? {
            facts.folded = outcome.folded;
            facts.transformed = outcome.transformed;
            match outcome.scope_opacity {
                Some(opacity) => {
                    // The shape's fill and stroke composite together through
                    // one isolated layer — the double-blend fact that made
                    // element opacity a refusal until this rung.
                    let scope = scope_item(&mut self.next_id, opacity);
                    self.items.push(scope);
                    self.items.push(FrameItem::Node(outcome.node));
                    self.items.push(FrameItem::ScopeEnd);
                    facts.has_scope = true;
                }
                None => {
                    facts.draws = outcome.draws;
                    self.items.push(FrameItem::Node(outcome.node));
                }
            }
        }
        if top_level {
            self.top_level_shapes.push(el.node_id());
        }
        Ok(facts)
    }
}

/// Mint one scope-begin item with a fresh owner. A scope owns identity and
/// provenance exactly as a node does — damage and diagnostics name it.
fn scope_item(next_id: &mut u64, opacity: f32) -> FrameItem {
    let scope_id = *next_id + 1;
    *next_id += 1;
    FrameItem::ScopeBegin(Scope {
        owner: VisualRef::new(Identity::new(scope_id), Provenance::new(scope_id)),
        effect: ScopeEffect::Opacity(
            ScopeOpacity::new(opacity).expect("a computed opacity strictly inside (0, 1)"),
        ),
    })
}

/// Whether the element's computed `transform` is anything but `none` — the
/// fact that breaks an enclosing scope's fold (Blink's paint-property
/// boundary, measured one code value apart from the fold).
fn element_has_computed_transform(el: HtmlElement<'_>) -> Result<bool, CompileError> {
    let data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    Ok(!data.styles.primary().clone_transform().0.is_empty())
}

/// Whether the element paints nothing *and* affects no other element's
/// painting, so it is neither compiled nor declared.
///
/// Both halves matter. `<symbol>`, `<clipPath>`, `<mask>`, `<marker>` and
/// the gradient elements also paint nothing directly, but they change what
/// referencing elements paint — skipping one silently would change pixels —
/// so they stay ordinary unsupported elements, declared by name until the
/// rung that consumes them. (`<defs>` graduated with the use/defs rung: the
/// use expansion consumes its reference effect, so the walk skips it in its
/// own branch above.)
pub(crate) fn is_non_rendering_element(tag: &str) -> bool {
    matches!(tag, "style" | "title" | "desc" | "metadata")
}

/// Compose an element's computed `transform` onto the transform it
/// inherits, giving the local→frame mapping for it and its subtree.
///
/// The computed value is the one place both spellings meet: the `transform`
/// *attribute* enters the cascade as a presentation hint (csscascade's
/// measured rewrite), so author CSS beats it — `transform: none` included —
/// an invalid CSS declaration falls back to it, and a malformed attribute
/// contributes nothing and renders untransformed, each exactly as Chromium
/// resolves the pair (the transform rung's probe matrix). SVG composes a
/// transform list left to right and an element's own list applies inside its
/// inherited mapping — which is exactly [`AffineTransform::compose`]'s
/// "apply `other` after `self`" order with the inherited mapping on the left.
/// Percentage translations resolve against the viewport's user-unit extent
/// ([`PercentBases`]), the measured reference box.
fn compose_element_transform(
    el: HtmlElement<'_>,
    inherited: AffineTransform,
    element_name: &str,
    bases: PercentBases,
) -> Result<AffineTransform, CompileError> {
    let data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();
    let transform = style.clone_transform();
    if transform.0.is_empty() {
        return Ok(inherited);
    }
    let own = computed_transform_to_affine(&transform, Some((bases.width, bases.height))).map_err(
        |refusal| {
            CompileError::UnsupportedStyle(match refusal {
                TransformRefusal::Function(name) => format!(
                    "transform on <{element_name}> uses {name}(), which is outside the 2D \
                     affine function set this slice consumes"
                ),
                TransformRefusal::Calc => format!(
                    "transform on <{element_name}> uses a calc() length, which is not yet \
                     consumed"
                ),
                // Unreachable with a supplied basis; named for the exhaustive
                // match rather than silently aliased to another hole.
                TransformRefusal::Percentage => {
                    format!("transform on <{element_name}> uses a percentage without a basis")
                }
            })
        },
    )?;
    let composed = inherited.compose(&own);
    // Each computed component is finite, but composing them can overflow.
    // The downstream contract refuses a non-finite transform for the whole
    // frame with no element named, which would turn one bad list into a
    // blank render; refuse it here, where the element is known and
    // best-effort leaves a single declared hole.
    if !composed
        .matrix
        .iter()
        .flatten()
        .all(|component| component.is_finite())
    {
        return Err(CompileError::NonFiniteTransform {
            element: element_name.to_string(),
        });
    }
    Ok(composed)
}

/// Compile a single shape element into a resolved node.
///
/// Local names match exactly: SVG element names are case-sensitive, and each
/// grammar entry already applies its own canonicalization (the HTML tokenizer
/// lowercases and foreign-content-adjusts; XML preserves authored case).
#[allow(clippy::too_many_arguments)]
fn compile_shape(
    el: HtmlElement<'_>,
    inherited: AffineTransform,
    context_paint_inherited: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    fold_opacity: f32,
    fonts: &textlayout::Environment,
) -> Result<Option<ShapeOutcome>, CompileError> {
    let tag = el.local_name_string();
    let transformed = element_has_computed_transform(el)?;
    // A shape's own `transform` composes inside the mapping it inherits
    // from the viewport and its ancestor containers, exactly as a
    // container's does.
    let transform = compose_element_transform(el, inherited, &tag, bases)?;
    let context_paint_transform =
        compose_element_transform(el, context_paint_inherited, &tag, bases)?;
    let outcome = match tag.as_str() {
        "rect" => compile_rect(
            el,
            transform,
            context_paint_transform,
            next_id,
            values,
            servers,
            paint_contexts,
            bases,
            fold_opacity,
        ),
        "circle" => compile_circle(
            el,
            transform,
            context_paint_transform,
            next_id,
            values,
            servers,
            paint_contexts,
            bases,
            fold_opacity,
        ),
        "ellipse" => compile_ellipse(
            el,
            transform,
            context_paint_transform,
            next_id,
            values,
            servers,
            paint_contexts,
            bases,
            fold_opacity,
        ),
        "path" => compile_path(
            el,
            transform,
            context_paint_transform,
            next_id,
            servers,
            paint_contexts,
            bases,
            fold_opacity,
        ),
        "text" => compile_text(
            el,
            transform,
            context_paint_transform,
            next_id,
            values,
            servers,
            paint_contexts,
            bases,
            fold_opacity,
            fonts,
        ),
        "line" => compile_line(
            el,
            transform,
            context_paint_transform,
            next_id,
            values,
            servers,
            paint_contexts,
            bases,
            fold_opacity,
        ),
        "polygon" => compile_points_shape(
            el,
            transform,
            context_paint_transform,
            next_id,
            PointsClosure::Closed,
            servers,
            paint_contexts,
            bases,
            fold_opacity,
        ),
        "polyline" => compile_points_shape(
            el,
            transform,
            context_paint_transform,
            next_id,
            PointsClosure::Open,
            servers,
            paint_contexts,
            bases,
            fold_opacity,
        ),
        other => Err(CompileError::UnsupportedElement(other.to_string())),
    }?;
    Ok(outcome.map(|outcome| ShapeOutcome {
        transformed,
        ..outcome
    }))
}

/// Compile one `<text>` element: the document's run, resolved once by the
/// text oracle and lowered as the resolved contract's path facts.
///
/// No font identity crosses into the contract — the glyphs arrive as
/// outlines, which keeps the resolved frame glyphless. Once the posture that
/// left the D-M shaped-text join undecided, this is now what the taken low
/// join mandates: the contract carries no text fact and no resource
/// reference, and promoting one is a registered re-opening (see
/// [the text-stage evidence](../../../docs/wg/consolidation/n0-join-point.md#the-text-stage-evidence)).
/// The run is admitted only inside the
/// ratified numeric domain, so a construct Chromium would snap refuses by
/// name here instead.
#[allow(clippy::too_many_arguments)]
fn compile_text(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    context_paint_transform: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    fold_opacity: f32,
    fonts: &textlayout::Environment,
) -> Result<Option<ShapeOutcome>, CompileError> {
    patrol_rendering_attributes(el, "text", TEXT_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(el, "text")?;
    let patrol = patrol_computed_style(el, true)?;
    match patrol.disposition {
        RenderDisposition::Renders => {}
        RenderDisposition::HiddenPaint | RenderDisposition::PrunedSubtree => return Ok(None),
    }
    if patrol.opacity == 0.0 {
        return Ok(None);
    }

    // A stroked run strokes glyph outlines, whose edges leave the admitted
    // numeric domain — the byte-exact gate could not hold. It refuses by
    // name rather than painting a fill-only approximation.
    if resolve_stroke(
        el,
        "text",
        servers,
        paint_contexts,
        Rectangle::from_xywh(0.0, 0.0, 1.0, 1.0),
        context_paint_transform,
        bases,
        1.0,
    )?
    .is_some()
    {
        return Err(CompileError::UnsupportedStroke(
            "stroke on <text> is outside the admitted text slice".to_string(),
        ));
    }

    // Only element children were walked, so the run's characters are read
    // here — the same DOM-children read the `<style>` patrol performs, and
    // for the same reason: a comment can split character data in two.
    let mut raw = String::new();
    for child_id in &el.dom_node().children {
        match &el.dom().node(*child_id).data {
            DemoNodeData::Text(text) => raw.push_str(text),
            // An element child is `<tspan>`, `<textPath>`, `<a>`, … — every
            // one re-positions or re-styles part of the run, none admitted.
            DemoNodeData::Element(child) => {
                return Err(CompileError::UnsupportedElement(
                    child.name.local.to_string(),
                ));
            }
            _ => {}
        }
    }
    let content = crate::svg_text::collapse_whitespace(&raw);

    let x = geometry_attr_f32(el, "x", values, bases)?.unwrap_or(0.0);
    let y = geometry_attr_f32(el, "y", values, bases)?.unwrap_or(0.0);
    let anchor = match get_attr(el, "text-anchor") {
        Some(value) => crate::svg_text::Anchor::parse(&value).ok_or_else(|| {
            CompileError::UnsupportedStyle(format!(
                "text-anchor \"{value}\" is not an admitted keyword"
            ))
        })?,
        None => crate::svg_text::Anchor::Start,
    };

    let data = el.borrow_data().expect("styled element");
    let style: &ComputedValues = data.styles.primary();
    let font_size = style.clone_font_size().used_size().px();
    // The cascade's family list is a preference order; the environment
    // answers exact declared names only, so the first entry is the request
    // and a miss refuses by name rather than walking to a second candidate
    // (v0 has no fallback).
    // The environment answers exact declared names only, so a generic
    // keyword (`serif`, `monospace`, … — including the initial value a
    // document that names no family computes to) selects nothing. It refuses
    // here rather than reaching the oracle as an empty name, so the
    // diagnostic says what is actually wrong with the document.
    let family = match style.clone_font_family().families.iter().next() {
        Some(style::values::computed::font::SingleFontFamily::FamilyName(name)) => {
            name.name.to_string()
        }
        Some(style::values::computed::font::SingleFontFamily::Generic(_)) | None => {
            drop(data);
            return Err(CompileError::UnsupportedStyle(
                "<text> resolves to a generic font family, which names no font in the declared \
                 environment — a family is declared by exact name or the run refuses"
                    .to_string(),
            ));
        }
    };
    drop(data);

    let path =
        crate::svg_text::resolve_text_path(&content, &family, font_size, x, y, anchor, fonts)
            .map_err(|error| CompileError::UnsupportedStyle(error.to_string()))?;
    let Some(path) = path else {
        // A run that resolves to no ink is an admitted nothing, not a node.
        return Ok(None);
    };

    shape_node(
        el,
        Geometry::Path(std::sync::Arc::new(path)),
        viewport,
        context_paint_transform,
        next_id,
        Strokable::RenderingDisabled,
        servers,
        paint_contexts,
        bases,
        patrol.opacity,
        fold_opacity,
        false,
    )
    .map(Some)
}

fn compile_rect(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    context_paint_transform: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    fold_opacity: f32,
) -> Result<Option<ShapeOutcome>, CompileError> {
    patrol_rendering_attributes(el, "rect", GEOMETRY_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(el, "rect")?;
    let patrol = patrol_computed_style(el, true)?;
    match patrol.disposition {
        RenderDisposition::Renders => {}
        // A hidden or display-pruned shape is Chromium's correct nothing —
        // admitted, and not a node.
        RenderDisposition::HiddenPaint | RenderDisposition::PrunedSubtree => return Ok(None),
    }
    // `opacity: 0` paints nothing — an admitted nothing, not a node
    // (measured: the sibling still paints).
    if patrol.opacity == 0.0 {
        return Ok(None);
    }
    let x = geometry_attr_f32(el, "x", values, bases)?.unwrap_or(0.0);
    let y = geometry_attr_f32(el, "y", values, bases)?.unwrap_or(0.0);
    let w = box_extent(geometry_attr_f32(el, "width", values, bases)?.unwrap_or(0.0));
    let h = box_extent(geometry_attr_f32(el, "height", values, bases)?.unwrap_or(0.0));
    let rect = Rectangle::from_xywh(x, y, w, h);
    let geometry = match rect_corner_radii(el, values, bases, w, h)? {
        Some((rx, ry)) => Geometry::Path(Arc::new(rounded_rect_path(rect, rx, ry)?)),
        None => Geometry::Rect(rect),
    };
    shape_node(
        el,
        geometry,
        viewport,
        context_paint_transform,
        next_id,
        box_strokable(w, h),
        servers,
        paint_contexts,
        bases,
        patrol.opacity,
        fold_opacity,
        false,
    )
    .map(Some)
}

fn compile_circle(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    context_paint_transform: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    fold_opacity: f32,
) -> Result<Option<ShapeOutcome>, CompileError> {
    patrol_rendering_attributes(el, "circle", GEOMETRY_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(el, "circle")?;
    let patrol = patrol_computed_style(el, false)?;
    match patrol.disposition {
        RenderDisposition::Renders => {}
        // A hidden or display-pruned shape is Chromium's correct nothing —
        // admitted, and not a node.
        RenderDisposition::HiddenPaint | RenderDisposition::PrunedSubtree => return Ok(None),
    }
    // `opacity: 0` paints nothing — an admitted nothing, not a node
    // (measured: the sibling still paints).
    if patrol.opacity == 0.0 {
        return Ok(None);
    }
    let cx = geometry_attr_f32(el, "cx", values, bases)?.unwrap_or(0.0);
    let cy = geometry_attr_f32(el, "cy", values, bases)?.unwrap_or(0.0);
    // SVG2 §10.3: a negative `r` is invalid and must be ignored, and a
    // computed value of zero disables rendering. Chromium clamps the used
    // value at layout (`LayoutSVGEllipse`: `std::max(radius, 0.f)`), so
    // negative and missing both resolve exactly as `r="0"`: the element is
    // admitted and paints nothing — an honest nothing, not a refusal.
    let r = geometry_attr_f32(el, "r", values, bases)?
        .unwrap_or(0.0)
        .max(0.0);
    let rect = Rectangle::from_xywh(cx - r, cy - r, r * 2.0, r * 2.0);
    shape_node(
        el,
        Geometry::Ellipse(rect),
        viewport,
        context_paint_transform,
        next_id,
        box_strokable(r, r),
        servers,
        paint_contexts,
        bases,
        patrol.opacity,
        fold_opacity,
        false,
    )
    .map(Some)
}

fn compile_ellipse(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    context_paint_transform: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    fold_opacity: f32,
) -> Result<Option<ShapeOutcome>, CompileError> {
    patrol_rendering_attributes(el, "ellipse", GEOMETRY_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(el, "ellipse")?;
    let patrol = patrol_computed_style(el, false)?;
    match patrol.disposition {
        RenderDisposition::Renders => {}
        // A hidden or display-pruned shape is Chromium's correct nothing —
        // admitted, and not a node.
        RenderDisposition::HiddenPaint | RenderDisposition::PrunedSubtree => return Ok(None),
    }
    // `opacity: 0` paints nothing — an admitted nothing, not a node
    // (measured: the sibling still paints).
    if patrol.opacity == 0.0 {
        return Ok(None);
    }
    let cx = geometry_attr_f32(el, "cx", values, bases)?.unwrap_or(0.0);
    let cy = geometry_attr_f32(el, "cy", values, bases)?.unwrap_or(0.0);
    // SVG2 §10.4: `rx`/`ry` initially `auto`; a negative value is invalid
    // and must be ignored, which Chromium treats as `auto` (frozen donor's
    // Chrome-confirmed reading, re-proved against Chromium 148: a single
    // negative radius adopts the other axis). `auto` adopts the other
    // radius; both `auto` resolve to zero; zero on either axis disables
    // rendering — the zero-extent oval below paints nothing.
    let rx = ellipse_radius(el, "rx", values, bases)?;
    let ry = ellipse_radius(el, "ry", values, bases)?;
    let (rx, ry) = match (rx, ry) {
        (Some(rx), Some(ry)) => (rx, ry),
        (Some(rx), None) => (rx, rx),
        (None, Some(ry)) => (ry, ry),
        (None, None) => (0.0, 0.0),
    };
    let rect = Rectangle::from_xywh(cx - rx, cy - ry, rx * 2.0, ry * 2.0);
    shape_node(
        el,
        Geometry::Ellipse(rect),
        viewport,
        context_paint_transform,
        next_id,
        box_strokable(rx, ry),
        servers,
        paint_contexts,
        bases,
        patrol.opacity,
        fold_opacity,
        false,
    )
    .map(Some)
}

/// Compile a `<path>`: the SVG path-data grammar into the resolved
/// contract's canonical command stream, under the cascaded fill rule.
///
/// `d` is the only geometry `<path>` carries — no lengths, so no percentage
/// basis to reject — and it is deliberately not read through
/// [`EffectiveValues`]: the closed sampling inventory covers scalar `x` on a
/// `<rect>`, and `d` is neither a scalar nor on the inventory.
///
/// A `d` that draws nothing — absent, empty, `none`'s effect, or contours that
/// only move — resolves to no node. SVG renders nothing for it, and no visual
/// fact is the honest way to carry nothing.
fn compile_path(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    context_paint_transform: AffineTransform,
    next_id: &mut u64,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    fold_opacity: f32,
) -> Result<Option<ShapeOutcome>, CompileError> {
    patrol_rendering_attributes(el, "path", GEOMETRY_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(el, "path")?;
    let patrol = patrol_computed_style(el, false)?;
    match patrol.disposition {
        RenderDisposition::Renders => {}
        // A hidden or display-pruned shape is Chromium's correct nothing —
        // admitted, and not a node.
        RenderDisposition::HiddenPaint | RenderDisposition::PrunedSubtree => return Ok(None),
    }
    // `opacity: 0` paints nothing — an admitted nothing, not a node
    // (measured: the sibling still paints).
    if patrol.opacity == 0.0 {
        return Ok(None);
    }
    let commands = match get_attr(el, "d") {
        None => Vec::new(),
        Some(value) => crate::svg_path::parse_path_data(&value).map_err(
            |crate::svg_path::PathDataError::Syntax { offset }| CompileError::BadPathData {
                element: "path".to_string(),
                offset,
                excerpt: excerpt_at(&value, offset),
            },
        )?,
    };
    if commands.is_empty() {
        return Ok(None);
    }
    let path = PathData::new(commands, resolve_fill_rule(el)?).map_err(|error| {
        // The producer normalizes into the contract's canonical form, so a
        // rejection here is this compiler's bug, not the document's. Refuse
        // by name rather than paint something the contract would not admit.
        CompileError::BadPathData {
            element: "path".to_string(),
            offset: 0,
            excerpt: error.to_string(),
        }
    })?;
    shape_node(
        el,
        Geometry::Path(Arc::new(path)),
        viewport,
        context_paint_transform,
        next_id,
        Strokable::Yes,
        servers,
        paint_contexts,
        bases,
        patrol.opacity,
        fold_opacity,
        false,
    )
    .map(Some)
}

/// Compile a `<line>` — as a two-command path, not a geometry kind of its own.
///
/// A line is a stroke-only shape: SVG gives it no interior, and Chromium paints
/// nothing for a filled `<line>` with no stroke (measured). A two-point path
/// carries exactly that — its fill has zero area and paints nothing — and
/// Chromium's `<line>` is **byte-identical** to the equivalent `<path>`
/// (measured), so the contract needs no line variant and the cap, join and
/// zero-length rules come out identical for free.
///
/// `x1`/`y1`/`x2`/`y2` default to zero, which makes a bare `<line>` a
/// zero-length segment: nothing under a butt cap, a dot under a round or square
/// one (measured). That is why the path normalization keeps a zero-length
/// segment.
fn compile_line(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    context_paint_transform: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    fold_opacity: f32,
) -> Result<Option<ShapeOutcome>, CompileError> {
    patrol_rendering_attributes(el, "line", GEOMETRY_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(el, "line")?;
    let patrol = patrol_computed_style(el, false)?;
    match patrol.disposition {
        RenderDisposition::Renders => {}
        // A hidden or display-pruned shape is Chromium's correct nothing —
        // admitted, and not a node.
        RenderDisposition::HiddenPaint | RenderDisposition::PrunedSubtree => return Ok(None),
    }
    // `opacity: 0` paints nothing — an admitted nothing, not a node
    // (measured: the sibling still paints).
    if patrol.opacity == 0.0 {
        return Ok(None);
    }
    let x1 = geometry_attr_f32(el, "x1", values, bases)?.unwrap_or(0.0);
    let y1 = geometry_attr_f32(el, "y1", values, bases)?.unwrap_or(0.0);
    let x2 = geometry_attr_f32(el, "x2", values, bases)?.unwrap_or(0.0);
    let y2 = geometry_attr_f32(el, "y2", values, bases)?.unwrap_or(0.0);
    let path = PathData::new(
        vec![
            rframe::PathCommand::MoveTo { x: x1, y: y1 },
            rframe::PathCommand::LineTo { x: x2, y: y2 },
        ],
        FillRule::NonZero,
    )
    .map_err(|error| CompileError::UnsupportedStroke(error.to_string()))?;
    shape_node(
        el,
        Geometry::Path(Arc::new(path)),
        viewport,
        context_paint_transform,
        next_id,
        Strokable::Yes,
        servers,
        paint_contexts,
        bases,
        patrol.opacity,
        fold_opacity,
        true,
    )
    .map(Some)
}

/// Whether a points shape closes its contour: the one semantic difference
/// between `<polygon>` and `<polyline>`. Everything else — the `points`
/// grammar, the fill (an open contour fills as if closed, so a filled
/// polyline and the same polygon paint identical interiors), the patrols —
/// is shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointsClosure {
    /// `<polygon>`: the contour closes, so its stroke paints the closing
    /// segment and a single-point polygon is the zero-length *closed*
    /// contour whose cap paints a dot (measured: Chromium renders
    /// `points="32,32"` under a square cap exactly as `M32 32Z`).
    Closed,
    /// `<polyline>`: the contour stays open — no closing stroke segment,
    /// and a single-point polyline is a neutral move-only contour that
    /// paints nothing under any cap (measured).
    Open,
}

/// Compile a `<polygon>` or `<polyline>` — as a line-segment path, not a
/// geometry kind of its own, exactly as `<line>` lowers.
///
/// The `points` list maps to `MoveTo` + `LineTo`* (+ `Close` for a
/// polygon). Chromium renders the valid coordinate-pair prefix of an
/// erroneous list; this slice refuses the whole element by name instead
/// (see [`CompileError::BadPoints`]). A missing or empty list is valid and
/// renders nothing, like an empty `d`.
fn compile_points_shape(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    context_paint_transform: AffineTransform,
    next_id: &mut u64,
    closure: PointsClosure,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    fold_opacity: f32,
) -> Result<Option<ShapeOutcome>, CompileError> {
    let element = match closure {
        PointsClosure::Closed => "polygon",
        PointsClosure::Open => "polyline",
    };
    patrol_rendering_attributes(el, element, GEOMETRY_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(el, element)?;
    let patrol = patrol_computed_style(el, false)?;
    match patrol.disposition {
        RenderDisposition::Renders => {}
        // A hidden or display-pruned shape is Chromium's correct nothing —
        // admitted, and not a node.
        RenderDisposition::HiddenPaint | RenderDisposition::PrunedSubtree => return Ok(None),
    }
    // `opacity: 0` paints nothing — an admitted nothing, not a node
    // (measured: the sibling still paints).
    if patrol.opacity == 0.0 {
        return Ok(None);
    }
    let points = match get_attr(el, "points") {
        None => Vec::new(),
        Some(value) => crate::svg_path::parse_points(&value).map_err(
            |crate::svg_path::PathDataError::Syntax { offset }| CompileError::BadPoints {
                element: element.to_string(),
                offset,
                excerpt: excerpt_at(&value, offset),
            },
        )?,
    };
    let Some(((first_x, first_y), rest)) = points.split_first() else {
        return Ok(None);
    };
    // A single-point polyline is a neutral move-only contour: it paints
    // nothing under any cap (measured), exactly as the path grammar's
    // lone moveto does, so it is admitted and is not a node.
    if rest.is_empty() && closure == PointsClosure::Open {
        return Ok(None);
    }
    let mut commands = Vec::with_capacity(points.len() + 2);
    commands.push(rframe::PathCommand::MoveTo {
        x: *first_x,
        y: *first_y,
    });
    // A single-point polygon is the zero-length *closed* contour, which
    // the contract carries only in its canonical `M x y L x y Z`
    // spelling — the same resolution the path grammar applies to
    // `M x y Z`, and the cap decides whether it paints (measured).
    if rest.is_empty() {
        commands.push(rframe::PathCommand::LineTo {
            x: *first_x,
            y: *first_y,
        });
    }
    for (x, y) in rest {
        commands.push(rframe::PathCommand::LineTo { x: *x, y: *y });
    }
    if closure == PointsClosure::Closed {
        commands.push(rframe::PathCommand::Close);
    }
    let path = PathData::new(commands, resolve_fill_rule(el)?).map_err(|error| {
        // The producer normalizes into the contract's canonical form, so a
        // rejection here is this compiler's bug, not the document's.
        CompileError::BadPoints {
            element: element.to_string(),
            offset: 0,
            excerpt: error.to_string(),
        }
    })?;
    shape_node(
        el,
        Geometry::Path(Arc::new(path)),
        viewport,
        context_paint_transform,
        next_id,
        Strokable::Yes,
        servers,
        paint_contexts,
        bases,
        patrol.opacity,
        fold_opacity,
        false,
    )
    .map(Some)
}

/// The authored text at an error offset, clipped to a readable excerpt on a
/// character boundary.
fn excerpt_at(value: &str, offset: usize) -> String {
    /// Long enough to show the offending token, short enough that a
    /// kilobyte-long `d` does not reach a terminal.
    const WIDTH: usize = 24;
    let start = (0..=offset.min(value.len()))
        .rev()
        .find(|index| value.is_char_boundary(*index))
        .unwrap_or(0);
    let end = (start..=(start + WIDTH).min(value.len()))
        .rev()
        .find(|index| value.is_char_boundary(*index))
        .unwrap_or(value.len());
    value[start..end].to_string()
}

/// The cascaded `fill-rule` — which regions of a self-overlapping path the
/// fill covers. Read as a typed computed value like `fill`, so the SVG2
/// precedence (presentation attribute below author rules), inheritance
/// through containers, and CSS keyword case-insensitivity all come from the
/// one cascade rather than from an attribute parse here.
fn resolve_fill_rule(el: HtmlElement<'_>) -> Result<FillRule, CompileError> {
    let data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();
    Ok(match style.clone_fill_rule() {
        StyloFillRule::Nonzero => FillRule::NonZero,
        StyloFillRule::Evenodd => FillRule::EvenOdd,
    })
}

/// A box primitive's two extents decide whether it renders at all, and a
/// negative extent is one of the ways it does not.
///
/// A negative `width`/`height` is an error that disables rendering of the
/// element, and Chromium renders the rest of the document around it (measured:
/// a sibling still paints). So the extent is clamped to zero rather than
/// carried negative — a negative extent would reach the downstream's geometry
/// validation and abort the whole render with an internal message naming no
/// element, which is exactly what best-effort exists to prevent.
fn box_extent(value: f32) -> f32 {
    value.max(0.0)
}

fn box_strokable(width: f32, height: f32) -> Strokable {
    if width > 0.0 && height > 0.0 {
        Strokable::Yes
    } else {
        Strokable::RenderingDisabled
    }
}

/// Whether a shape can carry a stroke at all.
///
/// A `<rect>` with a zero `width`/`height`, or a `<circle>`/`<ellipse>` with a
/// zero radius, **disables rendering of the element** — not just its fill.
/// Chromium paints nothing for a zero-extent *stroked* rect (measured), while a
/// naive stroke of a zero-extent box would draw a line. A path is different: a
/// zero-extent path is a zero-length segment, which strokes as a cap-shaped
/// dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Strokable {
    Yes,
    /// A box primitive with a non-positive extent: admitted, and it paints
    /// nothing at all.
    RenderingDisabled,
}

/// One compiled shape plus the facts the group-scope rung's walk decides
/// with: how many paint passes it draws, whether its own opacity needs an
/// isolated layer, and whether a fold or a transform already sits on it.
struct ShapeOutcome {
    node: FrameNode,
    /// Paint passes the node draws (fill + stroke).
    draws: usize,
    /// The shape's own opacity composites fill and stroke through one
    /// isolated layer — the walk wraps the node in a scope.
    scope_opacity: Option<f32>,
    /// An element-opacity fold landed on this node's paint.
    folded: bool,
    /// The element's computed `transform` is not `none` (breaks an
    /// enclosing scope's fold). Set by [`compile_shape`].
    transformed: bool,
}

/// The shared tail of every shape compile: resolve the typed fill and
/// stroke, apply the element opacity by the measured fold rule, and emit
/// the resolved node. The node's `bounds` is the frame-space transform of
/// its local geometry box — the exact-bounds law the n0 downstream
/// re-checks on admission.
///
/// `own_opacity` is the element's own computed opacity (already known
/// non-zero); `fold_opacity` is an enclosing container's fold factor
/// mid-replay. At most one differs from 1 — a container fold is only
/// eligible over an un-folded draw. A single-pass shape **folds** the
/// factor into that pass's alpha product by re-resolving the pass, so the
/// product still quantizes once (measured byte-identical in Chromium to
/// the paint-level fold); a two-pass shape asks the walk for an isolated
/// layer instead; a gradient pass refuses the fold by name — the paint
/// carries one quantized alpha, and Chromium composites the element
/// opacity *after* that quantization, which one slot cannot express.
#[allow(clippy::too_many_arguments)]
fn shape_node(
    el: HtmlElement<'_>,
    geometry: Geometry,
    viewport: AffineTransform,
    context_paint_transform: AffineTransform,
    next_id: &mut u64,
    strokable: Strokable,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    bases: PercentBases,
    own_opacity: f32,
    fold_opacity: f32,
    fill_never_paints: bool,
) -> Result<ShapeOutcome, CompileError> {
    let rect = geometry.local_box();
    let mut paints = resolve_fill(
        el,
        servers,
        paint_contexts,
        rect,
        context_paint_transform,
        bases,
        1.0,
    )?;
    let mut stroke = match strokable {
        Strokable::Yes => resolve_stroke(
            el,
            &el.local_name_string(),
            servers,
            paint_contexts,
            rect,
            context_paint_transform,
            bases,
            1.0,
        )?,
        Strokable::RenderingDisabled => None,
    };
    patrol_mixed_contour_cap(&geometry, stroke.as_ref())?;

    debug_assert!(
        own_opacity == 1.0 || fold_opacity == 1.0,
        "a container fold is never eligible over a shape with its own opacity"
    );
    let opacity = own_opacity * fold_opacity;
    // A `<line>`'s fill can never paint — SVG gives it no interior — so it
    // is not a pass an opacity composites (the paints stay on the node,
    // where their zero area paints the same nothing Chromium paints).
    let fill_passes = usize::from(!paints.is_empty() && !fill_never_paints);
    let draws = fill_passes + usize::from(stroke.is_some());
    let mut folded = false;
    let mut scope_opacity = None;
    if opacity < 1.0 && draws > 0 {
        if draws > 1 {
            scope_opacity = Some(opacity);
        } else {
            let pass_paints = if fill_passes == 1 {
                &paints
            } else {
                stroke
                    .as_ref()
                    .expect("the one draw is the stroke")
                    .paints()
            };
            if !pass_paints
                .iter()
                .all(|paint| matches!(paint, cg::Paint::Solid(_)))
            {
                return Err(CompileError::UnsupportedStyle(format!(
                    "opacity {opacity} over a gradient paint is not yet consumed (the paint \
                     carries one quantized alpha, and Chromium composites the element opacity \
                     after that quantization — expressing both needs a second paint-alpha \
                     factor)"
                )));
            }
            if fill_passes == 1 {
                paints = resolve_fill(
                    el,
                    servers,
                    paint_contexts,
                    rect,
                    context_paint_transform,
                    bases,
                    opacity,
                )?;
            } else {
                stroke = resolve_stroke(
                    el,
                    &el.local_name_string(),
                    servers,
                    paint_contexts,
                    rect,
                    context_paint_transform,
                    bases,
                    opacity,
                )?;
            }
            folded = true;
        }
    }

    let visual_id = *next_id + 1;
    let node = FrameNode {
        owner: VisualRef::new(Identity::new(visual_id), Provenance::new(visual_id)),
        transform: viewport,
        geometry,
        bounds: math2::rect_transform(rect, &viewport),
        paints,
        stroke,
    };
    *next_id += 1;
    Ok(ShapeOutcome {
        node,
        draws,
        scope_opacity,
        folded,
        transformed: false,
    })
}

/// Refuse a path that mixes open and closed contours while a non-butt
/// `stroke-linecap` is in force.
///
/// A cap is a per-*contour* property on a solid stroke: a closed contour has no
/// ends, so the cap is inert on it, and Chromium's raster agrees (its butt and
/// round captures of a closed solid contour are byte-identical). The consumer
/// holds one cap per draw, so it can honor that for a path whose contours are
/// *all* closed — it strokes them under butt, byte-exact at every width — and
/// for a path with none. A mixed solid path needs both caps at once, and the two
/// ways to give it them are both measurably wrong: one draw paints the closed
/// contours' rejoin with a cap Chromium does not (~84 of 2304 pixels below a
/// device pixel wide), and two draws composite the overlapping runs'
/// anti-aliased edges twice (32 to 47 pixels at 1.25 and 2 units).
///
/// A dash cycle changes that fact: every painted dash segment has ends even on
/// a closed contour, so one authored cap is correct for every contour. The
/// dashed path therefore bypasses this solid-only patrol.
///
/// So it refuses by name. This over-refuses — the one-draw error only appears
/// once the *device* width falls to about a pixel, which the compiler cannot
/// know — and over-refusal is the trade this slice makes every time. Serving
/// the case properly means stroking each contour to an outline and unioning
/// them into one filled path, which is its own rung.
fn patrol_mixed_contour_cap(
    geometry: &Geometry,
    stroke: Option<&rframe::Stroke>,
) -> Result<(), CompileError> {
    let Geometry::Path(path) = geometry else {
        return Ok(());
    };
    let Some(stroke) = stroke else {
        return Ok(());
    };
    if stroke.dash_intervals().is_some()
        || stroke.cap() == rframe::StrokeCap::Butt
        || path.all_contours_closed()
    {
        return Ok(());
    }
    if path
        .commands()
        .iter()
        .any(|command| matches!(command, rframe::PathCommand::Close))
    {
        return Err(CompileError::UnsupportedStroke(
            "a stroke-linecap other than butt on a path that mixes open and closed contours \
             needs a cap per contour, which this consumer cannot express in one draw"
                .to_string(),
        ));
    }
    Ok(())
}

/// The used corner radii of a `<rect>`, or `None` for sharp corners.
///
/// The resolution order is measured, not assumed: `auto` adopts the other
/// axis's *authored* value first, and each axis then clamps to half its own
/// extent independently — `rx="30"` on a 40-wide, 48-tall rect rounds as
/// `(20, 24)`, not `(20, 20)`. A negative value is invalid-and-ignored,
/// which Chromium treats as `auto` (the [`ellipse_radius`] rule), and a used
/// radius of zero on either axis squares every corner. Percentages resolve
/// through the shared geometry-attribute bases: `rx` against the viewport
/// width, `ry` against its height.
fn rect_corner_radii(
    el: HtmlElement<'_>,
    values: &EffectiveValues,
    bases: PercentBases,
    width: f32,
    height: f32,
) -> Result<Option<(f32, f32)>, CompileError> {
    let rx = ellipse_radius(el, "rx", values, bases)?;
    let ry = ellipse_radius(el, "ry", values, bases)?;
    let (Some(rx), Some(ry)) = (rx.or(ry), ry.or(rx)) else {
        return Ok(None);
    };
    if !(width > 0.0 && height > 0.0) {
        return Ok(None);
    }
    let rx = rx.min(width / 2.0);
    let ry = ry.min(height / 2.0);
    if rx <= 0.0 || ry <= 0.0 {
        return Ok(None);
    }
    Ok(Some((rx, ry)))
}

/// The rounded rect's canonical contour: four edges and four quarter-turn
/// conics of weight `cos 45°`, clockwise from the end of the top-left
/// corner — measured byte-identical to Chromium's rounded rect, circular
/// and elliptical corners alike. A fully-rounded axis makes its straight
/// edges zero-length; those are omitted and the conic chain carries the
/// contour alone.
fn rounded_rect_path(rect: Rectangle, rx: f32, ry: f32) -> Result<rframe::PathData, CompileError> {
    use rframe::PathCommand;
    let (x, y, w, h) = (rect.x, rect.y, rect.width, rect.height);
    let weight = std::f32::consts::FRAC_1_SQRT_2;
    let mut commands = vec![PathCommand::MoveTo { x: x + rx, y }];
    let edge = |commands: &mut Vec<PathCommand>, to_x: f32, to_y: f32| {
        let from = match *commands.last().expect("contour is open") {
            PathCommand::MoveTo { x, y }
            | PathCommand::LineTo { x, y }
            | PathCommand::ConicTo { x, y, .. } => (x, y),
            _ => unreachable!("the contour emits moves, lines and conics only"),
        };
        if from != (to_x, to_y) {
            commands.push(PathCommand::LineTo { x: to_x, y: to_y });
        }
    };
    edge(&mut commands, x + w - rx, y);
    commands.push(PathCommand::ConicTo {
        x1: x + w,
        y1: y,
        x: x + w,
        y: y + ry,
        weight,
    });
    edge(&mut commands, x + w, y + h - ry);
    commands.push(PathCommand::ConicTo {
        x1: x + w,
        y1: y + h,
        x: x + w - rx,
        y: y + h,
        weight,
    });
    edge(&mut commands, x + rx, y + h);
    commands.push(PathCommand::ConicTo {
        x1: x,
        y1: y + h,
        x,
        y: y + h - ry,
        weight,
    });
    edge(&mut commands, x, y + ry);
    commands.push(PathCommand::ConicTo {
        x1: x,
        y1: y,
        x: x + rx,
        y,
        weight,
    });
    commands.push(PathCommand::Close);
    rframe::PathData::new(commands, rframe::FillRule::NonZero).map_err(|error| {
        // Finite attribute reads make this contour canonical by
        // construction; a rejection here is arithmetic overflow on an
        // extreme authored geometry — this compiler's refusal, not a wrong
        // paint.
        CompileError::BadPathData {
            element: "rect".to_string(),
            offset: 0,
            excerpt: error.to_string(),
        }
    })
}

/// An ellipse `rx`/`ry` read: `None` is the `auto` used value, which
/// [`compile_ellipse`] resolves against the other axis.
///
/// A negative value is invalid per SVG2 §10.4 and must be ignored, which
/// Chromium implements as `auto` (`LayoutSVGEllipse`'s treat-as-auto path,
/// live-probed) — so it filters to `None` here.
///
/// The `auto` **keyword** is deliberately not read from an attribute.
/// Only the *CSS* property takes keywords: Blink parses geometry
/// presentation attributes with the SVGLength grammar, where `auto` is
/// invalid and maps an explicit `0px` hint — rendering nothing, not
/// adopting the other axis. That is the opposite of an absent attribute
/// (computed initial `auto`, which does adopt), so reading the keyword
/// here would paint an ellipse where Chromium paints none. The invalid
/// keyword instead reaches [`attr_f32`] and refuses loudly as a bad
/// number: over-refusal, never wrong pixels. (The root `width`/`height`
/// keyword read is *not* the analogous case — there the CSS sizing
/// properties genuinely accept `auto`.)
fn ellipse_radius(
    el: HtmlElement<'_>,
    name: &str,
    values: &EffectiveValues,
    bases: PercentBases,
) -> Result<Option<f32>, CompileError> {
    Ok(geometry_attr_f32(el, name, values, bases)?.filter(|value| *value >= 0.0))
}

/// Resolve the SVG `fill` paint from the typed cascaded value — the one
/// place fill meaning enters the compiler. Presentation hints, stylesheet
/// rules, and inline style attributes all feed this read through the one
/// Stylo cascade, with SVG2 precedence; `currentColor` resolves against the
/// cascaded `color`, and an invalid authored value falls back exactly as an
/// invalid CSS declaration would. A `url(#…)` paint resolves through the
/// document's paint-server table (the gradient rung). Standard context paints
/// recursively select the nearest `<use>` paint and resolve away before the
/// frame; Stylo's non-standard `context-* <fallback>` extension refuses by
/// name. `fill-opacity` belongs to the destination property (it is never
/// selected from the context owner) and folds into the paint's alpha — one
/// float multiply, quantized once for a solid, and carried as the gradient
/// paint's float opacity for a server.
#[derive(Clone, Copy)]
enum PaintProperty {
    Fill,
    Stroke,
}

struct SelectedPaint<'d> {
    value: SVGPaint,
    owner: HtmlElement<'d>,
    context: Option<PaintContext<'d>>,
}

fn computed_paint(el: HtmlElement<'_>, property: PaintProperty) -> Result<SVGPaint, CompileError> {
    let data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();
    Ok(match property {
        PaintProperty::Fill => style.clone_fill(),
        PaintProperty::Stroke => style.clone_stroke(),
    })
}

/// Select the eventual ordinary paint. The nearest use is the first context;
/// a selected context keyword continues one entry outward. The eventual
/// ordinary owner's computed style remains attached so `currentColor`, URL
/// fallback, and gradient reference-box ownership all resolve there.
fn select_paint<'d>(
    element: HtmlElement<'d>,
    property: PaintProperty,
    contexts: &[PaintContext<'d>],
) -> Result<Option<SelectedPaint<'d>>, String> {
    let mut owner = element;
    let mut property = property;
    let mut next_context = contexts.len();
    loop {
        let value = computed_paint(owner, property).map_err(|error| error.to_string())?;
        if matches!(
            value.kind,
            SVGPaintKind::ContextFill | SVGPaintKind::ContextStroke
        ) && !matches!(
            value.fallback,
            style::values::generics::svg::SVGPaintFallback::Unset
        ) {
            return Err(
                "a context paint carries Stylo's non-standard fallback extension; Chromium drops this declaration"
                    .to_string(),
            );
        }
        let selected_property = match value.kind {
            SVGPaintKind::ContextFill => Some(PaintProperty::Fill),
            SVGPaintKind::ContextStroke => Some(PaintProperty::Stroke),
            _ => None,
        };
        let Some(selected_property) = selected_property else {
            let context = if next_context < contexts.len() {
                Some(contexts[next_context])
            } else {
                None
            };
            return Ok(Some(SelectedPaint {
                value,
                owner,
                context,
            }));
        };
        if next_context == 0 {
            return Ok(None);
        }
        next_context -= 1;
        owner = contexts[next_context].element;
        property = selected_property;
    }
}

/// Map the eventual context owner's user space into the destination leaf's
/// local space. A singular destination transform paints no pixels, so no
/// gradient fact is needed and the caller resolves it to no paint.
fn context_reference_space(
    context: Option<PaintContext<'_>>,
    destination_box: Rectangle,
    destination_to_frame: AffineTransform,
) -> Result<Option<(Rectangle, AffineTransform)>, String> {
    match context {
        None => Ok(Some((destination_box, AffineTransform::identity()))),
        Some(context) => {
            // Chromium paints no context gradient through a singular
            // destination CTM (full or either one-axis singularity, across
            // fill/stroke and box/path geometry). The capability probe pins
            // each as a measured nothing.
            let Some(frame_to_destination) = destination_to_frame.inverse() else {
                return Ok(None);
            };
            let reference_box = context.reference_box.ok_or_else(|| {
                "the context element's geometry box is incomplete because its instance contains an unsupported descendant"
                    .to_string()
            })?;
            Ok(Some((
                reference_box,
                frame_to_destination.compose(&context.to_frame),
            )))
        }
    }
}

fn resolve_fill(
    el: HtmlElement<'_>,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    consumer_box: Rectangle,
    destination_to_frame: AffineTransform,
    bases: PercentBases,
    extra_opacity: f32,
) -> Result<PaintStack, CompileError> {
    let data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();
    // `extra_opacity` is the group-scope rung's fold factor: it joins the
    // colour's own alpha and `fill-opacity` in the one float product that
    // quantizes once (the translucency rung's law, extended by measurement
    // to the element-opacity factor).
    let opacity = match style.clone_fill_opacity() {
        SVGOpacity::Opacity(value) => value * extra_opacity,
        other => {
            return Err(CompileError::UnsupportedFill(format!(
                "fill-opacity {other:?} is a context value this slice does not consume"
            )));
        }
    };
    drop(data);
    let Some(selected) = select_paint(el, PaintProperty::Fill, paint_contexts)
        .map_err(CompileError::UnsupportedFill)?
    else {
        return Ok(PaintStack::empty());
    };
    let owner_data = selected
        .owner
        .borrow_data()
        .ok_or(CompileError::MissingComputedStyle)?;
    let owner_style: &ComputedValues = owner_data.styles.primary();
    let fill = selected.value;
    let fallback = || match &fill.fallback {
        style::values::generics::svg::SVGPaintFallback::Color(color) => {
            admitted_srgb(owner_style.resolve_color(color), opacity)
                .map(PaintStack::solid)
                .map_err(CompileError::UnsupportedFill)
        }
        _ => Ok(PaintStack::empty()),
    };
    match &fill.kind {
        SVGPaintKind::None => Ok(PaintStack::empty()),
        SVGPaintKind::Color(color) => admitted_srgb(owner_style.resolve_color(color), opacity)
            .map(PaintStack::solid)
            .map_err(CompileError::UnsupportedFill),
        SVGPaintKind::PaintServer(url) => {
            if extra_opacity != 1.0 {
                return Err(CompileError::UnsupportedFill(
                    "element opacity over a url() paint is not yet consumed (the fold cannot \
                     reach through a paint-server reference)"
                        .to_string(),
                ));
            }
            match resolve_paint_server_stack(
                servers,
                url,
                || context_reference_space(selected.context, consumer_box, destination_to_frame),
                consumer_box,
                bases,
                opacity,
                "fill",
            )? {
                Some(stack) => Ok(stack),
                None => fallback(),
            }
        }
        SVGPaintKind::ContextFill | SVGPaintKind::ContextStroke => {
            unreachable!("select_paint removes every context relation")
        }
    }
}

/// Resolve one `url(#…)` paint through the table. `Ok(None)` is an
/// **invalid reference** — the caller's authored fallback decides. A valid
/// reference yields its stack (possibly empty: the measured correct
/// nothings paint nothing and deliberately do not fall back).
fn resolve_paint_server_stack(
    servers: &PaintServers<'_>,
    url: &style::values::computed::url::ComputedUrl,
    reference_space: impl FnOnce() -> Result<Option<(Rectangle, AffineTransform)>, String>,
    destination_box: Rectangle,
    bases: PercentBases,
    paint_opacity: f32,
    property: &str,
) -> Result<Option<PaintStack>, CompileError> {
    let refusal = |reason: String| match property {
        "fill" => CompileError::UnsupportedFill(reason),
        _ => CompileError::UnsupportedStroke(reason),
    };
    let Some(resolved_url) = url.url() else {
        return Err(refusal("url(<invalid>)".to_string()));
    };
    let Some(fragment) = crate::svg_paint_server::same_document_fragment(resolved_url) else {
        return Err(refusal(format!(
            "url({resolved_url}) is not a same-document fragment, and external resources \
             are not resolved"
        )));
    };
    let valid_gradient = crate::svg_paint_server::classify(servers, fragment)
        .map_err(|reason| refusal(format!("url(#{fragment}): {reason}")))?;
    if !valid_gradient {
        return Ok(None);
    }
    let gradient_bases = GradientBases {
        width: bases.width,
        height: bases.height,
    };
    let resolved = crate::svg_paint_server::resolve(
        servers,
        fragment,
        destination_box,
        reference_space,
        gradient_bases,
        paint_opacity,
    )
    .map_err(|reason| refusal(format!("url(#{fragment}): {reason}")))?;
    Ok(match resolved {
        ResolvedPaintServer::Invalid => None,
        ResolvedPaintServer::Nothing => Some(PaintStack::empty()),
        ResolvedPaintServer::Solid(color) => Some(PaintStack::solid(color)),
        ResolvedPaintServer::Gradient(paint) => Some(
            PaintStack::try_from_paints(cg::Paints::new([paint]))
                .map_err(|error| refusal(error.to_string()))?,
        ),
    })
}

/// Length units whose basis this build does not have, and which therefore must
/// not reach a consumed length.
///
/// The cascade hands back an absolute `px` value, so the unit is gone by the
/// time a computed value is read — the gate has to look at the authored text.
/// Two families, both measured:
///
/// - **Viewport-relative** (`vw`/`vh`/`vmin`/`vmax` and their `sv`/`lv`/`dv`
///   variants). Chromium resolves these against the SVG viewport: `1vw` on a
///   64x64 document is 0.64px, byte-identical to an authored `0.64`. The
///   cascade's device is pinned at 1280x720, so it computes 12.8px — a
///   twentyfold error, and silent. Threading the document's real viewport into
///   the cascade's device is the honest fix and its own rung (it moves media
///   queries too); until then this refuses by name.
/// - **Font-metric** (`ex`/`ch`/`ic`/`cap`/`lh`/`rlh`, and their root-relative
///   twins `rex`/`rch`/`ric`/`rcap`). The cascade's font provider returns
///   placeholder metrics (x-height = half the font size), not measured ones:
///   Chromium paints a `1ex` stroke 7.18 units wide where this build computes
///   8.0, and `1rex` is the same measurement against the root. `ch` and `ric`
///   agreeing today is an accident of the default font, not a property this
///   engine holds, so the whole family refuses.
/// - **Container-query** (`cqw`/`cqh`/`cqi`/`cqb`/`cqmin`/`cqmax`). The pinned
///   Stylo drops these as invalid declarations — the computed value falls to
///   the initial 1 — where Chromium resolves them against the small-viewport
///   fallback (measured: `12.5cqw` on a 64x64 document paints 8). A different
///   failure shape from the device pin, the same silent divergence, the same
///   refusal.
///
/// `em` and `rem` are deliberately absent: they resolve against `font-size`,
/// which this build represents and which csscascade now admits as a
/// presentation attribute, so both are measured byte-exact — *provided* the
/// font-size that set the basis is itself trustworthy, which
/// [`poisons_font_basis`] patrols.
const LENGTH_UNITS_WITHOUT_A_BASIS: &[&str] = &[
    "vw", "vh", "vmin", "vmax", "vi", "vb", "svw", "svh", "svmin", "svmax", "svi", "svb", "lvw",
    "lvh", "lvmin", "lvmax", "lvi", "lvb", "dvw", "dvh", "dvmin", "dvmax", "dvi", "dvb", "ex",
    "rex", "ch", "rch", "ic", "ric", "cap", "rcap", "lh", "rlh", "cqw", "cqh", "cqi", "cqb",
    "cqmin", "cqmax",
];

/// Whether an authored length carries a unit whose basis this build lacks.
///
/// Scans for the unit as a token suffix rather than parsing CSS: a unit follows
/// a digit or a `.`, and is followed by something that is not alphanumeric. That
/// admits `calc(1vw + 2px)` into the refusal, which is correct — the calc
/// carries the same basis.
fn has_unit_without_a_basis(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for unit in LENGTH_UNITS_WITHOUT_A_BASIS {
        let mut from = 0;
        while let Some(offset) = lower[from..].find(unit) {
            let start = from + offset;
            let end = start + unit.len();
            let preceded_by_number =
                start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.');
            let ends_the_token = bytes
                .get(end)
                .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'-');
            if preceded_by_number && ends_the_token {
                return Some(unit);
            }
            from = start + 1;
        }
    }
    None
}

/// Whether an authored length carries `em` or `rem` — the two font-relative
/// units the admitted grammar resolves, whose basis is only as trustworthy as
/// the `font-size` that set it. Same token scan as
/// [`has_unit_without_a_basis`]; `0.5rem` never false-matches `em` because the
/// `e` is preceded by `r`, not a digit.
fn has_font_relative_unit(value: &str) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for unit in ["em", "rem"] {
        let mut from = 0;
        while let Some(offset) = lower[from..].find(unit) {
            let start = from + offset;
            let end = start + unit.len();
            let preceded_by_number =
                start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.');
            let ends_the_token = bytes
                .get(end)
                .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'-');
            if preceded_by_number && ends_the_token {
                return Some(unit);
            }
            from = start + 1;
        }
    }
    None
}

/// Whether authored `font-size`-bearing text would set an `em` basis this
/// patrol cannot vouch for: a basis-less unit (the device pin, measured
/// twentyfold wrong), `var()` indirection (the unit hides in a custom
/// property), or a CSS escape (the unit hides in the tokenizer). All three
/// were measured painting a wrong width silently before this scan.
fn poisons_font_basis(text: &str) -> Option<&'static str> {
    if let Some(unit) = has_unit_without_a_basis(text) {
        return Some(unit);
    }
    let lower = text.to_ascii_lowercase();
    if lower.contains("var(") {
        return Some("var()");
    }
    if lower.contains('\\') {
        return Some("a CSS escape");
    }
    None
}

/// Patrol every *attributable* ingress an authored stroke length property can
/// arrive through for a unit whose basis this build lacks: the presentation
/// attribute, the element's `style` attribute, and — because both supported
/// properties inherit — the same two on every ancestor.
///
/// A `<style>` sheet is the third ingress and is deliberately not here: it is
/// not attributable to one element without selector matching, so it is caught
/// document-level by [`stylesheet_findings`] instead, which refuses the whole
/// document under strict and declares once against the sheet under best-effort.
/// Between the two, all three spellings of the same declaration depart by name.
///
/// A CSS property name is case-insensitive, so the `style` leg lowercases before
/// it looks. It did not always: `style="STROKE-WIDTH:1vw"` painted a stroke
/// 12.8 units wide — the cascade's pinned 1280px device — where Chromium paints
/// 0.64, silently, because the guard here was a case-sensitive `contains`. The
/// sheet leg never had the bug; it compares with `eq_ignore_ascii_case`.
///
/// Both legs read the authored text coarsely — the `style` attribute is scanned
/// whole, so a basis-less unit belonging to some other property in the same
/// block refuses this element too. Over-refusal, never wrong pixels.
///
/// Three spellings can hide a unit from a text scan, and each was measured
/// painting a silently wrong width before it was refused here:
///
/// - **`var()` indirection**: the unit lives in a custom property
///   (`--w: 1vw; stroke-width: var(--w)` painted 12.8 where Chromium paints
///   0.64). Which declaration feeds the substitution is a resolver question,
///   not a patrol question, so any `var(` in stroke-width-bearing text refuses
///   — including a `var()` that would have resolved to an honest length.
/// - **CSS escapes**: `1\76 w` is `1vw` to the tokenizer and nothing to this
///   scan (measured: the same 12.8), and an escape can hide the property name
///   as well as the unit — so any escape in a `style` attribute in scope, or
///   in the stroke-width attribute's own value, refuses.
/// - **a poisoned `em` basis**: `em`/`rem` are admitted because `font-size` is
///   a basis this cascade has — but `font-size: 2vw` under a `stroke-width`
///   in `1em` painted 25.6 where Chromium paints 1.28. When the walk finds a
///   font-relative stroke-width, every authored `font-size` in scope — the
///   presentation attribute, `font`-bearing style attributes, and every
///   `<style>` sheet (a sheet is not attributable, so it is reached by a
///   descent from the root) — must pass [`poisons_font_basis`].
fn patrol_stroke_length_units(
    el: HtmlElement<'_>,
    element_name: &str,
    property: &str,
) -> Result<(), CompileError> {
    let mut font_relative: Option<&'static str> = None;
    let mut root = el;
    let mut ancestor = Some(el);
    while let Some(element) = ancestor {
        // An escape can hide the property name itself (`stroke-\77idth`), so
        // this check runs on every style attribute in scope, before the
        // property-bearing filter below could be fooled into skipping one.
        if let Some(style) = get_attr(element, "style")
            && style.contains('\\')
        {
            return Err(CompileError::UnsupportedStroke(format!(
                "a style attribute in scope of <{element_name}> carries a CSS escape this \
                 {property} patrol cannot read"
            )));
        }
        for value in [
            get_attr(element, property),
            get_attr(element, "style").filter(|style| css_declares_property(style, property)),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(unit) = has_unit_without_a_basis(&value) {
                return Err(CompileError::UnsupportedStroke(format!(
                    "a {property} in {unit} on <{element_name}> needs a basis this cascade does \
                     not have"
                )));
            }
            if value.contains('\\') {
                return Err(CompileError::UnsupportedStroke(format!(
                    "a {property} on <{element_name}> carries a CSS escape this patrol cannot \
                     read"
                )));
            }
            if value.to_ascii_lowercase().contains("var(") {
                return Err(CompileError::UnsupportedStroke(format!(
                    "a {property} on <{element_name}> resolves through var(), an indirection \
                     this patrol cannot follow"
                )));
            }
            if font_relative.is_none() {
                font_relative = has_font_relative_unit(&value);
            }
        }
        root = element;
        ancestor = element.traversal_parent();
    }

    let Some(unit) = font_relative else {
        return Ok(());
    };
    // The em basis is the cascaded font-size: first the attributable
    // spellings on the ancestor chain…
    let mut ancestor = Some(el);
    while let Some(element) = ancestor {
        for text in [
            get_attr(element, "font-size"),
            get_attr(element, "style").filter(|style| style.to_ascii_lowercase().contains("font")),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(poison) = poisons_font_basis(&text) {
                return Err(CompileError::UnsupportedStroke(format!(
                    "a {property} in {unit} on <{element_name}> under an authored font-size \
                     carrying {poison} needs a basis this cascade does not have"
                )));
            }
        }
        ancestor = element.traversal_parent();
    }
    // …then every sheet, which can match any ancestor without being
    // attributable to one.
    let mut stack = vec![root];
    while let Some(element) = stack.pop() {
        if element.local_name_string() == "style" {
            let mut sheet = String::new();
            for child_id in &element.dom_node().children {
                if let DemoNodeData::Text(text) = &element.dom().node(*child_id).data {
                    sheet.push_str(text);
                }
            }
            if let Some(poison) = stylesheet_font_size_poison(&sheet) {
                return Err(CompileError::UnsupportedStroke(format!(
                    "a {property} in {unit} on <{element_name}> under a stylesheet font-size \
                     carrying {poison} needs a basis this cascade does not have"
                )));
            }
        }
        let mut child = element.first_element_child();
        while let Some(c) = child {
            stack.push(c);
            child = c.next_element_sibling();
        }
    }
    Ok(())
}

fn patrol_stroke_width_units(el: HtmlElement<'_>, element_name: &str) -> Result<(), CompileError> {
    patrol_stroke_length_units(el, element_name, "stroke-width")
}

fn patrol_stroke_dasharray_units(
    el: HtmlElement<'_>,
    element_name: &str,
) -> Result<(), CompileError> {
    patrol_stroke_length_units(el, element_name, "stroke-dasharray")
}

/// Resolve the SVG stroke from the one cascade, as typed values — the same
/// ingress discipline as [`resolve_fill`], so presentation attributes,
/// stylesheet rules, inheritance through containers, unit-bearing lengths
/// (`8px`, `0.5em`) and CSS keyword case-insensitivity all come from the
/// cascade rather than from an attribute parse here.
///
/// `None` means nothing is stroked: `stroke: none`, or a zero width. Chromium
/// paints nothing in both cases (measured), so this is an admitted nothing and
/// not a hole.
///
/// A negative `stroke-width` never arrives: it fails the property's
/// non-negative grammar, so the cascade drops the declaration and this read
/// sees the inherited or initial value — exactly what Chromium paints.
fn resolve_stroke(
    el: HtmlElement<'_>,
    element_name: &str,
    servers: &PaintServers<'_>,
    paint_contexts: &[PaintContext<'_>],
    consumer_box: Rectangle,
    destination_to_frame: AffineTransform,
    bases: PercentBases,
    extra_opacity: f32,
) -> Result<Option<Stroke>, CompileError> {
    let data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();

    // The group-scope rung's fold factor joins the one float product —
    // see [`resolve_fill`].
    let opacity = match style.clone_stroke_opacity() {
        SVGOpacity::Opacity(value) => value * extra_opacity,
        other => {
            return Err(CompileError::UnsupportedStroke(format!(
                "stroke-opacity {other:?} is a context value this slice does not consume"
            )));
        }
    };
    drop(data);
    let Some(selected) = select_paint(el, PaintProperty::Stroke, paint_contexts)
        .map_err(CompileError::UnsupportedStroke)?
    else {
        return Ok(None);
    };
    let owner_data = selected
        .owner
        .borrow_data()
        .ok_or(CompileError::MissingComputedStyle)?;
    let owner_style: &ComputedValues = owner_data.styles.primary();
    let paint = selected.value;
    let stroke_fallback = || match &paint.fallback {
        style::values::generics::svg::SVGPaintFallback::Color(color) => {
            admitted_srgb(owner_style.resolve_color(color), opacity)
                .map(PaintStack::solid)
                .map_err(CompileError::UnsupportedStroke)
        }
        _ => Ok(PaintStack::empty()),
    };
    let paints = match paint.kind {
        SVGPaintKind::None => return Ok(None),
        SVGPaintKind::Color(ref color) => admitted_srgb(owner_style.resolve_color(color), opacity)
            .map(PaintStack::solid)
            .map_err(CompileError::UnsupportedStroke)?,
        SVGPaintKind::PaintServer(ref url) => {
            if extra_opacity != 1.0 {
                return Err(CompileError::UnsupportedStroke(
                    "element opacity over a url() paint is not yet consumed (the fold cannot \
                     reach through a paint-server reference)"
                        .to_string(),
                ));
            }
            // The stroke's paint box is the geometry's own box — the stroke's
            // inked reach beyond it pads (measured).
            match resolve_paint_server_stack(
                servers,
                url,
                || context_reference_space(selected.context, consumer_box, destination_to_frame),
                consumer_box,
                bases,
                opacity,
                "stroke",
            )? {
                Some(stack) => stack,
                None => stroke_fallback()?,
            }
        }
        SVGPaintKind::ContextFill | SVGPaintKind::ContextStroke => {
            unreachable!("select_paint removes every context relation")
        }
    };
    if paints.is_empty() {
        // A paint-server stroke that resolves to nothing painted (or an
        // invalid reference with no usable fallback) strokes nothing.
        return Ok(None);
    }

    // A percentage `stroke-width` resolves against the viewport's normalized
    // diagonal (measured: `10%` on a 64x64 viewport paints 6.4 units wide) —
    // the same basis chain the shape geometry percentages refuse on.
    let destination_data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    let destination_style: &ComputedValues = destination_data.styles.primary();
    let width = match destination_style.clone_stroke_width() {
        SVGLength::ContextValue => {
            return Err(CompileError::UnsupportedStroke(
                "stroke-width: context-value".to_string(),
            ));
        }
        SVGLength::LengthPercentage(width) => match width.0.to_length() {
            Some(length) => {
                // The unit is gone from a computed length, so the authored text
                // is what says whether its basis was one this build has.
                patrol_stroke_width_units(el, element_name)?;
                length.px()
            }
            // A pure percentage resolves against the viewport's normalized
            // diagonal (SVG2 §7.10; measured — `10%` of 64x64 paints 6.4
            // units). A calc() mixing lengths and percentages has neither a
            // computed length nor a pure percentage and stays refused.
            None => match width.0.to_percentage() {
                Some(percentage) => percentage.0 * bases.diagonal(),
                None => {
                    return Err(CompileError::UnsupportedStroke(
                        "a calc() stroke-width mixing lengths and percentages is not consumed"
                            .to_string(),
                    ));
                }
            },
        },
    };
    if width == 0.0 {
        return Ok(None);
    }

    // Computed values have already lost the authored unit and substitution
    // provenance. Patrol every ingress before looking at the typed list: a
    // basis-less unit or var() can compute to the empty initial value in this
    // build while Chromium still paints a dash cycle, so `Values([])` is not
    // permission to skip the authored-text guard.
    patrol_stroke_dasharray_units(el, element_name)?;
    let dash_intervals = match destination_style.clone_stroke_dasharray() {
        SVGStrokeDashArray::ContextValue => {
            return Err(CompileError::UnsupportedStroke(
                "stroke-dasharray: context-value".to_string(),
            ));
        }
        SVGStrokeDashArray::Values(values) if values.is_empty() => None,
        SVGStrokeDashArray::Values(values) => {
            let mut intervals = Vec::with_capacity(values.len() * 2);
            for value in values.iter() {
                intervals.push(value.0.resolve(Length::new(bases.diagonal())).px());
            }
            if !intervals.len().is_multiple_of(2) {
                intervals.extend_from_within(..);
            }
            match StrokeDashIntervals::new(intervals) {
                Ok(intervals) => intervals,
                Err(StrokeDashIntervalsError::UnrepresentableCycleLength) => {
                    return Err(CompileError::UnsupportedStroke(
                        "a stroke-dasharray cycle has a finite authored grammar but its \
                         resolved total is not representable by this frame contract"
                            .to_string(),
                    ));
                }
                Err(error) => {
                    return Err(CompileError::UnsupportedStroke(format!(
                        "stroke-dasharray did not resolve to one checked cycle: {error}"
                    )));
                }
            }
        }
    };

    let cap = match destination_style.clone_stroke_linecap() {
        StyloLinecap::Butt => StrokeCap::Butt,
        StyloLinecap::Round => StrokeCap::Round,
        StyloLinecap::Square => StrokeCap::Square,
    };
    // Three variants is the full supported grammar, not a subset: the
    // SVG2-only `miter-clip` and `arcs` are invalid declarations in Chromium
    // too (measured — byte-identical to `miter`, and an invalid style-attribute
    // spelling drops so a valid attribute survives), so Stylo's three-keyword
    // parse lands both admissions on the same fallback by construction.
    let join = match destination_style.clone_stroke_linejoin() {
        StyloLinejoin::Miter => StrokeJoin::Miter,
        StyloLinejoin::Round => StrokeJoin::Round,
        StyloLinejoin::Bevel => StrokeJoin::Bevel,
    };
    // Carried as resolved, including a limit below 1 that no miter can satisfy:
    // the backend bevels it, which is what Chromium does with the same value.
    let miter_limit = destination_style.clone_stroke_miterlimit().0;

    // The cascade's non-negative types make a rejection here unreachable from a
    // document, so it would be this compiler's bug — named, never painted.
    Stroke::new_with_dash_intervals(paints, width, cap, join, miter_limit, dash_intervals)
        .map_err(|error| CompileError::UnsupportedStroke(error.to_string()))
}

/// Admit a cascaded absolute color only where its fidelity is gated: the
/// opaque sRGB values the Chromium-baked primitive suite covers. Any other
/// color space would pass through an unverified conversion and per-channel
/// clamp refuses explicitly until its own capability step bakes fixtures.
///
/// The translucency rung folds paint alpha here: the color's own alpha and
/// the paint-level opacity (`fill-opacity` / `stroke-opacity`) multiply in
/// float and quantize **once** — Chromium composites the product, not the
/// quantized factors, and the multiplied cell is baked to pin the rounding.
///
/// The *reason* is returned rather than the error, so each caller names its own
/// property: the same unusable color is an unsupported `fill` in
/// [`resolve_fill`] and an unsupported `stroke` in [`resolve_stroke`], and a
/// declared hole that names the wrong property misdirects whoever reads it.
pub(crate) fn admitted_srgb(color: AbsoluteColor, paint_opacity: f32) -> Result<CGColor, String> {
    if color.color_space != ColorSpace::Srgb {
        return Err(format!(
            "color space {:?} is not yet gated against Chromium",
            color.color_space
        ));
    }
    let c = color.raw_components();
    Ok(CGColor::from_rgba(
        to_u8(c[0]),
        to_u8(c[1]),
        to_u8(c[2]),
        to_u8(color.alpha * paint_opacity),
    ))
}

fn to_u8(component: f32) -> u8 {
    (component.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn parse_viewbox(v: &str) -> Result<(f32, f32, f32, f32), CompileError> {
    // Support the explicit proving-shell grammar: four finite numbers
    // separated by ASCII whitespace and/or one comma. Empty comma groups are
    // malformed; reject them instead of filtering repeated/trailing commas.
    // More compact SVG number-list forms remain unsupported rather than
    // guessed.
    let comma_groups: Vec<&str> = v.split(',').collect();
    if comma_groups
        .iter()
        .any(|group| trim_svg_whitespace(group).is_empty())
    {
        return Err(CompileError::BadViewBox(v.to_string()));
    }
    let tokens: Vec<&str> = comma_groups
        .iter()
        .flat_map(|group| group.split_ascii_whitespace())
        .collect();
    if tokens.len() != 4 {
        return Err(CompileError::BadViewBox(v.to_string()));
    }
    let mut parts = [0.0f32; 4];
    for (index, token) in tokens.iter().enumerate() {
        let value = token
            .parse::<f32>()
            .map_err(|_| CompileError::BadViewBox(v.to_string()))?;
        if !value.is_finite() {
            return Err(CompileError::BadViewBox(v.to_string()));
        }
        parts[index] = value;
    }
    if parts[2] <= 0.0 || parts[3] <= 0.0 {
        return Err(CompileError::BadViewBox(v.to_string()));
    }
    Ok((parts[0], parts[1], parts[2], parts[3]))
}

/// The `preserveAspectRatio` fit mode: how the viewBox scales into the
/// viewport (SVG2 §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fit {
    /// `none`: non-uniform scale, each axis fills the viewport exactly.
    None,
    /// `meet`: uniform scale, the whole viewBox fits inside the viewport.
    Meet,
    /// `slice`: uniform scale, the viewBox covers the whole viewport (the
    /// overhang is cut by the frame's viewport clip).
    Slice,
}

/// One axis of the `preserveAspectRatio` alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Min,
    Mid,
    Max,
}

/// A parsed `preserveAspectRatio` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PreserveAspectRatio {
    fit: Fit,
    align_x: Align,
    align_y: Align,
}

impl Default for PreserveAspectRatio {
    /// The SVG default: `xMidYMid meet`.
    fn default() -> Self {
        Self {
            fit: Fit::Meet,
            align_x: Align::Mid,
            align_y: Align::Mid,
        }
    }
}

/// Parse `preserveAspectRatio` with the exact SVG2 grammar Chromium
/// implements: an alignment keyword (`none` or one of the nine
/// case-sensitive `x{Min,Mid,Max}Y{Min,Mid,Max}` forms) optionally followed
/// by `meet` or `slice`. The frozen donor's permissive fallback-to-default
/// (`crates/htmlcss/src/svg/dom/attrs.rs`) is deliberately not ported:
/// malformed grammar refuses loudly, the same posture as [`parse_viewbox`],
/// where Chromium silently renders the default `xMidYMid meet` mapping.
/// That includes the SVG 1.1 `defer` prefix: SVG2 dropped it, and Chromium
/// treats any value carrying it as unparseable (falling back to the
/// default), so it is malformed grammar here — a future rung must never
/// "restore" it by honoring the remainder of the value.
fn parse_preserve_aspect_ratio(v: &str) -> Result<PreserveAspectRatio, CompileError> {
    let tokens: Vec<&str> = v.split_ascii_whitespace().collect();
    let (align_token, fit_token) = match tokens.as_slice() {
        [align] => (*align, None),
        [align, fit] => (*align, Some(*fit)),
        _ => return Err(CompileError::BadPreserveAspectRatio(v.to_string())),
    };
    let (align_x, align_y) = match align_token {
        "none" | "xMinYMin" => (Align::Min, Align::Min),
        "xMidYMin" => (Align::Mid, Align::Min),
        "xMaxYMin" => (Align::Max, Align::Min),
        "xMinYMid" => (Align::Min, Align::Mid),
        "xMidYMid" => (Align::Mid, Align::Mid),
        "xMaxYMid" => (Align::Max, Align::Mid),
        "xMinYMax" => (Align::Min, Align::Max),
        "xMidYMax" => (Align::Mid, Align::Max),
        "xMaxYMax" => (Align::Max, Align::Max),
        _ => return Err(CompileError::BadPreserveAspectRatio(v.to_string())),
    };
    let fit = match (align_token, fit_token) {
        // `none` still permits an explicit meet|slice token grammatically;
        // the token is validated, then ignored per spec.
        ("none", None | Some("meet" | "slice")) => Fit::None,
        (_, None | Some("meet")) => Fit::Meet,
        (_, Some("slice")) => Fit::Slice,
        _ => return Err(CompileError::BadPreserveAspectRatio(v.to_string())),
    };
    Ok(PreserveAspectRatio {
        fit,
        align_x,
        align_y,
    })
}

/// The viewBox → viewport transform (SVG2 §8.2 `computeViewBoxTransform`):
/// translate to the aligned offset, scale by the fitted factors, translate
/// by the negated viewBox origin. A near-literal transplant of the frozen
/// donor's `compute_viewbox_matrix`
/// (`crates/htmlcss/src/svg/layout/viewport.rs`), itself Blink-shaped
/// (`core/svg/svg_svg_element.cc` ViewBoxToViewTransform as applied by
/// `core/paint/svg_root_painter.cc`); the root viewport origin is (0, 0)
/// here, so the donor's viewport-offset terms drop out.
fn viewbox_to_viewport_transform(
    viewport: (f32, f32),
    viewbox: (f32, f32, f32, f32),
    par: PreserveAspectRatio,
) -> AffineTransform {
    let (vp_w, vp_h) = viewport;
    let (vb_x, vb_y, vb_w, vb_h) = viewbox;
    let scale_x = vp_w / vb_w;
    let scale_y = vp_h / vb_h;
    let (sx, sy) = match par.fit {
        Fit::None => (scale_x, scale_y),
        Fit::Meet => {
            let s = scale_x.min(scale_y);
            (s, s)
        }
        Fit::Slice => {
            let s = scale_x.max(scale_y);
            (s, s)
        }
    };
    let dx = align_offset(par.align_x, vp_w, vb_w * sx);
    let dy = align_offset(par.align_y, vp_h, vb_h * sy);
    // translate(dx, dy) ∘ scale(sx, sy) ∘ translate(-vb_x, -vb_y)
    AffineTransform::from_acebdf(sx, 0.0, dx - vb_x * sx, 0.0, sy, dy - vb_y * sy)
}

/// One axis of the `preserveAspectRatio` alignment offset: where the scaled
/// viewBox extent sits inside the viewport extent.
fn align_offset(align: Align, viewport_extent: f32, scaled_viewbox_extent: f32) -> f32 {
    match align {
        Align::Min => 0.0,
        Align::Mid => (viewport_extent - scaled_viewbox_extent) / 2.0,
        Align::Max => viewport_extent - scaled_viewbox_extent,
    }
}

/// A root `width`/`height` read: an authored `auto` (the CSS-wide keyword
/// SVG2 gives these geometry properties, ASCII case-insensitive) is
/// literally the absent-attribute value — both resolve as `auto` — so it
/// reads as `None` instead of misreporting valid grammar as a bad number.
fn root_dimension_f32(
    svg: HtmlElement<'_>,
    name: &str,
    values: &EffectiveValues,
) -> Result<Option<f32>, CompileError> {
    if values.scalar(svg.node_id(), name).is_none()
        && get_attr(svg, name)
            .is_some_and(|value| trim_svg_whitespace(&value).eq_ignore_ascii_case("auto"))
    {
        return Ok(None);
    }
    effective_attr_f32(svg, name, values)
}

/// Refuse a root `width`/`height` authored as a percentage. `N%` is valid
/// SVG length grammar this slice does not yet resolve; refusing by name is
/// honest where the numeric parse would misreport it as
/// [`CompileError::BadNumber`] junk.
fn reject_percentage_dimension(svg: HtmlElement<'_>, attr: &str) -> Result<(), CompileError> {
    if let Some(value) = get_attr(svg, attr)
        && trim_svg_whitespace(&value).ends_with('%')
    {
        return Err(CompileError::UnsupportedSizing(format!(
            "percentage {attr}={value:?} on the root <svg> is not yet consumed"
        )));
    }
    Ok(())
}

fn reject_negative_dimension(attr: &str, value: f32) -> Result<(), CompileError> {
    if value < 0.0 {
        return Err(CompileError::InvalidDimension {
            attr: attr.to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

/// Read an element attribute by exact local name **in no namespace** from
/// its owning document session.
///
/// SVG attribute names are case-sensitive; each grammar entry already
/// applies its own canonicalization (the HTML tokenizer lowercases and
/// foreign-content-adjusts known SVG attributes to their canonical case;
/// XML preserves authored case), so an authored `viewbox` in XML is
/// honestly not `viewBox`.
///
/// The namespace check is load-bearing under the namespace-aware XML
/// entry: every SVG rendering attribute this compiler reads is defined in
/// no namespace, so a prefixed `foo:r` is a foreign attribute Chromium
/// ignores. Matching it on local name alone would consume it as geometry
/// and paint a shape the browser does not.
pub(crate) fn get_attr(element: HtmlElement<'_>, name: &str) -> Option<String> {
    if let DemoNodeData::Element(e) = &element.dom_node().data {
        for a in &e.attrs {
            if a.name.ns.as_ref().is_empty() && a.name.local.as_ref() == name {
                return Some(a.value.to_string());
            }
        }
    }
    None
}

/// The effective scalar value of an attribute: the frame request's animated
/// override when one targets this node, the authored attribute otherwise.
fn effective_attr_f32(
    element: HtmlElement<'_>,
    name: &str,
    values: &EffectiveValues,
) -> Result<Option<f32>, CompileError> {
    if let Some(value) = values.scalar(element.node_id(), name) {
        return Ok(Some(value));
    }
    attr_f32(element, name)
}

/// [`effective_attr_f32`] for shape geometry, where the SVG length grammar
/// admits a percentage: `<number>%`, no space, resolved against the
/// attribute's axis basis. A sampled override is already resolved user
/// units and never a percentage. Anything else malformed stays the
/// [`CompileError::BadNumber`] refusal the plain read gives it.
fn geometry_attr_f32(
    element: HtmlElement<'_>,
    name: &str,
    values: &EffectiveValues,
    bases: PercentBases,
) -> Result<Option<f32>, CompileError> {
    if let Some(value) = values.scalar(element.node_id(), name) {
        return Ok(Some(value));
    }
    let Some(v) = get_attr(element, name) else {
        return Ok(None);
    };
    let trimmed = trim_svg_whitespace(&v);
    if let Some(number) = trimmed.strip_suffix('%') {
        if !dots_carry_digits(number) {
            return Err(CompileError::BadNumber {
                attr: name.to_string(),
                value: v.clone(),
            });
        }
        let parsed = number.parse::<f32>().ok().filter(|value| value.is_finite());
        let Some(parsed) = parsed else {
            return Err(CompileError::BadNumber {
                attr: name.to_string(),
                value: v,
            });
        };
        return Ok(Some(parsed / 100.0 * bases.axis(name)));
    }
    attr_f32(element, name)
}

/// Whether every `.` in the token is followed by an ASCII digit. Rust's
/// float grammar is a superset of the SVG/CSS number grammar: a trailing
/// dot (`32.`, `3.e2`) parses as f32 but is an invalid number token to
/// Chromium, whose invalid attribute resolves to the property's initial
/// value — a different geometry than a parsed `32`. The other
/// Rust-accepted finite forms (`+3`, `.5`, `1e2`, `1E+2`) are valid SVG
/// numbers.
pub(crate) fn dots_carry_digits(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| *byte != b'.' || bytes.get(index + 1).is_some_and(u8::is_ascii_digit))
}

/// Strip the whitespace an SVG attribute value may carry around its
/// content — exactly the five ASCII characters the SVG/CSS grammars call
/// whitespace, never Rust's Unicode `str::trim` set. A value padded with
/// NBSP or U+3000 is invalid to Chromium (which falls back to the
/// property's initial value); trimming it here would parse a number the
/// browser never sees and silently paint different geometry, so anything
/// outside this set stays in the token and refuses as a bad number.
pub(crate) fn trim_svg_whitespace(value: &str) -> &str {
    value.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0C'))
}

fn attr_f32(element: HtmlElement<'_>, name: &str) -> Result<Option<f32>, CompileError> {
    match get_attr(element, name) {
        None => Ok(None),
        Some(v) => {
            let trimmed = trim_svg_whitespace(&v);
            if !dots_carry_digits(trimmed) {
                return Err(CompileError::BadNumber {
                    attr: name.to_string(),
                    value: v.clone(),
                });
            }
            let parsed = trimmed
                .parse::<f32>()
                .map_err(|_| CompileError::BadNumber {
                    attr: name.to_string(),
                    value: v.clone(),
                })?;
            if !parsed.is_finite() {
                return Err(CompileError::BadNumber {
                    attr: name.to_string(),
                    value: v,
                });
            }
            Ok(Some(parsed))
        }
    }
}
