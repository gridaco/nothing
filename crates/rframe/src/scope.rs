//! The compositing scope: the group fact of the resolved contract.
//!
//! Opacity, blend, and filter scopes state that the items they enclose
//! composite as **one isolated group** — their effect applies to the group's
//! composite, never per item. A clip scope instead constrains paint coverage
//! without isolation. A fact a producer *could* state on one paint pass is
//! stated there instead. [`crate::PaintAlphaFactor`], for
//! example, modulates each paint entry without isolation after its own alpha
//! materializes. A scope is the byte-distinct fact that no such per-paint
//! statement can express — a translucent group whose contents overlap, or a
//! fill and its stroke composited together.
//!
//! What a scope refuses is what this crate refuses: an effect that retains an
//! unresolved lookup or external handle stays a producer refusal by name.
//! Geometric clipping is carried as resolved path coverage. Image masking uses
//! its own checked two-phase [`crate::Mask`] contract. Image filtering enters
//! only as a fully resolved, bounded [`crate::FilterProgram`]; authored lookup
//! and names never cross the contract.

use crate::clip::ClipPath;
use crate::filter::Filter;
use crate::frame::VisualRef;
use math2::Rectangle;
use math2::transform::AffineTransform;

/// Why an opacity cannot be a scope fact.
///
/// A scope opacity lives in the **open** unit interval: for an opacity-only
/// effect, `1` is identity (the producer omits the scope) and `0` composites
/// nothing (the producer emits nothing). A [`ScopeBlend`] expresses unit
/// opacity with `None` while retaining its isolation boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeOpacityError {
    pub value: f32,
}

impl std::fmt::Display for ScopeOpacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "scope opacity {} is outside the open unit interval",
            self.value
        )
    }
}

impl std::error::Error for ScopeOpacityError {}

/// A checked group opacity factor: finite and strictly between 0 and 1.
///
/// This is numeric only. The ordinary isolated group is [`ScopeOpacityGroup`];
/// a [`ScopeBlend`] reuses this factor but owns its own source declaration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeOpacity(f32);

impl ScopeOpacity {
    pub fn new(value: f32) -> Result<Self, ScopeOpacityError> {
        if value.is_finite() && value > 0.0 && value < 1.0 {
            Ok(Self(value))
        } else {
            Err(ScopeOpacityError { value })
        }
    }

    pub fn get(self) -> f32 {
        self.0
    }
}

/// The admitted blend functions for an isolated group's final composition.
///
/// These operate on unpremultiplied group and backdrop color channels; alpha
/// uses source-over for all three modes. This vocabulary is deliberately
/// narrower than [`cg::BlendMode`] and separate from [`crate::FilterBlend`]:
/// it admits neither arbitrary leaf blends nor two-image filter operations.
/// Additive composition and backdrop-preserving groups are inexpressible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeBlendMode {
    /// Use the completed group's color unchanged: `B(b, s) = s`.
    Normal,
    /// Multiply backdrop and completed-group color: `B(b, s) = b * s`.
    Multiply,
    /// Complement the product of complements: `B(b, s) = b + s - b * s`.
    Screen,
}

/// Why a complete isolated-source domain cannot cross the resolved contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsolatedSourceDomainError {
    /// The local rectangle is non-finite, empty, or has unrepresentable endpoints.
    InvalidRectangle,
    /// The map is non-finite or has no supported finite inverse.
    InvalidTransform,
    /// Mapped corners or their enclosing rectangle are non-finite or collapse.
    InvalidMappedBounds,
}

impl std::fmt::Display for IsolatedSourceDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidRectangle => {
                "an isolated-source domain must have finite local bounds with positive extents and ordered endpoints"
            }
            Self::InvalidTransform => {
                "an isolated-source domain map must have finite members and a supported finite inverse"
            }
            Self::InvalidMappedBounds => {
                "an isolated-source domain must map to finite, strictly positive bounds"
            }
        })
    }
}

impl std::error::Error for IsolatedSourceDomainError {}

