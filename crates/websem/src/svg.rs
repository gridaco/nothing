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
//! Deliberately narrow: the proving shell supports only the enumerated
//! viewport/fill cases around an outer `<svg>` and solid-filled `<rect>`,
//! `<circle>`, and `<ellipse>` children, plus one retained exact-time
//! `<animate attributeName="x">` slice (rects only). Root sizing
//! follows SVG2 §8.2: explicit `width`/`height` win; a missing dimension is
//! `auto` and resolves to 100% of the host-established [`InitialViewport`]
//! (standalone entry only — the inline HTML entry refuses until CSS
//! replaced-element sizing is implemented); `viewBox` maps user units into
//! the viewport under the full `preserveAspectRatio` grammar.
//! [`CompileError`] makes patrolled static rejection cases explicit and
//! [`crate::AnimationError`] closes the sampled standalone dynamic inventory.
//! Inline HTML remains Base-only until its document-wide inventory is closed.
//! This is not yet an exhaustive SVG-surface validator or an SVG capability
//! claim.
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
//! ([`RENDERING_ATTRIBUTES_NOT_CONSUMED`]), while attributes outside the
//! SVG rendering vocabulary stay ignored exactly as Chromium ignores them;
//! the cascaded surface is patrolled for the enumerated properties
//! `opacity`, `display: none`, `visibility`, and shape `stroke` beside the
//! typed `fill`/`fill-opacity` reads. Cascaded properties beyond that
//! enumeration remain a **named open boundary** of the slice — not a
//! coverage claim.
//!
//! ## SVG paint boundary
//! Paint is consumed from the one Stylo cascade as typed values:
//! [`resolve_fill`] reads the computed SVG `fill` longhand, which
//! presentation hints (admitted set: `fill`), stylesheet rules, and inline
//! style attributes all feed with SVG2 precedence — csscascade owns every
//! ingress. `currentColor` resolves against the cascaded `color`; invalid
//! authored values fall back exactly as invalid CSS declarations. The
//! admitted value surface is opaque sRGB solid colors — exactly what the
//! Chromium-baked primitive suite gates pixel-exactly. Everything else
//! refuses explicitly until its own capability step bakes fixtures: paint
//! servers, context paints, non-initial `fill-opacity`, non-sRGB color
//! spaces, and translucent fills (`tests/typed_fill.rs` pins each).
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
use style::computed_values::visibility::T as Visibility;
use style::dom::TElement;
use style::properties::ComputedValues;
use style::thread_state::{self, ThreadState};
use style::values::computed::{SVGOpacity, Size};
use style::values::generics::svg::SVGPaintKind;

use cg::CGColor;
use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::frame::{Frame, FrameNode, Geometry, Identity, Provenance, SolidPaintStack, VisualRef};
use std::sync::Arc;

