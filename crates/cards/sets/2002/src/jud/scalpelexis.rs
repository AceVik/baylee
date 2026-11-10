//! Scalpelexis — {4}{U} — Creature — Beast
//! Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.)
//! Oracle: Whenever this creature deals combat damage to a player, that player exiles the top four cards of their library. If two or more of those cards have the same name, repeat this process.
//! Set: 10E #105 — Tenth Edition | Scryfall ID: ad3ca697-bdf4-41df-973d-22a0ce89ad35 | Oracle ID: 99eb50ef-352f-47c4-91e3-32813cbe0649
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCALPELEXIS,
    oracle_id = "99eb50ef-352f-47c4-91e3-32813cbe0649",
    scryfall_id = "ad3ca697-bdf4-41df-973d-22a0ce89ad35",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Scalpelexis",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(1),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
