//! Contract tests for resolved image-filter programs.

use std::sync::Arc;

use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    Filter, FilterBlend, FilterChannelTables, FilterColorSpace, FilterComposite,
    FilterConvolveEdgeMode, FilterDisplacementChannel, FilterError, FilterInput, FilterLightSource,
    FilterMorphology, FilterNode, FilterPrimitive, FilterProgram, FilterProgramError,
    FilterTurbulenceKind, MAX_FILTER_CONVOLVE_KERNEL_VALUES, MAX_FILTER_NODES,
};

fn blur(input: FilterInput, sigma_x: f32, sigma_y: f32) -> FilterNode {
    FilterNode::new(
        Arc::from([input]),
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
        Arc::from([FilterInput::Source]),
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
fn operation_arities_and_all_input_edges_are_checked() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let bad_composite = FilterNode::new(
        Arc::from([FilterInput::Source]),
        region,
        FilterColorSpace::Srgb,
        FilterPrimitive::Composite {
            operator: FilterComposite::Over,
        },
    );
    assert_eq!(
        FilterProgram::new(Arc::from([bad_composite])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 2,
            actual: 1,
        })
    );

    let source = FilterNode::new(
        Arc::from([]),
        region,
        FilterColorSpace::Srgb,
        FilterPrimitive::SolidColor {
            color: cg::CGColor32F::from_rgba8(cg::CGColor::RED),
        },
    );
    let bad_merge = FilterNode::new(
        Arc::from([FilterInput::Node(0), FilterInput::Node(2)]),
        region,
        FilterColorSpace::Srgb,
        FilterPrimitive::Merge,
    );
    assert_eq!(
        FilterProgram::new(Arc::from([source, bad_merge])),
        Err(FilterProgramError::InputIsNotEarlier { node: 1, input: 2 })
    );
}

#[test]
fn blend_has_two_ordered_inputs_and_the_closed_mode_vocabulary() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let modes = [
        FilterBlend::Normal,
        FilterBlend::Multiply,
        FilterBlend::Screen,
        FilterBlend::Overlay,
        FilterBlend::Darken,
        FilterBlend::Lighten,
        FilterBlend::ColorDodge,
        FilterBlend::ColorBurn,
        FilterBlend::HardLight,
        FilterBlend::SoftLight,
        FilterBlend::Difference,
        FilterBlend::Exclusion,
        FilterBlend::Hue,
        FilterBlend::Saturation,
        FilterBlend::Color,
        FilterBlend::Luminosity,
    ];
    assert_eq!(modes.len(), 16);

    for mode in modes {
        let blend = |inputs| {
            FilterNode::new(
                inputs,
                region,
                FilterColorSpace::Srgb,
                FilterPrimitive::Blend { mode },
            )
        };
        assert_eq!(
            FilterProgram::new(Arc::from([blend(Arc::from([FilterInput::Source]))])),
            Err(FilterProgramError::InvalidInputCount {
                node: 0,
                expected: 2,
                actual: 1,
            })
        );
        FilterProgram::new(Arc::from([blend(Arc::from([
            FilterInput::Source,
            FilterInput::SourceAlpha,
        ]))]))
        .expect("foreground and backdrop are a checked two-input blend");
    }
}

#[test]
fn offset_and_arithmetic_scalars_must_be_finite() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let offset = FilterNode::new(
        Arc::from([FilterInput::Source]),
        region,
        FilterColorSpace::Srgb,
        FilterPrimitive::Offset {
            dx: f32::INFINITY,
            dy: 0.0,
        },
    );
    assert_eq!(
        FilterProgram::new(Arc::from([offset])),
        Err(FilterProgramError::InvalidOffset { node: 0 })
    );

    let composite = FilterNode::new(
        Arc::from([FilterInput::Source, FilterInput::SourceAlpha]),
        region,
        FilterColorSpace::Srgb,
        FilterPrimitive::Composite {
            operator: FilterComposite::Arithmetic {
                k1: 0.0,
                k2: f32::NAN,
                k3: 0.0,
                k4: 0.0,
            },
        },
    );
    assert_eq!(
        FilterProgram::new(Arc::from([composite])),
        Err(FilterProgramError::InvalidComposite { node: 0 })
    );
}

