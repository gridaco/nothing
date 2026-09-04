//! Declared font environments needed by refusal fixtures.

const AHEM_BYTES: &[u8] = include_bytes!("../../../../fixtures/web-first/fonts/ahem.ttf");
const AHEM_SHA256: [u8; 32] = [
    0xb7, 0x19, 0xec, 0xb3, 0x1c, 0x5b, 0x21, 0xfc, 0x57, 0x3c, 0x03, 0xf6, 0x42, 0x1c, 0x74, 0xac,
    0x63, 0xc2, 0x71, 0xa5, 0xa3, 0xff, 0x84, 0x1e, 0x34, 0xf9, 0x70, 0x5f, 0xb9, 0x4b, 0x84, 0x48,
];
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
const BUNGEE_BYTES: &[u8] = include_bytes!("../../../../fixtures/fonts/Bungee/Bungee-Regular.ttf");
const BUNGEE_SHA256: [u8; 32] = [
    0xb9, 0x0c, 0x3c, 0xa4, 0x43, 0x71, 0x3b, 0x07, 0x0c, 0xb1, 0xde, 0xc6, 0xa3, 0xbb, 0x1e, 0xf7,
    0x57, 0x2c, 0x2b, 0x56, 0x5c, 0x43, 0x1d, 0x9a, 0x85, 0xd7, 0x4b, 0xbf, 0xa0, 0x7e, 0x24, 0xcc,
];

pub(crate) fn unsupported_environment(id: &str) -> textlayout::Environment {
    if id == "svg-text-family-ambiguous" {
        return textlayout::Environment::new(vec![
            textlayout::FontResource {
                key: textlayout::FontKey::new(AHEM_SHA256),
                family: "Duo".to_string(),
                face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
                face_index: 0,
                bytes: std::sync::Arc::from(AHEM_BYTES),
            },
            textlayout::FontResource {
                key: textlayout::FontKey::new(BUNGEE_SHA256),
                family: "Duo".to_string(),
                face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
                face_index: 0,
                bytes: std::sync::Arc::from(BUNGEE_BYTES),
            },
        ]);
    }
    if matches!(
        id,
        "svg-text-face-synthesis-required"
            | "svg-text-family-generic"
            | "svg-text-family-missing-glyph-fallback"
    ) {
        return textlayout::Environment::new(vec![
            textlayout::FontResource {
                key: textlayout::FontKey::new(AHEM_SHA256),
                family: "Ahem".to_string(),
                face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
                face_index: 0,
                bytes: std::sync::Arc::from(AHEM_BYTES),
            },
            textlayout::FontResource {
                key: textlayout::FontKey::new(BUNGEE_SHA256),
                family: "Bungee".to_string(),
                face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
                face_index: 0,
                bytes: std::sync::Arc::from(BUNGEE_BYTES),
            },
        ]);
    }
    let resource = match id {
        "svg-text-combining-missing-glyph" => textlayout::FontResource {
            key: textlayout::FontKey::new(AHEM_SHA256),
            family: "Ahem".to_string(),
            face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
            face_index: 0,
            bytes: std::sync::Arc::from(AHEM_BYTES),
        },
        "svg-text-combining-malformed" | "svg-text-combining-unlisted-mark" => {
            textlayout::FontResource {
                key: textlayout::FontKey::new(BUNGEE_SHA256),
                family: "Bungee".to_string(),
                face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
                face_index: 0,
                bytes: std::sync::Arc::from(BUNGEE_BYTES),
            }
        }
        "svg-text-geometry-grid" => textlayout::FontResource {
            key: textlayout::FontKey::new(ALLERTA_SHA256),
            family: "Allerta".to_string(),
            face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
            face_index: 0,
            bytes: std::sync::Arc::from(ALLERTA_BYTES),
        },
        "svg-text-cluster-mapping" => textlayout::FontResource {
            key: textlayout::FontKey::new(PT_SERIF_SHA256),
            family: "PT Serif".to_string(),
            face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
            face_index: 0,
            bytes: std::sync::Arc::from(PT_SERIF_BYTES),
        },
        _ => return textlayout::Environment::default(),
    };
    textlayout::Environment::new(vec![resource])
}
