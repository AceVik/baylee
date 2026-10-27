//! Dirge of Dread — {2}{B} — Sorcery
//! Oracle: All creatures gain fear until end of turn. (They can't be blocked except by artifact creatures and/or black creatures.)
//! Oracle: Cycling {1}{B} ({1}{B}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may have target creature gain fear until end of turn.
//! Set: A25 #86 — Masters 25 | Scryfall ID: d69a1a58-6aab-4feb-850c-31fd2220bbc0 | Oracle ID: be7b16ef-32aa-40d5-b287-c5e79d52d6b9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DIRGE_OF_DREAD,
    oracle_id = "be7b16ef-32aa-40d5-b287-c5e79d52d6b9",
    scryfall_id = "d69a1a58-6aab-4feb-850c-31fd2220bbc0",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Dirge of Dread",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
