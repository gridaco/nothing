//! Declared font environments needed by refusal fixtures.

const ALLERTA_BYTES: &[u8] =
    include_bytes!("../../../../fixtures/fonts/Allerta/Allerta-Regular.ttf");
const ALLERTA_SHA256: [u8; 32] = [
    0x16, 0xd6, 0x91, 0x52, 0x27, 0xc7, 0x56, 0x07, 0x25, 0xc0, 0x37, 0xc9, 0xc9, 0x31, 0x63, 0xcb,
    0xa5, 0x36, 0x7c, 0x3e, 0xf4, 0xcf, 0x2e, 0xc1, 0x2b, 0xf4, 0x0b, 0x9e, 0xb2, 0x98, 0x4a, 0x6b,
];

pub(crate) fn unsupported_environment(id: &str) -> textlayout::Environment {
    if id != "svg-text-geometry-grid" {
        return textlayout::Environment::default();
    }
    textlayout::Environment::new(vec![textlayout::FontResource {
        key: textlayout::FontKey::new(ALLERTA_SHA256),
        family: "Allerta".to_string(),
        face_index: 0,
        bytes: std::sync::Arc::from(ALLERTA_BYTES),
    }])
}
