//! Resolved image masking — one source-neutral two-phase composite.
//!
//! A mask has two painter-ordered phases. The target phase composites the
//! masked content into an isolated group. The source phase paints ordinary
//! resolved frame nodes into a second isolated group, clips that group to one
//! resolved geometric region, converts it to alpha when required, and
//! multiplies it into the target group.
//!
//! The URL lookup, source element, coordinate-system attributes, and source
//! grammar are producer work. None crosses this contract. [`Mask`] carries
//! only the eventual alpha/luminance choice and resolved geometric region;
//! the source image itself remains ordinary [`crate::FrameItem`] paint between
//! the checked stream's mask-source and mask-end markers.

use crate::clip::ClipPath;
use crate::frame::VisualRef;

/// How the resolved mask-source composite becomes the target alpha multiplier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskMode {
    /// Use the source composite's alpha channel directly.
    Alpha,
    /// Convert unpremultiplied source color to luminance, then multiply by its
    /// alpha channel.
    Luminance,
}

/// One resolved mask effect and its opaque diagnostic owner.
#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub owner: VisualRef,
    mode: MaskMode,
    region: ClipPath,
}

impl Mask {
    #[must_use]
    pub const fn new(owner: VisualRef, mode: MaskMode, region: ClipPath) -> Self {
        Self {
            owner,
            mode,
            region,
        }
    }

    #[must_use]
    pub const fn mode(&self) -> MaskMode {
        self.mode
    }

    /// The transformed mask painting area as one checked geometric clip.
    #[must_use]
    pub const fn region(&self) -> &ClipPath {
        &self.region
    }
}
