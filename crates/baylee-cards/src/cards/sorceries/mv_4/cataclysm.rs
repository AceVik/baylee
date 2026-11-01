//! Cataclysm — {2}{W}{W} — Sorcery
//! Oracle: Each player chooses from among the permanents they control an artifact, a creature, an enchantment, and a land, then sacrifices the rest.
//! Set: TPR #8 — Tempest Remastered | Scryfall ID: 713160be-bcfd-4ef5-9457-c02b926ba06e | Oracle ID: b0479ca4-e925-4aa7-a4fa-f7e01f899ac5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CATACLYSM,
    oracle_id = "b0479ca4-e925-4aa7-a4fa-f7e01f899ac5",
    scryfall_id = "713160be-bcfd-4ef5-9457-c02b926ba06e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Cataclysm",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
