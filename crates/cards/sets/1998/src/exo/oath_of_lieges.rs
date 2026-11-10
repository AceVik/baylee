//! Oath of Lieges — {1}{W} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player who controls more lands than they do and is their opponent. The first player may search their library for a basic land card, put that card onto the battlefield, then shuffle.
//! Set: EXO #11 — Exodus | Scryfall ID: 470a2092-eeda-4557-8cee-ac401b61a225 | Oracle ID: 87038354-6762-4069-be34-7d40ae308a8d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_LIEGES,
    oracle_id = "87038354-6762-4069-be34-7d40ae308a8d",
    scryfall_id = "470a2092-eeda-4557-8cee-ac401b61a225",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Oath of Lieges",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
