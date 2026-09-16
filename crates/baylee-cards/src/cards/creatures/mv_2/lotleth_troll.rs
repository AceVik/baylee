//! Lotleth Troll — {B}{G} — Creature — Zombie Troll
//! Oracle: Trample
//! Oracle: Discard a creature card: Put a +1/+1 counter on this creature.
//! Oracle: {B}: Regenerate this creature.
//! Set: 2X2 #245 — Double Masters 2022 | Scryfall ID: cc138550-a797-4a57-91b3-626aac1b1edd | Oracle ID: 61b1d7e5-6155-4204-b110-35a890551ec8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LOTLETH_TROLL,
    oracle_id = "61b1d7e5-6155-4204-b110-35a890551ec8",
    scryfall_id = "cc138550-a797-4a57-91b3-626aac1b1edd",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Lotleth Troll",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::TROLL],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
