//! Zombie Mob — {2}{B}{B} — Creature — Zombie
//! Oracle: This creature enters with a +1/+1 counter on it for each creature card in your graveyard.
//! Oracle: When this creature enters, exile all creature cards from your graveyard.
//! Set: MIR #153 — Mirage | Scryfall ID: ab85551f-c9cc-409c-9fb5-a45de695e521 | Oracle ID: c56f0f4c-c85a-4931-b22a-42dc5d1dc7dd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ZOMBIE_MOB,
    oracle_id = "c56f0f4c-c85a-4931-b22a-42dc5d1dc7dd",
    scryfall_id = "ab85551f-c9cc-409c-9fb5-a45de695e521",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Zombie Mob",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
