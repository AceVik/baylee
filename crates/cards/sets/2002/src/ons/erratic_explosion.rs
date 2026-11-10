//! Erratic Explosion — {2}{R} — Sorcery
//! Oracle: Choose any target. Reveal cards from the top of your library until you reveal a nonland card. Erratic Explosion deals damage equal to that card's mana value to that permanent or player. Put the revealed cards on the bottom of your library in any order.
//! Set: PCA #41 — Planechase Anthology | Scryfall ID: 14c5e0aa-d491-4caf-86c7-555ee1bb637b | Oracle ID: 8e560a3a-c81b-43b3-b622-5c66ef655647
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ERRATIC_EXPLOSION,
    oracle_id = "8e560a3a-c81b-43b3-b622-5c66ef655647",
    scryfall_id = "14c5e0aa-d491-4caf-86c7-555ee1bb637b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Erratic Explosion",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
