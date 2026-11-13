//! Parallel Evolution — {3}{G}{G} — Sorcery
//! Oracle: For each creature token on the battlefield, its controller creates a token that's a copy of that creature.
//! Oracle: Flashback {4}{G}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: TOR #136 — Torment | Scryfall ID: 73cce010-a8e7-477b-9179-bbad38aa6438 | Oracle ID: 963cfdbe-775c-4267-a617-66f10e9d7f4f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PARALLEL_EVOLUTION,
    oracle_id = "963cfdbe-775c-4267-a617-66f10e9d7f4f",
    scryfall_id = "73cce010-a8e7-477b-9179-bbad38aa6438",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Parallel Evolution",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
