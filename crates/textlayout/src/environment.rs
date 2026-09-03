//! The resolution environment: a manifest of exact font bytes, never an
//! ambient promise.
//!
//! A family name, file path, or OS handle is not a font identity — the same
//! name may resolve to different bytes. Identity here is a caller-supplied
//! content digest: the *host* (n0_cli, a test, a bake) verifies the bytes it
//! loads against the digest it was given and refuses before constructing an
//! environment, so a [`FontKey`] inside this crate is already-verified
//! identity, propagated opaquely into every artifact. There is no silent
//! system fallback because there is no system: an empty environment resolves
//! no text at all.

use std::sync::Arc;

use regex_syntax::hir::{ClassUnicode, ClassUnicodeRange};

use crate::face_descriptor::StaticFaceDescriptor;

/// Declared content identity of one font resource: the SHA-256 digest of its
/// exact bytes, as stated by the host that loaded them. Opaque to this crate
/// and carried into every resolved artifact.
///
/// The declaration is the host's responsibility and the host's law: the
/// text-oracle brief requires a hash-bearing surface verified before any
/// pixel, and that verification gate lands with the first host. In-crate
/// hashing is deliberately absent — it would grow the dependency perimeter,
/// which is a decision, not a convenience — so this type never claims a
/// verification it did not perform.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontKey([u8; 32]);

impl FontKey {
    pub const fn new(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    pub const fn digest(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for FontKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The short prefix is enough to name the identity in diagnostics.
        write!(
            f,
            "FontKey({:02x}{:02x}{:02x}{:02x}…)",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

/// One declared font: verified identity, the exact bytes, the face index for
/// collections, the family name the environment answers to, and its complete
/// static face descriptor.
///
/// The declared family is part of the manifest, not metadata read from the
/// font — the environment answers exactly the names its host declared, so a
/// renamed file cannot silently satisfy a different family.
#[derive(Clone, Debug)]
pub struct FontResource {
    pub key: FontKey,
    pub family: String,
    pub face_descriptor: StaticFaceDescriptor,
    pub face_index: u32,
    pub bytes: Arc<[u8]>,
}

/// The complete set of fonts a resolution may use. Empty by default: text
/// resolved against an empty environment is a typed refusal, never tofu.
#[derive(Clone, Debug, Default)]
pub struct Environment {
    fonts: Vec<FontResource>,
}

pub(crate) enum FamilyMatch<'a> {
    None,
    NoExactFace { family_resources: usize },
    Unique(&'a FontResource),
    AmbiguousExact { matching_resources: usize },
}

impl Environment {
    pub fn new(fonts: Vec<FontResource>) -> Self {
        Self { fonts }
    }

    /// Select within one declared family under oracle v6's exact static face
    /// policy and measured declared-family comparison.
    ///
    /// Any family resource makes the candidate a reached boundary. Exactly
    /// one complete descriptor match selects; zero is an exact miss and more
    /// than one is an ambiguity. The complete manifest is examined before an
    /// answer, so environment vector order never breaks a tuple tie.
    pub(crate) fn match_face(
        &self,
        family: &str,
        requested: StaticFaceDescriptor,
    ) -> FamilyMatch<'_> {
        let mut first_exact_match = None;
        let mut family_resources = 0;
        let mut matching_resources = 0;
        for font in &self.fonts {
            if family_names_match(family, &font.family) {
                family_resources += 1;
                if font.face_descriptor == requested {
                    matching_resources += 1;
                    first_exact_match.get_or_insert(font);
                }
            }
        }

        if family_resources == 0 {
            return FamilyMatch::None;
        }
        if matching_resources == 0 {
            return FamilyMatch::NoExactFace { family_resources };
        }
        if matching_resources > 1 {
            return FamilyMatch::AmbiguousExact { matching_resources };
        }
        match first_exact_match {
            Some(resource) => FamilyMatch::Unique(resource),
            None => unreachable!("one exact resource was counted but not retained"),
        }
    }

    pub fn fonts(&self) -> &[FontResource] {
        &self.fonts
    }
}

/// The explicitly measured declared-family comparison retained by oracle v6.
///
/// Exact equality is checked first, including supplementary scalars. For
/// unequal strings, each scalar must have one peer in the same Unicode 17
/// simple-fold class and both must be in the BMP. `regex-syntax` 0.8.11 pins
/// Unicode 16; Unicode 17 added exactly three BMP C/S pairs and changed or
/// removed none, so the measured additions live explicitly beside the table.
/// No scalar expansion, supplementary folding, or normalization occurs.
fn family_names_match(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    if left.chars().count() != right.chars().count() {
        return false;
    }

    for (left, right) in left.chars().zip(right.chars()) {
        if left == right {
            continue;
        }
        if u32::from(left) > u32::from(u16::MAX) || u32::from(right) > u32::from(u16::MAX) {
            return false;
        }
        if !unicode_17_bmp_simple_fold_eq(left, right) {
            return false;
        }
    }
    true
}

fn unicode_17_bmp_simple_fold_eq(left: char, right: char) -> bool {
    if matches!(
        (left, right),
        ('\u{A7CE}', '\u{A7CF}')
            | ('\u{A7CF}', '\u{A7CE}')
            | ('\u{A7D2}', '\u{A7D3}')
            | ('\u{A7D3}', '\u{A7D2}')
            | ('\u{A7D4}', '\u{A7D5}')
            | ('\u{A7D5}', '\u{A7D4}')
    ) {
        return true;
    }

    let mut class = ClassUnicode::new([ClassUnicodeRange::new(left, left)]);
    class.case_fold_simple();
    class
        .ranges()
        .iter()
        .any(|range| range.start() <= right && right <= range.end())
}

#[cfg(test)]
mod tests {
    use super::family_names_match;

    #[test]
    fn family_matching_has_the_measured_bmp_simple_fold_boundary() {
        for (left, right) in [
            ("Å", "å"),
            ("Σ", "ς"),
            ("K", "k"),
            ("ſ", "s"),
            ("\u{A7CE}", "\u{A7CF}"),
            ("\u{A7D2}", "\u{A7D3}"),
            ("\u{A7D4}", "\u{A7D5}"),
        ] {
            assert!(
                family_names_match(left, right),
                "{left:?} must match {right:?}"
            );
            assert!(
                family_names_match(right, left),
                "{right:?} must match {left:?}"
            );
        }

        assert!(!family_names_match("Maße", "MASSE"));
        assert!(!family_names_match("𐐀", "𐐨"));
        assert!(!family_names_match("Åhem", "A\u{030A}hem"));
        assert!(family_names_match("𐐀", "𐐀"));
    }
}
