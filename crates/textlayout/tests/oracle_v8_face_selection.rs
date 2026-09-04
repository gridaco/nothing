//! Oracle v8 ordered-family selection, static nearest-face matching, and
//! cluster-safe fallback.
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
        assert_eq!(layout.primary_face().key, expected_key);
    }
}

fn selected_key(
    resources: Vec<FontResource>,
    requested: StaticFaceDescriptor,
) -> Result<FontKey, ResolveError> {
    resolve(
        &attributed_with_descriptor("X", named(&["Nearest"]), requested),
        &Environment::new(resources),
    )
    .map(|layout| layout.primary_face().key)
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
fn stretch_search_changes_direction_at_normal() {
    let below = descriptor(400, FontStretch::ExtraCondensed, FontStyle::Normal);
    let above = descriptor(400, FontStretch::SemiCondensed, FontStyle::Normal);
    let resources = vec![
        resource_with_descriptor(AHEM, "Nearest", AHEM_KEY, below),
        resource_with_descriptor(BUNGEE, "Nearest", BUNGEE_KEY, above),
    ];
    assert_eq!(
        selected_key(
            resources,
            descriptor(400, FontStretch::Condensed, FontStyle::Normal)
        ),
        Ok(AHEM_KEY),
        "at/below normal searches the condensed side first"
    );

    let below = descriptor(400, FontStretch::Normal, FontStyle::Normal);
    let above = descriptor(400, FontStretch::Expanded, FontStyle::Normal);
    let resources = vec![
        resource_with_descriptor(AHEM, "Nearest", AHEM_KEY, below),
        resource_with_descriptor(BUNGEE, "Nearest", BUNGEE_KEY, above),
    ];
    assert_eq!(
        selected_key(
            resources,
            descriptor(400, FontStretch::SemiExpanded, FontStyle::Normal)
        ),
        Ok(BUNGEE_KEY),
        "above normal searches the expanded side first"
    );
}

#[test]
fn weight_search_preserves_all_three_regions_and_the_400_500_seam() {
    for (requested, lower, upper, expected) in [
        (350, 300, 400, AHEM_KEY),
        (400, 300, 500, BUNGEE_KEY),
        (450, 400, 500, BUNGEE_KEY),
        (500, 400, 600, AHEM_KEY),
        (600, 500, 700, BUNGEE_KEY),
        (700, 600, 800, BUNGEE_KEY),
    ] {
        let resources = vec![
            resource_with_descriptor(
                AHEM,
                "Nearest",
                AHEM_KEY,
                descriptor(lower, FontStretch::Normal, FontStyle::Normal),
            ),
            resource_with_descriptor(
                BUNGEE,
                "Nearest",
                BUNGEE_KEY,
                descriptor(upper, FontStretch::Normal, FontStyle::Normal),
            ),
        ];
        assert_eq!(
            selected_key(
                resources,
                descriptor(requested, FontStretch::Normal, FontStyle::Normal)
            ),
            Ok(expected),
            "directional winner for requested weight {requested}"
        );
    }
}

#[test]
fn matching_is_lexicographic_stretch_then_style_then_weight() {
    let stretch_winner = descriptor(300, FontStretch::Condensed, FontStyle::Italic);
    let later_axis_winner = descriptor(400, FontStretch::Normal, FontStyle::Normal);
    assert_eq!(
        selected_key(
            vec![
                resource_with_descriptor(AHEM, "Nearest", AHEM_KEY, stretch_winner),
                resource_with_descriptor(BUNGEE, "Nearest", BUNGEE_KEY, later_axis_winner),
            ],
            descriptor(400, FontStretch::Condensed, FontStyle::Italic),
        ),
        Ok(AHEM_KEY),
        "stretch filters the family before style and weight"
    );

    let style_winner = descriptor(300, FontStretch::Normal, FontStyle::Italic);
    let weight_winner = descriptor(400, FontStretch::Normal, FontStyle::Normal);
    assert_eq!(
        selected_key(
            vec![
                resource_with_descriptor(AHEM, "Nearest", AHEM_KEY, style_winner),
                resource_with_descriptor(BUNGEE, "Nearest", BUNGEE_KEY, weight_winner),
            ],
            descriptor(400, FontStretch::Normal, FontStyle::Italic),
        ),
        Ok(AHEM_KEY),
        "style filters the stretch winners before weight"
    );
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

    assert_eq!(ahem_first.primary_face().key, AHEM_KEY);
    assert_eq!(ahem_first.primary_face().face_index, 0);
    assert_eq!(ahem_first.advance(), 20.0);
    assert_eq!(allerta_first.primary_face().key, ALLERTA_KEY);
    assert_ne!(allerta_first.advance(), ahem_first.advance());
    assert_eq!(ahem_first.oracle_version(), "textlayout-v8");
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

    assert_eq!(layout.primary_face().key, AHEM_KEY);
}

#[test]
fn reached_family_that_requires_synthesis_does_not_try_a_later_family() {
    let requested = descriptor(700, FontStretch::Normal, FontStyle::Normal);
    let environment = Environment::new(vec![
        resource(AHEM, "Reached", AHEM_KEY),
        resource_with_descriptor(BUNGEE, "Later", BUNGEE_KEY, requested),
    ]);

    let error = resolve(
        &attributed_with_descriptor("X", named(&["Reached", "Later"]), requested),
        &environment,
    )
    .expect_err("a reached family is final even when its winner needs synthesis");

    assert_eq!(
        error,
        ResolveError::SyntheticFaceRequired {
            candidate_index: 0,
            family: "Reached".to_string(),
            requested,
            selected: StaticFaceDescriptor::NORMAL,
            synthetic_weight: true,
            synthetic_style: false,
        }
    );
}

#[test]
fn synthesis_is_a_typed_boundary_after_selection() {
    for (requested, selected, synthetic_weight, synthetic_style) in [
        (
            descriptor(700, FontStretch::Normal, FontStyle::Normal),
            descriptor(400, FontStretch::Normal, FontStyle::Normal),
            true,
            false,
        ),
        (
            descriptor(400, FontStretch::Normal, FontStyle::Italic),
            descriptor(400, FontStretch::Normal, FontStyle::Normal),
            false,
            true,
        ),
        (
            descriptor(700, FontStretch::Normal, FontStyle::Italic),
            descriptor(400, FontStretch::Normal, FontStyle::Normal),
            true,
            true,
        ),
    ] {
        let error = selected_key(
            vec![resource_with_descriptor(
                AHEM, "Nearest", AHEM_KEY, selected,
            )],
            requested,
        )
        .expect_err("the outline profile cannot realize synthetic faces");
        assert_eq!(
            error,
            ResolveError::SyntheticFaceRequired {
                candidate_index: 0,
                family: "Nearest".to_string(),
                requested,
                selected,
                synthetic_weight,
                synthetic_style,
            }
        );
    }

    assert_eq!(
        selected_key(
            vec![resource_with_descriptor(
                AHEM,
                "Nearest",
                AHEM_KEY,
                descriptor(600, FontStretch::Normal, FontStyle::Normal),
            )],
            descriptor(900, FontStretch::Normal, FontStyle::Normal),
        ),
        Ok(AHEM_KEY),
        "a selected declared weight at 600 suppresses synthetic bold"
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
        selected: requested,
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
fn nearest_winning_tuple_ties_refuse_independently_of_environment_order() {
    let requested = descriptor(350, FontStretch::Normal, FontStyle::Normal);
    let selected = descriptor(300, FontStretch::Normal, FontStyle::Normal);
    let first = resource_with_descriptor(AHEM, "Nearest", AHEM_KEY, selected);
    let second = resource_with_descriptor(ALLERTA, "nEAREST", ALLERTA_KEY, selected);
    let other = resource_with_descriptor(
        BUNGEE,
        "NEAREST",
        BUNGEE_KEY,
        descriptor(400, FontStretch::Normal, FontStyle::Normal),
    );

    for resources in [
        vec![first.clone(), other.clone(), second.clone()],
        vec![second, other, first],
    ] {
        assert_eq!(
            selected_key(resources, requested),
            Err(ResolveError::AmbiguousFace {
                candidate_index: 0,
                family: "Nearest".to_string(),
                requested,
                selected,
                matching_resources: 2,
            })
        );
    }
}

#[test]
fn normal_default_tuple_preserves_one_face_resolution() {
    let environment = Environment::new(vec![resource(AHEM, "Ahem", AHEM_KEY)]);
    let layout = resolve(&attributed("X", named(&["Ahem"])), &environment)
        .expect("one normal request and resource retain one-face behavior");

    assert_eq!(layout.primary_face().key, AHEM_KEY);
    assert_eq!(layout.primary_face().face_index, 0);
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
            assert_eq!(
                layout.primary_face().key,
                AHEM_KEY,
                "{declared:?} / {requested:?}"
            );
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
            layout.primary_face().key,
            BUNGEE_KEY,
            "{declared:?} / {requested:?}"
        );
    }

    let exact_supplementary = Environment::new(vec![resource(AHEM, "𐐀", AHEM_KEY)]);
    let layout = resolve(&attributed("X", named(&["𐐀"])), &exact_supplementary)
        .expect("exact equality remains valid for every scalar");
    assert_eq!(layout.primary_face().key, AHEM_KEY);
}

#[test]
fn named_generic_spelling_and_generic_policy_are_distinct() {
    let environment = Environment::new(vec![resource(AHEM, "serif", AHEM_KEY)]);
    let named_layout = resolve(
        &attributed("X", vec![FontFamily::named("serif")]),
        &environment,
    )
    .expect("a named family whose spelling is generic-like remains a name");
    assert_eq!(named_layout.primary_face().key, AHEM_KEY);

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

    assert_eq!(layout.primary_face().key, AHEM_KEY);
}

#[test]
fn a_missing_cluster_retries_later_families_and_records_exact_face_runs() {
    let environment = Environment::new(vec![
        resource(AHEM, "First", AHEM_KEY),
        resource(BUNGEE, "Second", BUNGEE_KEY),
    ]);
    let source = "Ax\u{0301}Z";

    let layout = resolve(
        &attributed(source, named(&["First", "Second"])),
        &environment,
    )
    .expect("a complete cluster missing from the first face falls back whole");

    assert_eq!(layout.primary_face().key, AHEM_KEY);
    assert_eq!(
        layout
            .faces()
            .iter()
            .map(|face| face.key)
            .collect::<Vec<_>>(),
        vec![AHEM_KEY, BUNGEE_KEY]
    );
    assert_eq!(layout.face_runs().len(), 3);
    assert_eq!(layout.face_runs()[0].source_utf8(), 0..1);
    assert_eq!(layout.face_runs()[0].source_utf16(), 0..1);
    assert_eq!(layout.face_runs()[0].source_scalars(), 0..1);
    assert_eq!(layout.face_runs()[0].clusters(), 0..1);
    assert_eq!(layout.face_runs()[0].glyphs(), 0..1);
    assert_eq!(layout.face_runs()[0].resolved_face_index(), 0);
    assert_eq!(layout.face_runs()[0].origin_x(), 0.0);
    assert_eq!(layout.face_runs()[0].advance(), 20.0);
    assert_eq!(layout.face_runs()[1].source_utf8(), 1..4);
    assert_eq!(layout.face_runs()[1].source_utf16(), 1..3);
    assert_eq!(layout.face_runs()[1].source_scalars(), 1..3);
    assert_eq!(layout.face_runs()[1].clusters(), 1..2);
    assert_eq!(layout.face_runs()[1].glyphs(), 1..3);
    assert_eq!(layout.face_runs()[1].resolved_face_index(), 1);
    assert_eq!(layout.face_runs()[1].origin_x(), 20.0);
    assert_eq!(layout.face_runs()[2].source_utf8(), 4..5);
    assert_eq!(layout.face_runs()[2].source_utf16(), 3..4);
    assert_eq!(layout.face_runs()[2].source_scalars(), 3..4);
    assert_eq!(layout.face_runs()[2].clusters(), 2..3);
    assert_eq!(layout.face_runs()[2].glyphs(), 3..4);
    assert_eq!(layout.face_runs()[2].resolved_face_index(), 0);
    assert_eq!(
        layout.face_runs()[2].origin_x(),
        20.0 + layout.face_runs()[1].advance()
    );
    assert_eq!(layout.face_runs()[2].advance(), 20.0);
    assert_eq!(
        layout
            .glyphs()
            .iter()
            .map(|glyph| glyph.resolved_face_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 1, 0]
    );
    assert_eq!(layout.shaping_chunks()[0].face_runs(), 0..3);
    assert_eq!(layout.shaping_chunks()[0].clusters(), 0..3);
    assert_eq!(layout.shaping_chunks()[0].glyphs(), 0..4);
    assert_eq!(
        layout.shaping_chunks()[0].advance(),
        layout.face_runs().iter().map(|run| run.advance()).sum()
    );
}

#[test]
fn adjacent_clusters_using_one_fallback_face_coalesce_into_one_shaping_run() {
    let environment = Environment::new(vec![
        resource(AHEM, "Primary", AHEM_KEY),
        resource(BUNGEE, "Fallback", BUNGEE_KEY),
    ]);
    let layout = resolve(
        &attributed("''", named(&["Primary", "Fallback"])),
        &environment,
    )
    .expect("adjacent fallback clusters retain one same-face shaping run");

    assert_eq!(layout.primary_face().key, AHEM_KEY);
    assert_eq!(layout.face_runs().len(), 1);
    assert_eq!(layout.face_runs()[0].source_utf8(), 0..2);
    assert_eq!(layout.face_runs()[0].clusters(), 0..2);
    assert_eq!(layout.face_runs()[0].glyphs(), 0..2);
    assert_eq!(layout.face_runs()[0].resolved_face_index(), 1);
    assert_eq!(layout.shaping_chunks()[0].face_runs(), 0..1);
}

#[test]
fn canonical_composition_keeps_the_complete_cluster_in_the_primary_face() {
    let environment = Environment::new(vec![
        resource(AHEM, "First", AHEM_KEY),
        resource(BUNGEE, "Second", BUNGEE_KEY),
    ]);
    let layout = resolve(
        &attributed("A\u{0301}", named(&["First", "Second"])),
        &environment,
    )
    .expect("the shaper proves canonical composition without cmap-per-scalar guessing");

    assert_eq!(layout.faces().len(), 1);
    assert_eq!(layout.primary_face().key, AHEM_KEY);
    assert_eq!(layout.face_runs().len(), 1);
    assert_eq!(layout.face_runs()[0].source_utf8(), 0..3);
    assert_eq!(layout.clusters().len(), 1);
    assert_eq!(layout.clusters()[0].source_scalars(), 0..2);
    assert_eq!(layout.glyphs().len(), 1);
    assert_eq!(layout.glyphs()[0].resolved_face_index, 0);
}

#[test]
fn fallback_does_not_search_another_face_inside_the_reached_family() {
    let bold = descriptor(700, FontStretch::Normal, FontStyle::Normal);
    let environment = Environment::new(vec![
        resource(AHEM, "Family", AHEM_KEY),
        resource_with_descriptor(BUNGEE, "Family", BUNGEE_KEY, bold),
        resource(BUNGEE, "Later", BUNGEE_KEY),
    ]);
    let layout = resolve(&attributed("'", named(&["Family", "Later"])), &environment)
        .expect("a missing glyph advances the family list, not the family's face list");

    assert_eq!(layout.primary_face().key, AHEM_KEY);
    assert_eq!(layout.glyphs()[0].resolved_face_index, 1);
    assert_eq!(layout.faces()[1].key, BUNGEE_KEY);
    assert_eq!(layout.face_runs()[0].resolved_face_index(), 1);
}

#[test]
fn each_fallback_family_repeats_static_nearest_face_matching() {
    let requested = descriptor(350, FontStretch::Normal, FontStyle::Normal);
    let environment = Environment::new(vec![
        resource_with_descriptor(AHEM, "Primary", AHEM_KEY, requested),
        resource_with_descriptor(
            BUNGEE,
            "Fallback",
            BUNGEE_KEY,
            descriptor(300, FontStretch::Normal, FontStyle::Normal),
        ),
        resource_with_descriptor(
            ALLERTA,
            "Fallback",
            ALLERTA_KEY,
            descriptor(400, FontStretch::Normal, FontStyle::Normal),
        ),
    ]);
    let layout = resolve(
        &attributed_with_descriptor("'", named(&["Primary", "Fallback"]), requested),
        &environment,
    )
    .expect("the fallback family repeats the measured 350 -> 300 weight search");

    assert_eq!(layout.primary_face().key, AHEM_KEY);
    assert_eq!(layout.faces()[1].key, BUNGEE_KEY);
    assert_eq!(layout.glyphs()[0].resolved_face_index, 1);
}

#[test]
fn fallback_keeps_primary_metrics_when_every_outline_uses_a_later_face() {
    let environment = Environment::new(vec![
        resource(AHEM, "Primary", AHEM_KEY),
        resource(BUNGEE, "Fallback", BUNGEE_KEY),
    ]);
    let layout = resolve(
        &attributed("'", named(&["Primary", "Fallback"])),
        &environment,
    )
    .expect("the later face supplies ink while the primary face supplies metrics");
    let fallback_only = resolve(&attributed("'", named(&["Fallback"])), &environment)
        .expect("the explicit fallback face is an independent geometry control");

    assert_eq!(layout.primary_face().key, AHEM_KEY);
    assert_eq!(
        (layout.metrics().ascent, layout.metrics().descent),
        (16.0, 4.0)
    );
    assert_eq!(layout.glyphs()[0].resolved_face_index, 1);
    assert_eq!(layout.faces()[1].key, BUNGEE_KEY);
    assert_eq!(layout.advance(), fallback_only.advance());
    assert_eq!(layout.ink_bounds(), fallback_only.ink_bounds());
    assert_ne!(layout.metrics(), fallback_only.metrics());
}

#[test]
fn reached_generic_and_required_fallback_synthesis_remain_typed_boundaries() {
    let normal_environment = Environment::new(vec![
        resource(AHEM, "Primary", AHEM_KEY),
        resource(BUNGEE, "Fallback", BUNGEE_KEY),
    ]);
    let generic = resolve(
        &attributed(
            "'",
            vec![FontFamily::named("Primary"), FontFamily::generic("serif")],
        ),
        &normal_environment,
    )
    .expect_err("a reached generic remains an undeclared external policy boundary");
    assert_eq!(
        generic,
        ResolveError::UnmappedGenericFamily {
            candidate_index: 1,
            family: "serif".to_string(),
        }
    );

    let bold = descriptor(700, FontStretch::Normal, FontStyle::Normal);
    let synthesis_environment = Environment::new(vec![
        resource_with_descriptor(AHEM, "Primary", AHEM_KEY, bold),
        resource(BUNGEE, "Fallback", BUNGEE_KEY),
    ]);
    let synthesis = resolve(
        &attributed_with_descriptor("'", named(&["Primary", "Fallback"]), bold),
        &synthesis_environment,
    )
    .expect_err("a fallback outline cannot silently synthesize bold");
    assert_eq!(
        synthesis,
        ResolveError::SyntheticFaceRequired {
            candidate_index: 1,
            family: "Fallback".to_string(),
            requested: bold,
            selected: StaticFaceDescriptor::NORMAL,
            synthetic_weight: true,
            synthetic_style: false,
        }
    );
}

#[test]
fn exhausting_declared_faces_refuses_the_first_missing_scalar_by_name() {
    let environment = Environment::new(vec![resource(AHEM, "Only", AHEM_KEY)]);
    let error = resolve(&attributed("x\u{0301}", named(&["Only"])), &environment)
        .expect_err("partial-cluster tofu is forbidden");

    assert_eq!(
        error,
        ResolveError::MissingGlyph {
            byte_index: 1,
            character: '\u{0301}',
        }
    );
}
