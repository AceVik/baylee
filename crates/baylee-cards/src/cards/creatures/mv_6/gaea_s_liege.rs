//! Gaea's Liege — {3}{G}{G}{G} — Creature — Avatar
//! Oracle: As long as Gaea's Liege isn't attacking, its power and toughness are each equal to the number of Forests you control. As long as Gaea's Liege is attacking, its power and toughness are each equal to the number of Forests defending player controls.
//! Oracle: {T}: Target land becomes a Forest until this creature leaves the battlefield.
//! Set: TSB #78 — Time Spiral Timeshifted | Scryfall ID: 3ade8d4a-6a47-4a01-9a0f-ff866055fd49 | Oracle ID: 8d134a60-e1e5-4163-8bdc-36af91567185
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GAEA_S_LIEGE,
    oracle_id = "8d134a60-e1e5-4163-8bdc-36af91567185",
    scryfall_id = "3ade8d4a-6a47-4a01-9a0f-ff866055fd49",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Gaea's Liege",
        mana_cost = mana!("{3}{G}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
