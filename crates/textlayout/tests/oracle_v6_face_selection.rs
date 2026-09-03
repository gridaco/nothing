//! Oracle v6 ordered family and exact static face selection.
//!
//! These tests use only attributed text and explicit font resources. They
//! prove selection and refusal before any caller-specific projection exists.

use std::sync::Arc;

use textlayout::{
    AttributedText, Environment, FontFamily, FontKey, FontResource, FontStretch, FontStyle,
    FontWeight, ResolveError, SourceRunTag, StaticFaceDescriptor, Style, resolve,
};

const AHEM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/web-first/fonts/ahem.ttf"
);
const ALLERTA: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/fonts/Allerta/Allerta-Regular.ttf"
);
const BUNGEE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/fonts/Bungee/Bungee-Regular.ttf"
);

const AHEM_KEY: FontKey = FontKey::new([0xA1; 32]);
const ALLERTA_KEY: FontKey = FontKey::new([0xB2; 32]);
const BUNGEE_KEY: FontKey = FontKey::new([0xC3; 32]);
const SOURCE_RUN: SourceRunTag = SourceRunTag::new(0);

fn descriptor(weight: u16, stretch: FontStretch, style: FontStyle) -> StaticFaceDescriptor {
    StaticFaceDescriptor::new(
        FontWeight::new(weight).expect("test weight is representable"),
        stretch,
        style,
    )
}

fn resource_with_descriptor(
    path: &str,
    family: &str,
    key: FontKey,
    face_descriptor: StaticFaceDescriptor,
) -> FontResource {
    let bytes: Arc<[u8]> = std::fs::read(path).expect("fixture font bytes").into();
    FontResource {
        key,
        family: family.to_string(),
        face_descriptor,
        face_index: 0,
        bytes,
    }
}

fn resource(path: &str, family: &str, key: FontKey) -> FontResource {
    resource_with_descriptor(path, family, key, StaticFaceDescriptor::NORMAL)
}

fn attributed_with_descriptor(
    source: &str,
    families: Vec<FontFamily>,
    face_descriptor: StaticFaceDescriptor,
) -> AttributedText {
    AttributedText::single_source_run(
        source.to_string(),
        Style {
            families,
            face_descriptor,
            size: 20.0,
        },
        SOURCE_RUN,
    )
}

fn attributed(source: &str, families: Vec<FontFamily>) -> AttributedText {
    attributed_with_descriptor(source, families, StaticFaceDescriptor::NORMAL)
}

fn named(names: &[&str]) -> Vec<FontFamily> {
    names.iter().map(|name| FontFamily::named(*name)).collect()
}

fn assert_selection_ignores_environment_order(
    first: FontResource,
    second: FontResource,
    requested: StaticFaceDescriptor,
    expected_key: FontKey,
) {
    let orders = [vec![first.clone(), second.clone()], vec![second, first]];

    for environment_resources in orders {
        let layout = resolve(
            &attributed_with_descriptor("X", named(&["Exact"]), requested),
            &Environment::new(environment_resources),
        )
        .expect("one exact tuple selects independently of environment order");
        assert_eq!(layout.face().key, expected_key);
    }
}

#[test]
fn each_descriptor_component_and_the_complete_tuple_select_exactly() {
    let cases = [
        descriptor(700, FontStretch::Normal, FontStyle::Normal),
        descriptor(400, FontStretch::SemiExpanded, FontStyle::Normal),
        descriptor(400, FontStretch::Normal, FontStyle::Italic),
        descriptor(700, FontStretch::Expanded, FontStyle::Italic),
    ];

    for requested in cases {
        assert_selection_ignores_environment_order(
            resource(AHEM, "Exact", AHEM_KEY),
            resource_with_descriptor(ALLERTA, "Exact", ALLERTA_KEY, requested),
            requested,
            ALLERTA_KEY,
        );
    }
}

#[test]
fn request_order_not_environment_order_selects_the_first_reached_family() {
    let environment = Environment::new(vec![
        resource(ALLERTA, "Allerta", ALLERTA_KEY),
        resource(AHEM, "Ahem", AHEM_KEY),
    ]);

    let ahem_first = resolve(&attributed("X", named(&["Ahem", "Allerta"])), &environment)
        .expect("the first requested declared family resolves");
    let allerta_first = resolve(&attributed("X", named(&["Allerta", "Ahem"])), &environment)
        .expect("reversing only the request selects the other face");

    assert_eq!(ahem_first.face().key, AHEM_KEY);
    assert_eq!(ahem_first.face().face_index, 0);
    assert_eq!(ahem_first.advance(), 20.0);
    assert_eq!(allerta_first.face().key, ALLERTA_KEY);
    assert_ne!(allerta_first.advance(), ahem_first.advance());
    assert_eq!(ahem_first.oracle_version(), "textlayout-v6");
    assert_eq!(ahem_first.oracle_version(), textlayout::ORACLE_VERSION);
}

