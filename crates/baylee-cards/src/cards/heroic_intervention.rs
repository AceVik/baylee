//! Heroic Intervention — {1}{G} — Instant
//! Oracle: Permanents you control gain hexproof and indestructible until end of turn.
//! Set: FDN #211 — Foundations | Scryfall ID: e32c67d1-187f-40df-b3b3-6036f5c92834 | Oracle ID: 24882fa2-3fe9-4c1b-aa3d-0e6488b9db27
// IMPLEMENTED — team hexproof + indestructible until end of turn (layer 6).

static HEXPROOF_INDESTRUCTIBLE: KeywordSet = KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE);

use baylee_cards_dsl::prelude::*;

card! {
    index: 70,
    oracle_id: "24882fa2-3fe9-4c1b-aa3d-0e6488b9db27",
    scryfall_id: "e32c67d1-187f-40df-b3b3-6036f5c92834",
    faces: &[face! {
        name: "Heroic Intervention",
        mana_cost: baylee_core::mana!("{1}{G}"),
        types: TypeSet::INSTANT,
    }],
    color_identity: ColorSet::from_slice(&[Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::CreateContinuousEffect {
            layer: Layer::Ability,
            filter: &Filter::ControlledByYou,
            modifier: Modifier::AddKeyword(HEXPROOF_INDESTRUCTIBLE),
            duration: Duration::UntilEndOfTurn,
        }])],
}
