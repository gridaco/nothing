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
/// collections, and the family name the environment answers to.
///
/// The declared family is part of the manifest, not metadata read from the
/// font — the environment answers exactly the names its host declared, so a
/// renamed file cannot silently satisfy a different family.
#[derive(Clone, Debug)]
pub struct FontResource {
    pub key: FontKey,
    pub family: String,
    pub face_index: u32,
    pub bytes: Arc<[u8]>,
}

/// The complete set of fonts a resolution may use. Empty by default: text
/// resolved against an empty environment is a typed refusal, never tofu.
#[derive(Clone, Debug, Default)]
pub struct Environment {
    fonts: Vec<FontResource>,
}

impl Environment {
    pub fn new(fonts: Vec<FontResource>) -> Self {
        Self { fonts }
    }

    /// Exact, case-sensitive family match — oracle v3 has no fallback list,
    /// no aliasing, and no synthesis. First declaration wins so a manifest
    /// is order-deterministic.
    pub(crate) fn find(&self, family: &str) -> Option<&FontResource> {
        self.fonts.iter().find(|font| font.family == family)
    }

    pub fn fonts(&self) -> &[FontResource] {
        &self.fonts
    }
}
