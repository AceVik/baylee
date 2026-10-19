//! Dust to Dust — {1}{W}{W} — Sorcery
//! Oracle: Exile two target artifacts.
//! Set: MBC #71 — Mystery Booster Commander Edition | Scryfall ID: 0daac8e4-12cb-42d0-8a9b-1fbede68967b | Oracle ID: b17eb586-0272-4d02-be75-de06ea044b6f

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DUST_TO_DUST,
    oracle_id = "b17eb586-0272-4d02-be75-de06ea044b6f",
    scryfall_id = "0daac8e4-12cb-42d0-8a9b-1fbede68967b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dust to Dust",
        mana_cost = mana!("{1}{W}{W}"),
        types = TypeSet::SORCERY,
        abilities = &[spell!(
            &[Effect::Exile {
                target: TargetSpec::Object(&Filter::ARTIFACT),
            }],
            targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::ARTIFACT), 2)),
        )],
    ),],
);
