//! Barbarian Bully — {2}{R} — Creature — Human Barbarian
//! Oracle: Discard a card at random: This creature gets +2/+2 until end of turn unless a player has this creature deal 4 damage to them. Activate only once each turn.
//! Set: JUD #79 — Judgment | Scryfall ID: e38f0f9f-ad7b-48da-89f3-b3e5346a3b71 | Oracle ID: 5a88f616-986d-4e97-9457-26c783489123
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARBARIAN_BULLY,
    oracle_id = "5a88f616-986d-4e97-9457-26c783489123",
    scryfall_id = "e38f0f9f-ad7b-48da-89f3-b3e5346a3b71",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Barbarian Bully",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BARBARIAN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
