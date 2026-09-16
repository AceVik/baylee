//! Mana Vault — {1} — Artifact
//! Oracle: This artifact doesn't untap during your untap step.
//! Oracle: At the beginning of your upkeep, you may pay {4}. If you do, untap this artifact.
//! Oracle: At the beginning of your draw step, if this artifact is tapped, it deals 1 damage to you.
//! Oracle: {T}: Add {C}{C}{C}.
//! Set: 2X2 #308 — Double Masters 2022 | Scryfall ID: c1a31d52-a407-4ded-bfca-cc812f11afa0 | Oracle ID: 736892cb-a34b-4bb9-b56c-e26e3db207a2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_VAULT,
    oracle_id = "736892cb-a34b-4bb9-b56c-e26e3db207a2",
    scryfall_id = "c1a31d52-a407-4ded-bfca-cc812f11afa0",
    faces = &[face!(
        name = "Mana Vault",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
