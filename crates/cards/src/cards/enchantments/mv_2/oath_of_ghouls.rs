//! Oath of Ghouls — {1}{B} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand.
//! Set: EXO #69 — Exodus | Scryfall ID: 1102f35a-ae62-479d-b61c-31a82978aedd | Oracle ID: b9490b52-6c3f-437b-b2c2-9e81c4b43b61
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_GHOULS,
    oracle_id = "b9490b52-6c3f-437b-b2c2-9e81c4b43b61",
    scryfall_id = "1102f35a-ae62-479d-b61c-31a82978aedd",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Oath of Ghouls",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
