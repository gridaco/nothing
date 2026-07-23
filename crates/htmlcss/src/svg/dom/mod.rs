//! SVG DOM view over `csscascade::DemoDom`.
//!
//! The compatibility renderer reuses the same arena storage type as the HTML
//! front-end, but not the same document session or cascade. Standalone SVG and
//! serialized inline-SVG subtrees are parsed in a separate invocation using
//! html5ever foreign-content handling, then styled by the temporary SVG
//! matcher. This is not yet a conforming SVG/XML grammar entry.
//!
//! This subdirectory exposes thin helpers for navigating that tree as SVG:
//! - [`parser`]: parse SVG bytes into a `DemoDom` and locate the `<svg>`
//!   root.
//! - [`element`]: tag-kind dispatch, attribute lookup, child iteration.
//! - [`attrs`]: parsers for `length`, `color`, `viewBox`, `transform`,
//!   `points`, etc.
//! - [`href`], [`path_d`]: URL/fragment reference and path-data helpers.
//!
//! Blink anchor: `core/svg/svg_*_element.{h,cc}`. Blink's shared document and
//! cascade remain the target topology, not a property of this compatibility
//! module.

pub mod attrs;
pub mod element;
pub mod href;
pub mod parser;
pub mod path_d;
