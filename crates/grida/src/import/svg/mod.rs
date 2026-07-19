//! SVG → Grida import pipeline.
//!
//! The subsystem's **product is the IR**: [`SVGPackedScene`], built from
//! usvg by [`packed_scene`]/[`from_usvg`] out of the spec-faithful value
//! types in [`crate::cg::svg`]. The v1 node model is one *consumer* of
//! that IR: [`pack`] (scene-graph adapter) and [`grida`] (`.grida` FBS
//! bytes) are the sink side of the seam, and `paint` is their
//! IR→runtime-paint projection. Nothing on the IR side may import the
//! node model — enforced by `tests/svg_import_architecture.rs` (the SVG
//! sink inversion, gridaco/nothing#29).
//!
//! Pure SVG-string transformations (sanitize, optimize, parse) live in
//! [`crate::formats::svg`].

pub mod from_usvg;
pub mod grida;
pub mod pack;
pub mod packed_scene;
mod paint;

pub use packed_scene::SVGPackedScene;
