//! Mesmeric Fiend — {1}{B} — Creature — Nightmare Horror
//! Oracle: When this creature enters, target opponent reveals their hand and you choose a nonland card from it. Exile that card.
//! Oracle: When this creature leaves the battlefield, return the exiled card to its owner's hand.
//! Set: A25 #97 — Masters 25 | Scryfall ID: 864930ce-7c56-4ad2-9b90-57b10e5a0842 | Oracle ID: fb46b300-8a39-40d2-8bc7-31b90c822e22
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MESMERIC_FIEND,
    oracle_id = "fb46b300-8a39-40d2-8bc7-31b90c822e22",
    scryfall_id = "864930ce-7c56-4ad2-9b90-57b10e5a0842",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mesmeric Fiend",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::HORROR],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
