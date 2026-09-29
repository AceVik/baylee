//! Crystal Rod — {1} — Artifact
//! Oracle: Whenever a player casts a blue spell, you may pay {1}. If you do, you gain 1 life.
//! Set: 8ED #295 — Eighth Edition | Scryfall ID: bbe82b91-69e2-4528-b80a-a61183c352ad | Oracle ID: e68bc048-1009-46a5-97d6-ec77a18067da
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CRYSTAL_ROD,
    oracle_id = "e68bc048-1009-46a5-97d6-ec77a18067da",
    scryfall_id = "bbe82b91-69e2-4528-b80a-a61183c352ad",
    faces = &[face!(
        name = "Crystal Rod",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
