//! Sword of the Meek — {2} — Artifact — Equipment
//! Oracle: Equipped creature gets +1/+2.
//! Oracle: Equip {2}
//! Oracle: Whenever a 1/1 creature you control enters, you may return this card from your graveyard to the battlefield, then attach it to that creature.
//! Set: 2XM #299 — Double Masters | Scryfall ID: 5a0c2773-3205-4ac4-b31c-c54fb06fdd7c | Oracle ID: 215c287d-56a5-46da-b49e-8524b6d320a4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SWORD_OF_THE_MEEK,
    oracle_id = "215c287d-56a5-46da-b49e-8524b6d320a4",
    scryfall_id = "5a0c2773-3205-4ac4-b31c-c54fb06fdd7c",
    faces = &[face!(
        name = "Sword of the Meek",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
