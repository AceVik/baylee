//! Primal Growth — {2}{G} — Sorcery
//! Oracle: Kicker—Sacrifice a creature. (You may sacrifice a creature in addition to any other costs as you cast this spell.)
//! Oracle: Search your library for a basic land card, put that card onto the battlefield, then shuffle. If this spell was kicked, instead search your library for up to two basic land cards, put them onto the battlefield, then shuffle.
//! Set: CMA #137 — Commander Anthology | Scryfall ID: 6664675f-1ef6-49ba-8f08-cfeac2f38ec7 | Oracle ID: 3cbdf37b-7fe3-4791-b33c-8591158b0ce5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PRIMAL_GROWTH,
    oracle_id = "3cbdf37b-7fe3-4791-b33c-8591158b0ce5",
    scryfall_id = "6664675f-1ef6-49ba-8f08-cfeac2f38ec7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Primal Growth",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