/// The complete, already-enclosed local domain of one isolated group source.
///
/// The producer has finished discovering contributions and enclosing them in
/// source-local coordinates. Materialize the isolated source over this domain
/// against transparent black, then apply the owning [`ScopeOpacityGroup`]'s
/// final opacity or [`ScopeBlend`]'s combined final opacity and blend.
/// The domain includes the producer's resolved non-painted
/// extent contributions; it is neither a tight geometry box nor a supplemental
/// margin. Reconstructing it from visible paints, enlarging it conservatively,
/// or enclosing contributors after mapping states a different source.
///
/// `source_to_stream` maps the already-enclosed rectangle into its containing
/// [`crate::FrameItems`] coordinates: frame space for a frame, tile-local space
/// for a repeating program. Child node transforms keep their existing meaning;
/// this mapping is not an inherited transform. Each nested isolation boundary owns
/// its own declaration and completed source; it does not donate its descendants
/// as fresh geometry to an enclosing boundary.
///
/// This fact supplies no paint and introduces no geometric clip. Output clipping
/// stays a separate operation. It carries no host view, device grid, allocation,
/// or raster policy. Current-view mapping and device enclosure remain execution
/// work. A consumer unable to honor a declaration must refuse it, not ignore it
/// or replace its map with identity.
///
/// Construction checks numerical usability only. It neither performs enclosure
/// nor proves that the producer's declaration is complete. There is no integer
/// coordinate requirement: the producer has resolved the enclosure in its own
/// source space, whose unit need not be a device pixel. Empty domains are not
/// admitted; absence is represented by [`ScopeOpacityGroup::source_domain`] or
/// [`ScopeBlend::source_domain`] returning `None`, which makes no completeness
/// assertion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IsolatedSourceDomain {
    rect: Rectangle,
    source_to_stream: AffineTransform,
}

impl IsolatedSourceDomain {
    /// Check an already-enclosed rectangle and retain both supplied facts exactly.
    ///
    /// Local endpoints must be finite and strictly ordered in `f32`. The map
    /// must have a finite determinant and an inverse supported by
    /// [`AffineTransform::inverse`] (including its small-determinant refusal);
    /// every inverse member must also be finite. Mapped corners and their
    /// enclosing rectangle must remain finite and strictly positive in `f32`.
    /// These checks make no promise about a later host view.
    pub fn new(
        rect: Rectangle,
        source_to_stream: AffineTransform,
    ) -> Result<Self, IsolatedSourceDomainError> {
        if !valid_domain_rectangle(rect) {
            return Err(IsolatedSourceDomainError::InvalidRectangle);
        }
        let [[a, c, _], [b, d, _]] = source_to_stream.matrix;
        if !source_to_stream
            .matrix
            .into_iter()
            .flatten()
            .all(f32::is_finite)
            || !(a * d - b * c).is_finite()
            || !source_to_stream
                .inverse()
                .is_some_and(|inverse| inverse.matrix.into_iter().flatten().all(f32::is_finite))
        {
            return Err(IsolatedSourceDomainError::InvalidTransform);
        }
        let corners = rect
            .corners()
            .map(|point| math2::vector2::transform(point, &source_to_stream));
        // Check every corner before bounding: min/max can otherwise hide NaN.
        if !corners.into_iter().flatten().all(f32::is_finite)
            || !valid_domain_rectangle(Rectangle::from_points(&corners))
        {
            return Err(IsolatedSourceDomainError::InvalidMappedBounds);
        }
        Ok(Self {
            rect,
            source_to_stream,
        })
    }

    /// The complete already-enclosed rectangle in source-local coordinates.
    #[must_use]
    pub const fn rect(self) -> Rectangle {
        self.rect
    }

    /// The exact source-local to containing-stream map, without a host view.
    #[must_use]
    pub const fn source_to_stream(self) -> AffineTransform {
        self.source_to_stream
    }
}

