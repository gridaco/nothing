mod types;

pub const rect_1: types::RectangleNode = types::RectangleNode {
    x: 0.0,
    y: 0.0,
    color: types::RGBA {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    },
    sigma: 0.0,
    size: types::Size { x: 100.0, y: 100.0 },
    corner_radius: types::CornerRadius {
        topleft: 0.0,
        topright: 0.0,
        bottomright: 0.0,
        bottomleft: 0.0,
    },
};
