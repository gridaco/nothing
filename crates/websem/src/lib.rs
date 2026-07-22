//! `websem` — the Web-first track's Web semantic front-end.
//!
//! PROVISIONAL · INTERNAL · BREAKABLE. Compiles the Web semantic family —
//! HTML-with-inline-SVG and standalone SVG — into the source-neutral
//! [`rframe::Frame`], through **one** namespace-aware document
//! (`csscascade::DemoDom`) and **one** browser-grade cascade
//! (`csscascade::CascadeDriver`, Stylo). Inline SVG is compiled in place from
//! the shared document — never serialized and reparsed — and its descendant
//! style comes from the surrounding cascade. The current standalone function
//! parses a bare `<svg>` through html5ever foreign-content handling; it is a
//! scaffold, **not** the conforming SVG/XML grammar entry the amendment
//! requires before capability work.
//!
//! This crate produces the contract; it does not paint. It touches no legacy
//! import path, no node model, no `.grida` codec, and no backend
//! (`tests/architecture.rs` locks these out). See the
//! [Web-First Amendment](../../../docs/wg/consolidation/web-first.md).

pub mod svg;

pub use svg::{CompileError, compile_html_inline_svg, compile_standalone_svg};
