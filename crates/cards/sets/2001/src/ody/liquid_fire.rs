//! Liquid Fire — {4}{R}{R} — Sorcery
//! Oracle: As an additional cost to cast this spell, choose a number between 0 and 5.
//! Oracle: Liquid Fire deals X damage to target creature and 5 minus X damage to that creature's controller, where X is the chosen number.
//! Set: ODY #201 — Odyssey | Scryfall ID: 8b836d4c-38f3-4661-9dc7-0c9baaa595a5 | Oracle ID: 867d603d-3aa4-4160-9112-a18f795f3d53
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIQUID_FIRE,
    oracle_id = "867d603d-3aa4-4160-9112-a18f795f3d53",
    scryfall_id = "8b836d4c-38f3-4661-9dc7-0c9baaa595a5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Liquid Fire",
        mana_cost = mana!("{4}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
