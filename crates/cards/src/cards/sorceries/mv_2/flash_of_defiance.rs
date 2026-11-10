//! Flash of Defiance — {1}{R} — Sorcery
//! Oracle: Green creatures and white creatures can't block this turn.
//! Oracle: Flashback—{1}{R}, Pay 3 life. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: TOR #99 — Torment | Scryfall ID: 3e550776-b806-4f66-8aab-e6a5dac6d5cc | Oracle ID: 9f66855d-3ae8-4163-a5f1-aa23a9cd06a5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLASH_OF_DEFIANCE,
    oracle_id = "9f66855d-3ae8-4163-a5f1-aa23a9cd06a5",
    scryfall_id = "3e550776-b806-4f66-8aab-e6a5dac6d5cc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Flash of Defiance",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
