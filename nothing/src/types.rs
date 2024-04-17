pub struct CornerRadius {
    pub topleft: f32,
    pub topright: f32,
    pub bottomright: f32,
    pub bottomleft: f32,
}

pub struct RGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

pub struct Size {
    pub x: f32,
    pub y: f32,
}

pub struct RectangleNode {
    pub x: f32,
    pub y: f32,
    pub color: RGBA,
    pub sigma: f32,
    pub size: Size,
    pub corner_radius: CornerRadius,
}
