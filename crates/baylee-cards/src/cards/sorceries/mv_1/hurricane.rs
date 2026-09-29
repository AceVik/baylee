//! Hurricane — {X}{G} — Sorcery
//! Oracle: Hurricane deals X damage to each creature with flying and each player.
//! Set: 10E #270 — Tenth Edition | Scryfall ID: 371d83e8-f514-433c-bc6a-e0eeef3fab2a | Oracle ID: 9c021685-4017-49c7-9f58-2ae0243361a0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HURRICANE,
    oracle_id = "9c021685-4017-49c7-9f58-2ae0243361a0",
    scryfall_id = "371d83e8-f514-433c-bc6a-e0eeef3fab2a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hurricane",
        mana_cost = mana!("{X}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
