//! `textlayout` — PROVISIONAL · INTERNAL · BREAKABLE.
//!
//! The Web family's **text resolution oracle** — the producer-side
//! implementation of the single text-resolution contract
//! ([`docs/wg/feat-paragraph/text-layout.md`]), at its smallest honest
//! profile:
//!
//! ```text
//! attributed text + explicit font environment
//!     -> resolved text layout | typed resolution failure     (oracle v8)
//! ```
//!
//! **Oracle v8 resolves one layout-affecting style over printable-ASCII plus
//! the canonical precomposed Latin-1 letters whose decomposition is one ASCII
//! Latin base and one combining mark, plus one U+0301 or U+030B after an ASCII
//! Latin base. Complete source-run coverage may carry opaque caller tags;
//! those boundaries do not split shaping, and every cluster and glyph keeps
//! the tag covering the cluster's first source scalar. Explicit complete
//! shaping chunks may split the source into independent shaping operations;
//! their resolved records retain global source/cluster/glyph ranges, a
//! canonical local origin, and local advance. It is horizontal and
//! left-to-right; a cluster is either one direct scalar/glyph or one
//! base-plus-mark composing to one glyph or attaching one zero-advance glyph
//! with explicit x/y offsets. There is no wrapping, authored placement,
//! anchoring or synthesis. For each admitted direct or base-plus-mark cluster,
//! its ordered font-family request selects the first named family whose one
//! statically matched face shapes that complete cluster. Canonical composition
//! therefore stays in an earlier face while a genuinely missing mark moves
//! with its base. Adjacent clusters selecting one resource shape as one face
//! run; the artifact records a primary metrics face, every used face/run, and
//! each glyph's exact face identity. Static face matching remains the measured
//! CSS Fonts lexicographic stretch/style/weight search; another face in the
//! same family is never a glyph fallback. A reached generic, winning-tuple
//! tie, required synthesis, exhausted list, or partial-cluster-only result is
//! a typed refusal.**
//! The repertoire is an explicit admit-list enforced by the resolver
//! itself — never an accident of a font's coverage — and everything outside
//! the profile is a typed refusal, not an approximation. Coverage grows by
//! oracle version, exactly as the RFD prescribes.
//!
//! This crate is the *Web family's* producer, not an engine-wide text
//! service — by decision, not by accident of youth: the D-M shaped-text
//! stage was taken **low** on the two-producer spike
//! ([`docs/wg/consolidation/n0-join-point.md`]), so the n0 family keeps its
//! own text artifact and font registry, and this crate never becomes the
//! anointed shared resolver. The Web producer depends on this crate; nothing
//! here depends back (n0 consumes it as a dev-dependency for the spike's
//! evidence alone, and locks it out of its shipping graph).
//!
//! A module's identity is what it refuses. This one:
//!
//! - **owns no font discovery.** The environment is a manifest of exact
//!   bytes handed in by the caller; there is no ambient font database, no
//!   file read, no network. (`tests/architecture.rs` forbids the escape
//!   hatches.)
//! - **owns no render contract.** No resolved-frame types, no paint, no
//!   backend. Glyph geometry leaves through [`OutlineSink`] in the node's
//!   local y-down space, and a consumer lowers it into its own vocabulary.
//! - **owns no clock and no estimate.** Resolution is a pure function of its
//!   declared inputs; a width guessed before shaping is a different answer,
//!   so no such guess exists here.
//!
//! [`docs/wg/feat-paragraph/text-layout.md`]: ../../../docs/wg/feat-paragraph/text-layout.md
//! [`docs/wg/consolidation/n0-join-point.md`]: ../../../docs/wg/consolidation/n0-join-point.md

mod artifact;
mod environment;
mod face_descriptor;
mod resolve;
mod source;

pub use artifact::{
    BoundsBox, LineMetrics, OutlineSink, PlacedGlyph, ResolvedFace, ResolvedFaceRun,
    ResolvedShapingChunk, ResolvedTextLayout, ShapingCluster,
};
pub use environment::{Environment, FontKey, FontResource};
pub use face_descriptor::{
    FontStretch, FontStyle, FontWeight, InvalidFontWeight, StaticFaceDescriptor,
};
pub use resolve::{AttributedText, FontFamily, ResolveError, Style, resolve};
pub use source::{
    ShapingChunk, ShapingChunkCoverageError, SourceRun, SourceRunCoverageError, SourceRunTag,
};

/// The complete geometry-producing policy this crate currently implements.
///
/// Any change that can alter a glyph choice, position, metric, or bound —
/// including a shaping-dependency upgrade that changes output, which is why
/// the manifest pins the shaper exactly — requires a new version. A resolved
/// artifact records the version that produced it; "latest" is not a durable
/// identity.
///
/// Opaque source-run tags remain metadata. Explicit shaping chunks are
/// geometry-producing policy: each chunk is shaped independently, which can
/// change glyph positioning at its boundaries; those records advanced v3 to
/// v4. Ordered declared-family selection can change the exact face and glyph
/// identity, so its pinned BMP simple-fold policy advanced v4 to v5. Exact
/// static same-family descriptor selection can change face and glyph identity,
/// so it advances v5 to v6. Directional nearest-face selection can turn a v6
/// exact miss into a concrete face and therefore advances v6 to v7; synthetic
/// realization remains a typed boundary and does not enter the artifact.
/// Cluster-safe fallback changes both shaping segmentation and glyph identity,
/// so the primary/per-glyph multi-face artifact advances v7 to v8.
pub const ORACLE_VERSION: &str = "textlayout-v8";
