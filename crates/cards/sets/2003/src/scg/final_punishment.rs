//! Final Punishment — {3}{B}{B} — Sorcery
//! Oracle: Target player loses life equal to the damage already dealt to that player this turn.
//! Set: 9ED #131 — Ninth Edition | Scryfall ID: 72df60bb-3309-43e0-ab4a-cec1390fc7cf | Oracle ID: 74344ca9-b057-4864-a374-3b399389765f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FINAL_PUNISHMENT,
    oracle_id = "74344ca9-b057-4864-a374-3b399389765f",
    scryfall_id = "72df60bb-3309-43e0-ab4a-cec1390fc7cf",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Final Punishment",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
