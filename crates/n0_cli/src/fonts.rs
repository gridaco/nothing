//! The host's font declarations:
//! `--font FAMILY=PATH@sha256:HEX[;weight=N][;style=normal|italic][;stretch=POINT]`.
//!
//! A family name is not a font identity — the same name resolves to
//! different bytes on different machines — so a declaration carries the
//! digest of the bytes it means, and the host **verifies before rendering**.
//! A mismatch refuses; it never becomes a silently different pixel.
//!
//! This is the host side of the hermetic environment the ratified
//! [text-oracle method](../../../docs/wg/consolidation/text-oracle.md)
//! requires. The engine never reads a font file and never consults an
//! ambient font database: the environment it resolves against is exactly
//! what was declared here, and an undeclared family refuses by name.

use std::path::Path;
use std::sync::Arc;

use sha2::{Digest, Sha256};

/// One parsed `--font` declaration, before its bytes are read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FontDeclaration {
    pub(crate) family: String,
    pub(crate) path: String,
    /// Lowercase hex SHA-256 of the bytes this declaration means.
    pub(crate) digest: String,
    pub(crate) face_descriptor: textlayout::StaticFaceDescriptor,
}

/// Parse one hash-pinned font resource plus its optional exact static face
/// facts. Omitted facts are the normal 400/100% tuple.
///
/// The family is everything before the first `=`, and the digest is the first
/// 64 characters after the last `@sha256:`. Any remaining bytes are the
/// semicolon-prefixed descriptor list, so a path may contain `=` and `@` and
/// only the exact digest marker terminates it.
pub(crate) fn parse_declaration(spec: &str) -> Result<FontDeclaration, String> {
    const MARKER: &str = "@sha256:";
    let Some((family, rest)) = spec.split_once('=') else {
        return Err(format!(
            "font declaration {spec:?} must look like FAMILY=PATH@sha256:HEX[;weight=N][;style=normal|italic][;stretch=POINT]"
        ));
    };
    if family.is_empty() {
        return Err(format!("font declaration {spec:?} names no family"));
    }
    let Some(marker_at) = rest.rfind(MARKER) else {
        return Err(format!(
            "font declaration {spec:?} carries no @sha256: digest — a family name is not a font \
             identity, so the bytes it means are declared and verified"
        ));
    };
    let path = &rest[..marker_at];
    let declaration = &rest[marker_at + MARKER.len()..];
    if path.is_empty() {
        return Err(format!("font declaration {spec:?} names no path"));
    }
    if declaration.len() < 64 {
        return Err(format!(
            "font declaration {spec:?} carries {declaration:?}, which is not a 64-character hex SHA-256"
        ));
    }
    let (digest, descriptor_suffix) = declaration.split_at(64);
    if !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "font declaration {spec:?} carries {digest:?}, which is not a 64-character hex SHA-256"
        ));
    }
    let face_descriptor = parse_face_descriptor(spec, descriptor_suffix)?;
    Ok(FontDeclaration {
        family: family.to_string(),
        path: path.to_string(),
        digest: digest.to_ascii_lowercase(),
        face_descriptor,
    })
}