#[test]
fn drop_shadow_has_one_input_and_checked_geometry() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let shadow = |inputs: Arc<[FilterInput]>, dx, dy, sigma_x, sigma_y| {
        FilterNode::new(
            inputs,
            region,
            FilterColorSpace::LinearRgb,
            FilterPrimitive::DropShadow {
                dx,
                dy,
                sigma_x,
                sigma_y,
                color: cg::CGColor32F::from_rgba8(cg::CGColor::RED),
            },
        )
    };

    assert_eq!(
        FilterProgram::new(Arc::from([shadow(Arc::from([]), 2.0, 2.0, 2.0, 2.0)])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 1,
            actual: 0,
        })
    );
    for (dx, dy, sigma_x, sigma_y) in [
        (f32::INFINITY, 0.0, 0.0, 0.0),
        (0.0, f32::NAN, 0.0, 0.0),
        (0.0, 0.0, -1.0, 0.0),
        (0.0, 0.0, 0.0, f32::INFINITY),
    ] {
        assert_eq!(
            FilterProgram::new(Arc::from([shadow(
                Arc::from([FilterInput::Source]),
                dx,
                dy,
                sigma_x,
                sigma_y,
            )])),
            Err(FilterProgramError::InvalidDropShadow { node: 0 })
        );
    }

    FilterProgram::new(Arc::from([shadow(
        Arc::from([FilterInput::Source]),
        -2.5,
        1.25,
        0.0,
        3.0,
    )]))
    .expect("finite signed displacement and non-negative sigma are resolved facts");
}

#[test]
fn color_matrix_has_one_input_and_only_finite_coefficients() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let matrix = |inputs: Arc<[FilterInput]>, coefficient| {
        let mut matrix = [0.0; 20];
        matrix[0] = coefficient;
        FilterNode::new(
            inputs,
            region,
            FilterColorSpace::Srgb,
            FilterPrimitive::ColorMatrix { matrix },
        )
    };

    assert_eq!(
        FilterProgram::new(Arc::from([matrix(Arc::from([]), 1.0)])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 1,
            actual: 0,
        })
    );
    assert_eq!(
        FilterProgram::new(Arc::from([matrix(
            Arc::from([FilterInput::Source]),
            f32::INFINITY,
        )])),
        Err(FilterProgramError::InvalidColorMatrix { node: 0 })
    );
    FilterProgram::new(Arc::from([matrix(Arc::from([FilterInput::Source]), -3.5)]))
        .expect("finite signed color coefficients are resolved facts");
}

#[test]
fn component_transfer_has_one_input_and_four_exact_named_tables() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let identity = std::array::from_fn(|index| index as u8);
    let mut alpha = identity;
    alpha[0] = 127;
    let tables = FilterChannelTables::new(identity, [17; 256], [29; 256], alpha);
    assert_eq!(tables.red()[255], 255);
    assert_eq!(tables.green()[128], 17);
    assert_eq!(tables.blue()[128], 29);
    assert_eq!(tables.alpha()[0], 127);

    let transfer = |inputs| {
        FilterNode::new(
            inputs,
            region,
            FilterColorSpace::Srgb,
            FilterPrimitive::ComponentTransfer {
                tables: tables.clone(),
            },
        )
    };
    assert_eq!(
        FilterProgram::new(Arc::from([transfer(Arc::from([]))])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 1,
            actual: 0,
        })
    );
    let program = FilterProgram::new(Arc::from([transfer(Arc::from([FilterInput::Source]))]))
        .expect("a full four-table transfer is checked by construction");
    assert!(program.may_paint_transparent_input());
}

#[test]
fn morphology_has_one_input_and_checked_non_negative_radii() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let morphology = |inputs: Arc<[FilterInput]>, operator, radius_x, radius_y| {
        FilterNode::new(
            inputs,
            region,
            FilterColorSpace::Srgb,
            FilterPrimitive::Morphology {
                operator,
                radius_x,
                radius_y,
            },
        )
    };

    assert_eq!(
        FilterProgram::new(Arc::from([morphology(
            Arc::from([]),
            FilterMorphology::Dilate,
            2.0,
            3.0,
        )])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 1,
            actual: 0,
        })
    );
    for (radius_x, radius_y) in [
        (-1.0, 0.0),
        (0.0, -1.0),
        (f32::INFINITY, 0.0),
        (0.0, f32::NAN),
    ] {
        assert_eq!(
            FilterProgram::new(Arc::from([morphology(
                Arc::from([FilterInput::Source]),
                FilterMorphology::Erode,
                radius_x,
                radius_y,
            )])),
            Err(FilterProgramError::InvalidMorphology { node: 0 })
        );
    }

    for operator in [FilterMorphology::Erode, FilterMorphology::Dilate] {
        let program = FilterProgram::new(Arc::from([morphology(
            Arc::from([FilterInput::Source]),
            operator,
            0.0,
            256.0,
        )]))
        .expect("finite non-negative local radii are resolved facts");
        assert!(!program.may_paint_transparent_input());
    }
}

