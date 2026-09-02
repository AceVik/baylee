//! Crib Swap — {2}{W} — Kindred Instant — Shapeshifter
//! Oracle: Changeling (This card is every creature type.)
//! Oracle: Exile target creature. Its controller creates a 1/1 colorless Shapeshifter creature token with changeling.
//! Set: C18 #12 — Commander 2018 | Scryfall ID: 8f2fb3c6-af75-47a3-9f97-521872c32890 | Oracle ID: 2987c385-011a-4032-a516-a46d1e9dc9e8
// IMPLEMENTED — kindred/changeling + exile with shapeshifter token.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

use crate::tokens::SHAPESHIFTER_1_1_CHANGELING as SHAPESHIFTER_TOKEN;

card! {
    index: 26,
    oracle_id: "2987c385-011a-4032-a516-a46d1e9dc9e8",
    scryfall_id: "8f2fb3c6-af75-47a3-9f97-521872c32890",
    faces: &[face! {
        name: "Crib Swap",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::KINDRED.union(TypeSet::INSTANT),
        subtypes: &[creature::SHAPESHIFTER],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::CHANGELING,
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[
            Effect::Exile {
                target: TargetSpec::Object(&Filter::CREATURE),
            },
            Effect::CreateTokenForTargetController {
                token: &SHAPESHIFTER_TOKEN,
            },
        ], targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))],
}
