//! Cabal Therapy — {B} — Sorcery
//! Oracle: Choose a nonland card name. Target player reveals their hand and discards all cards with that name.
//! Oracle: Flashback—Sacrifice a creature. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: EMA #83 — Eternal Masters | Scryfall ID: 2f5b12f0-986f-43d0-a81b-64111e7f17e6 | Oracle ID: 018107d1-ebf7-4657-bc91-c8374af55cc5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CABAL_THERAPY,
    oracle_id = "018107d1-ebf7-4657-bc91-c8374af55cc5",
    scryfall_id = "2f5b12f0-986f-43d0-a81b-64111e7f17e6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cabal Therapy",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
