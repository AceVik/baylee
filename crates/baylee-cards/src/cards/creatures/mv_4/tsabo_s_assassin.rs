//! Tsabo's Assassin — {2}{B}{B} — Creature — Phyrexian Zombie Assassin
//! Oracle: {T}: Destroy target creature if it shares a color with the most common color among all permanents or a color tied for most common. A creature destroyed this way can't be regenerated.
//! Set: INV #128 — Invasion | Scryfall ID: 0047302d-4e3d-4327-9bb2-ecd5b00b00e3 | Oracle ID: 4d00c44b-8953-4b18-af73-2944321b6cd1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TSABO_S_ASSASSIN,
    oracle_id = "4d00c44b-8953-4b18-af73-2944321b6cd1",
    scryfall_id = "0047302d-4e3d-4327-9bb2-ecd5b00b00e3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Tsabo's Assassin",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::ZOMBIE,
            subtypes::creature::ASSASSIN
        ],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
