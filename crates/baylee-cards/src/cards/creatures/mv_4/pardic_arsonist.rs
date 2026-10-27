//! Pardic Arsonist — {2}{R}{R} — Creature — Human Barbarian
//! Oracle: Threshold — As long as there are seven or more cards in your graveyard, this creature has "When this creature enters, it deals 3 damage to any target."
//! Set: TOR #105 — Torment | Scryfall ID: 1c2538fa-2eb7-47ca-a1cf-e5546e26e584 | Oracle ID: 9ece4b78-9554-4e4e-8491-68a6091f9c6a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PARDIC_ARSONIST,
    oracle_id = "9ece4b78-9554-4e4e-8491-68a6091f9c6a",
    scryfall_id = "1c2538fa-2eb7-47ca-a1cf-e5546e26e584",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Pardic Arsonist",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::BARBARIAN],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
