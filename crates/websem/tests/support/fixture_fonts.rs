//! Declared font environments needed by refusal fixtures.

const ALLERTA_BYTES: &[u8] =
    include_bytes!("../../../../fixtures/fonts/Allerta/Allerta-Regular.ttf");
const ALLERTA_SHA256: [u8; 32] = [
    0x16, 0xd6, 0x91, 0x52, 0x27, 0xc7, 0x56, 0x07, 0x25, 0xc0, 0x37, 0xc9, 0xc9, 0x31, 0x63, 0xcb,
    0xa5, 0x36, 0x7c, 0x3e, 0xf4, 0xcf, 0x2e, 0xc1, 0x2b, 0xf4, 0x0b, 0x9e, 0xb2, 0x98, 0x4a, 0x6b,
];
const PT_SERIF_BYTES: &[u8] =
    include_bytes!("../../../../fixtures/fonts/PT_Serif/PTSerif-Regular.ttf");
const PT_SERIF_SHA256: [u8; 32] = [
    0x13, 0xd9, 0xf8, 0x2f, 0x41, 0xfc, 0xd7, 0xd2, 0x81, 0x3d, 0xc0, 0xa4, 0x4a, 0x96, 0x39, 0xde,
    0xc0, 0xc1, 0xe9, 0xa9, 0x22, 0xab, 0x96, 0xc7, 0xde, 0x8d, 0xec, 0x46, 0x7c, 0x3d, 0xec, 0x55,
];

pub(crate) fn unsupported_environment(id: &str) -> textlayout::Environment {
    let resource = match id {
        "svg-text-combining-sequence" | "svg-text-geometry-grid" => textlayout::FontResource {
            key: textlayout::FontKey::new(ALLERTA_SHA256),
            family: "Allerta".to_string(),
            face_index: 0,
            bytes: std::sync::Arc::from(ALLERTA_BYTES),
        },
        "svg-text-cluster-mapping" => textlayout::FontResource {
            key: textlayout::FontKey::new(PT_SERIF_SHA256),
            family: "PT Serif".to_string(),
            face_index: 0,
            bytes: std::sync::Arc::from(PT_SERIF_BYTES),
        },
        _ => return textlayout::Environment::default(),
    };
    textlayout::Environment::new(vec![resource])
}
