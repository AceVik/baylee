//! Ledger Shredder — {1}{U} — Creature — Bird Advisor
//! Oracle: Flying
//! Oracle: Whenever a player casts their second spell each turn, this creature connives. (Draw a card, then discard a card. If you discarded a nonland card, put a +1/+1 counter on this creature.)
//! Set: SNC #46 — Streets of New Capenna | Scryfall ID: 7ea4b5bc-18a4-45db-a56a-ab3f8bd2fb0d | Oracle ID: e9117015-1050-44dd-a46b-e7ffe2085fae
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LEDGER_SHREDDER,
    oracle_id = "e9117015-1050-44dd-a46b-e7ffe2085fae",
    scryfall_id = "7ea4b5bc-18a4-45db-a56a-ab3f8bd2fb0d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Ledger Shredder",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::ADVISOR],
        power = Some(1),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
