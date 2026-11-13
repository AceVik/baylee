//! Strength of Cedars — {4}{G} — Instant — Arcane
//! Oracle: Target creature gets +X/+X until end of turn, where X is the number of lands you control.
//! Set: CHK #245 — Champions of Kamigawa | Scryfall ID: 85ab46f5-d3e1-403f-92e7-f40de51a3d4d | Oracle ID: 61b29c75-00d0-4ddb-9e27-cfd47302830e
// IMPLEMENTED — PumpTarget on the target creature, +X/+X where X is
// Amount::CountOf over your lands on the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STRENGTH_OF_CEDARS,
    oracle_id = "61b29c75-00d0-4ddb-9e27-cfd47302830e",
    scryfall_id = "85ab46f5-d3e1-403f-92e7-f40de51a3d4d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Strength of Cedars",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::INSTANT,
        subtypes = &[subtypes::spell::ARCANE],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::PumpTarget {
            power: Amount::CountOf {
                filter: &Filter::YOUR_LAND,
                zone: ZoneSel::Battlefield,
            },
            toughness: Amount::CountOf {
                filter: &Filter::YOUR_LAND,
                zone: ZoneSel::Battlefield,
            },
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