#[test]
fn unavailable_named_family_falls_through_to_the_next_request() {
    let environment = Environment::new(vec![resource(AHEM, "Available", AHEM_KEY)]);
    let layout = resolve(
        &attributed("X", named(&["Absent", "Available"])),
        &environment,
    )
    .expect("only an unavailable named candidate falls through");

    assert_eq!(layout.face().key, AHEM_KEY);
}

#[test]
fn reached_family_exact_miss_does_not_try_a_later_family() {
    let requested = descriptor(700, FontStretch::Normal, FontStyle::Normal);
    let environment = Environment::new(vec![
        resource(AHEM, "Reached", AHEM_KEY),
        resource_with_descriptor(BUNGEE, "Later", BUNGEE_KEY, requested),
    ]);

    let error = resolve(
        &attributed_with_descriptor("X", named(&["Reached", "Later"]), requested),
        &environment,
    )
    .expect_err("a reached family is final even when its descriptor misses");

    assert_eq!(
        error,
        ResolveError::NoExactFace {
            candidate_index: 0,
            family: "Reached".to_string(),
            requested,
            family_resources: 1,
        }
    );
}

#[test]
fn exact_tuple_ties_refuse_independently_of_environment_order() {
    let requested = StaticFaceDescriptor::NORMAL;
    let first = resource(AHEM, "Tied", AHEM_KEY);
    let second = resource(ALLERTA, "tIED", ALLERTA_KEY);
    let nonmatching = resource_with_descriptor(
        BUNGEE,
        "TIED",
        BUNGEE_KEY,
        descriptor(400, FontStretch::Normal, FontStyle::Italic),
    );
    let expected = ResolveError::AmbiguousFace {
        candidate_index: 0,
        family: "Tied".to_string(),
        requested,
        matching_resources: 2,
    };

    for resources in [
        vec![first.clone(), nonmatching.clone(), second.clone()],
        vec![second, nonmatching, first],
    ] {
        let error = resolve(
            &attributed_with_descriptor("X", named(&["Tied"]), requested),
            &Environment::new(resources),
        )
        .expect_err("two exact tuples cannot inherit environment order");
        let diagnostic = error.to_string();
        assert!(
            diagnostic.contains("ambiguous"),
            "the stable ambiguity reason must remain classifiable"
        );
        assert!(
            diagnostic.contains("matches 2 declared resources"),
            "the registered resource-count reason must remain stable"
        );
        assert_eq!(error, expected);
    }
}

#[test]
fn normal_default_tuple_preserves_one_face_resolution() {
    let environment = Environment::new(vec![resource(AHEM, "Ahem", AHEM_KEY)]);
    let layout = resolve(&attributed("X", named(&["Ahem"])), &environment)
        .expect("one normal request and resource retain one-face behavior");

    assert_eq!(layout.face().key, AHEM_KEY);
    assert_eq!(layout.face().face_index, 0);
    assert_eq!(layout.advance(), 20.0);
}

#[test]
fn static_descriptor_domain_is_finite_and_lossless() {
    assert_eq!(FontWeight::MIN.value(), 1);
    assert_eq!(FontWeight::MAX.value(), 1000);
    assert_eq!(FontWeight::new(0).unwrap_err().value(), 0);
    assert_eq!(FontWeight::new(1001).unwrap_err().value(), 1001);

    let stretches = [
        FontStretch::UltraCondensed,
        FontStretch::ExtraCondensed,
        FontStretch::Condensed,
        FontStretch::SemiCondensed,
        FontStretch::Normal,
        FontStretch::SemiExpanded,
        FontStretch::Expanded,
        FontStretch::ExtraExpanded,
        FontStretch::UltraExpanded,
    ];
    for stretch in stretches {
        let exact = StaticFaceDescriptor::new(FontWeight::MIN, stretch, FontStyle::Italic);
        assert_eq!(exact.weight(), FontWeight::MIN);
        assert_eq!(exact.stretch(), stretch);
        assert_eq!(exact.style(), FontStyle::Italic);
    }
}

