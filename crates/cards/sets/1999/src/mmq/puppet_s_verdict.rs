//! Puppet's Verdict — {1}{R}{R} — Instant
//! Oracle: Flip a coin. If you win the flip, destroy all creatures with power 2 or less. If you lose the flip, destroy all creatures with power 3 or greater.
//! Set: MMQ #208 — Mercadian Masques | Scryfall ID: 052b743a-456d-49c3-881e-4f30c7645fa5 | Oracle ID: e07a7c75-b7b7-4c2d-bd7d-5f056cde871d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PUPPET_S_VERDICT,
    oracle_id = "e07a7c75-b7b7-4c2d-bd7d-5f056cde871d",
    scryfall_id = "052b743a-456d-49c3-881e-4f30c7645fa5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Puppet's Verdict",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
