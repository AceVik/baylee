//! Gempalm Avenger — {5}{W} — Creature — Human Soldier
//! Oracle: Cycling {2}{W} ({2}{W}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, Soldier creatures get +1/+1 and gain first strike until end of turn.
//! Set: DDO #9 — Duel Decks: Elspeth vs. Kiora | Scryfall ID: 3267bf08-7585-4e2e-b472-97a357d264dc | Oracle ID: cb9e6860-591f-4758-aaa3-14efa0a775f5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GEMPALM_AVENGER,
    oracle_id = "cb9e6860-591f-4758-aaa3-14efa0a775f5",
    scryfall_id = "3267bf08-7585-4e2e-b472-97a357d264dc",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Gempalm Avenger",
        mana_cost = mana!("{5}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
