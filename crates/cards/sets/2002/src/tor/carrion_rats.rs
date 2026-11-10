//! Carrion Rats — {B} — Creature — Rat
//! Oracle: Whenever this creature attacks or blocks, any player may exile a card from their graveyard. If a player does, this creature assigns no combat damage this turn.
//! Set: TOR #54 — Torment | Scryfall ID: 2efa2579-c048-4506-babc-ec1c29bb99a8 | Oracle ID: 1aa6b31b-cd30-4741-abc6-2ef72e445630
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CARRION_RATS,
    oracle_id = "1aa6b31b-cd30-4741-abc6-2ef72e445630",
    scryfall_id = "2efa2579-c048-4506-babc-ec1c29bb99a8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Carrion Rats",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::RAT],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
