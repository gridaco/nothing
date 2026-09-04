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

use crate::face_descriptor::{FontStretch, FontStyle, FontWeight, StaticFaceDescriptor};

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
    Unique {
        resource: &'a FontResource,
        selected: StaticFaceDescriptor,
    },
    AmbiguousWinner {
        selected: StaticFaceDescriptor,
        matching_resources: usize,
    },
}

impl Environment {
    pub fn new(fonts: Vec<FontResource>) -> Self {
        Self { fonts }
    }

    /// Select within one declared family under oracle v8's static CSS Fonts
    /// matching policy and measured declared-family comparison.
    ///
    /// Any family resource makes the candidate a reached matching set.
    /// Matching narrows the complete family lexicographically by stretch,
    /// style, then weight. Exactly one resource at the winning complete
    /// descriptor selects; more than one is an ambiguity. The caller may
    /// advance to another family when that one winner cannot shape a complete
    /// cluster, but never searches another descriptor in this family. The
    /// complete manifest is examined before an answer, so environment vector
    /// order never becomes stylesheet source order and never breaks a
    /// winning-tuple tie.
    pub(crate) fn match_face(
        &self,
        family: &str,
        requested: StaticFaceDescriptor,
    ) -> FamilyMatch<'_> {
        let family_resources = self
            .fonts
            .iter()
            .filter(|font| family_names_match(family, &font.family))
            .collect::<Vec<_>>();
        if family_resources.is_empty() {
            return FamilyMatch::None;
        }

        let stretch = select_stretch(
            family_resources
                .iter()
                .map(|resource| resource.face_descriptor.stretch()),
            requested.stretch(),
        );
        let style = select_style(
            family_resources
                .iter()
                .filter(|resource| resource.face_descriptor.stretch() == stretch)
                .map(|resource| resource.face_descriptor.style()),
            requested.style(),
        );
        let weight = select_weight(
            family_resources
                .iter()
                .filter(|resource| {
                    resource.face_descriptor.stretch() == stretch
                        && resource.face_descriptor.style() == style
                })
                .map(|resource| resource.face_descriptor.weight()),
            requested.weight(),
        );
        let selected = StaticFaceDescriptor::new(weight, stretch, style);

        let mut matching = family_resources
            .into_iter()
            .filter(|resource| resource.face_descriptor == selected);
        let first = matching
            .next()
            .expect("each matching axis selected a descriptor carried by one resource");
        let matching_resources = 1 + matching.count();
        if matching_resources > 1 {
            return FamilyMatch::AmbiguousWinner {
                selected,
                matching_resources,
            };
        }
        FamilyMatch::Unique {
            resource: first,
            selected,
        }
    }

    pub fn fonts(&self) -> &[FontResource] {
        &self.fonts
    }
}

/// CSS Fonts' width search over the finite nine-point static profile. Below
/// and at normal, the condensed side wins before the expanded side; above
/// normal, that direction reverses. An exact point therefore wins naturally.
fn select_stretch(
    available: impl Iterator<Item = FontStretch>,
    requested: FontStretch,
) -> FontStretch {
    let requested_rank = stretch_rank(requested);
    let available = available.collect::<Vec<_>>();
    let selected = if requested_rank <= stretch_rank(FontStretch::Normal) {
        available
            .iter()
            .copied()
            .filter(|stretch| stretch_rank(*stretch) <= requested_rank)
            .max_by_key(|stretch| stretch_rank(*stretch))
            .or_else(|| {
                available
                    .iter()
                    .copied()
                    .filter(|stretch| stretch_rank(*stretch) > requested_rank)
                    .min_by_key(|stretch| stretch_rank(*stretch))
            })
    } else {
        available
            .iter()
            .copied()
            .filter(|stretch| stretch_rank(*stretch) >= requested_rank)
            .min_by_key(|stretch| stretch_rank(*stretch))
            .or_else(|| {
                available
                    .iter()
                    .copied()
                    .filter(|stretch| stretch_rank(*stretch) < requested_rank)
                    .max_by_key(|stretch| stretch_rank(*stretch))
            })
    };
    selected.expect("a reached family has at least one stretch")
}

fn stretch_rank(stretch: FontStretch) -> u8 {
    match stretch {
        FontStretch::UltraCondensed => 0,
        FontStretch::ExtraCondensed => 1,
        FontStretch::Condensed => 2,
        FontStretch::SemiCondensed => 3,
        FontStretch::Normal => 4,
        FontStretch::SemiExpanded => 5,
        FontStretch::Expanded => 6,
        FontStretch::ExtraExpanded => 7,
        FontStretch::UltraExpanded => 8,
    }
}

/// The finite profile has only upright and italic faces. CSS style matching
/// prefers the requested class and falls back to the other class only when it
/// is absent after stretch matching.
fn select_style(available: impl Iterator<Item = FontStyle>, requested: FontStyle) -> FontStyle {
    let available = available.collect::<Vec<_>>();
    if available.contains(&requested) {
        requested
    } else {
        match requested {
            FontStyle::Normal => FontStyle::Italic,
            FontStyle::Italic => FontStyle::Normal,
        }
    }
}

/// CSS Fonts' three-region static weight search. The 400..=500 interval is
/// intentionally asymmetric: search upward only through 500, then below the
/// request, then above 500.
fn select_weight(available: impl Iterator<Item = FontWeight>, requested: FontWeight) -> FontWeight {
    let requested = requested.value();
    let available = available.map(FontWeight::value).collect::<Vec<_>>();
    let selected = if requested < 400 {
        available
            .iter()
            .copied()
            .filter(|weight| *weight <= requested)
            .max()
            .or_else(|| {
                available
                    .iter()
                    .copied()
                    .filter(|weight| *weight > requested)
                    .min()
            })
    } else if requested <= 500 {
        available
            .iter()
            .copied()
            .filter(|weight| *weight >= requested && *weight <= 500)
            .min()
            .or_else(|| {
                available
                    .iter()
                    .copied()
                    .filter(|weight| *weight < requested)
                    .max()
            })
            .or_else(|| {
                available
                    .iter()
                    .copied()
                    .filter(|weight| *weight > 500)
                    .min()
            })
    } else {
        available
            .iter()
            .copied()
            .filter(|weight| *weight >= requested)
            .min()
            .or_else(|| {
                available
                    .iter()
                    .copied()
                    .filter(|weight| *weight < requested)
                    .max()
            })
    }
    .expect("stretch/style filtering retains at least one weight");
    FontWeight::new(selected).expect("selected a declared representable static weight")
}

/// The explicitly measured declared-family comparison retained by oracle v8.
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
