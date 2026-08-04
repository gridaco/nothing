//! `textlayout` — PROVISIONAL · INTERNAL · BREAKABLE.
//!
//! The Web family's **text resolution oracle** — the producer-side
//! implementation of the single text-resolution contract
//! ([`docs/wg/feat-paragraph/text-layout.md`]), at its smallest honest
//! profile:
//!
//! ```text
//! attributed text + explicit font environment
//!     -> resolved text layout | typed resolution failure     (oracle v0)
//! ```
//!
//! **Oracle v0 resolves one style run of printable-ASCII, horizontal,
//! left-to-right text with no wrapping, no fallback, and no synthesis.**
//! The repertoire is an explicit admit-list enforced by the resolver
//! itself — never an accident of a font's coverage — and everything outside
//! the profile is a typed refusal, not an approximation. Coverage grows by
//! oracle version, exactly as the RFD prescribes.
//!
//! This crate is the *Web family's* producer, not an engine-wide text
//! service: whether the n0 family shares a text artifact with it is the open
//! D-M shaped-text stage ([`docs/wg/consolidation/n0-join-point.md`]), and
//! building this crate as the anointed shared resolver would decide that
//! join by construction. The Web producer depends on this crate; nothing
//! here depends back.
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
mod resolve;

pub use artifact::{
    BoundsBox, LineMetrics, OutlineSink, PlacedGlyph, ResolvedFace, ResolvedTextLayout,
};
pub use environment::{Environment, FontKey, FontResource};
pub use resolve::{AttributedText, ResolveError, Style, resolve};

/// The complete geometry-producing policy this crate currently implements.
///
/// Any change that can alter a glyph choice, position, metric, or bound —
/// including a shaping-dependency upgrade that changes output, which is why
/// the manifest pins the shaper exactly — requires a new version. A resolved
/// artifact records the version that produced it; "latest" is not a durable
/// identity.
pub const ORACLE_VERSION: &str = "textlayout-v0";
