//! Serpent Generator — {6} — Artifact
//! Oracle: {4}, {T}: Create a 1/1 colorless Snake artifact creature token. It has "Whenever this creature deals damage to a player, that player gets a poison counter." (A player with ten or more poison counters loses the game.)
//! Set: ME1 #164 — Masters Edition | Scryfall ID: 5e2ee41c-5592-42bd-8db2-92b3233b1d61 | Oracle ID: 0e3a11f4-c880-4932-bc7e-7aea8a9472bc

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SERPENT_GENERATOR,
    oracle_id = "0e3a11f4-c880-4932-bc7e-7aea8a9472bc",
    scryfall_id = "5e2ee41c-5592-42bd-8db2-92b3233b1d61",
    faces = &[face!(
        name = "Serpent Generator",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!("{4}", TapSelf),
        &[Effect::CreateToken {
            token: &crate::tokens::SNAKE_1_1_ARTIFACT_POISON
        }]
    ),],
);
