//! Rite of Replication — {2}{U}{U} — Sorcery
//! Oracle: Kicker {5} (You may pay an additional {5} as you cast this spell.)
//! Oracle: Create a token that's a copy of target creature. If this spell was kicked, create five of those tokens instead.
//! Set: SOC #202 — Secrets of Strixhaven Commander | Scryfall ID: 5032d71d-d9f8-498c-97d1-271c2e9c1c47 | Oracle ID: fb60739e-1dc3-481d-a056-ad72e665c680
// IMPLEMENTED — kicker + 1 or 5 token copies.

use baylee_cards_dsl::prelude::*;

card! {
    index: 135,
    oracle_id: "fb60739e-1dc3-481d-a056-ad72e665c680",
    scryfall_id: "5032d71d-d9f8-498c-97d1-271c2e9c1c47",
    faces: &[face! {
        name: "Rite of Replication",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::SORCERY,
        additional_costs: &[Cost {
            mana: baylee_core::mana!("{5}"),
            parts: &[],
        }],
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::CreateTokenCopyOf {
            target: Some(TargetSpec::Object(&Filter::CREATURE)),
            kicked_bonus: 4,
        }], targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))],
}