#[test]
fn turbulence_is_a_zero_input_generated_source_with_checked_parameters() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let turbulence = |inputs: Arc<[FilterInput]>,
                      kind,
                      base_frequency_x,
                      base_frequency_y,
                      num_octaves,
                      seed,
                      stitch_tiles| {
        FilterNode::new(
            inputs,
            region,
            FilterColorSpace::LinearRgb,
            FilterPrimitive::Turbulence {
                kind,
                base_frequency_x,
                base_frequency_y,
                num_octaves,
                seed,
                stitch_tiles,
            },
        )
    };

    assert_eq!(
        FilterProgram::new(Arc::from([turbulence(
            Arc::from([FilterInput::Source]),
            FilterTurbulenceKind::Turbulence,
            0.1,
            0.2,
            1,
            0.0,
            false,
        )])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 0,
            actual: 1,
        })
    );

    for (base_frequency_x, base_frequency_y, num_octaves, seed) in [
        (-0.1, 0.0, 1, 0.0),
        (0.0, -0.1, 1, 0.0),
        (f32::INFINITY, 0.0, 1, 0.0),
        (0.0, f32::NAN, 1, 0.0),
        (0.0, 0.0, 10, 0.0),
        (0.0, 0.0, 1, f32::INFINITY),
    ] {
        assert_eq!(
            FilterProgram::new(Arc::from([turbulence(
                Arc::from([]),
                FilterTurbulenceKind::FractalNoise,
                base_frequency_x,
                base_frequency_y,
                num_octaves,
                seed,
                true,
            )])),
            Err(FilterProgramError::InvalidTurbulence { node: 0 })
        );
    }

    for kind in [
        FilterTurbulenceKind::Turbulence,
        FilterTurbulenceKind::FractalNoise,
    ] {
        for num_octaves in [0, 1, 9] {
            let program = FilterProgram::new(Arc::from([turbulence(
                Arc::from([]),
                kind,
                0.0,
                256.0,
                num_octaves,
                -7.5,
                true,
            )]))
            .expect("finite non-negative frequencies and at most nine octaves are resolved facts");
            assert!(program.may_paint_transparent_input());
        }
    }
}

#[test]
fn displacement_map_has_two_ordered_inputs_and_checked_channel_vocabulary() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let displacement = |inputs: Arc<[FilterInput]>, scale, x_channel, y_channel| {
        FilterNode::new(
            inputs,
            region,
            FilterColorSpace::Srgb,
            FilterPrimitive::DisplacementMap {
                scale,
                x_channel,
                y_channel,
            },
        )
    };

    assert_eq!(
        FilterProgram::new(Arc::from([displacement(
            Arc::from([FilterInput::Source]),
            20.0,
            FilterDisplacementChannel::Red,
            FilterDisplacementChannel::Green,
        )])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 2,
            actual: 1,
        })
    );
    assert_eq!(
        FilterProgram::new(Arc::from([displacement(
            Arc::from([FilterInput::Source, FilterInput::SourceAlpha]),
            f32::NAN,
            FilterDisplacementChannel::Blue,
            FilterDisplacementChannel::Alpha,
        )])),
        Err(FilterProgramError::InvalidDisplacementMap { node: 0 })
    );

    let channels = [
        FilterDisplacementChannel::Red,
        FilterDisplacementChannel::Green,
        FilterDisplacementChannel::Blue,
        FilterDisplacementChannel::Alpha,
    ];
    for x_channel in channels {
        for y_channel in channels {
            let program = FilterProgram::new(Arc::from([displacement(
                Arc::from([FilterInput::Source, FilterInput::SourceAlpha]),
                -256.5,
                x_channel,
                y_channel,
            )]))
            .expect("finite signed scale and two named channels are resolved facts");
            assert!(!program.may_paint_transparent_input());
        }
    }
}

