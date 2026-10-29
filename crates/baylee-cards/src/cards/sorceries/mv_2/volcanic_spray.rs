//! Volcanic Spray — {1}{R} — Sorcery
//! Oracle: Volcanic Spray deals 1 damage to each creature without flying and each player.
//! Oracle: Flashback {1}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #226 — Odyssey | Scryfall ID: 97daab4b-d934-4a3f-a043-f7c9c1dd32bf | Oracle ID: dcf3cd72-3c3d-4847-83be-105966b0cd6a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::VOLCANIC_SPRAY,
    oracle_id = "dcf3cd72-3c3d-4847-83be-105966b0cd6a",
    scryfall_id = "97daab4b-d934-4a3f-a043-f7c9c1dd32bf",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Volcanic Spray",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
