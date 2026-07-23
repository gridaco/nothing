//! The **private** drawlist — rframe's own compiled form between the resolved
//! [`Frame`](crate::frame::Frame) contract and the painter.
//!
//! Per the settled display-list ruling, a drawlist is per-engine and never a
//! contract. This one is deliberately tiny (the ops the first slice needs) and
//! Skia-free (enforced by `tests/architecture.rs`); it exists so the resolved
//! contract stays a *description* and the painter stays a pure executor.

use math2::Rectangle;
use math2::transform::AffineTransform;

use cg::CGColor;

use crate::frame::{Frame, Geometry};

/// One drawing operation, in frame space, in paint order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DrawItem {
    /// Push a clip to the given frame-space rectangle for subsequent items,
    /// until the matching [`DrawItem::Restore`].
    ClipRect(Rectangle),
    /// Fill a local-space rectangle, transformed into frame space, with a
    /// solid color.
    FillRect {
        rect: Rectangle,
        transform: AffineTransform,
        color: CGColor,
    },
    /// Pop the most recent clip.
    Restore,
}

/// An ordered list of drawing operations.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DrawList {
    pub(crate) items: Vec<DrawItem>,
}

/// Lower a resolved [`Frame`] into a private [`DrawList`].
///
/// The frame's viewport becomes a top-level clip; each node's paint stack is
/// emitted bottom-first. Every construct the contract can express and the
/// drawlist cannot yet lower is an explicit `panic!` at the boundary — never a
/// silent shim (there are none for the first slice).
pub(crate) fn build(frame: &Frame) -> DrawList {
    let mut items = Vec::new();
    items.push(DrawItem::ClipRect(frame.bounds));

    for node in &frame.nodes {
        for solid in node.paints.iter() {
            match &node.geometry {
                Geometry::Rect(rect) => {
                    items.push(DrawItem::FillRect {
                        rect: *rect,
                        transform: node.transform,
                        color: solid.color,
                    });
                }
            }
        }
    }

    items.push(DrawItem::Restore);
    DrawList { items }
}
