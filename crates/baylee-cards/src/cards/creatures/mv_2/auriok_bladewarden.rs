//! Auriok Bladewarden — {1}{W} — Creature — Human Soldier
//! Oracle: {T}: Target creature gets +X/+X until end of turn, where X is this creature's power.
//! Set: MRD #3 — Mirrodin | Scryfall ID: 51605471-1198-4296-a9b5-3bee14ac2091 | Oracle ID: 2884e332-707c-4a97-adc1-e1317a513ff4
// IMPLEMENTED — {T}, target creature: +X/+X until end of turn, X read off the
// ability's own source (Amount::SourcePower).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AURIOK_BLADEWARDEN,
    oracle_id = "2884e332-707c-4a97-adc1-e1317a513ff4",
    scryfall_id = "51605471-1198-4296-a9b5-3bee14ac2091",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Auriok Bladewarden",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::PumpTarget {
            power: Amount::SourcePower,
            toughness: Amount::SourcePower,
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
