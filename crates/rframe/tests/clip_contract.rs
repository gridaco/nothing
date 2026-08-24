use math2::Rectangle;
use math2::transform::AffineTransform;
use rframe::{
    ClipGeometry, ClipGeometryError, ClipLayer, ClipLayerError, ClipPath, ClipPathError, Geometry,
    MAX_CLIP_GEOMETRIES_PER_LAYER, MAX_CLIP_LAYERS,
};

fn rect(x: f32, y: f32, width: f32, height: f32) -> ClipGeometry {
    ClipGeometry::new(
        AffineTransform::identity(),
        Geometry::Rect(Rectangle::from_xywh(x, y, width, height)),
    )
    .expect("test clip rectangle is resolved")
}

#[test]
fn a_layer_is_a_union_and_layers_intersect() {
    let first = ClipLayer::new(vec![
        rect(0.0, 0.0, 10.0, 10.0),
        rect(20.0, 0.0, 10.0, 10.0),
    ])
    .unwrap();
    assert_eq!(
        first.bounds(),
        Some(Rectangle::from_xywh(0.0, 0.0, 30.0, 10.0))
    );

    let second = ClipLayer::new(vec![rect(5.0, -5.0, 20.0, 20.0)]).unwrap();
    let clip = ClipPath::new(vec![first, second]).unwrap();
    assert_eq!(
        clip.bounds(),
        Some(Rectangle::from_xywh(5.0, 0.0, 20.0, 10.0))
    );
}

#[test]
fn an_empty_layer_is_a_valid_clip_all_fact() {
    let clip = ClipPath::new(vec![ClipLayer::new(Vec::new()).unwrap()]).unwrap();
    assert_eq!(clip.layers().len(), 1);
    assert!(clip.layers()[0].geometries().is_empty());
    assert_eq!(clip.bounds(), None);
}

#[test]
fn geometry_checks_local_and_transformed_finiteness() {
    assert_eq!(
        ClipGeometry::new(
            AffineTransform::identity(),
            Geometry::Rect(Rectangle::from_xywh(0.0, 0.0, -1.0, 1.0)),
        ),
        Err(ClipGeometryError::InvalidGeometry)
    );
    assert_eq!(
        ClipGeometry::new(
            AffineTransform::from_acebdf(1.0, 0.0, f32::INFINITY, 0.0, 1.0, 0.0),
            Geometry::Rect(Rectangle::from_xywh(0.0, 0.0, 1.0, 1.0)),
        ),
        Err(ClipGeometryError::NonFiniteTransform)
    );
    assert_eq!(
        ClipGeometry::new(
            AffineTransform::from_acebdf(f32::MAX, 0.0, 0.0, 0.0, f32::MAX, 0.0),
            Geometry::Rect(Rectangle::from_xywh(0.0, 0.0, 2.0, 2.0)),
        ),
        Err(ClipGeometryError::NonFiniteBounds)
    );
}

#[test]
fn construction_caps_both_dimensions() {
    let geometries = (0..=MAX_CLIP_GEOMETRIES_PER_LAYER)
        .map(|index| rect(index as f32, 0.0, 1.0, 1.0))
        .collect::<Vec<_>>();
    assert_eq!(
        ClipLayer::new(geometries),
        Err(ClipLayerError {
            count: MAX_CLIP_GEOMETRIES_PER_LAYER + 1,
        })
    );

    let layers = (0..=MAX_CLIP_LAYERS)
        .map(|_| ClipLayer::new(Vec::new()).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        ClipPath::new(layers),
        Err(ClipPathError::TooManyLayers {
            count: MAX_CLIP_LAYERS + 1,
        })
    );
    assert_eq!(ClipPath::new(Vec::new()), Err(ClipPathError::NoLayers));
}
