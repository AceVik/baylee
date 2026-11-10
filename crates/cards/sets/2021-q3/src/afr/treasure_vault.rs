//! Treasure Vault — (no cost) — Artifact Land
//! Oracle: {T}: Add {C}.
//! Oracle: {X}{X}, {T}, Sacrifice this land: Create X Treasure tokens.
//! Set: AFR #261 — Adventures in the Forgotten Realms | Scryfall ID: a0931eb3-b403-4b1a-ad46-a7b0a51bb9a4 | Oracle ID: 3c43efd6-b1a8-452c-ae20-9a936c3340ab
// IMPLEMENTED — {T} for {C}, plus {X}{X}, {T}, Sacrifice this: X Treasures.

use crate::tokens::TREASURE;
use baylee_cards_dsl::prelude::*;

card!(
    index = index::TREASURE_VAULT,
    oracle_id = "3c43efd6-b1a8-452c-ae20-9a936c3340ab",
    scryfall_id = "a0931eb3-b403-4b1a-ad46-a7b0a51bb9a4",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Treasure Vault",
        types = TypeSet::ARTIFACT.union(TypeSet::LAND),
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{X}{X}", TapSelf, SacrificeSelf),
            &[Effect::CreateTokenN {
                token: &TREASURE,
                amount: Amount::X,
            }]
        ),
    ],
);