#[test]
fn convolution_has_one_input_and_a_bounded_finite_checked_kernel() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let convolve = |inputs: Arc<[FilterInput]>,
                    order_x,
                    order_y,
                    kernel: Arc<[f32]>,
                    gain,
                    bias,
                    target_x,
                    target_y,
                    edge_mode,
                    preserve_alpha| {
        FilterNode::new(
            inputs,
            region,
            FilterColorSpace::Srgb,
            FilterPrimitive::ConvolveMatrix {
                order_x,
                order_y,
                kernel,
                gain,
                bias,
                target_x,
                target_y,
                edge_mode,
                preserve_alpha,
            },
        )
    };

    assert_eq!(
        FilterProgram::new(Arc::from([convolve(
            Arc::from([]),
            1,
            1,
            Arc::from([1.0]),
            1.0,
            0.0,
            0,
            0,
            FilterConvolveEdgeMode::None,
            false,
        )])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 1,
            actual: 0,
        })
    );

    for node in [
        convolve(
            Arc::from([FilterInput::Source]),
            0,
            1,
            Arc::from([]),
            1.0,
            0.0,
            0,
            0,
            FilterConvolveEdgeMode::Duplicate,
            false,
        ),
        convolve(
            Arc::from([FilterInput::Source]),
            2,
            2,
            Arc::from([1.0, 0.0, 0.0]),
            1.0,
            0.0,
            0,
            0,
            FilterConvolveEdgeMode::Wrap,
            false,
        ),
        convolve(
            Arc::from([FilterInput::Source]),
            1,
            1,
            Arc::from([f32::NAN]),
            1.0,
            0.0,
            0,
            0,
            FilterConvolveEdgeMode::None,
            false,
        ),
        convolve(
            Arc::from([FilterInput::Source]),
            1,
            1,
            Arc::from([1.0]),
            f32::INFINITY,
            0.0,
            0,
            0,
            FilterConvolveEdgeMode::None,
            false,
        ),
        convolve(
            Arc::from([FilterInput::Source]),
            1,
            1,
            Arc::from([1.0]),
            1.0,
            f32::NAN,
            0,
            0,
            FilterConvolveEdgeMode::None,
            false,
        ),
        convolve(
            Arc::from([FilterInput::Source]),
            2,
            2,
            Arc::from([1.0, 0.0, 0.0, 0.0]),
            1.0,
            0.0,
            2,
            0,
            FilterConvolveEdgeMode::None,
            false,
        ),
        convolve(
            Arc::from([FilterInput::Source]),
            257,
            1,
            vec![0.0; MAX_FILTER_CONVOLVE_KERNEL_VALUES + 1].into(),
            1.0,
            0.0,
            0,
            0,
            FilterConvolveEdgeMode::None,
            false,
        ),
    ] {
        assert_eq!(
            FilterProgram::new(Arc::from([node])),
            Err(FilterProgramError::InvalidConvolveMatrix { node: 0 })
        );
    }

    for edge_mode in [
        FilterConvolveEdgeMode::Duplicate,
        FilterConvolveEdgeMode::Wrap,
        FilterConvolveEdgeMode::None,
    ] {
        let ordinary = FilterProgram::new(Arc::from([convolve(
            Arc::from([FilterInput::Source]),
            16,
            16,
            vec![0.0; MAX_FILTER_CONVOLVE_KERNEL_VALUES].into(),
            -2.0,
            -0.5,
            15,
            15,
            edge_mode,
            false,
        )]))
        .expect("the maximum finite checked kernel is a resolved fact");
        assert!(!ordinary.may_paint_transparent_input());

        let additive = FilterProgram::new(Arc::from([convolve(
            Arc::from([FilterInput::Source]),
            1,
            1,
            Arc::from([1.0]),
            1.0,
            0.25,
            0,
            0,
            edge_mode,
            false,
        )]))
        .expect("positive alpha bias can create output");
        assert!(additive.may_paint_transparent_input());

        let preserved = FilterProgram::new(Arc::from([convolve(
            Arc::from([FilterInput::Source]),
            1,
            1,
            Arc::from([1.0]),
            1.0,
            0.25,
            0,
            0,
            edge_mode,
            true,
        )]))
        .expect("preserved alpha suppresses generated RGB on a transparent input");
        assert!(!preserved.may_paint_transparent_input());
    }
}

