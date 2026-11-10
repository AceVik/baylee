//! Mother of Runes — {W} — Creature — Human Cleric
//! Oracle: {T}: Target creature you control gains protection from the color of your choice until end of turn.
//! Set: CLB #702 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: a5e19147-e459-43a6-8ef0-e37968a462e3 | Oracle ID: 60433b48-d27f-413c-905c-43839b1943f1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MOTHER_OF_RUNES,
    oracle_id = "60433b48-d27f-413c-905c-43839b1943f1",
    scryfall_id = "a5e19147-e459-43a6-8ef0-e37968a462e3",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mother of Runes",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::ProtectionFromChosenColor {
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
    )],
);
