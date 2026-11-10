//! Spellbane Centaur — {2}{G} — Creature — Centaur
//! Oracle: Creatures you control can't be the targets of blue spells or abilities from blue sources.
//! Set: ODY #271 — Odyssey | Scryfall ID: 72194a6e-481d-4b07-922b-53f919bfa316 | Oracle ID: 81de0cb1-6665-41eb-8dfc-bb30192e1359
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPELLBANE_CENTAUR,
    oracle_id = "81de0cb1-6665-41eb-8dfc-bb30192e1359",
    scryfall_id = "72194a6e-481d-4b07-922b-53f919bfa316",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Spellbane Centaur",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CENTAUR],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
