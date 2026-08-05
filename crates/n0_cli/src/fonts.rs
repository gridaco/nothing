//! The host's font declarations: `--font FAMILY=PATH@sha256:HEX`.
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
}

/// Parse one `FAMILY=PATH@sha256:HEX`.
///
/// The family is everything before the first `=`, the digest everything
/// after the last `@sha256:` — so a path may contain `=` and `@`, and only
/// the exact digest marker terminates it.
pub(crate) fn parse_declaration(spec: &str) -> Result<FontDeclaration, String> {
    const MARKER: &str = "@sha256:";
    let Some((family, rest)) = spec.split_once('=') else {
        return Err(format!(
            "font declaration {spec:?} must look like FAMILY=PATH@sha256:HEX"
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
    let digest = &rest[marker_at + MARKER.len()..];
    if path.is_empty() {
        return Err(format!("font declaration {spec:?} names no path"));
    }
    if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "font declaration {spec:?} carries {digest:?}, which is not a 64-character hex SHA-256"
        ));
    }
    Ok(FontDeclaration {
        family: family.to_string(),
        path: path.to_string(),
        digest: digest.to_ascii_lowercase(),
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
        }])
        .expect("the pinned bytes match their recorded digest");
        assert_eq!(environment.fonts().len(), 1);
        assert_eq!(environment.fonts()[0].family, "Ahem");

        let wrong = "0".repeat(64);
        let error = load_environment(&[FontDeclaration {
            family: "Ahem".to_string(),
            path,
            digest: wrong,
        }])
        .expect_err("a font that is not the declared identity must refuse before rendering");
        assert!(error.contains("is not the declared identity"));
    }
}
