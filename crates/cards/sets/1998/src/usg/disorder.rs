//! Disorder — {1}{R} — Sorcery
//! Oracle: Disorder deals 2 damage to each white creature and each player who controls a white creature.
//! Set: 7ED #179 — Seventh Edition | Scryfall ID: b6d11422-60a9-4386-8e7f-dd7dcdac58d8 | Oracle ID: 586da84b-1384-4da2-983d-340c22b73eb8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

// Held back: Unsafe rule approximation found in Argon PR 394: missing printed cost, restriction, drawback, choice or target/zone semantics; the concrete finding for this card is recorded in docs/history/2026-10-23-argon-review.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DISORDER,
    oracle_id = "586da84b-1384-4da2-983d-340c22b73eb8",
    scryfall_id = "b6d11422-60a9-4386-8e7f-dd7dcdac58d8",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Disorder",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
