//! Raise Dead — {B} — Sorcery
//! Oracle: Return target creature card from your graveyard to your hand.
//! Set: W17 #18 — Welcome Deck 2017 | Scryfall ID: 4950c3c2-80c1-4447-ac38-cf40f76b9545 | Oracle ID: cbc9c731-181a-4f00-a7b0-eb7e56eac2ea
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAISE_DEAD,
    oracle_id = "cbc9c731-181a-4f00-a7b0-eb7e56eac2ea",
    scryfall_id = "4950c3c2-80c1-4447-ac38-cf40f76b9545",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Raise Dead",
        mana_cost = mana!("{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