#[test]
fn diffuse_lighting_has_one_input_and_a_checked_shared_light_source() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let lighting = |inputs: Arc<[FilterInput]>, surface_scale, diffuse_constant, color, light| {
        FilterNode::new(
            inputs,
            region,
            FilterColorSpace::LinearRgb,
            FilterPrimitive::DiffuseLighting {
                surface_scale,
                diffuse_constant,
                color,
                light,
            },
        )
    };
    let distant = FilterLightSource::Distant {
        direction: [0.5, -0.5, std::f32::consts::FRAC_1_SQRT_2],
    };

    assert_eq!(
        FilterProgram::new(Arc::from([lighting(
            Arc::from([]),
            1.0,
            1.0,
            cg::CGColor::WHITE,
            distant,
        )])),
        Err(FilterProgramError::InvalidInputCount {
            node: 0,
            expected: 1,
            actual: 0,
        })
    );

    for light in [
        distant,
        FilterLightSource::Point {
            location: [-4.0, 8.0, 12.0],
        },
        FilterLightSource::Spot {
            location: [2.0, 3.0, 8.0],
            target: [7.0, 6.0, 0.0],
            falloff_exponent: 128.0,
            cutoff_angle: -35.0,
        },
    ] {
        let program = FilterProgram::new(Arc::from([lighting(
            Arc::from([FilterInput::SourceAlpha]),
            -3.0,
            0.0,
            cg::CGColor::from_rgb(0x25, 0x63, 0xeb),
            light,
        )]))
        .expect("a finite diffuse operation with any resolved light kind is admitted");
        assert!(program.may_paint_transparent_input());
    }

    for node in [
        lighting(
            Arc::from([FilterInput::Source]),
            f32::INFINITY,
            1.0,
            cg::CGColor::WHITE,
            distant,
        ),
        lighting(
            Arc::from([FilterInput::Source]),
            1.0,
            -1.0,
            cg::CGColor::WHITE,
            distant,
        ),
        lighting(
            Arc::from([FilterInput::Source]),
            1.0,
            1.0,
            cg::CGColor::from_rgba(255, 255, 255, 128),
            distant,
        ),
        lighting(
            Arc::from([FilterInput::Source]),
            1.0,
            1.0,
            cg::CGColor::WHITE,
            FilterLightSource::Distant {
                direction: [0.0, 0.0, 0.0],
            },
        ),
        lighting(
            Arc::from([FilterInput::Source]),
            1.0,
            1.0,
            cg::CGColor::WHITE,
            FilterLightSource::Point {
                location: [0.0, f32::NAN, 0.0],
            },
        ),
        lighting(
            Arc::from([FilterInput::Source]),
            1.0,
            1.0,
            cg::CGColor::WHITE,
            FilterLightSource::Spot {
                location: [0.0, 0.0, 1.0],
                target: [0.0, 0.0, 0.0],
                falloff_exponent: 129.0,
                cutoff_angle: 0.0,
            },
        ),
    ] {
        assert_eq!(
            FilterProgram::new(Arc::from([node])),
            Err(FilterProgramError::InvalidDiffuseLighting { node: 0 })
        );
    }
}

#[test]
fn a_filter_invocation_names_when_its_source_is_fully_transparent() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let program = FilterProgram::new(Arc::from([FilterNode::new(
        Arc::from([]),
        region,
        FilterColorSpace::LinearRgb,
        FilterPrimitive::Turbulence {
            kind: FilterTurbulenceKind::Turbulence,
            base_frequency_x: 0.1,
            base_frequency_y: 0.2,
            num_octaves: 2,
            seed: 3.0,
            stitch_tiles: false,
        },
    )]))
    .expect("generated source program");
    let filter = Filter::new(AffineTransform::identity(), region, program)
        .expect("checked filter invocation");
    assert!(!filter.source_is_transparent());
    assert!(filter.with_transparent_source().source_is_transparent());
}

#[test]
fn generated_and_additive_nodes_keep_a_transparent_input_scope_alive() {
    let region = Rectangle::from_xywh(0.0, 0.0, 10.0, 10.0);
    let node = |primitive| {
        FilterNode::new(
            Arc::from([FilterInput::Source]),
            region,
            FilterColorSpace::Srgb,
            primitive,
        )
    };
    let mut additive = [0.0; 20];
    additive[19] = 0.5;
    let matrix = FilterProgram::new(Arc::from([node(FilterPrimitive::ColorMatrix {
        matrix: additive,
    })]))
    .expect("finite matrix");
    assert!(matrix.may_paint_transparent_input());

    let identity = FilterProgram::new(Arc::from([node(FilterPrimitive::ColorMatrix {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ],
    })]))
    .expect("finite matrix");
    assert!(!identity.may_paint_transparent_input());

    let solid = FilterProgram::new(Arc::from([FilterNode::new(
        Arc::from([]),
        region,
        FilterColorSpace::Srgb,
        FilterPrimitive::SolidColor {
            color: cg::CGColor32F::from_rgba8(cg::CGColor::RED),
        },
    )]))
    .expect("solid source");
    assert!(solid.may_paint_transparent_input());
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
