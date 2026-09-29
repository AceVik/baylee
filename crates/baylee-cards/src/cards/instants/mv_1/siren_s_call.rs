//! Siren's Call — {U} — Instant
//! Oracle: Cast this spell only during an opponent's turn, before attackers are declared.
//! Oracle: Creatures the active player controls attack this turn if able.
//! Oracle: At the beginning of the next end step, destroy all non-Wall creatures that player controls that didn't attack this turn. Ignore this effect for each creature the player didn't control continuously since the beginning of the turn.
//! Set: 4ED #101 — Fourth Edition | Scryfall ID: 51832cfb-0a2e-4674-bb36-38027a71ac6d | Oracle ID: 269fc857-a052-4f0a-9759-467ccf42bebb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SIREN_S_CALL,
    oracle_id = "269fc857-a052-4f0a-9759-467ccf42bebb",
    scryfall_id = "51832cfb-0a2e-4674-bb36-38027a71ac6d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Siren's Call",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
