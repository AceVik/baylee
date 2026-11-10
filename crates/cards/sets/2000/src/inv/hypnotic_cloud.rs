//! Hypnotic Cloud — {1}{B} — Sorcery
//! Oracle: Kicker {4} (You may pay an additional {4} as you cast this spell.)
//! Oracle: Target player discards a card. If this spell was kicked, that player discards three cards instead.
//! Set: DDM #67 — Duel Decks: Jace vs. Vraska | Scryfall ID: 62e55607-4c1f-46ab-aae7-391b65a125c4 | Oracle ID: 277757c6-4c4c-4a4c-84f6-4a64fe3b3bfe
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HYPNOTIC_CLOUD,
    oracle_id = "277757c6-4c4c-4a4c-84f6-4a64fe3b3bfe",
    scryfall_id = "62e55607-4c1f-46ab-aae7-391b65a125c4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Hypnotic Cloud",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
