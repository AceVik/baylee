//! Deep Reconnaissance — {2}{G} — Sorcery
//! Oracle: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Oracle: Flashback {4}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #236 — Odyssey | Scryfall ID: 148ce683-3911-4b77-9e07-3985766a3777 | Oracle ID: a823e880-3d5f-4186-a7da-e710a7db583c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEEP_RECONNAISSANCE,
    oracle_id = "a823e880-3d5f-4186-a7da-e710a7db583c",
    scryfall_id = "148ce683-3911-4b77-9e07-3985766a3777",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Deep Reconnaissance",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