#[test]
fn bmp_simple_fold_pairs_select_while_expansion_supplementary_and_normalization_do_not() {
    for (declared, requested) in [
        ("Å", "å"),
        ("Σ", "ς"),
        ("K", "k"),
        ("ſ", "s"),
        ("\u{A7CE}", "\u{A7CF}"),
        ("\u{A7D2}", "\u{A7D3}"),
        ("\u{A7D4}", "\u{A7D5}"),
    ] {
        for (declared, requested) in [(declared, requested), (requested, declared)] {
            let environment = Environment::new(vec![resource(AHEM, declared, AHEM_KEY)]);
            let layout = resolve(&attributed("X", named(&[requested])), &environment)
                .expect("a representative BMP simple-fold equivalent must select");
            assert_eq!(layout.face().key, AHEM_KEY, "{declared:?} / {requested:?}");
        }
    }

    for (declared, requested) in [("Maße", "MASSE"), ("𐐀", "𐐨"), ("Åhem", "A\u{030A}hem")] {
        let environment = Environment::new(vec![
            resource(AHEM, declared, AHEM_KEY),
            resource(BUNGEE, "Fallback", BUNGEE_KEY),
        ]);
        let layout = resolve(
            &attributed("X", named(&[requested, "Fallback"])),
            &environment,
        )
        .expect("a non-matching named candidate must fall through");
        assert_eq!(
            layout.face().key,
            BUNGEE_KEY,
            "{declared:?} / {requested:?}"
        );
    }

    let exact_supplementary = Environment::new(vec![resource(AHEM, "𐐀", AHEM_KEY)]);
    let layout = resolve(&attributed("X", named(&["𐐀"])), &exact_supplementary)
        .expect("exact equality remains valid for every scalar");
    assert_eq!(layout.face().key, AHEM_KEY);
}

#[test]
fn named_generic_spelling_and_generic_policy_are_distinct() {
    let environment = Environment::new(vec![resource(AHEM, "serif", AHEM_KEY)]);
    let named_layout = resolve(
        &attributed("X", vec![FontFamily::named("serif")]),
        &environment,
    )
    .expect("a named family whose spelling is generic-like remains a name");
    assert_eq!(named_layout.face().key, AHEM_KEY);

    let error = resolve(
        &attributed(
            "X",
            vec![FontFamily::generic("serif"), FontFamily::named("serif")],
        ),
        &environment,
    )
    .expect_err("a reached generic is a policy boundary, not a named lookup");
    assert_eq!(
        error,
        ResolveError::UnmappedGenericFamily {
            candidate_index: 0,
            family: "serif".to_string(),
        }
    );
}

#[test]
fn incomplete_family_selection_refuses_by_typed_reason() {
    let environment = Environment::new(vec![resource(AHEM, "Available", AHEM_KEY)]);

    let empty = resolve(&attributed("X", Vec::new()), &environment)
        .expect_err("an empty request cannot select a face");
    assert_eq!(empty, ResolveError::EmptyFamilyList);

    let absent = resolve(&attributed("X", named(&["First", "Second"])), &environment)
        .expect_err("an exhausted named request cannot select a face");
    assert_eq!(
        absent,
        ResolveError::NoMatchingFamily {
            families: vec!["First".to_string(), "Second".to_string()],
        }
    );
}

#[test]
fn a_successful_earlier_match_makes_every_later_candidate_inert() {
    let environment = Environment::new(vec![
        resource(AHEM, "Chosen", AHEM_KEY),
        resource(ALLERTA, "Ambiguous", ALLERTA_KEY),
        resource(BUNGEE, "aMBIGUOUS", BUNGEE_KEY),
    ]);
    let layout = resolve(
        &attributed(
            "X",
            vec![
                FontFamily::named("Chosen"),
                FontFamily::generic("serif"),
                FontFamily::named("Ambiguous"),
            ],
        ),
        &environment,
    )
    .expect("selection is complete after the first unique exact match");

    assert_eq!(layout.face().key, AHEM_KEY);
}

#[test]
fn a_missing_glyph_in_the_selected_face_does_not_retry_the_family_list() {
    let environment = Environment::new(vec![
        resource(AHEM, "First", AHEM_KEY),
        resource(BUNGEE, "Second", BUNGEE_KEY),
    ]);
    let source = "Ax\u{0301}Z";

    resolve(&attributed(source, named(&["Second"])), &environment)
        .expect("the later face proves it can shape the admitted source");
    let error = resolve(
        &attributed(source, named(&["First", "Second"])),
        &environment,
    )
    .expect_err("glyph coverage cannot restart an already completed selection");

    assert_eq!(
        error,
        ResolveError::MissingGlyph {
            byte_index: 2,
            character: '\u{0301}',
        }
    );
}
