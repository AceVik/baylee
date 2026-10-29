//! Allied Strategies — {4}{U} — Sorcery
//! Oracle: Domain — Target player draws a card for each basic land type among lands they control.
//! Set: DDE #63 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: 2dea7475-af87-4cee-bfdc-804351a632b5 | Oracle ID: a5ce2ccd-8c78-4749-8854-cb5fa5f4738e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ALLIED_STRATEGIES,
    oracle_id = "a5ce2ccd-8c78-4749-8854-cb5fa5f4738e",
    scryfall_id = "2dea7475-af87-4cee-bfdc-804351a632b5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Allied Strategies",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
