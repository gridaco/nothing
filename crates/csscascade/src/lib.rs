//! csscascade — CSS Cascade & Style Resolution Engine
//!
//! Takes an HTML DOM tree and produces fully resolved computed styles for every
//! element, powered by Servo's Stylo engine.
//!
//! # Quick start
//!
//! ```ignore
//! use csscascade::{
//!     adapter::DocumentSession,
//!     cascade::CascadeDriver,
//!     dom::DemoDom,
//! };
//! use style::thread_state::{self, ThreadState};
//!
//! thread_state::initialize(ThreadState::LAYOUT);
//!
//! let dom = DemoDom::parse_from_bytes(html.as_bytes()).unwrap();
//! let mut session = DocumentSession::new(dom);
//! CascadeDriver::new(&mut session).style_document();
//! // Every element now carries computed styles via element.borrow_data()
//! ```

pub mod adapter;
pub mod cascade;
pub mod dom;
mod svg_transform;
pub mod svg_use;

use style::servo::media_features::PointerCapabilities;

/// Interaction-media profile declared by the current static renderer.
///
/// This must not use [`PointerCapabilities::default`]: upstream chooses that
/// value from the compilation target, which would make identical source
/// cascade differently across hosts. A future host-selectable profile should
/// replace this function as one explicit cascade-environment input.
pub(crate) fn static_desktop_pointer_capabilities() -> PointerCapabilities {
    PointerCapabilities::FINE | PointerCapabilities::HOVER
}
