//! Rappelling Scouts — {2}{W}{W} — Creature — Human Rebel Scout
//! Oracle: Flying
//! Oracle: {2}{W}: This creature gains protection from the color of your choice until end of turn.
//! Set: MMQ #41 — Mercadian Masques | Scryfall ID: 113b8366-e6d0-423e-af4b-52c1e08ed446 | Oracle ID: c71eb9b5-2017-4e6c-8140-f9c8af65893f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RAPPELLING_SCOUTS,
    oracle_id = "c71eb9b5-2017-4e6c-8140-f9c8af65893f",
    scryfall_id = "113b8366-e6d0-423e-af4b-52c1e08ed446",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rappelling Scouts",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::REBEL,
            subtypes::creature::SCOUT
        ],
        power = Some(1),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!("{2}{W}"),
        &[Effect::ProtectionFromChosenColor {
            duration: Duration::UntilEndOfTurn,
        }]
    ),],
);
