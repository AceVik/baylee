//! Balancing Act — {2}{W}{W} — Sorcery
//! Oracle: Each player chooses a number of permanents they control equal to the number of permanents controlled by the player who controls the fewest, then sacrifices the rest. Each player discards cards the same way.
//! Set: ODY #10 — Odyssey | Scryfall ID: 7d6cb368-43f7-4d06-9cd5-492df5766567 | Oracle ID: 4c68282b-776f-463a-9652-f611538c53ee
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BALANCING_ACT,
    oracle_id = "4c68282b-776f-463a-9652-f611538c53ee",
    scryfall_id = "7d6cb368-43f7-4d06-9cd5-492df5766567",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Balancing Act",
        mana_cost = mana!("{2}{W}{W}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