/// Compatibility name for [`IsolatedSourceDomain`], shared by blend and opacity.
pub use IsolatedSourceDomain as BlendSourceDomain;

/// Compatibility name for [`IsolatedSourceDomainError`].
pub use IsolatedSourceDomainError as BlendSourceDomainError;

fn valid_domain_rectangle(rect: Rectangle) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > 0.0
        && (rect.x + rect.width).is_finite()
        && (rect.y + rect.height).is_finite()
        && rect.x + rect.width > rect.x
        && rect.y + rect.height > rect.y
}

/// One isolated group's final opacity and optional complete source domain.
///
/// Children paint in stream order against transparent black. Apply the checked
/// opacity once to the completed source's premultiplied color and alpha, then
/// composite source-over into the enclosing backdrop. The factor is never
/// distributed over child paints, fills, or strokes.
///
/// An optional [`IsolatedSourceDomain`] declares where that completed source
/// materializes before final opacity. It adds no nested operation, paint, clip,
/// or inherited transform, and cannot make an empty group meaningful. Absence
/// makes no completeness assertion. Numerical checks do not prove completeness;
/// the producer establishes it, and a consumer must honor the declaration or
/// refuse it. Backdrop-preserving opacity and execution policy are inexpressible.
///
/// [`ScopeOpacity`] remains the numeric factor shared with [`ScopeBlend`]. A
/// blend's optional opacity cannot carry a competing source declaration:
///
/// ```compile_fail
/// use rframe::{ScopeBlend, ScopeBlendMode, ScopeOpacity, ScopeOpacityGroup};
/// let group = ScopeOpacityGroup::new(ScopeOpacity::new(0.5).unwrap());
/// let blend = ScopeBlend::new(ScopeBlendMode::Multiply, Some(group));
/// ```
///
/// ```
/// use math2::{Rectangle, transform::AffineTransform};
/// use rframe::{IsolatedSourceDomain, ScopeEffect, ScopeOpacity, ScopeOpacityGroup};
/// let factor = ScopeOpacity::new(0.375)?;
/// let source = IsolatedSourceDomain::new(
///     Rectangle::from_xywh(-2.0, -3.0, 24.0, 20.0),
///     AffineTransform::identity(),
/// )?;
/// let group = ScopeOpacityGroup::new(factor).with_source_domain(source);
/// assert_eq!(group.opacity(), factor);
/// assert_eq!(group.source_domain(), Some(source));
/// let effect = ScopeEffect::Opacity(group);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeOpacityGroup {
    opacity: ScopeOpacity,
    source_domain: Option<IsolatedSourceDomain>,
}

impl ScopeOpacityGroup {
    /// State ordinary isolated opacity with no source completeness assertion.
    #[must_use]
    pub const fn new(opacity: ScopeOpacity) -> Self {
        Self {
            opacity,
            source_domain: None,
        }
    }

    /// Declare the complete source domain without changing the final operation.
    #[must_use]
    pub const fn with_source_domain(mut self, domain: IsolatedSourceDomain) -> Self {
        self.source_domain = Some(domain);
        self
    }

    /// The final factor, applied once to the completed source.
    #[must_use]
    pub const fn opacity(self) -> ScopeOpacity {
        self.opacity
    }

    /// The declared complete source domain, or no completeness assertion.
    #[must_use]
    pub const fn source_domain(self) -> Option<IsolatedSourceDomain> {
        self.source_domain
    }
}

