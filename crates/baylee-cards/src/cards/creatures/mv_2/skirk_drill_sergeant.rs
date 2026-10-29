//! Skirk Drill Sergeant — {1}{R} — Creature — Goblin
//! Oracle: Whenever this creature or another Goblin dies, you may pay {2}{R}. If you do, reveal the top card of your library. If it's a Goblin permanent card, put it onto the battlefield. Otherwise, put it into your graveyard.
//! Set: EVG #49 — Duel Decks Anthology: Elves vs. Goblins | Scryfall ID: da50d139-bf87-4fae-a41a-d2c343b7992b | Oracle ID: eb0d97a3-6494-4011-9f60-1b12b165a995
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SKIRK_DRILL_SERGEANT,
    oracle_id = "eb0d97a3-6494-4011-9f60-1b12b165a995",
    scryfall_id = "da50d139-bf87-4fae-a41a-d2c343b7992b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Skirk Drill Sergeant",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GOBLIN],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
