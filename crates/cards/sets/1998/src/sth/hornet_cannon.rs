//! Hornet Cannon — {4} — Artifact
//! Oracle: {3}, {T}: Create a 1/1 colorless Insect artifact creature token with flying and haste named Hornet. Destroy it at the beginning of the next end step.
//! Set: DDE #28 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: a3ea39a8-48d1-4a58-8662-88841eabec92 | Oracle ID: 595ae6f1-e66a-4774-943a-9d5368f4a3b4
// IMPLEMENTED — creates a 1/1 Hornet artifact creature token with flying and haste, destroyed at the next end step.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::HORNET_CANNON,
    oracle_id = "595ae6f1-e66a-4774-943a-9d5368f4a3b4",
    scryfall_id = "a3ea39a8-48d1-4a58-8662-88841eabec92",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hornet Cannon",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[
            Effect::CreateToken {
                token: &generated_tokens::HORNET_ARTIFACT_1_1_FLYING_HASTE,
            },
            Effect::AtNextEndStepForCreatedTokens {
                effects: &[Effect::destroy(TargetSpec::EventObject)],
            },
        ],
    )],
);
