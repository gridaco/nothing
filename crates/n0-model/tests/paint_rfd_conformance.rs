//! Law-level conformance harness for the ratified paint-model RFD.
//!
//! The bindings below project each real vocabulary into observations owned by
//! the test. Neither implementation is the oracle. The RFD is the contract;
//! differences which cannot satisfy it are named in the companion gap report.

use std::fmt::Debug;

const GAP_REPORT: &str = include_str!("../../../docs/wg/consolidation/paint-vocabulary-gap.md");

const CG_GAPS: &[&str] = &[
    "P1-CG-COLOR-CANONICAL",
    "P1-CG-STROKE-APPLICATION",
    "P1-CG-TEXT-STROKE",
];

const N0_GAPS: &[&str] = &["P1-N0-DECORATION-COLOR", "P1-N0-TEXT-STROKE"];

const PINNED_AMENDMENTS: &[&str] = &["AMD-DIAMOND-CLAMP", "AMD-RUN-FILL-TRISTATE"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaintKind {
    Solid,
    Linear,
    Radial,
    Sweep,
    Diamond,
    Image,
}

impl PaintKind {
    const ALL: [PaintKind; 6] = [
        PaintKind::Solid,
        PaintKind::Linear,
        PaintKind::Radial,
        PaintKind::Sweep,
        PaintKind::Diamond,
        PaintKind::Image,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Blend {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl Blend {
    const ALL: [Blend; 16] = [
        Blend::Normal,
        Blend::Multiply,
        Blend::Screen,
        Blend::Overlay,
        Blend::Darken,
        Blend::Lighten,
        Blend::ColorDodge,
        Blend::ColorBurn,
        Blend::HardLight,
        Blend::SoftLight,
        Blend::Difference,
        Blend::Exclusion,
        Blend::Hue,
        Blend::Saturation,
        Blend::Color,
        Blend::Luminosity,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tile {
    Clamp,
    Repeated,
    Mirror,
    Decal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PaintObservation {
    kind: PaintKind,
    active: bool,
    opacity_bits: u32,
    blend: Blend,
    visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GradientObservation {
    kind: PaintKind,
    endpoints: Option<([u32; 2], [u32; 2])>,
    tile: Option<Tile>,
    transform_bits: [u32; 6],
    stop_count: usize,
    first_stop: Option<(u32, u32)>,
    active: bool,
    opacity_bits: u32,
    blend: Blend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResourceObservation {
    Hash(String),
    Rid(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectFitObservation {
    Contain,
    Cover,
    Fill,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepeatObservation {
    RepeatX,
    RepeatY,
    Repeat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ImageFitObservation {
    Fit(ObjectFitObservation),
    Transform([u32; 6]),
    Tile {
        scale_bits: u32,
        repeat: RepeatObservation,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImageVocabularyObservation {
    resources: Vec<ResourceObservation>,
    fits: Vec<ImageFitObservation>,
    object_fits: Vec<ObjectFitObservation>,
    repeats: Vec<RepeatObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImageObservation {
    resource: ResourceObservation,
    fit: ImageFitObservation,
    quarter_turns: u8,
    alignment_bits: [u32; 2],
    filter_bits: [u32; 7],
    active: bool,
    opacity_bits: u32,
    blend: Blend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunFillState {
    Inherit,
    ExplicitEmpty,
    Override(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrokeWidthObservation {
    None,
    Uniform(u32),
    Rectangular([u32; 4]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrokeAlignObservation {
    Inside,
    Center,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrokeCapObservation {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StrokeJoinObservation {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StrokeObservation {
    widths: Vec<StrokeWidthObservation>,
    aligns: Vec<StrokeAlignObservation>,
    caps: Vec<StrokeCapObservation>,
    joins: Vec<StrokeJoinObservation>,
    default_miter_bits: u32,
    dash_pattern_bits: Vec<u32>,
}

trait PaintVocabulary {
    type Paint: Clone + Debug;
    type Paints;

    fn name() -> &'static str;
    fn color_from_argb(argb: u32) -> ([u8; 4], u32);
    fn byte_float_byte(alpha: u8) -> u8;
    fn blends() -> Vec<Blend>;
    fn tiles() -> Vec<Tile>;
    fn paint(kind: PaintKind, active: bool, opacity: f32, blend: Blend) -> Self::Paint;
    fn observe_paint(paint: &Self::Paint) -> PaintObservation;
    fn stack(paints: Vec<Self::Paint>) -> Self::Paints;
    fn push(stack: &mut Self::Paints, paint: Self::Paint);
    fn stack_kinds(stack: &Self::Paints) -> Vec<PaintKind>;
    fn stack_observations(stack: &Self::Paints) -> Vec<PaintObservation>;
    fn stack_is_empty(stack: &Self::Paints) -> bool;
    fn gradient_defaults() -> Vec<GradientObservation>;
    fn gradient_sentinels() -> Vec<GradientObservation>;
    fn stop() -> (u32, u32);
    fn image_vocabulary() -> ImageVocabularyObservation;
    fn image_neutral_witness() -> ImageObservation;
    fn image_sentinel() -> ImageObservation;
    fn run_fill_states() -> Vec<RunFillState>;
    fn stroke_surface() -> StrokeObservation;
}

struct Cg;

impl Cg {
    fn blend(value: Blend) -> cg::BlendMode {
        match value {
            Blend::Normal => cg::BlendMode::Normal,
            Blend::Multiply => cg::BlendMode::Multiply,
            Blend::Screen => cg::BlendMode::Screen,
            Blend::Overlay => cg::BlendMode::Overlay,
            Blend::Darken => cg::BlendMode::Darken,
            Blend::Lighten => cg::BlendMode::Lighten,
            Blend::ColorDodge => cg::BlendMode::ColorDodge,
            Blend::ColorBurn => cg::BlendMode::ColorBurn,
            Blend::HardLight => cg::BlendMode::HardLight,
            Blend::SoftLight => cg::BlendMode::SoftLight,
            Blend::Difference => cg::BlendMode::Difference,
            Blend::Exclusion => cg::BlendMode::Exclusion,
            Blend::Hue => cg::BlendMode::Hue,
            Blend::Saturation => cg::BlendMode::Saturation,
            Blend::Color => cg::BlendMode::Color,
            Blend::Luminosity => cg::BlendMode::Luminosity,
        }
    }

    fn observe_blend(value: cg::BlendMode) -> Blend {
        match value {
            cg::BlendMode::Normal => Blend::Normal,
            cg::BlendMode::Multiply => Blend::Multiply,
            cg::BlendMode::Screen => Blend::Screen,
            cg::BlendMode::Overlay => Blend::Overlay,
            cg::BlendMode::Darken => Blend::Darken,
            cg::BlendMode::Lighten => Blend::Lighten,
            cg::BlendMode::ColorDodge => Blend::ColorDodge,
            cg::BlendMode::ColorBurn => Blend::ColorBurn,
            cg::BlendMode::HardLight => Blend::HardLight,
            cg::BlendMode::SoftLight => Blend::SoftLight,
            cg::BlendMode::Difference => Blend::Difference,
            cg::BlendMode::Exclusion => Blend::Exclusion,
            cg::BlendMode::Hue => Blend::Hue,
            cg::BlendMode::Saturation => Blend::Saturation,
            cg::BlendMode::Color => Blend::Color,
            cg::BlendMode::Luminosity => Blend::Luminosity,
        }
    }

    fn observe_kind(value: &cg::Paint) -> PaintKind {
        match value {
            cg::Paint::Solid(_) => PaintKind::Solid,
            cg::Paint::LinearGradient(_) => PaintKind::Linear,
            cg::Paint::RadialGradient(_) => PaintKind::Radial,
            cg::Paint::SweepGradient(_) => PaintKind::Sweep,
            cg::Paint::DiamondGradient(_) => PaintKind::Diamond,
            cg::Paint::Image(_) => PaintKind::Image,
        }
    }

    fn observe_tile(value: cg::TileMode) -> Tile {
        match value {
            cg::TileMode::Clamp => Tile::Clamp,
            cg::TileMode::Repeated => Tile::Repeated,
            cg::TileMode::Mirror => Tile::Mirror,
            cg::TileMode::Decal => Tile::Decal,
        }
    }

    fn gradient(
        kind: PaintKind,
        endpoints: Option<(cg::Alignment, cg::Alignment)>,
        tile: Option<cg::TileMode>,
        transform: math2::transform::AffineTransform,
        stops: &[cg::GradientStop],
        active: bool,
        opacity: f32,
        blend: cg::BlendMode,
    ) -> GradientObservation {
        GradientObservation {
            kind,
            endpoints: endpoints.map(|(from, to)| {
                (
                    [from.0.to_bits(), from.1.to_bits()],
                    [to.0.to_bits(), to.1.to_bits()],
                )
            }),
            tile: tile.map(Self::observe_tile),
            transform_bits: [
                transform.matrix[0][0].to_bits(),
                transform.matrix[1][0].to_bits(),
                transform.matrix[0][1].to_bits(),
                transform.matrix[1][1].to_bits(),
                transform.matrix[0][2].to_bits(),
                transform.matrix[1][2].to_bits(),
            ],
            stop_count: stops.len(),
            first_stop: stops.first().map(|stop| {
                (
                    stop.offset.to_bits(),
                    u32::from(stop.color.a) << 24
                        | u32::from(stop.color.r) << 16
                        | u32::from(stop.color.g) << 8
                        | u32::from(stop.color.b),
                )
            }),
            active,
            opacity_bits: opacity.to_bits(),
            blend: Self::observe_blend(blend),
        }
    }

    fn observe_transform(transform: math2::transform::AffineTransform) -> [u32; 6] {
        [
            transform.matrix[0][0].to_bits(),
            transform.matrix[1][0].to_bits(),
            transform.matrix[0][1].to_bits(),
            transform.matrix[1][1].to_bits(),
            transform.matrix[0][2].to_bits(),
            transform.matrix[1][2].to_bits(),
        ]
    }

    fn observe_resource(value: cg::ResourceRef) -> ResourceObservation {
        match value {
            cg::ResourceRef::HASH(value) => ResourceObservation::Hash(value),
            cg::ResourceRef::RID(value) => ResourceObservation::Rid(value),
        }
    }

    fn observe_object_fit(value: math2::box_fit::BoxFit) -> ObjectFitObservation {
        match value {
            math2::box_fit::BoxFit::Contain => ObjectFitObservation::Contain,
            math2::box_fit::BoxFit::Cover => ObjectFitObservation::Cover,
            math2::box_fit::BoxFit::Fill => ObjectFitObservation::Fill,
            math2::box_fit::BoxFit::None => ObjectFitObservation::None,
        }
    }

    fn observe_repeat(value: cg::ImageRepeat) -> RepeatObservation {
        match value {
            cg::ImageRepeat::RepeatX => RepeatObservation::RepeatX,
            cg::ImageRepeat::RepeatY => RepeatObservation::RepeatY,
            cg::ImageRepeat::Repeat => RepeatObservation::Repeat,
        }
    }

    fn observe_image_fit(value: cg::ImagePaintFit) -> ImageFitObservation {
        match value {
            cg::ImagePaintFit::Fit(value) => {
                ImageFitObservation::Fit(Self::observe_object_fit(value))
            }
            cg::ImagePaintFit::Transform(value) => {
                ImageFitObservation::Transform(Self::observe_transform(value))
            }
            cg::ImagePaintFit::Tile(cg::ImageTile { scale, repeat }) => ImageFitObservation::Tile {
                scale_bits: scale.to_bits(),
                repeat: Self::observe_repeat(repeat),
            },
        }
    }

    fn observe_image(value: cg::ImagePaint) -> ImageObservation {
        let cg::ImagePaint {
            active,
            image,
            quarter_turns,
            alignement,
            fit,
            opacity,
            blend_mode,
            filters,
        } = value;
        let cg::ImageFilters {
            exposure,
            contrast,
            saturation,
            temperature,
            tint,
            highlights,
            shadows,
        } = filters;
        ImageObservation {
            resource: Self::observe_resource(image),
            fit: Self::observe_image_fit(fit),
            quarter_turns,
            alignment_bits: [alignement.0.to_bits(), alignement.1.to_bits()],
            filter_bits: [
                exposure.to_bits(),
                contrast.to_bits(),
                saturation.to_bits(),
                temperature.to_bits(),
                tint.to_bits(),
                highlights.to_bits(),
                shadows.to_bits(),
            ],
            active,
            opacity_bits: opacity.to_bits(),
            blend: Self::observe_blend(blend_mode),
        }
    }
}

impl PaintVocabulary for Cg {
    type Paint = cg::Paint;
    type Paints = cg::Paints;

    fn name() -> &'static str {
        "cg"
    }

    fn color_from_argb(argb: u32) -> ([u8; 4], u32) {
        let color = cg::CGColor::from_u32_argb(argb);
        let projected = u32::from(color.a) << 24
            | u32::from(color.r) << 16
            | u32::from(color.g) << 8
            | u32::from(color.b);
        ([color.r, color.g, color.b, color.a], projected)
    }

    fn byte_float_byte(alpha: u8) -> u8 {
        cg::CGColor::from_rgb(0x12, 0x34, 0x56)
            .with_multiplier(f32::from(alpha) / 255.0)
            .a
    }

    fn blends() -> Vec<Blend> {
        [
            cg::BlendMode::Normal,
            cg::BlendMode::Multiply,
            cg::BlendMode::Screen,
            cg::BlendMode::Overlay,
            cg::BlendMode::Darken,
            cg::BlendMode::Lighten,
            cg::BlendMode::ColorDodge,
            cg::BlendMode::ColorBurn,
            cg::BlendMode::HardLight,
            cg::BlendMode::SoftLight,
            cg::BlendMode::Difference,
            cg::BlendMode::Exclusion,
            cg::BlendMode::Hue,
            cg::BlendMode::Saturation,
            cg::BlendMode::Color,
            cg::BlendMode::Luminosity,
        ]
        .into_iter()
        .map(Self::observe_blend)
        .collect()
    }

    fn tiles() -> Vec<Tile> {
        [
            cg::TileMode::Clamp,
            cg::TileMode::Repeated,
            cg::TileMode::Mirror,
            cg::TileMode::Decal,
        ]
        .into_iter()
        .map(Self::observe_tile)
        .collect()
    }

    fn paint(kind: PaintKind, active: bool, opacity: f32, blend: Blend) -> Self::Paint {
        let blend_mode = Self::blend(blend);
        match kind {
            PaintKind::Solid => cg::Paint::Solid(cg::SolidPaint {
                active,
                color: cg::CGColor::from_rgba(
                    0x12,
                    0x34,
                    0x56,
                    (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
                ),
                blend_mode,
            }),
            PaintKind::Linear => cg::Paint::LinearGradient(cg::LinearGradientPaint {
                active,
                opacity,
                blend_mode,
                ..Default::default()
            }),
            PaintKind::Radial => cg::Paint::RadialGradient(cg::RadialGradientPaint {
                active,
                opacity,
                blend_mode,
                ..Default::default()
            }),
            PaintKind::Sweep => cg::Paint::SweepGradient(cg::SweepGradientPaint {
                active,
                opacity,
                blend_mode,
                ..Default::default()
            }),
            PaintKind::Diamond => cg::Paint::DiamondGradient(cg::DiamondGradientPaint {
                active,
                opacity,
                blend_mode,
                ..Default::default()
            }),
            PaintKind::Image => cg::Paint::Image(cg::ImagePaint {
                active,
                image: cg::ResourceRef::RID("fixture://paint-rfd".into()),
                quarter_turns: 0,
                alignement: cg::Alignment::CENTER,
                fit: cg::ImagePaintFit::Fit(math2::box_fit::BoxFit::Cover),
                opacity,
                blend_mode,
                filters: Default::default(),
            }),
        }
    }

    fn observe_paint(paint: &Self::Paint) -> PaintObservation {
        PaintObservation {
            kind: Self::observe_kind(paint),
            active: paint.active(),
            opacity_bits: paint.opacity().to_bits(),
            blend: Self::observe_blend(paint.blend_mode()),
            visible: paint.visible(),
        }
    }

    fn stack(paints: Vec<Self::Paint>) -> Self::Paints {
        cg::Paints::new(paints)
    }

    fn push(stack: &mut Self::Paints, paint: Self::Paint) {
        stack.push(paint);
    }

    fn stack_kinds(stack: &Self::Paints) -> Vec<PaintKind> {
        stack.iter().map(Self::observe_kind).collect()
    }

    fn stack_observations(stack: &Self::Paints) -> Vec<PaintObservation> {
        stack.iter().map(Self::observe_paint).collect()
    }

    fn stack_is_empty(stack: &Self::Paints) -> bool {
        stack.is_empty()
    }

    fn gradient_defaults() -> Vec<GradientObservation> {
        let cg::LinearGradientPaint {
            active: linear_active,
            xy1,
            xy2,
            tile_mode: linear_tile,
            transform: linear_transform,
            stops: linear_stops,
            opacity: linear_opacity,
            blend_mode: linear_blend,
        } = cg::LinearGradientPaint::default();
        let cg::RadialGradientPaint {
            active: radial_active,
            transform: radial_transform,
            stops: radial_stops,
            opacity: radial_opacity,
            blend_mode: radial_blend,
            tile_mode: radial_tile,
        } = cg::RadialGradientPaint::default();
        let cg::SweepGradientPaint {
            active: sweep_active,
            transform: sweep_transform,
            stops: sweep_stops,
            opacity: sweep_opacity,
            blend_mode: sweep_blend,
        } = cg::SweepGradientPaint::default();
        let cg::DiamondGradientPaint {
            active: diamond_active,
            transform: diamond_transform,
            stops: diamond_stops,
            opacity: diamond_opacity,
            blend_mode: diamond_blend,
        } = cg::DiamondGradientPaint::default();
        vec![
            Self::gradient(
                PaintKind::Linear,
                Some((xy1, xy2)),
                Some(linear_tile),
                linear_transform,
                &linear_stops,
                linear_active,
                linear_opacity,
                linear_blend,
            ),
            Self::gradient(
                PaintKind::Radial,
                None,
                Some(radial_tile),
                radial_transform,
                &radial_stops,
                radial_active,
                radial_opacity,
                radial_blend,
            ),
            Self::gradient(
                PaintKind::Sweep,
                None,
                None,
                sweep_transform,
                &sweep_stops,
                sweep_active,
                sweep_opacity,
                sweep_blend,
            ),
            Self::gradient(
                PaintKind::Diamond,
                None,
                None,
                diamond_transform,
                &diamond_stops,
                diamond_active,
                diamond_opacity,
                diamond_blend,
            ),
        ]
    }

    fn gradient_sentinels() -> Vec<GradientObservation> {
        use math2::transform::AffineTransform;

        let transform = AffineTransform::from_acebdf(2.0, 3.0, 5.0, 4.0, 6.0, 7.0);
        let stops = vec![cg::GradientStop {
            offset: 0.375,
            color: cg::CGColor::from_u32_argb(0x8040_2010),
        }];
        let cg::LinearGradientPaint {
            active: linear_active,
            xy1,
            xy2,
            tile_mode: linear_tile,
            transform: linear_transform,
            stops: linear_stops,
            opacity: linear_opacity,
            blend_mode: linear_blend,
        } = cg::LinearGradientPaint {
            active: false,
            xy1: cg::Alignment(-0.75, 0.25),
            xy2: cg::Alignment(0.625, -0.5),
            tile_mode: cg::TileMode::Mirror,
            transform,
            stops: stops.clone(),
            opacity: 0.625,
            blend_mode: cg::BlendMode::SoftLight,
        };
        let cg::RadialGradientPaint {
            active: radial_active,
            transform: radial_transform,
            stops: radial_stops,
            opacity: radial_opacity,
            blend_mode: radial_blend,
            tile_mode: radial_tile,
        } = cg::RadialGradientPaint {
            active: false,
            transform,
            stops: stops.clone(),
            opacity: 0.625,
            blend_mode: cg::BlendMode::SoftLight,
            tile_mode: cg::TileMode::Decal,
        };
        let cg::SweepGradientPaint {
            active: sweep_active,
            transform: sweep_transform,
            stops: sweep_stops,
            opacity: sweep_opacity,
            blend_mode: sweep_blend,
        } = cg::SweepGradientPaint {
            active: false,
            transform,
            stops: stops.clone(),
            opacity: 0.625,
            blend_mode: cg::BlendMode::SoftLight,
        };
        let cg::DiamondGradientPaint {
            active: diamond_active,
            transform: diamond_transform,
            stops: diamond_stops,
            opacity: diamond_opacity,
            blend_mode: diamond_blend,
        } = cg::DiamondGradientPaint {
            active: false,
            transform,
            stops,
            opacity: 0.625,
            blend_mode: cg::BlendMode::SoftLight,
        };

        vec![
            Self::gradient(
                PaintKind::Linear,
                Some((xy1, xy2)),
                Some(linear_tile),
                linear_transform,
                &linear_stops,
                linear_active,
                linear_opacity,
                linear_blend,
            ),
            Self::gradient(
                PaintKind::Radial,
                None,
                Some(radial_tile),
                radial_transform,
                &radial_stops,
                radial_active,
                radial_opacity,
                radial_blend,
            ),
            Self::gradient(
                PaintKind::Sweep,
                None,
                None,
                sweep_transform,
                &sweep_stops,
                sweep_active,
                sweep_opacity,
                sweep_blend,
            ),
            Self::gradient(
                PaintKind::Diamond,
                None,
                None,
                diamond_transform,
                &diamond_stops,
                diamond_active,
                diamond_opacity,
                diamond_blend,
            ),
        ]
    }

    fn stop() -> (u32, u32) {
        let stop = cg::GradientStop {
            offset: 0.25,
            color: cg::CGColor::from_u32_argb(0x8040_2010),
        };
        let (_, argb) = Self::color_from_argb(
            u32::from(stop.color.a) << 24
                | u32::from(stop.color.r) << 16
                | u32::from(stop.color.g) << 8
                | u32::from(stop.color.b),
        );
        (stop.offset.to_bits(), argb)
    }

    fn image_vocabulary() -> ImageVocabularyObservation {
        use math2::{box_fit::BoxFit, transform::AffineTransform};
        ImageVocabularyObservation {
            resources: [
                cg::ResourceRef::HASH("hash".into()),
                cg::ResourceRef::RID("rid".into()),
            ]
            .into_iter()
            .map(Self::observe_resource)
            .collect(),
            fits: [
                cg::ImagePaintFit::Fit(BoxFit::Contain),
                cg::ImagePaintFit::Transform(AffineTransform::from_acebdf(
                    2.0, 3.0, 5.0, 4.0, 6.0, 7.0,
                )),
                cg::ImagePaintFit::Tile(cg::ImageTile {
                    scale: 0.625,
                    repeat: cg::ImageRepeat::RepeatY,
                }),
            ]
            .into_iter()
            .map(Self::observe_image_fit)
            .collect(),
            object_fits: [BoxFit::Contain, BoxFit::Cover, BoxFit::Fill, BoxFit::None]
                .into_iter()
                .map(Self::observe_object_fit)
                .collect(),
            repeats: [
                cg::ImageRepeat::RepeatX,
                cg::ImageRepeat::RepeatY,
                cg::ImageRepeat::Repeat,
            ]
            .into_iter()
            .map(Self::observe_repeat)
            .collect(),
        }
    }

    fn image_neutral_witness() -> ImageObservation {
        let image = match Self::paint(PaintKind::Image, true, 1.0, Blend::Normal) {
            cg::Paint::Image(image) => image,
            _ => unreachable!(),
        };
        Self::observe_image(image)
    }

    fn image_sentinel() -> ImageObservation {
        Self::observe_image(cg::ImagePaint {
            active: false,
            image: cg::ResourceRef::HASH("hash-sentinel".into()),
            quarter_turns: 3,
            alignement: cg::Alignment(-0.75, 0.25),
            fit: cg::ImagePaintFit::Tile(cg::ImageTile {
                scale: 0.625,
                repeat: cg::ImageRepeat::RepeatY,
            }),
            opacity: 0.375,
            blend_mode: cg::BlendMode::ColorBurn,
            filters: cg::ImageFilters {
                exposure: 0.1,
                contrast: 0.2,
                saturation: 0.3,
                temperature: 0.4,
                tint: 0.5,
                highlights: 0.6,
                shadows: 0.7,
            },
        })
    }

    fn run_fill_states() -> Vec<RunFillState> {
        fn observe(run: &cg::StyledTextRun) -> RunFillState {
            match &run.fills {
                None => RunFillState::Inherit,
                Some(fills) if fills.is_empty() => RunFillState::ExplicitEmpty,
                Some(fills) => RunFillState::Override(fills.len()),
            }
        }
        fn run(fills: Option<Vec<cg::Paint>>) -> cg::StyledTextRun {
            cg::StyledTextRun {
                start: 0,
                end: 1,
                style: cg::TextStyleRec::from_font("sans-serif", 16.0),
                fills,
                strokes: None,
                stroke_width: None,
                stroke_align: None,
            }
        }
        vec![
            observe(&run(None)),
            observe(&run(Some(vec![]))),
            observe(&run(Some(vec![Self::paint(
                PaintKind::Solid,
                true,
                1.0,
                Blend::Normal,
            )]))),
        ]
    }

    fn stroke_surface() -> StrokeObservation {
        fn observe_width(value: cg::stroke_width::StrokeWidth) -> StrokeWidthObservation {
            match value {
                cg::stroke_width::StrokeWidth::None => StrokeWidthObservation::None,
                cg::stroke_width::StrokeWidth::Uniform(value) => {
                    StrokeWidthObservation::Uniform(value.to_bits())
                }
                cg::stroke_width::StrokeWidth::Rectangular(value) => {
                    StrokeWidthObservation::Rectangular([
                        value.stroke_top_width.to_bits(),
                        value.stroke_right_width.to_bits(),
                        value.stroke_bottom_width.to_bits(),
                        value.stroke_left_width.to_bits(),
                    ])
                }
            }
        }
        fn observe_align(value: cg::StrokeAlign) -> StrokeAlignObservation {
            match value {
                cg::StrokeAlign::Inside => StrokeAlignObservation::Inside,
                cg::StrokeAlign::Center => StrokeAlignObservation::Center,
                cg::StrokeAlign::Outside => StrokeAlignObservation::Outside,
            }
        }
        fn observe_cap(value: cg::StrokeCap) -> StrokeCapObservation {
            match value {
                cg::StrokeCap::Butt => StrokeCapObservation::Butt,
                cg::StrokeCap::Round => StrokeCapObservation::Round,
                cg::StrokeCap::Square => StrokeCapObservation::Square,
            }
        }
        fn observe_join(value: cg::StrokeJoin) -> StrokeJoinObservation {
            match value {
                cg::StrokeJoin::Miter => StrokeJoinObservation::Miter,
                cg::StrokeJoin::Round => StrokeJoinObservation::Round,
                cg::StrokeJoin::Bevel => StrokeJoinObservation::Bevel,
            }
        }
        StrokeObservation {
            widths: [
                cg::stroke_width::StrokeWidth::None,
                cg::stroke_width::StrokeWidth::Uniform(1.0),
                cg::stroke_width::StrokeWidth::Rectangular(
                    cg::stroke_width::RectangularStrokeWidth {
                        stroke_top_width: 1.0,
                        stroke_right_width: 2.0,
                        stroke_bottom_width: 3.0,
                        stroke_left_width: 4.0,
                    },
                ),
            ]
            .into_iter()
            .map(observe_width)
            .collect(),
            aligns: [
                cg::StrokeAlign::Inside,
                cg::StrokeAlign::Center,
                cg::StrokeAlign::Outside,
            ]
            .into_iter()
            .map(observe_align)
            .collect(),
            caps: [
                cg::StrokeCap::Butt,
                cg::StrokeCap::Round,
                cg::StrokeCap::Square,
            ]
            .into_iter()
            .map(observe_cap)
            .collect(),
            joins: [
                cg::StrokeJoin::Miter,
                cg::StrokeJoin::Round,
                cg::StrokeJoin::Bevel,
            ]
            .into_iter()
            .map(observe_join)
            .collect(),
            default_miter_bits: cg::StrokeMiterLimit::default().0.to_bits(),
            dash_pattern_bits: cg::stroke_dasharray::StrokeDashArray::new(vec![1.0, 2.0])
                .0
                .into_iter()
                .map(f32::to_bits)
                .collect(),
        }
    }
}

struct N0;

impl N0 {
    fn blend(value: Blend) -> n0_model::model::BlendMode {
        use n0_model::model::BlendMode as N;
        match value {
            Blend::Normal => N::Normal,
            Blend::Multiply => N::Multiply,
            Blend::Screen => N::Screen,
            Blend::Overlay => N::Overlay,
            Blend::Darken => N::Darken,
            Blend::Lighten => N::Lighten,
            Blend::ColorDodge => N::ColorDodge,
            Blend::ColorBurn => N::ColorBurn,
            Blend::HardLight => N::HardLight,
            Blend::SoftLight => N::SoftLight,
            Blend::Difference => N::Difference,
            Blend::Exclusion => N::Exclusion,
            Blend::Hue => N::Hue,
            Blend::Saturation => N::Saturation,
            Blend::Color => N::Color,
            Blend::Luminosity => N::Luminosity,
        }
    }

    fn observe_blend(value: n0_model::model::BlendMode) -> Blend {
        use n0_model::model::BlendMode as N;
        match value {
            N::Normal => Blend::Normal,
            N::Multiply => Blend::Multiply,
            N::Screen => Blend::Screen,
            N::Overlay => Blend::Overlay,
            N::Darken => Blend::Darken,
            N::Lighten => Blend::Lighten,
            N::ColorDodge => Blend::ColorDodge,
            N::ColorBurn => Blend::ColorBurn,
            N::HardLight => Blend::HardLight,
            N::SoftLight => Blend::SoftLight,
            N::Difference => Blend::Difference,
            N::Exclusion => Blend::Exclusion,
            N::Hue => Blend::Hue,
            N::Saturation => Blend::Saturation,
            N::Color => Blend::Color,
            N::Luminosity => Blend::Luminosity,
        }
    }

    fn observe_kind(value: &n0_model::model::Paint) -> PaintKind {
        use n0_model::model::Paint as P;
        match value {
            P::Solid(_) => PaintKind::Solid,
            P::LinearGradient(_) => PaintKind::Linear,
            P::RadialGradient(_) => PaintKind::Radial,
            P::SweepGradient(_) => PaintKind::Sweep,
            P::DiamondGradient(_) => PaintKind::Diamond,
            P::Image(_) => PaintKind::Image,
        }
    }

    fn observe_tile(value: n0_model::model::TileMode) -> Tile {
        use n0_model::model::TileMode as T;
        match value {
            T::Clamp => Tile::Clamp,
            T::Repeated => Tile::Repeated,
            T::Mirror => Tile::Mirror,
            T::Decal => Tile::Decal,
        }
    }

    fn gradient(
        kind: PaintKind,
        endpoints: Option<(n0_model::model::Alignment, n0_model::model::Alignment)>,
        tile: Option<n0_model::model::TileMode>,
        transform: n0_model::math::Affine,
        stops: &[n0_model::model::GradientStop],
        active: bool,
        opacity: f32,
        blend: n0_model::model::BlendMode,
    ) -> GradientObservation {
        GradientObservation {
            kind,
            endpoints: endpoints.map(|(from, to)| {
                (
                    [from.0.to_bits(), from.1.to_bits()],
                    [to.0.to_bits(), to.1.to_bits()],
                )
            }),
            tile: tile.map(Self::observe_tile),
            transform_bits: [
                transform.a.to_bits(),
                transform.b.to_bits(),
                transform.c.to_bits(),
                transform.d.to_bits(),
                transform.e.to_bits(),
                transform.f.to_bits(),
            ],
            stop_count: stops.len(),
            first_stop: stops
                .first()
                .map(|stop| (stop.offset.to_bits(), stop.color.argb())),
            active,
            opacity_bits: opacity.to_bits(),
            blend: Self::observe_blend(blend),
        }
    }

    fn observe_transform(transform: n0_model::math::Affine) -> [u32; 6] {
        [
            transform.a.to_bits(),
            transform.b.to_bits(),
            transform.c.to_bits(),
            transform.d.to_bits(),
            transform.e.to_bits(),
            transform.f.to_bits(),
        ]
    }

    fn observe_resource(value: n0_model::model::ResourceRef) -> ResourceObservation {
        match value {
            n0_model::model::ResourceRef::Hash(value) => ResourceObservation::Hash(value),
            n0_model::model::ResourceRef::Rid(value) => ResourceObservation::Rid(value),
        }
    }

    fn observe_object_fit(value: n0_model::model::BoxFit) -> ObjectFitObservation {
        match value {
            n0_model::model::BoxFit::Contain => ObjectFitObservation::Contain,
            n0_model::model::BoxFit::Cover => ObjectFitObservation::Cover,
            n0_model::model::BoxFit::Fill => ObjectFitObservation::Fill,
            n0_model::model::BoxFit::None => ObjectFitObservation::None,
        }
    }

    fn observe_repeat(value: n0_model::model::ImageRepeat) -> RepeatObservation {
        match value {
            n0_model::model::ImageRepeat::RepeatX => RepeatObservation::RepeatX,
            n0_model::model::ImageRepeat::RepeatY => RepeatObservation::RepeatY,
            n0_model::model::ImageRepeat::Repeat => RepeatObservation::Repeat,
        }
    }

    fn observe_image_fit(value: n0_model::model::ImagePaintFit) -> ImageFitObservation {
        use n0_model::model as n;
        match value {
            n::ImagePaintFit::Fit(value) => {
                ImageFitObservation::Fit(Self::observe_object_fit(value))
            }
            n::ImagePaintFit::Transform(value) => {
                ImageFitObservation::Transform(Self::observe_transform(value))
            }
            n::ImagePaintFit::Tile(n::ImageTile { scale, repeat }) => ImageFitObservation::Tile {
                scale_bits: scale.to_bits(),
                repeat: Self::observe_repeat(repeat),
            },
        }
    }

    fn observe_image(value: n0_model::model::ImagePaint) -> ImageObservation {
        use n0_model::model as n;
        let n::ImagePaint {
            active,
            image,
            quarter_turns,
            alignment,
            fit,
            opacity,
            blend_mode,
            filters,
        } = value;
        let n::ImageFilters {
            exposure,
            contrast,
            saturation,
            temperature,
            tint,
            highlights,
            shadows,
        } = filters;
        ImageObservation {
            resource: Self::observe_resource(image),
            fit: Self::observe_image_fit(fit),
            quarter_turns,
            alignment_bits: [alignment.0.to_bits(), alignment.1.to_bits()],
            filter_bits: [
                exposure.to_bits(),
                contrast.to_bits(),
                saturation.to_bits(),
                temperature.to_bits(),
                tint.to_bits(),
                highlights.to_bits(),
                shadows.to_bits(),
            ],
            active,
            opacity_bits: opacity.to_bits(),
            blend: Self::observe_blend(blend_mode),
        }
    }
}

impl PaintVocabulary for N0 {
    type Paint = n0_model::model::Paint;
    type Paints = n0_model::model::Paints;

    fn name() -> &'static str {
        "n0-model"
    }

    fn color_from_argb(argb: u32) -> ([u8; 4], u32) {
        let color = n0_model::model::Color(argb);
        (
            [
                ((color.argb() >> 16) & 0xff) as u8,
                ((color.argb() >> 8) & 0xff) as u8,
                (color.argb() & 0xff) as u8,
                color.alpha(),
            ],
            color.argb(),
        )
    }

    fn byte_float_byte(alpha: u8) -> u8 {
        n0_model::model::Color(0xff12_3456)
            .with_opacity(f32::from(alpha) / 255.0)
            .alpha()
    }

    fn blends() -> Vec<Blend> {
        use n0_model::model::BlendMode as N;
        [
            N::Normal,
            N::Multiply,
            N::Screen,
            N::Overlay,
            N::Darken,
            N::Lighten,
            N::ColorDodge,
            N::ColorBurn,
            N::HardLight,
            N::SoftLight,
            N::Difference,
            N::Exclusion,
            N::Hue,
            N::Saturation,
            N::Color,
            N::Luminosity,
        ]
        .into_iter()
        .map(Self::observe_blend)
        .collect()
    }

    fn tiles() -> Vec<Tile> {
        use n0_model::model::TileMode as T;
        [T::Clamp, T::Repeated, T::Mirror, T::Decal]
            .into_iter()
            .map(Self::observe_tile)
            .collect()
    }

    fn paint(kind: PaintKind, active: bool, opacity: f32, blend: Blend) -> Self::Paint {
        use n0_model::model as n;
        let blend_mode = Self::blend(blend);
        match kind {
            PaintKind::Solid => n::Paint::Solid(n::SolidPaint {
                active,
                color: n::Color(0x0012_3456).with_opacity(opacity),
                blend_mode,
            }),
            PaintKind::Linear => n::Paint::LinearGradient(n::LinearGradientPaint {
                active,
                opacity,
                blend_mode,
                ..Default::default()
            }),
            PaintKind::Radial => n::Paint::RadialGradient(n::RadialGradientPaint {
                active,
                opacity,
                blend_mode,
                ..Default::default()
            }),
            PaintKind::Sweep => n::Paint::SweepGradient(n::SweepGradientPaint {
                active,
                opacity,
                blend_mode,
                ..Default::default()
            }),
            PaintKind::Diamond => n::Paint::DiamondGradient(n::DiamondGradientPaint {
                active,
                opacity,
                blend_mode,
                ..Default::default()
            }),
            PaintKind::Image => {
                let mut image = n::ImagePaint::from_rid("fixture://paint-rfd");
                image.active = active;
                image.opacity = opacity;
                image.blend_mode = blend_mode;
                n::Paint::Image(image)
            }
        }
    }

    fn observe_paint(paint: &Self::Paint) -> PaintObservation {
        PaintObservation {
            kind: Self::observe_kind(paint),
            active: paint.active(),
            opacity_bits: paint.opacity().to_bits(),
            blend: Self::observe_blend(paint.blend_mode()),
            visible: paint.visible(),
        }
    }

    fn stack(paints: Vec<Self::Paint>) -> Self::Paints {
        n0_model::model::Paints::new(paints)
    }

    fn push(stack: &mut Self::Paints, paint: Self::Paint) {
        stack.push(paint);
    }

    fn stack_kinds(stack: &Self::Paints) -> Vec<PaintKind> {
        stack.iter().map(Self::observe_kind).collect()
    }

    fn stack_observations(stack: &Self::Paints) -> Vec<PaintObservation> {
        stack.iter().map(Self::observe_paint).collect()
    }

    fn stack_is_empty(stack: &Self::Paints) -> bool {
        stack.is_empty()
    }

    fn gradient_defaults() -> Vec<GradientObservation> {
        use n0_model::model as n;
        let n::LinearGradientPaint {
            active: linear_active,
            xy1,
            xy2,
            tile_mode: linear_tile,
            transform: linear_transform,
            stops: linear_stops,
            opacity: linear_opacity,
            blend_mode: linear_blend,
        } = n::LinearGradientPaint::default();
        let n::RadialGradientPaint {
            active: radial_active,
            transform: radial_transform,
            stops: radial_stops,
            opacity: radial_opacity,
            blend_mode: radial_blend,
            tile_mode: radial_tile,
        } = n::RadialGradientPaint::default();
        let n::SweepGradientPaint {
            active: sweep_active,
            transform: sweep_transform,
            stops: sweep_stops,
            opacity: sweep_opacity,
            blend_mode: sweep_blend,
        } = n::SweepGradientPaint::default();
        let n::DiamondGradientPaint {
            active: diamond_active,
            transform: diamond_transform,
            stops: diamond_stops,
            opacity: diamond_opacity,
            blend_mode: diamond_blend,
        } = n::DiamondGradientPaint::default();
        vec![
            Self::gradient(
                PaintKind::Linear,
                Some((xy1, xy2)),
                Some(linear_tile),
                linear_transform,
                &linear_stops,
                linear_active,
                linear_opacity,
                linear_blend,
            ),
            Self::gradient(
                PaintKind::Radial,
                None,
                Some(radial_tile),
                radial_transform,
                &radial_stops,
                radial_active,
                radial_opacity,
                radial_blend,
            ),
            Self::gradient(
                PaintKind::Sweep,
                None,
                None,
                sweep_transform,
                &sweep_stops,
                sweep_active,
                sweep_opacity,
                sweep_blend,
            ),
            Self::gradient(
                PaintKind::Diamond,
                None,
                None,
                diamond_transform,
                &diamond_stops,
                diamond_active,
                diamond_opacity,
                diamond_blend,
            ),
        ]
    }

    fn gradient_sentinels() -> Vec<GradientObservation> {
        use n0_model::math::Affine;
        use n0_model::model as n;

        let transform = Affine {
            a: 2.0,
            b: 4.0,
            c: 3.0,
            d: 6.0,
            e: 5.0,
            f: 7.0,
        };
        let stops = vec![n::GradientStop {
            offset: 0.375,
            color: n::Color(0x8040_2010),
        }];
        let n::LinearGradientPaint {
            active: linear_active,
            xy1,
            xy2,
            tile_mode: linear_tile,
            transform: linear_transform,
            stops: linear_stops,
            opacity: linear_opacity,
            blend_mode: linear_blend,
        } = n::LinearGradientPaint {
            active: false,
            xy1: n::Alignment(-0.75, 0.25),
            xy2: n::Alignment(0.625, -0.5),
            tile_mode: n::TileMode::Mirror,
            transform,
            stops: stops.clone(),
            opacity: 0.625,
            blend_mode: n::BlendMode::SoftLight,
        };
        let n::RadialGradientPaint {
            active: radial_active,
            transform: radial_transform,
            stops: radial_stops,
            opacity: radial_opacity,
            blend_mode: radial_blend,
            tile_mode: radial_tile,
        } = n::RadialGradientPaint {
            active: false,
            transform,
            stops: stops.clone(),
            opacity: 0.625,
            blend_mode: n::BlendMode::SoftLight,
            tile_mode: n::TileMode::Decal,
        };
        let n::SweepGradientPaint {
            active: sweep_active,
            transform: sweep_transform,
            stops: sweep_stops,
            opacity: sweep_opacity,
            blend_mode: sweep_blend,
        } = n::SweepGradientPaint {
            active: false,
            transform,
            stops: stops.clone(),
            opacity: 0.625,
            blend_mode: n::BlendMode::SoftLight,
        };
        let n::DiamondGradientPaint {
            active: diamond_active,
            transform: diamond_transform,
            stops: diamond_stops,
            opacity: diamond_opacity,
            blend_mode: diamond_blend,
        } = n::DiamondGradientPaint {
            active: false,
            transform,
            stops,
            opacity: 0.625,
            blend_mode: n::BlendMode::SoftLight,
        };

        vec![
            Self::gradient(
                PaintKind::Linear,
                Some((xy1, xy2)),
                Some(linear_tile),
                linear_transform,
                &linear_stops,
                linear_active,
                linear_opacity,
                linear_blend,
            ),
            Self::gradient(
                PaintKind::Radial,
                None,
                Some(radial_tile),
                radial_transform,
                &radial_stops,
                radial_active,
                radial_opacity,
                radial_blend,
            ),
            Self::gradient(
                PaintKind::Sweep,
                None,
                None,
                sweep_transform,
                &sweep_stops,
                sweep_active,
                sweep_opacity,
                sweep_blend,
            ),
            Self::gradient(
                PaintKind::Diamond,
                None,
                None,
                diamond_transform,
                &diamond_stops,
                diamond_active,
                diamond_opacity,
                diamond_blend,
            ),
        ]
    }

    fn stop() -> (u32, u32) {
        let stop = n0_model::model::GradientStop {
            offset: 0.25,
            color: n0_model::model::Color(0x8040_2010),
        };
        (stop.offset.to_bits(), stop.color.argb())
    }

    fn image_vocabulary() -> ImageVocabularyObservation {
        use n0_model::math::Affine;
        use n0_model::model as n;
        ImageVocabularyObservation {
            resources: [
                n::ResourceRef::Hash("hash".into()),
                n::ResourceRef::Rid("rid".into()),
            ]
            .into_iter()
            .map(Self::observe_resource)
            .collect(),
            fits: [
                n::ImagePaintFit::Fit(n::BoxFit::Contain),
                n::ImagePaintFit::Transform(Affine {
                    a: 2.0,
                    b: 4.0,
                    c: 3.0,
                    d: 6.0,
                    e: 5.0,
                    f: 7.0,
                }),
                n::ImagePaintFit::Tile(n::ImageTile {
                    scale: 0.625,
                    repeat: n::ImageRepeat::RepeatY,
                }),
            ]
            .into_iter()
            .map(Self::observe_image_fit)
            .collect(),
            object_fits: [
                n::BoxFit::Contain,
                n::BoxFit::Cover,
                n::BoxFit::Fill,
                n::BoxFit::None,
            ]
            .into_iter()
            .map(Self::observe_object_fit)
            .collect(),
            repeats: [
                n::ImageRepeat::RepeatX,
                n::ImageRepeat::RepeatY,
                n::ImageRepeat::Repeat,
            ]
            .into_iter()
            .map(Self::observe_repeat)
            .collect(),
        }
    }

    fn image_neutral_witness() -> ImageObservation {
        use n0_model::model as n;
        let image = match Self::paint(PaintKind::Image, true, 1.0, Blend::Normal) {
            n::Paint::Image(image) => image,
            _ => unreachable!(),
        };
        Self::observe_image(image)
    }

    fn image_sentinel() -> ImageObservation {
        use n0_model::model as n;
        Self::observe_image(n::ImagePaint {
            active: false,
            image: n::ResourceRef::Hash("hash-sentinel".into()),
            quarter_turns: 3,
            alignment: n::Alignment(-0.75, 0.25),
            fit: n::ImagePaintFit::Tile(n::ImageTile {
                scale: 0.625,
                repeat: n::ImageRepeat::RepeatY,
            }),
            opacity: 0.375,
            blend_mode: n::BlendMode::ColorBurn,
            filters: n::ImageFilters {
                exposure: 0.1,
                contrast: 0.2,
                saturation: 0.3,
                temperature: 0.4,
                tint: 0.5,
                highlights: 0.6,
                shadows: 0.7,
            },
        })
    }

    fn run_fill_states() -> Vec<RunFillState> {
        use n0_model::model as n;
        fn observe(run: &n::StyledTextRun) -> RunFillState {
            match &run.fills {
                None => RunFillState::Inherit,
                Some(fills) if fills.is_empty() => RunFillState::ExplicitEmpty,
                Some(fills) => RunFillState::Override(fills.len()),
            }
        }
        fn run(fills: Option<n::Paints>) -> n::StyledTextRun {
            n::StyledTextRun {
                start: 0,
                end: 1,
                style: n::TextStyleRec::default(),
                fills,
            }
        }
        vec![
            observe(&run(None)),
            observe(&run(Some(n::Paints::default()))),
            observe(&run(Some(n::Paints::new([Self::paint(
                PaintKind::Solid,
                true,
                1.0,
                Blend::Normal,
            )])))),
        ]
    }

    fn stroke_surface() -> StrokeObservation {
        use n0_model::model as n;
        fn observe_width(value: n::StrokeWidth) -> StrokeWidthObservation {
            match value {
                n::StrokeWidth::None => StrokeWidthObservation::None,
                n::StrokeWidth::Uniform(value) => StrokeWidthObservation::Uniform(value.to_bits()),
                n::StrokeWidth::Rectangular(value) => StrokeWidthObservation::Rectangular([
                    value.stroke_top_width.to_bits(),
                    value.stroke_right_width.to_bits(),
                    value.stroke_bottom_width.to_bits(),
                    value.stroke_left_width.to_bits(),
                ]),
            }
        }
        fn observe_align(value: n::StrokeAlign) -> StrokeAlignObservation {
            match value {
                n::StrokeAlign::Inside => StrokeAlignObservation::Inside,
                n::StrokeAlign::Center => StrokeAlignObservation::Center,
                n::StrokeAlign::Outside => StrokeAlignObservation::Outside,
            }
        }
        fn observe_cap(value: n::StrokeCap) -> StrokeCapObservation {
            match value {
                n::StrokeCap::Butt => StrokeCapObservation::Butt,
                n::StrokeCap::Round => StrokeCapObservation::Round,
                n::StrokeCap::Square => StrokeCapObservation::Square,
            }
        }
        fn observe_join(value: n::StrokeJoin) -> StrokeJoinObservation {
            match value {
                n::StrokeJoin::Miter => StrokeJoinObservation::Miter,
                n::StrokeJoin::Round => StrokeJoinObservation::Round,
                n::StrokeJoin::Bevel => StrokeJoinObservation::Bevel,
            }
        }
        let default = n::Stroke::default_for(&n::Payload::Shape {
            desc: n::ShapeDesc::Rect,
        })
        .expect("rect has a native stroke default");
        assert!(default.dash_array.is_none());
        let stroke = n::Stroke {
            paints: n::Paints::solid(n::Color::BLACK),
            width: n::StrokeWidth::Uniform(1.0),
            align: n::StrokeAlign::Inside,
            cap: n::StrokeCap::Butt,
            join: n::StrokeJoin::Miter,
            miter_limit: 5.0,
            dash_array: Some(vec![1.0, 2.0]),
        };
        let n::Stroke {
            paints,
            width,
            align,
            cap,
            join,
            miter_limit,
            dash_array,
        } = stroke.clone();
        assert_eq!(paints.len(), 1);
        assert_eq!(width, n::StrokeWidth::Uniform(1.0));
        assert_eq!(align, n::StrokeAlign::Inside);
        assert_eq!(cap, n::StrokeCap::Butt);
        assert_eq!(join, n::StrokeJoin::Miter);
        assert_eq!(miter_limit, 5.0);
        assert_eq!(dash_array, Some(vec![1.0, 2.0]));
        let mut second = stroke.clone();
        second.width = n::StrokeWidth::Uniform(2.0);
        second.paints = n::Paints::solid(n::Color(0xffff_0000));
        let mut node = n::Node::new(
            0,
            n::Header::new(n::SizeIntent::Fixed(10.0), n::SizeIntent::Fixed(10.0)),
            n::Payload::Text {
                content: "paint".into(),
                font_size: 16.0,
            },
        );
        node.strokes = vec![stroke.clone(), second];
        assert_eq!(node.strokes.len(), 2);
        assert_eq!(node.strokes[0].width, n::StrokeWidth::Uniform(1.0));
        assert_eq!(node.strokes[1].width, n::StrokeWidth::Uniform(2.0));

        StrokeObservation {
            widths: [
                n::StrokeWidth::None,
                stroke.width,
                n::StrokeWidth::Rectangular(n::RectangularStrokeWidth {
                    stroke_top_width: 1.0,
                    stroke_right_width: 2.0,
                    stroke_bottom_width: 3.0,
                    stroke_left_width: 4.0,
                }),
            ]
            .into_iter()
            .map(observe_width)
            .collect(),
            aligns: [
                n::StrokeAlign::Inside,
                n::StrokeAlign::Center,
                n::StrokeAlign::Outside,
            ]
            .into_iter()
            .map(observe_align)
            .collect(),
            caps: [
                n::StrokeCap::Butt,
                n::StrokeCap::Round,
                n::StrokeCap::Square,
            ]
            .into_iter()
            .map(observe_cap)
            .collect(),
            joins: [
                n::StrokeJoin::Miter,
                n::StrokeJoin::Round,
                n::StrokeJoin::Bevel,
            ]
            .into_iter()
            .map(observe_join)
            .collect(),
            default_miter_bits: default.miter_limit.to_bits(),
            dash_pattern_bits: stroke
                .dash_array
                .expect("sentinel keeps its dash pattern")
                .into_iter()
                .map(f32::to_bits)
                .collect(),
        }
    }
}

fn expected_gradient_defaults() -> Vec<GradientObservation> {
    vec![
        GradientObservation {
            kind: PaintKind::Linear,
            endpoints: Some((
                [(-1.0_f32).to_bits(), 0.0_f32.to_bits()],
                [1.0_f32.to_bits(), 0.0_f32.to_bits()],
            )),
            tile: Some(Tile::Clamp),
            transform_bits: [
                1.0_f32.to_bits(),
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
                1.0_f32.to_bits(),
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
            ],
            stop_count: 0,
            first_stop: None,
            active: true,
            opacity_bits: 1.0_f32.to_bits(),
            blend: Blend::Normal,
        },
        GradientObservation {
            kind: PaintKind::Radial,
            endpoints: None,
            tile: Some(Tile::Clamp),
            transform_bits: [
                1.0_f32.to_bits(),
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
                1.0_f32.to_bits(),
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
            ],
            stop_count: 0,
            first_stop: None,
            active: true,
            opacity_bits: 1.0_f32.to_bits(),
            blend: Blend::Normal,
        },
        GradientObservation {
            kind: PaintKind::Sweep,
            endpoints: None,
            tile: None,
            transform_bits: [
                1.0_f32.to_bits(),
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
                1.0_f32.to_bits(),
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
            ],
            stop_count: 0,
            first_stop: None,
            active: true,
            opacity_bits: 1.0_f32.to_bits(),
            blend: Blend::Normal,
        },
        GradientObservation {
            kind: PaintKind::Diamond,
            endpoints: None,
            tile: None,
            transform_bits: [
                1.0_f32.to_bits(),
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
                1.0_f32.to_bits(),
                0.0_f32.to_bits(),
                0.0_f32.to_bits(),
            ],
            stop_count: 0,
            first_stop: None,
            active: true,
            opacity_bits: 1.0_f32.to_bits(),
            blend: Blend::Normal,
        },
    ]
}

fn expected_gradient_sentinels() -> Vec<GradientObservation> {
    let transform_bits = [
        2.0_f32.to_bits(),
        4.0_f32.to_bits(),
        3.0_f32.to_bits(),
        6.0_f32.to_bits(),
        5.0_f32.to_bits(),
        7.0_f32.to_bits(),
    ];
    let common = |kind, endpoints, tile| GradientObservation {
        kind,
        endpoints,
        tile,
        transform_bits,
        stop_count: 1,
        first_stop: Some((0.375_f32.to_bits(), 0x8040_2010)),
        active: false,
        opacity_bits: 0.625_f32.to_bits(),
        blend: Blend::SoftLight,
    };
    vec![
        common(
            PaintKind::Linear,
            Some((
                [(-0.75_f32).to_bits(), 0.25_f32.to_bits()],
                [0.625_f32.to_bits(), (-0.5_f32).to_bits()],
            )),
            Some(Tile::Mirror),
        ),
        common(PaintKind::Radial, None, Some(Tile::Decal)),
        common(PaintKind::Sweep, None, None),
        common(PaintKind::Diamond, None, None),
    ]
}

fn expected_image_vocabulary() -> ImageVocabularyObservation {
    ImageVocabularyObservation {
        resources: vec![
            ResourceObservation::Hash("hash".into()),
            ResourceObservation::Rid("rid".into()),
        ],
        fits: vec![
            ImageFitObservation::Fit(ObjectFitObservation::Contain),
            ImageFitObservation::Transform([
                2.0_f32.to_bits(),
                4.0_f32.to_bits(),
                3.0_f32.to_bits(),
                6.0_f32.to_bits(),
                5.0_f32.to_bits(),
                7.0_f32.to_bits(),
            ]),
            ImageFitObservation::Tile {
                scale_bits: 0.625_f32.to_bits(),
                repeat: RepeatObservation::RepeatY,
            },
        ],
        object_fits: vec![
            ObjectFitObservation::Contain,
            ObjectFitObservation::Cover,
            ObjectFitObservation::Fill,
            ObjectFitObservation::None,
        ],
        repeats: vec![
            RepeatObservation::RepeatX,
            RepeatObservation::RepeatY,
            RepeatObservation::Repeat,
        ],
    }
}

fn expected_image_neutral_witness() -> ImageObservation {
    ImageObservation {
        resource: ResourceObservation::Rid("fixture://paint-rfd".into()),
        fit: ImageFitObservation::Fit(ObjectFitObservation::Cover),
        quarter_turns: 0,
        alignment_bits: [0.0_f32.to_bits(), 0.0_f32.to_bits()],
        filter_bits: [0.0_f32.to_bits(); 7],
        active: true,
        opacity_bits: 1.0_f32.to_bits(),
        blend: Blend::Normal,
    }
}

fn expected_image_sentinel() -> ImageObservation {
    ImageObservation {
        resource: ResourceObservation::Hash("hash-sentinel".into()),
        fit: ImageFitObservation::Tile {
            scale_bits: 0.625_f32.to_bits(),
            repeat: RepeatObservation::RepeatY,
        },
        quarter_turns: 3,
        alignment_bits: [(-0.75_f32).to_bits(), 0.25_f32.to_bits()],
        filter_bits: [
            0.1_f32.to_bits(),
            0.2_f32.to_bits(),
            0.3_f32.to_bits(),
            0.4_f32.to_bits(),
            0.5_f32.to_bits(),
            0.6_f32.to_bits(),
            0.7_f32.to_bits(),
        ],
        active: false,
        opacity_bits: 0.375_f32.to_bits(),
        blend: Blend::ColorBurn,
    }
}

fn expected_stroke_surface() -> StrokeObservation {
    StrokeObservation {
        widths: vec![
            StrokeWidthObservation::None,
            StrokeWidthObservation::Uniform(1.0_f32.to_bits()),
            StrokeWidthObservation::Rectangular([
                1.0_f32.to_bits(),
                2.0_f32.to_bits(),
                3.0_f32.to_bits(),
                4.0_f32.to_bits(),
            ]),
        ],
        aligns: vec![
            StrokeAlignObservation::Inside,
            StrokeAlignObservation::Center,
            StrokeAlignObservation::Outside,
        ],
        caps: vec![
            StrokeCapObservation::Butt,
            StrokeCapObservation::Round,
            StrokeCapObservation::Square,
        ],
        joins: vec![
            StrokeJoinObservation::Miter,
            StrokeJoinObservation::Round,
            StrokeJoinObservation::Bevel,
        ],
        default_miter_bits: 4.0_f32.to_bits(),
        dash_pattern_bits: vec![1.0_f32.to_bits(), 2.0_f32.to_bits()],
    }
}

fn assert_common_laws<V: PaintVocabulary>() {
    for argb in [
        0x0000_0000,
        0xffff_ffff,
        0x8040_2010,
        0x0102_0304,
        0xfeff_007f,
    ] {
        let (rgba, round_trip) = V::color_from_argb(argb);
        assert_eq!(round_trip, argb, "{} ARGB round trip", V::name());
        assert_eq!(
            rgba,
            [
                ((argb >> 16) & 0xff) as u8,
                ((argb >> 8) & 0xff) as u8,
                (argb & 0xff) as u8,
                (argb >> 24) as u8,
            ],
            "{} straight channel projection",
            V::name()
        );
    }

    for alpha in 0..=u8::MAX {
        assert_eq!(
            V::byte_float_byte(alpha),
            alpha,
            "{} byte -> unit float -> rounded byte for {alpha}",
            V::name()
        );
    }

    assert_eq!(V::blends(), Blend::ALL, "{} blend vocabulary", V::name());
    for blend in Blend::ALL {
        assert_eq!(
            V::observe_paint(&V::paint(PaintKind::Linear, true, 1.0, blend)).blend,
            blend,
            "{} blend constructor for {blend:?}",
            V::name()
        );
    }
    assert_eq!(
        V::tiles(),
        [Tile::Clamp, Tile::Repeated, Tile::Mirror, Tile::Decal],
        "{} tile vocabulary",
        V::name()
    );

    for kind in PaintKind::ALL {
        let paint = V::paint(kind, true, 1.0, Blend::Multiply);
        assert_eq!(
            V::observe_paint(&paint),
            PaintObservation {
                kind,
                active: true,
                opacity_bits: 1.0_f32.to_bits(),
                blend: Blend::Multiply,
                visible: true,
            },
            "{} common state for {kind:?}",
            V::name()
        );
        let inactive = V::paint(kind, false, 1.0, Blend::Normal);
        assert!(
            !V::observe_paint(&inactive).visible,
            "{} inactive",
            V::name()
        );
        let transparent = V::paint(kind, true, 0.0, Blend::Normal);
        assert!(
            !V::observe_paint(&transparent).visible,
            "{} zero-opacity {kind:?}",
            V::name()
        );
    }

    for kind in PaintKind::ALL
        .into_iter()
        .filter(|kind| *kind != PaintKind::Solid)
    {
        for opacity in [-0.25, f32::NAN] {
            let paint = V::paint(kind, true, opacity, Blend::Normal);
            assert!(
                !V::observe_paint(&paint).visible,
                "{} {kind:?} opacity {opacity:?} must not be visible",
                V::name()
            );
        }
    }
    let visible = V::paint(PaintKind::Linear, true, 0.25, Blend::Normal);
    assert!(V::observe_paint(&visible).visible);

    let mut stack = V::stack(vec![
        V::paint(PaintKind::Solid, true, 1.0, Blend::Multiply),
        V::paint(PaintKind::Linear, true, 1.0, Blend::Screen),
    ]);
    V::push(
        &mut stack,
        V::paint(PaintKind::Image, true, 1.0, Blend::Overlay),
    );
    assert_eq!(
        V::stack_kinds(&stack),
        vec![PaintKind::Solid, PaintKind::Linear, PaintKind::Image],
        "{} stores entry zero bottommost and push appends on top",
        V::name()
    );
    let observations = V::stack_observations(&stack);
    assert_eq!(
        observations
            .iter()
            .map(|paint| paint.blend)
            .collect::<Vec<_>>(),
        vec![Blend::Multiply, Blend::Screen, Blend::Overlay]
    );
    let filtering = V::stack(vec![
        V::paint(PaintKind::Solid, true, 1.0, Blend::Normal),
        V::paint(PaintKind::Linear, true, 0.0, Blend::Normal),
        V::paint(PaintKind::Image, true, 1.0, Blend::Normal),
    ]);
    let survivors = V::stack_observations(&filtering)
        .into_iter()
        .filter(|paint| paint.visible)
        .map(|paint| paint.kind)
        .collect::<Vec<_>>();
    assert_eq!(survivors, vec![PaintKind::Solid, PaintKind::Image]);
    assert!(V::stack_is_empty(&V::stack(vec![])));

    assert_eq!(V::gradient_defaults(), expected_gradient_defaults());
    assert_eq!(V::gradient_sentinels(), expected_gradient_sentinels());
    assert_eq!(V::stop(), (0.25_f32.to_bits(), 0x8040_2010));
    assert_eq!(V::image_vocabulary(), expected_image_vocabulary());
    assert_eq!(V::image_neutral_witness(), expected_image_neutral_witness());
    assert_eq!(V::image_sentinel(), expected_image_sentinel());
    assert_eq!(
        V::run_fill_states(),
        vec![
            RunFillState::Inherit,
            RunFillState::ExplicitEmpty,
            RunFillState::Override(1),
        ],
        "{} run fills retain absent, explicit-empty, and override",
        V::name()
    );
}

#[test]
fn cg_satisfies_the_shared_laws_and_keeps_its_partial_text_surface_visible() {
    assert_common_laws::<Cg>();
    assert_eq!(Cg::stroke_surface(), expected_stroke_surface());

    let run = cg::StyledTextRun {
        start: 0,
        end: 1,
        style: cg::TextStyleRec::from_font("sans-serif", 16.0),
        fills: None,
        strokes: Some(cg::Paints::new([Cg::paint(
            PaintKind::Solid,
            true,
            1.0,
            Blend::Normal,
        )])),
        stroke_width: Some(2.0),
        stroke_align: Some(cg::StrokeAlign::Center),
    };
    let cg::StyledTextRun {
        start: _,
        end: _,
        style: _,
        fills: _,
        strokes,
        stroke_width,
        stroke_align,
    } = run;
    assert_eq!(strokes.map(|paints| paints.len()), Some(1));
    assert_eq!(stroke_width, Some(2.0));
    assert_eq!(stroke_align, Some(cg::StrokeAlign::Center));

    let mut decoration = cg::TextDecorationRec::underline();
    decoration.text_decoration_color = Some(cg::CGColor::RED);
    let cg::TextDecorationRec {
        text_decoration_line: _,
        text_decoration_color,
        text_decoration_style: _,
        text_decoration_skip_ink: _,
        text_decoration_thickness: _,
    } = decoration;
    assert_eq!(text_decoration_color, Some(cg::CGColor::RED));
}

#[test]
fn n0_model_satisfies_the_shared_laws_and_keeps_its_sparse_text_surface_visible() {
    assert_common_laws::<N0>();
    assert_eq!(N0::stroke_surface(), expected_stroke_surface());

    let run = n0_model::model::StyledTextRun {
        start: 0,
        end: 1,
        style: n0_model::model::TextStyleRec::default(),
        fills: Some(n0_model::model::Paints::solid(
            n0_model::model::Color::BLACK,
        )),
    };
    let n0_model::model::StyledTextRun {
        start: _,
        end: _,
        style: _,
        fills,
    } = run;
    assert_eq!(fills.map(|paints| paints.len()), Some(1));
}

#[test]
fn every_expected_gap_and_pinned_amendment_is_in_the_report() {
    for id in CG_GAPS.iter().chain(N0_GAPS).chain(PINNED_AMENDMENTS) {
        assert!(
            GAP_REPORT.contains(id),
            "paint vocabulary gap report must name {id}"
        );
    }
}

#[test]
fn cg_color_ramp_helpers_never_mint_a_non_finite_stop() {
    for stops in [
        cg::LinearGradientPaint::from_colors(vec![]).stops,
        cg::LinearGradientPaint::from_colors(vec![cg::CGColor::RED]).stops,
        cg::RadialGradientPaint::from_colors(vec![]).stops,
        cg::RadialGradientPaint::from_colors(vec![cg::CGColor::RED]).stops,
    ] {
        assert!(stops.iter().all(|stop| stop.offset.is_finite()));
    }
    assert_eq!(
        cg::LinearGradientPaint::from_colors(vec![cg::CGColor::RED]).stops[0].offset,
        0.0
    );
    assert_eq!(
        cg::RadialGradientPaint::from_colors(vec![cg::CGColor::RED]).stops[0].offset,
        0.0
    );

    for stops in [
        cg::LinearGradientPaint::from_colors(vec![cg::CGColor::RED, cg::CGColor::GREEN]).stops,
        cg::RadialGradientPaint::from_colors(vec![cg::CGColor::RED, cg::CGColor::GREEN]).stops,
    ] {
        assert_eq!(
            stops
                .iter()
                .map(|stop| (stop.offset, stop.color))
                .collect::<Vec<_>>(),
            vec![(0.0, cg::CGColor::RED), (1.0, cg::CGColor::GREEN)]
        );
    }
    for stops in [
        cg::LinearGradientPaint::from_colors(vec![
            cg::CGColor::RED,
            cg::CGColor::GREEN,
            cg::CGColor::BLUE,
        ])
        .stops,
        cg::RadialGradientPaint::from_colors(vec![
            cg::CGColor::RED,
            cg::CGColor::GREEN,
            cg::CGColor::BLUE,
        ])
        .stops,
    ] {
        assert_eq!(
            stops
                .iter()
                .map(|stop| (stop.offset, stop.color))
                .collect::<Vec<_>>(),
            vec![
                (0.0, cg::CGColor::RED),
                (0.5, cg::CGColor::GREEN),
                (1.0, cg::CGColor::BLUE),
            ]
        );
    }
}