fn parse_face_descriptor(
    spec: &str,
    suffix: &str,
) -> Result<textlayout::StaticFaceDescriptor, String> {
    if suffix.is_empty() {
        return Ok(textlayout::StaticFaceDescriptor::NORMAL);
    }
    let Some(descriptors) = suffix.strip_prefix(';') else {
        return Err(format!(
            "font declaration {spec:?} has bytes after its 64-character SHA-256 without a ';' descriptor separator"
        ));
    };
    if descriptors.is_empty() {
        return Err(format!(
            "font declaration {spec:?} has an empty face descriptor"
        ));
    }

    let mut weight = None;
    let mut style = None;
    let mut stretch = None;
    for field in descriptors.split(';') {
        let Some((name, value)) = field.split_once('=') else {
            return Err(format!(
                "font declaration {spec:?} has malformed face descriptor {field:?}"
            ));
        };
        match name {
            "weight" if weight.is_none() => {
                let value = value.parse::<u16>().map_err(|_| {
                    format!("font declaration {spec:?} has invalid static weight {value:?}")
                })?;
                weight = Some(textlayout::FontWeight::new(value).map_err(|error| {
                    format!("font declaration {spec:?} has invalid static {error}")
                })?);
            }
            "style" if style.is_none() => {
                style = Some(match value {
                    "normal" => textlayout::FontStyle::Normal,
                    "italic" => textlayout::FontStyle::Italic,
                    _ => {
                        return Err(format!(
                            "font declaration {spec:?} has unsupported static style {value:?}; expected normal or italic"
                        ));
                    }
                });
            }
            "stretch" if stretch.is_none() => {
                stretch = Some(parse_stretch(value).ok_or_else(|| {
                    format!("font declaration {spec:?} has unsupported static stretch {value:?}")
                })?);
            }
            "weight" | "style" | "stretch" => {
                return Err(format!(
                    "font declaration {spec:?} repeats face descriptor {name:?}"
                ));
            }
            _ => {
                return Err(format!(
                    "font declaration {spec:?} has unknown face descriptor {name:?}"
                ));
            }
        }
    }

    Ok(textlayout::StaticFaceDescriptor::new(
        weight.unwrap_or(textlayout::FontWeight::NORMAL),
        stretch.unwrap_or(textlayout::FontStretch::Normal),
        style.unwrap_or(textlayout::FontStyle::Normal),
    ))
}

fn parse_stretch(value: &str) -> Option<textlayout::FontStretch> {
    Some(match value {
        "ultra-condensed" | "50%" => textlayout::FontStretch::UltraCondensed,
        "extra-condensed" | "62.5%" => textlayout::FontStretch::ExtraCondensed,
        "condensed" | "75%" => textlayout::FontStretch::Condensed,
        "semi-condensed" | "87.5%" => textlayout::FontStretch::SemiCondensed,
        "normal" | "100%" => textlayout::FontStretch::Normal,
        "semi-expanded" | "112.5%" => textlayout::FontStretch::SemiExpanded,
        "expanded" | "125%" => textlayout::FontStretch::Expanded,
        "extra-expanded" | "150%" => textlayout::FontStretch::ExtraExpanded,
        "ultra-expanded" | "200%" => textlayout::FontStretch::UltraExpanded,
        _ => return None,
    })
}

