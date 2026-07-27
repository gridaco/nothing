//! Narrow source-native styled-DOM compatibility seam.
//!
//! This module exposes only the parse/cascade operation and the per-element
//! resolved-style projection needed across the transitional crate boundary.
//! It is not the renderer's source-neutral contract.

pub use crate::collect::styled_of;
pub use crate::frontend::parse_and_style;
