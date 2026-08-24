//! Contract tests for resolved image-filter programs.

use std::sync::Arc;

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    Filter, FilterColorSpace, FilterError, FilterInput, FilterNode, FilterPrimitive, FilterProgram,
    FilterProgramError, MAX_FILTER_NODES,
};

fn blur(input: FilterInput, sigma_x: f32, sigma_y: f32) -> FilterNode {
    FilterNode::new(
        input,
        Rectangle::from_xywh(-4.0, -4.0, 48.0, 48.0),
        FilterColorSpace::LinearRgb,
        FilterPrimitive::GaussianBlur { sigma_x, sigma_y },
    )
}

#[test]
fn a_bounded_earlier_node_graph_is_admitted() {
    let program = FilterProgram::new(Arc::from([
        blur(FilterInput::Source, 2.0, 0.0),
        blur(FilterInput::Node(0), 0.0, 3.0),
        blur(FilterInput::SourceAlpha, 1.0, 1.0),
    ]))
    .expect("all node inputs are resolved and acyclic");
    assert_eq!(program.iter().count(), 3);

    let filter = Filter::new(
        AffineTransform::identity(),
        Rectangle::from_xywh(-4.0, -4.0, 48.0, 48.0),
        program,
    )
    .expect("finite operation space and positive region are a filter fact");
    assert_eq!(filter.program().iter().count(), 3);
}

#[test]
fn empty_and_over_bound_programs_are_refused() {
    assert_eq!(
        FilterProgram::new(Arc::from([])),
        Err(FilterProgramError::Empty)
    );
    assert_eq!(
        FilterProgram::new(Arc::from(
            vec![blur(FilterInput::Source, 0.0, 0.0); MAX_FILTER_NODES + 1].into_boxed_slice()
        )),
        Err(FilterProgramError::TooManyNodes)
    );
}

#[test]
fn every_node_reference_must_point_backward() {
    assert_eq!(
        FilterProgram::new(Arc::from([blur(FilterInput::Node(0), 2.0, 2.0)])),
        Err(FilterProgramError::InputIsNotEarlier { node: 0, input: 0 })
    );
    assert_eq!(
        FilterProgram::new(Arc::from([
            blur(FilterInput::Source, 2.0, 2.0),
            blur(FilterInput::Node(2), 2.0, 2.0),
            blur(FilterInput::Node(0), 2.0, 2.0),
        ])),
        Err(FilterProgramError::InputIsNotEarlier { node: 1, input: 2 })
    );
}

#[test]
fn invalid_regions_and_blur_scalars_never_cross_the_contract() {
    let bad_region = FilterNode::new(
        FilterInput::Source,
        Rectangle::from_xywh(0.0, 0.0, 0.0, 10.0),
        FilterColorSpace::Srgb,
        FilterPrimitive::GaussianBlur {
            sigma_x: 1.0,
            sigma_y: 1.0,
        },
    );
    assert_eq!(
        FilterProgram::new(Arc::from([bad_region])),
        Err(FilterProgramError::InvalidRegion { node: 0 })
    );
    for (sigma_x, sigma_y) in [(-1.0, 0.0), (0.0, f32::INFINITY), (f32::NAN, 1.0)] {
        assert_eq!(
            FilterProgram::new(Arc::from([blur(FilterInput::Source, sigma_x, sigma_y)])),
            Err(FilterProgramError::InvalidGaussianBlur { node: 0 })
        );
    }
}

#[test]
fn the_effect_rejects_an_invalid_operation_space_or_outer_region() {
    let program = || {
        FilterProgram::new(Arc::from([blur(FilterInput::Source, 2.0, 2.0)])).expect("valid program")
    };
    assert_eq!(
        Filter::new(
            AffineTransform::from_acebdf(1.0, 0.0, f32::INFINITY, 0.0, 1.0, 0.0),
            Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0),
            program(),
        ),
        Err(FilterError::InvalidTransform)
    );
    assert_eq!(
        Filter::new(
            AffineTransform::identity(),
            Rectangle::from_xywh(0.0, 0.0, 10.0, -1.0),
            program(),
        ),
        Err(FilterError::InvalidRegion)
    );
}
