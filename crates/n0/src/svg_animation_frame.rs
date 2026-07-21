//! Thin explicit Base/Sample adapter from the bounded retained SVG-animation
//! frontend to the ordinary n0 frame seam.
//!
//! This module does not make n0 a general SVG importer. [`compile_latest`]
//! deliberately follows the moving latest cumulative proving profile, while
//! [`render_base`] preserves authored state and [`render_sample`] requires one
//! caller-supplied [`SampleTime`]. The caller owns the canvas, clear color,
//! allocation, cadence, I/O, and encoding.

use std::sync::Arc;

use n0_model::animation::SampleTime;
use n0_model::math::Affine;
use n0_model::model::NodeId;
use n0_model::resolve::{Report, ResolveOptions};
use n0_model::svg_animation::{
    CompiledSvgAnimation, SourceSnapshot, SvgAnimationError, SvgAnimationSource,
};

use crate::frame::{self, FrameExecutionError, FrameProduct, FrameRequest, FrameRequestError};
use crate::paint::PaintCtx;

/// Parse and compile one retained source with the newest cumulative proving
/// profile.
///
/// Versioned profile methods remain the conformance entries. This moving
/// pointer exists for diagnostic hosts that deliberately follow the complete
/// currently accepted profile.
pub fn compile_latest(
    identity: impl Into<Arc<str>>,
    source: impl Into<Arc<str>>,
) -> Result<CompiledSvgAnimation, SvgAnimationError> {
    SvgAnimationSource::parse(SourceSnapshot::new(identity, source))?.into_compiled_latest()
}

/// One resolver outcome that a strict frame host may not silently paint.
/// Clamp reports remain valid resolved output and are not represented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedIntent {
    IgnoredByRule {
        node: NodeId,
        field: &'static str,
        rule: &'static str,
    },
    ErrorByRule {
        node: NodeId,
        field: &'static str,
        rule: &'static str,
    },
}

impl UnresolvedIntent {
    fn parts(self) -> (NodeId, &'static str, &'static str) {
        match self {
            Self::IgnoredByRule { node, field, rule } | Self::ErrorByRule { node, field, rule } => {
                (node, field, rule)
            }
        }
    }
}

impl std::fmt::Display for UnresolvedIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (node, field, rule) = self.parts();
        write!(
            f,
            "node {node} could not resolve `{field}` while rendering animation: {rule}"
        )
    }
}

impl std::error::Error for UnresolvedIntent {}

/// Failure to construct, validate, or execute one bounded profile frame.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderError {
    Frame(FrameRequestError),
    UnresolvedIntent(UnresolvedIntent),
    Execution(FrameExecutionError),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Frame(error) => error.fmt(f),
            Self::UnresolvedIntent(error) => error.fmt(f),
            Self::Execution(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Frame(error) => Some(error),
            Self::UnresolvedIntent(error) => Some(error),
            Self::Execution(error) => Some(error),
        }
    }
}

impl From<FrameRequestError> for RenderError {
    fn from(error: FrameRequestError) -> Self {
        Self::Frame(error)
    }
}

impl From<UnresolvedIntent> for RenderError {
    fn from(error: UnresolvedIntent) -> Self {
        Self::UnresolvedIntent(error)
    }
}

impl From<FrameExecutionError> for RenderError {
    fn from(error: FrameExecutionError) -> Self {
        Self::Execution(error)
    }
}

fn first_unresolved_intent(reports: &[Report]) -> Option<UnresolvedIntent> {
    reports.iter().find_map(|report| match *report {
        Report::IgnoredByRule { node, field, rule } => {
            Some(UnresolvedIntent::IgnoredByRule { node, field, rule })
        }
        Report::ErrorByRule { node, field, rule } => {
            Some(UnresolvedIntent::ErrorByRule { node, field, rule })
        }
        Report::Clamped { .. } => None,
    })
}

/// Render the authored Base frame from a retained, already compiled source.
///
/// This entry constructs [`FrameRequest::Base`] directly; it does not sample
/// the compiled animation program at zero or at any other implicit time.
pub fn render_base(
    canvas: &skia_safe::Canvas,
    compiled: &CompiledSvgAnimation,
    ctx: &PaintCtx,
) -> Result<FrameProduct, RenderError> {
    render_request(canvas, compiled, FrameRequest::Base, ctx)
}

/// Render one exact-time Sample frame from a retained, already compiled source.
///
/// The required [`SampleTime`] is the complete semantic time input. No Base
/// fallback, zero-time default, or ambient clock is available through this
/// entry.
pub fn render_sample(
    canvas: &skia_safe::Canvas,
    compiled: &CompiledSvgAnimation,
    time: SampleTime,
    ctx: &PaintCtx,
) -> Result<FrameProduct, RenderError> {
    render_request(
        canvas,
        compiled,
        FrameRequest::Sample {
            program: compiled.animation(),
            time,
        },
        ctx,
    )
}

/// Both public policies converge only after the caller has made the explicit
/// Base-or-Sample choice. Frame construction and resolver-report validation
/// complete before checked identity-view execution touches `canvas`.
fn render_request(
    canvas: &skia_safe::Canvas,
    compiled: &CompiledSvgAnimation,
    request: FrameRequest<'_>,
    ctx: &PaintCtx,
) -> Result<FrameProduct, RenderError> {
    let options = ResolveOptions {
        viewport: compiled.viewport(),
        ..Default::default()
    };
    let product = frame::resolve_and_build_request(compiled.document(), request, &options, ctx)?;
    if let Some(error) = first_unresolved_intent(&product.resolved().reports) {
        return Err(error.into());
    }
    product.execute(canvas, &Affine::IDENTITY, ctx)?;
    Ok(product)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_resolution_accepts_clamps_and_rejects_the_first_unresolved_intent() {
        let reports = [
            Report::Clamped {
                node: 1,
                field: "width",
                from: -1.0,
                to: 0.0,
            },
            Report::IgnoredByRule {
                node: 2,
                field: "x/y",
                rule: "layout owns position",
            },
            Report::ErrorByRule {
                node: 3,
                field: "width",
                rule: "no natural size",
            },
        ];
        assert_eq!(
            first_unresolved_intent(&reports),
            Some(UnresolvedIntent::IgnoredByRule {
                node: 2,
                field: "x/y",
                rule: "layout owns position",
            })
        );
        assert_eq!(first_unresolved_intent(&reports[..1]), None);
    }

    #[test]
    fn strict_resolution_preserves_error_by_rule_as_structured_data() {
        let reports = [Report::ErrorByRule {
            node: 7,
            field: "path",
            rule: "coordinates are not finite",
        }];
        let error = first_unresolved_intent(&reports).unwrap();
        assert_eq!(
            error,
            UnresolvedIntent::ErrorByRule {
                node: 7,
                field: "path",
                rule: "coordinates are not finite",
            }
        );
        assert_eq!(
            error.to_string(),
            "node 7 could not resolve `path` while rendering animation: coordinates are not finite"
        );
    }
}
