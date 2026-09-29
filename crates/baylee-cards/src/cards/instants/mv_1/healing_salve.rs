//! Healing Salve — {W} — Instant
//! Oracle: Choose one —
//! Oracle: • Target player gains 3 life.
//! Oracle: • Prevent the next 3 damage that would be dealt to any target this turn.
//! Set: DVD #14 — Duel Decks Anthology: Divine vs. Demonic | Scryfall ID: 0ff82aba-9022-4eff-a6dc-67365360d646 | Oracle ID: 8da8644c-75a1-4fe9-8e94-900d948d631c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HEALING_SALVE,
    oracle_id = "8da8644c-75a1-4fe9-8e94-900d948d631c",
    scryfall_id = "0ff82aba-9022-4eff-a6dc-67365360d646",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Healing Salve",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