/// Read and verify every declaration into the engine's font environment.
///
/// Verification is not advisory: bytes whose digest differs from the
/// declaration are refused here, before a frame exists, so no render can
/// proceed against a font the host did not mean.
pub(crate) fn load_environment(
    declarations: &[FontDeclaration],
) -> Result<textlayout::Environment, String> {
    let mut resources = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        let bytes = std::fs::read(Path::new(&declaration.path))
            .map_err(|error| format!("cannot read font {}: {error}", declaration.path))?;
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != declaration.digest {
            return Err(format!(
                "font {} is not the declared identity: expected sha256 {}, read {actual}",
                declaration.path, declaration.digest
            ));
        }
        let mut digest = [0u8; 32];
        for (index, slot) in digest.iter_mut().enumerate() {
            *slot = u8::from_str_radix(&actual[index * 2..index * 2 + 2], 16)
                .expect("hex from a hex formatter");
        }
        resources.push(textlayout::FontResource {
            key: textlayout::FontKey::new(digest),
            family: declaration.family.clone(),
            face_descriptor: declaration.face_descriptor,
            face_index: 0,
            bytes: Arc::from(bytes),
        });
    }
    Ok(textlayout::Environment::new(resources))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEX: &str = "b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448";

    #[test]
    fn a_well_formed_declaration_parses() {
        let parsed = parse_declaration(&format!("Ahem=fixtures/ahem.ttf@sha256:{HEX}")).unwrap();
        assert_eq!(
            parsed,
            FontDeclaration {
                family: "Ahem".to_string(),
                path: "fixtures/ahem.ttf".to_string(),
                digest: HEX.to_string(),
                face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
            }
        );
    }

    #[test]
    fn a_path_may_carry_the_delimiters() {
        // Only the *last* `@sha256:` terminates the path, and only the first
        // `=` ends the family — so awkward real paths still parse.
        let parsed =
            parse_declaration(&format!("My Font=/tmp/a=b@1/font.ttf@sha256:{HEX}")).unwrap();
        assert_eq!(parsed.family, "My Font");
        assert_eq!(parsed.path, "/tmp/a=b@1/font.ttf");
    }

    #[test]
    fn exact_static_face_descriptors_parse_without_order_or_hidden_defaults() {
        let parsed = parse_declaration(&format!(
            "Face=f.ttf@sha256:{HEX};style=italic;stretch=75%;weight=700"
        ))
        .unwrap();
        assert_eq!(
            parsed.face_descriptor,
            textlayout::StaticFaceDescriptor::new(
                textlayout::FontWeight::new(700).unwrap(),
                textlayout::FontStretch::Condensed,
                textlayout::FontStyle::Italic,
            )
        );

        let keyword =
            parse_declaration(&format!("Face=f.ttf@sha256:{HEX};stretch=condensed")).unwrap();
        assert_eq!(
            keyword.face_descriptor,
            textlayout::StaticFaceDescriptor::new(
                textlayout::FontWeight::NORMAL,
                textlayout::FontStretch::Condensed,
                textlayout::FontStyle::Normal,
            )
        );
    }

    #[test]
    fn malformed_or_wider_face_descriptors_refuse_at_the_host_boundary() {
        for suffix in [
            ";weight=0",
            ";weight=400.5",
            ";style=oblique",
            ";stretch=80%",
            ";weight=700;weight=400",
            ";axis=wght",
            ";",
        ] {
            let spec = format!("Face=f.ttf@sha256:{HEX}{suffix}");
            assert!(parse_declaration(&spec).is_err(), "{spec}");
        }
    }

    #[test]
    fn an_undeclared_digest_refuses() {
        let error = parse_declaration("Ahem=fixtures/ahem.ttf").unwrap_err();
        assert!(error.contains("a family name is not a font identity"));
    }

    #[test]
    fn a_malformed_digest_refuses() {
        for spec in [
            format!("Ahem=f.ttf@sha256:{}", &HEX[..63]),
            format!("Ahem=f.ttf@sha256:{}z", &HEX[..63]),
            "Ahem=f.ttf@sha256:".to_string(),
        ] {
            let error = parse_declaration(&spec).unwrap_err();
            assert!(error.contains("hex SHA-256"), "{spec}: {error}");
        }
    }

    #[test]
    fn an_empty_family_or_path_refuses() {
        assert!(parse_declaration(&format!("=f.ttf@sha256:{HEX}")).is_err());
        assert!(parse_declaration(&format!("Ahem=@sha256:{HEX}")).is_err());
    }

    #[test]
    fn the_pinned_gate_font_verifies_and_a_wrong_digest_does_not() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = root
            .join("fixtures/web-first/fonts/ahem.ttf")
            .display()
            .to_string();

        let environment = load_environment(&[FontDeclaration {
            family: "Ahem".to_string(),
            path: path.clone(),
            digest: HEX.to_string(),
            face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
        }])
        .expect("the pinned bytes match their recorded digest");
        assert_eq!(environment.fonts().len(), 1);
        assert_eq!(environment.fonts()[0].family, "Ahem");

        let wrong = "0".repeat(64);
        let error = load_environment(&[FontDeclaration {
            family: "Ahem".to_string(),
            path,
            digest: wrong,
            face_descriptor: textlayout::StaticFaceDescriptor::NORMAL,
        }])
        .expect_err("a font that is not the declared identity must refuse before rendering");
        assert!(error.contains("is not the declared identity"));
    }
}