use crate::effective_values::EffectiveValues;
use crate::svg_animation::{AnimationInventory, is_animation_element};

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
    /// An element the slice does not support (the outer `<svg>` plus
    /// `<rect>`, `<circle>`, and `<ellipse>` children are admitted).
    UnsupportedElement(String),
    /// A `fill` value the slice cannot resolve.
    UnsupportedFill(String),
    /// A numeric attribute failed to parse.
    BadNumber { attr: String, value: String },
    /// Viewport sizing needs a default/CSS sizing path this slice lacks.
    UnsupportedSizing(String),
    /// A shape geometry attribute in valid SVG length grammar the slice
    /// cannot yet resolve: percentages, whose basis chain (viewport axes,
    /// and the normalized diagonal for `<circle r>`) is not consumed.
    /// Refusing by name is honest where the numeric parse would misreport
    /// the value as [`CompileError::BadNumber`] junk.
    UnsupportedLength {
        element: String,
        attr: String,
        value: String,
    },
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

    fn from_source(
        source: Arc<str>,
        entry: SourceEntry,
        mode: CompileMode,
        initial_viewport: Option<InitialViewport>,
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
        let (svg_root, compilation, animation) = {
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
            // A stylesheet declaration the cascade cannot represent would
            // paint as if absent wherever its selector matches. It is not
            // attributable to one element without selector matching, so it
            // refuses at the document level in both admissions. The scan
            // starts at the document root: the HTML entry's stylesheet
            // commonly lives in <head>, outside the compiled SVG subtree.
            if let Some(property) = unrepresented_stylesheet_property(root) {
                return Err(CompileError::UnsupportedStyle(format!(
                    "a stylesheet declares {property}, which this cascade does not represent"
                )));
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
            let compilation = compile_svg_element(
                svg,
                &EffectiveValues::base(),
                mode,
                &mut degradations,
                initial_viewport,
            )?;
            let animation = AnimationInventory::inspect(svg, &compilation.materialized, entry);
            (svg.node_id(), compilation, animation)
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
        // skips deterministically and its sink is discarded.
        let compilation = compile_svg_element(
            svg,
            &values,
            self.mode,
            &mut Vec::new(),
            self.initial_viewport,
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
            CompileError::BadNumber { attr, value } => {
                write!(f, "attribute {attr}={value:?} is not a number")
            }
            CompileError::UnsupportedSizing(reason) => {
                write!(f, "unsupported SVG viewport sizing: {reason}")
            }
            CompileError::UnsupportedLength {
                element,
                attr,
                value,
            } => write!(
                f,
                "unsupported length {attr}={value:?} on <{element}>: the percentage basis is \
                 not yet consumed"
            ),
            CompileError::InvalidDimension { attr, value } => {
                write!(f, "invalid SVG viewport dimension {attr}={value:?}")
            }
            CompileError::BadViewBox(v) => write!(f, "viewBox {v:?} is invalid"),
            CompileError::BadPreserveAspectRatio(v) => {
                write!(f, "preserveAspectRatio {v:?} is invalid")
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
    if el.local_name_string() == "script" {
        return true;
    }
    let mut child = el.first_element_child();
    while let Some(c) = child {
        if subtree_contains_script(c) {
            return true;
        }
        child = c.next_element_sibling();
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
    "transform",
    "transform-origin",
    "opacity",
    "display",
    "visibility",
    "overflow",
    "clip",
    "clip-path",
    "clip-rule",
    "mask",
    "filter",
    "color",
    "fill-opacity",
    "fill-rule",
    "stroke",
    "stroke-width",
    "stroke-opacity",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "paint-order",
    "shape-rendering",
    "image-rendering",
    "color-rendering",
    "color-interpolation",
    "vector-effect",
    "requiredFeatures",
    "requiredExtensions",
    "systemLanguage",
];

/// Rendering attributes additionally rejected on `<rect>`: rounded corners
/// are not painted by the slice.
const RECT_RENDERING_ATTRIBUTES_NOT_CONSUMED: &[&str] = &["rx", "ry"];

/// CSS properties that move, clip, or recolor Chromium's pixels and that
/// the pinned servo-mode Stylo build cannot represent at all — every one is
/// `engine = "gecko"`-gated in `longhands.toml`, so the cascade drops the
/// declaration at parse and no computed value survives for
/// [`patrol_computed_style`] to read.
///
/// The attribute spellings of several of these are already refused by
/// [`RENDERING_ATTRIBUTES_NOT_CONSUMED`]; this list closes the *CSS* leg,
/// which would otherwise render silently as if the declaration were
/// absent. It is a closed list of known-dangerous names, not a claim about
/// every property: names outside it that Stylo does represent are reachable
/// through the computed-level patrol, and the remainder stays the named
/// open boundary the module doc declares.
const CASCADE_PROPERTIES_NOT_REPRESENTED: &[&str] = &[
    "transform",
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
];

/// The first [`CASCADE_PROPERTIES_NOT_REPRESENTED`] property name declared
/// in a CSS fragment — a `style` attribute's declaration block or a
/// `<style>` element's whole text.
///
/// This reads the authored text rather than the cascade because the cascade
/// is exactly what discards these declarations. It is a deliberately simple
/// scanner over `{`/`}`/`;`-delimited chunks: it can only ever *add* a
/// refusal for one of the enumerated names (loud), never admit one, and it
/// makes no claim to be a CSS parser.
fn unrepresented_property(css: &str) -> Option<String> {
    for chunk in css.split([';', '{', '}']) {
        let Some((name, _)) = chunk.split_once(':') else {
            continue;
        };
        let name = trim_svg_whitespace(name).to_ascii_lowercase();
        if CASCADE_PROPERTIES_NOT_REPRESENTED.contains(&name.as_str()) {
            return Some(name);
        }
    }
    None
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

/// The first `<style>` element in the document whose CSS declares a
/// property the cascade drops but Chromium paints with.
///
/// A stylesheet is not attributable to one element without running
/// selector matching, so this refuses at the document level in both
/// admissions — the same posture as the other document-level contracts.
/// Over-refusal (a rule that matches only skipped elements) is loud; the
/// alternative is silently painting an untransformed, unclipped, or
/// unfiltered shape.
fn unrepresented_stylesheet_property(element: HtmlElement<'_>) -> Option<String> {
    if element.local_name_string() == "style" {
        for child_id in &element.dom_node().children {
            if let DemoNodeData::Text(text) = &element.dom().node(*child_id).data
                && let Some(property) = unrepresented_property(text)
            {
                return Some(property);
            }
        }
    }
    let mut child = element.first_element_child();
    while let Some(c) = child {
        if let Some(property) = unrepresented_stylesheet_property(c) {
            return Some(property);
        }
        child = c.next_element_sibling();
    }
    None
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
fn patrol_computed_style(
    element: HtmlElement<'_>,
    include_stroke: bool,
    include_css_sizing: bool,
) -> Result<(), CompileError> {
    let data = element
        .borrow_data()
        .ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();
    let opacity = style.clone_opacity();
    if opacity != 1.0 {
        return Err(CompileError::UnsupportedStyle(format!(
            "opacity {opacity} is not yet consumed"
        )));
    }
    if style.clone_display().is_none() {
        return Err(CompileError::UnsupportedStyle(
            "display: none is not yet consumed".to_string(),
        ));
    }
    let visibility = style.clone_visibility();
    if !matches!(visibility, Visibility::Visible) {
        return Err(CompileError::UnsupportedStyle(format!(
            "visibility {visibility:?} is not yet consumed"
        )));
    }
    if include_stroke && !matches!(style.clone_stroke().kind, SVGPaintKind::None) {
        return Err(CompileError::UnsupportedStyle(
            "stroke paint is not yet consumed".to_string(),
        ));
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
    Ok(())
}

/// Refuse a shape geometry attribute authored as a percentage — the
/// sibling of [`reject_percentage_dimension`] for admitted shape elements.
/// This one is element-level, so best-effort declares-and-skips the shape
/// instead of refusing the document.
fn reject_percentage_geometry(
    element: HtmlElement<'_>,
    element_name: &str,
    attrs: &[&str],
) -> Result<(), CompileError> {
    for attr in attrs {
        if let Some(value) = get_attr(element, attr)
            && trim_svg_whitespace(&value).ends_with('%')
        {
            return Err(CompileError::UnsupportedLength {
                element: element_name.to_string(),
                attr: (*attr).to_string(),
                value,
            });
        }
    }
    Ok(())
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
    /// Source nodes materialized as frame nodes, in document order — the
    /// animation inventory's candidate target set (the inventory narrows it
    /// further: the admitted sampling slice targets `<rect>` only).
    materialized: Vec<NodeId>,
}

/// The one SVG compiler. Base and Sample(time) both enter here; the only
/// difference between them is the [`EffectiveValues`] view the attribute
/// reads resolve through. `mode` decides what a beyond-slice child does:
/// strict returns its error, best-effort records it in `degradations` and
/// leaves the child out of the frame. The checks up to and including the
/// outer viewport mapping are document-level and refuse in both modes.
fn compile_svg_element(
    svg: HtmlElement<'_>,
    values: &EffectiveValues,
    mode: CompileMode,
    degradations: &mut Vec<Degradation>,
    initial_viewport: Option<InitialViewport>,
) -> Result<FrameCompilation, CompileError> {
    // The outer <svg> is the canvas contract: a rendering attribute or a
    // cascaded value the slice cannot honor here would wrong every pixel,
    // so the root patrols are document-level in both modes.
    patrol_rendering_attributes(svg, "svg", &[])?;
    patrol_style_attribute(svg, "svg")?;
    patrol_computed_style(svg, false, true)?;
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

    let mut nodes = Vec::new();
    let mut materialized = Vec::new();
    let mut next_id = 0u64;
    let mut ordinals = HashMap::<String, usize>::new();
    let mut child = svg.first_element_child();
    while let Some(c) = child {
        let tag = c.local_name_string();
        let ordinal = ordinals.entry(tag.clone()).or_default();
        *ordinal += 1;
        // `<style>` is a non-rendering element: its CSS enters the one
        // cascade (csscascade collects it); it materializes nothing here.
        // Animation elements likewise contribute values, not geometry.
        if !is_animation_element(&tag) && tag != "style" {
            match compile_shape(c, viewport, &mut next_id, values) {
                Ok(node) => {
                    nodes.push(node);
                    materialized.push(c.node_id());
                }
                Err(error) => match mode {
                    CompileMode::Strict => return Err(error),
                    CompileMode::BestEffort => degradations.push(Degradation {
                        path: format!("svg/{tag}[{ordinal}]"),
                        action: DegradationAction::Skipped,
                        reason: error.to_string(),
                    }),
                },
            }
        }
        child = c.next_element_sibling();
    }

    Ok(FrameCompilation {
        frame: Frame {
            owner: VisualRef::new(Identity::new(0), Provenance::new(0)),
            bounds: frame_bounds,
            nodes,
        },
        materialized,
    })
}

/// Compile a single shape element into a resolved node.
///
/// Local names match exactly: SVG element names are case-sensitive, and each
/// grammar entry already applies its own canonicalization (the HTML tokenizer
/// lowercases and foreign-content-adjusts; XML preserves authored case).
fn compile_shape(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
) -> Result<FrameNode, CompileError> {
    let tag = el.local_name_string();
    match tag.as_str() {
        "rect" => compile_rect(el, viewport, next_id, values),
        "circle" => compile_circle(el, viewport, next_id, values),
        "ellipse" => compile_ellipse(el, viewport, next_id, values),
        other => Err(CompileError::UnsupportedElement(other.to_string())),
    }
}

fn compile_rect(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
) -> Result<FrameNode, CompileError> {
    patrol_rendering_attributes(el, "rect", RECT_RENDERING_ATTRIBUTES_NOT_CONSUMED)?;
    patrol_style_attribute(el, "rect")?;
    patrol_computed_style(el, true, true)?;
    reject_percentage_geometry(el, "rect", &["x", "y", "width", "height"])?;
    let x = effective_attr_f32(el, "x", values)?.unwrap_or(0.0);
    let y = effective_attr_f32(el, "y", values)?.unwrap_or(0.0);
    let w = effective_attr_f32(el, "width", values)?.unwrap_or(0.0);
    let h = effective_attr_f32(el, "height", values)?.unwrap_or(0.0);
    let rect = Rectangle::from_xywh(x, y, w, h);
    shape_node(el, Geometry::Rect(rect), viewport, next_id)
}

fn compile_circle(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
) -> Result<FrameNode, CompileError> {
    patrol_rendering_attributes(el, "circle", &[])?;
    patrol_style_attribute(el, "circle")?;
    patrol_computed_style(el, true, false)?;
    reject_percentage_geometry(el, "circle", &["cx", "cy", "r"])?;
    let cx = effective_attr_f32(el, "cx", values)?.unwrap_or(0.0);
    let cy = effective_attr_f32(el, "cy", values)?.unwrap_or(0.0);
    // SVG2 §10.3: a negative `r` is invalid and must be ignored, and a
    // computed value of zero disables rendering. Chromium clamps the used
    // value at layout (`LayoutSVGEllipse`: `std::max(radius, 0.f)`), so
    // negative and missing both resolve exactly as `r="0"`: the element is
    // admitted and paints nothing — an honest nothing, not a refusal.
    let r = effective_attr_f32(el, "r", values)?.unwrap_or(0.0).max(0.0);
    let rect = Rectangle::from_xywh(cx - r, cy - r, r * 2.0, r * 2.0);
    shape_node(el, Geometry::Ellipse(rect), viewport, next_id)
}

fn compile_ellipse(
    el: HtmlElement<'_>,
    viewport: AffineTransform,
    next_id: &mut u64,
    values: &EffectiveValues,
) -> Result<FrameNode, CompileError> {
    patrol_rendering_attributes(el, "ellipse", &[])?;
    patrol_style_attribute(el, "ellipse")?;
    patrol_computed_style(el, true, false)?;
    reject_percentage_geometry(el, "ellipse", &["cx", "cy", "rx", "ry"])?;
    let cx = effective_attr_f32(el, "cx", values)?.unwrap_or(0.0);
    let cy = effective_attr_f32(el, "cy", values)?.unwrap_or(0.0);
    // SVG2 §10.4: `rx`/`ry` initially `auto`; a negative value is invalid
    // and must be ignored, which Chromium treats as `auto` (frozen donor's
    // Chrome-confirmed reading, re-proved against Chromium 148: a single
    // negative radius adopts the other axis). `auto` adopts the other
    // radius; both `auto` resolve to zero; zero on either axis disables
    // rendering — the zero-extent oval below paints nothing.
    let rx = ellipse_radius(el, "rx", values)?;
    let ry = ellipse_radius(el, "ry", values)?;
    let (rx, ry) = match (rx, ry) {
        (Some(rx), Some(ry)) => (rx, ry),
        (Some(rx), None) => (rx, rx),
        (None, Some(ry)) => (ry, ry),
        (None, None) => (0.0, 0.0),
    };
    let rect = Rectangle::from_xywh(cx - rx, cy - ry, rx * 2.0, ry * 2.0);
    shape_node(el, Geometry::Ellipse(rect), viewport, next_id)
}

/// The shared tail of every shape compile: resolve the typed fill and emit
/// the resolved node. The node's `bounds` is the frame-space transform of
/// its local geometry box — the exact-bounds law the n0 downstream
/// re-checks on admission.
fn shape_node(
    el: HtmlElement<'_>,
    geometry: Geometry,
    viewport: AffineTransform,
    next_id: &mut u64,
) -> Result<FrameNode, CompileError> {
    let rect = match &geometry {
        Geometry::Rect(rect) | Geometry::Ellipse(rect) => *rect,
    };
    let fill = resolve_fill(el)?;
    let paints = match fill {
        Some(color) => SolidPaintStack::solid(color),
        None => SolidPaintStack::empty(),
    };

    let visual_id = *next_id + 1;
    let node = FrameNode {
        owner: VisualRef::new(Identity::new(visual_id), Provenance::new(visual_id)),
        transform: viewport,
        geometry,
        bounds: math2::rect_transform(rect, &viewport),
        paints,
    };
    *next_id += 1;
    Ok(node)
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
) -> Result<Option<f32>, CompileError> {
    Ok(effective_attr_f32(el, name, values)?.filter(|value| *value >= 0.0))
}

/// Resolve the SVG `fill` paint from the typed cascaded value — the one
/// place paint meaning enters the compiler. Presentation hints, stylesheet
/// rules, and inline style attributes all feed this read through the one
/// Stylo cascade, with SVG2 precedence; `currentColor` resolves against the
/// cascaded `color`, and an invalid authored value falls back exactly as an
/// invalid CSS declaration would. Paint servers and context paints are
/// refused explicitly. `fill-opacity` is deliberately not yet consumed —
/// its admission is a later capability step.
fn resolve_fill(el: HtmlElement<'_>) -> Result<Option<CGColor>, CompileError> {
    let data = el.borrow_data().ok_or(CompileError::MissingComputedStyle)?;
    let style: &ComputedValues = data.styles.primary();
    // The slice consumes no fill-opacity: a non-initial cascaded value would
    // silently render opaque where Chromium renders translucent, so it
    // refuses explicitly until its own capability step admits it.
    match style.clone_fill_opacity() {
        SVGOpacity::Opacity(1.0) => {}
        other => {
            return Err(CompileError::UnsupportedFill(format!(
                "fill-opacity {other:?} is not yet consumed"
            )));
        }
    }
    let fill = style.clone_fill();
    match fill.kind {
        SVGPaintKind::None => Ok(None),
        SVGPaintKind::Color(color) => admitted_srgb(style.resolve_color(&color)).map(Some),
        SVGPaintKind::PaintServer(url) => Err(CompileError::UnsupportedFill(
            url.url()
                .map_or_else(|| "url(<invalid>)".to_string(), |url| format!("url({url})")),
        )),
        SVGPaintKind::ContextFill => Err(CompileError::UnsupportedFill("context-fill".to_string())),
        SVGPaintKind::ContextStroke => {
            Err(CompileError::UnsupportedFill("context-stroke".to_string()))
        }
    }
}

/// Admit a cascaded absolute color only where its fidelity is gated: the
/// opaque sRGB values the Chromium-baked primitive suite covers. Any other
/// color space would pass through an unverified conversion and per-channel
/// clamp, and a translucent fill would ship unverified compositing — both
/// refuse explicitly until their own capability steps bake fixtures.
fn admitted_srgb(color: AbsoluteColor) -> Result<CGColor, CompileError> {
    if color.color_space != ColorSpace::Srgb {
        return Err(CompileError::UnsupportedFill(format!(
            "color space {:?} is not yet gated against Chromium",
            color.color_space
        )));
    }
    if color.alpha != 1.0 {
        return Err(CompileError::UnsupportedFill(format!(
            "translucent fill (alpha {}) is not yet gated against Chromium",
            color.alpha
        )));
    }
    let c = color.raw_components();
    Ok(CGColor::from_rgb(to_u8(c[0]), to_u8(c[1]), to_u8(c[2])))
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
fn get_attr(element: HtmlElement<'_>, name: &str) -> Option<String> {
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

/// Whether every `.` in the token is followed by an ASCII digit. Rust's
/// float grammar is a superset of the SVG/CSS number grammar: a trailing
/// dot (`32.`, `3.e2`) parses as f32 but is an invalid number token to
/// Chromium, whose invalid attribute resolves to the property's initial
/// value — a different geometry than a parsed `32`. The other
/// Rust-accepted finite forms (`+3`, `.5`, `1e2`, `1E+2`) are valid SVG
/// numbers.
fn dots_carry_digits(value: &str) -> bool {
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
fn trim_svg_whitespace(value: &str) -> &str {
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
