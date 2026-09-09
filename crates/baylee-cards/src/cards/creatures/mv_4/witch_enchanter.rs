//! Witch Enchanter // Witch-Blessed Meadow — {3}{W} — Creature — Human Warlock // Land
//! Set: MH3 #239 — Modern Horizons 3 | Scryfall ID: 62061e7c-cf19-4f03-b8fa-2bdba62d6b0b | Oracle ID: 0355249a-8e4e-41db-9cea-1b901faffbe6
//! Face: Witch Enchanter — {3}{W} — Creature — Human Warlock
//! Face: Witch-Blessed Meadow —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1324,
    oracle_id: "0355249a-8e4e-41db-9cea-1b901faffbe6",
    scryfall_id: "62061e7c-cf19-4f03-b8fa-2bdba62d6b0b",
    color_identity: ColorSet::from_slice(&[Color::White]),
    faces: &[
    face! {
        name: "Witch Enchanter",
        mana_cost: baylee_core::mana!("{3}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::WARLOCK],
        power: Some(2),
        toughness: Some(2),
    },
    face! {
        name: "Witch-Blessed Meadow",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
