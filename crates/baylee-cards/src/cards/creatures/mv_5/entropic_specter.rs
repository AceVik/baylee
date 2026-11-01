//! Entropic Specter — {3}{B}{B} — Creature — Specter Spirit
//! Oracle: Flying
//! Oracle: As this creature enters, choose an opponent.
//! Oracle: Entropic Specter's power and toughness are each equal to the number of cards in the chosen player's hand.
//! Oracle: Whenever this creature deals damage to a player, that player discards a card.
//! Set: EXO #61 — Exodus | Scryfall ID: bdb04d81-b0ab-4bc7-935d-c31005887240 | Oracle ID: a9c44443-85c7-4d0a-82ab-36137151e4e5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENTROPIC_SPECTER,
    oracle_id = "a9c44443-85c7-4d0a-82ab-36137151e4e5",
    scryfall_id = "bdb04d81-b0ab-4bc7-935d-c31005887240",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Entropic Specter",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPECTER, subtypes::creature::SPIRIT],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
