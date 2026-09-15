//! Umara Wizard // Umara Skyfalls — {4}{U} — Creature — Merfolk Wizard // Land
//! Oracle: Whenever you cast an instant, sorcery, or Wizard spell, this creature gains flying until end of turn.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: ZNR #86 — Zendikar Rising | Scryfall ID: 890eee8d-a339-4143-adfa-1b17ec10c099 | Oracle ID: 6bc668f4-8fc7-4aaf-891b-277d8328b376
//! Face: Umara Wizard — {4}{U} — Creature — Merfolk Wizard
//! Face: Umara Skyfalls —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = 20597,
    oracle_id = "6bc668f4-8fc7-4aaf-891b-277d8328b376",
    scryfall_id = "890eee8d-a339-4143-adfa-1b17ec10c099",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Umara Wizard",
            mana_cost = mana!("{4}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::WIZARD],
            power = Some(4),
            toughness = Some(3),
        ),
        face!(name = "Umara Skyfalls", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
