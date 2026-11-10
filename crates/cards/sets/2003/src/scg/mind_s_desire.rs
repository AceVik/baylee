//! Mind's Desire — {4}{U}{U} — Sorcery
//! Oracle: Shuffle your library. Then exile the top card of your library. Until end of turn, you may play that card without paying its mana cost.
//! Oracle: Storm (When you cast this spell, copy it for each spell cast before it this turn.)
//! Set: C21 #123 — Commander 2021 | Scryfall ID: 17ef3058-46b8-4ec4-950f-c721919c4ac1 | Oracle ID: b3739b5a-5731-4c64-a244-815a363b0d5c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_S_DESIRE,
    oracle_id = "b3739b5a-5731-4c64-a244-815a363b0d5c",
    scryfall_id = "17ef3058-46b8-4ec4-950f-c721919c4ac1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mind's Desire",
        mana_cost = mana!("{4}{U}{U}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
