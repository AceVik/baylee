//! Breaking Point — {1}{R}{R} — Sorcery
//! Oracle: Any player may have Breaking Point deal 6 damage to them. If no one does, destroy all creatures. Creatures destroyed this way can't be regenerated.
//! Set: DDK #67 — Duel Decks: Sorin vs. Tibalt | Scryfall ID: 73e348dd-2d8a-4ee6-a834-2d5861566043 | Oracle ID: a648b9ed-810a-41db-af40-3b5b4658db90
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BREAKING_POINT,
    oracle_id = "a648b9ed-810a-41db-af40-3b5b4658db90",
    scryfall_id = "73e348dd-2d8a-4ee6-a834-2d5861566043",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Breaking Point",
        mana_cost = mana!("{1}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
