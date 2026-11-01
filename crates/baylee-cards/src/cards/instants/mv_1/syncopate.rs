//! Syncopate — {X}{U} — Instant
//! Oracle: Counter target spell unless its controller pays {X}. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.
//! Set: FIN #80 — Final Fantasy | Scryfall ID: 83e1cd8d-5b97-427a-be75-2a1947f9c59b | Oracle ID: 318f7f70-e374-40ef-8afb-3389c10461d8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYNCOPATE,
    oracle_id = "318f7f70-e374-40ef-8afb-3389c10461d8",
    scryfall_id = "83e1cd8d-5b97-427a-be75-2a1947f9c59b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Syncopate",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