/// One isolated group's combined final blend and optional opacity.
///
/// Children paint in order against transparent black. Their completed
/// composite is the source; the enclosing composite at this position in
/// painter order is the backdrop. Apply the optional opacity to the completed
/// source's premultiplied color and alpha, then blend and composite it over
/// that backdrop using [`ScopeBlendMode`]. Opacity and blend belong to this
/// **one** final operation: splitting them into nested scopes changes the
/// backdrop seen by the blend and can add an intermediate quantization step.
/// Neither operation is distributed over child paints, fills, or strokes.
///
/// `None` means unit opacity, including for [`ScopeBlendMode::Normal`]. Such a
/// scope still isolates descendants and must not be erased just because its
/// final blend and opacity are neutral. Painting children directly into the
/// enclosing backdrop is represented by **no scope**. Zero opacity resolves
/// to no emitted group before construction, as with [`ScopeEffect::Opacity`].
///
/// Non-normal blending depends on the enclosing backdrop even when all
/// enclosed items are unchanged. Equality of this fact and its children does
/// not prove equality of the final blended result across different backdrops.
/// This names visual meaning, never a layer allocation, backdrop copy, cache
/// policy, or authored group.
///
/// An optional [`IsolatedSourceDomain`] states the complete source domain before
/// this final operation. Absence preserves the existing group meaning without
/// asserting completeness. A domain neither creates another scope nor makes an
/// empty group meaningful, and equality of domains does not erase the backdrop
/// dependency.
///
/// ```
/// use rframe::{ScopeBlend, ScopeBlendMode, ScopeOpacity};
///
/// let isolated = ScopeBlend::new(ScopeBlendMode::Normal, None);
/// assert_eq!(isolated.opacity(), None); // unit opacity, still isolated
/// let translucent = ScopeBlend::new(
///     ScopeBlendMode::Multiply,
///     Some(ScopeOpacity::new(0.5)?),
/// );
/// assert_eq!(translucent.mode(), ScopeBlendMode::Multiply);
/// # Ok::<(), rframe::ScopeOpacityError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeBlend {
    mode: ScopeBlendMode,
    opacity: Option<ScopeOpacity>,
    source_domain: Option<IsolatedSourceDomain>,
}

impl ScopeBlend {
    /// Combine an admitted blend function and already-checked opacity.
    /// `None` means opacity 1; it never means absence of isolation.
    #[must_use]
    pub const fn new(mode: ScopeBlendMode, opacity: Option<ScopeOpacity>) -> Self {
        Self {
            mode,
            opacity,
            source_domain: None,
        }
    }

    /// Declare the complete source domain without changing the final operation.
    #[must_use]
    pub const fn with_source_domain(mut self, domain: IsolatedSourceDomain) -> Self {
        self.source_domain = Some(domain);
        self
    }

    /// The declared complete source domain, or no completeness assertion.
    #[must_use]
    pub const fn source_domain(self) -> Option<IsolatedSourceDomain> {
        self.source_domain
    }

    /// The blend function used only when the completed group joins its backdrop.
    #[must_use]
    pub const fn mode(self) -> ScopeBlendMode {
        self.mode
    }

    /// The final group opacity, or `None` for unit opacity.
    #[must_use]
    pub const fn opacity(self) -> Option<ScopeOpacity> {
        self.opacity
    }
}

/// The compositing effect a scope applies to its group's composite.
#[derive(Clone, Debug, PartialEq)]
pub enum ScopeEffect {
    /// Materialize one isolated source, optionally over its declared complete
    /// domain, then apply final opacity once and composite source-over.
    Opacity(ScopeOpacityGroup),
    /// Isolate the group, then blend its completed composite with the enclosing
    /// backdrop at the optional opacity in one final operation. Normal blend
    /// at unit opacity still retains this scope's isolation boundary.
    Blend(ScopeBlend),
    /// Intersect every enclosed paint with resolved geometric coverage.
    /// Unlike opacity this creates no isolated layer: the clip is paint state,
    /// and its path facts reference no source or external resource.
    Clip(ClipPath),
    /// Apply one resolved image-filter program to the isolated group, hard
    /// clipped to its resolved effect region.
    Filter(Filter),
}

/// One compositing scope: its owner and its effect.
///
/// The owner is the source construct whose compositing the scope states
/// (a group element, a shape whose fill and stroke composite together) —
/// the same opaque identity/provenance a node carries, because damage and
/// diagnostics need to name a scope exactly as they name a node.
#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    pub owner: VisualRef,
    pub effect: ScopeEffect,
}
