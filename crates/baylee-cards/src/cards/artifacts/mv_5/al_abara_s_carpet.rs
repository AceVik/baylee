//! Al-abara's Carpet — {5} — Artifact
//! Oracle: {5}, {T}: Prevent all damage that would be dealt to you this turn by attacking creatures without flying.
//! Set: ME4 #177 — Masters Edition IV | Scryfall ID: e9f381b3-fbfd-4bd0-87e8-5c2615f8e8e8 | Oracle ID: 0f5b0c77-1e3d-46a1-ae0e-03ed79196cd9
// PARTIAL — nothing is built: the prevention cannot be stated.
// NOT SUPPORTED: "{5}, {T}: Prevent all damage that would be dealt to you
// this turn by attacking creatures without flying." — no effect or modifier
// prevents damage to a player for a turn; `Effect::PreventNextDamage` is one
// finite shield, `Effect::PreventAllCombatDamageThisTurn` covers every
// source, and `Effect::PreventNextFromChosenSource` protects against one
// source of the controller's choice.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AL_ABARA_S_CARPET,
    oracle_id = "0f5b0c77-1e3d-46a1-ae0e-03ed79196cd9",
    scryfall_id = "e9f381b3-fbfd-4bd0-87e8-5c2615f8e8e8",
    faces = &[face!(
        name = "Al-abara's Carpet",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no effect or modifier prevents all damage to a player from attacking \
         creatures for a turn",
    ),
);
