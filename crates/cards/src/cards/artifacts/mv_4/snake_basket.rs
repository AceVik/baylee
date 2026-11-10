//! Snake Basket — {4} — Artifact
//! Oracle: {X}, Sacrifice this artifact: Create X 1/1 green Snake creature tokens. Activate only as a sorcery.
//! Set: 6ED #312 — Classic Sixth Edition | Scryfall ID: 3a9bc174-1d10-49bf-af4f-2f6061b1796e | Oracle ID: 12236231-7c6e-4edc-87dc-737115843185
// IMPLEMENTED

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::SNAKE_BASKET,
    oracle_id = "12236231-7c6e-4edc-87dc-737115843185",
    scryfall_id = "3a9bc174-1d10-49bf-af4f-2f6061b1796e",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Snake Basket",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{X}", SacrificeSelf),
        &[Effect::CreateTokenN {
            token: &generated_tokens::SNAKE_1_1_GREEN,
            amount: Amount::X,
        }],
        timing = ActivationTiming::SorcerySpeed,
    )],
);
