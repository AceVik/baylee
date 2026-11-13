//! Dawn of the Dead — {2}{B}{B}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, you lose 1 life.
//! Oracle: At the beginning of your upkeep, you may return target creature card from your graveyard to the battlefield. That creature gains haste until end of turn. Exile it at the beginning of the next end step.
//! Set: TOR #59 — Torment | Scryfall ID: baabadd3-3fc1-4c9a-ad3b-c79d60fb3390 | Oracle ID: 309e61f5-1fc7-4481-8d3a-9b35d335c99f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DAWN_OF_THE_DEAD,
    oracle_id = "309e61f5-1fc7-4481-8d3a-9b35d335c99f",
    scryfall_id = "baabadd3-3fc1-4c9a-ad3b-c79d60fb3390",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Dawn of the Dead",
        mana_cost = mana!("{2}{B}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
