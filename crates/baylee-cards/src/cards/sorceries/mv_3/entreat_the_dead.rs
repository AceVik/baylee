//! Entreat the Dead — {X}{X}{B}{B}{B} — Sorcery
//! Oracle: Return X target creature cards from your graveyard to the battlefield.
//! Oracle: Miracle {X}{B}{B} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)
//! Set: C18 #15 — Commander 2018 | Scryfall ID: 31a147bb-37ef-4a52-82e2-160a53323516 | Oracle ID: 2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd
// IMPLEMENTED — X-target mass reanimation + miracle cast.

use baylee_cards_dsl::prelude::*;

card! {
    index: 43,
    oracle_id: "2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd",
    scryfall_id: "31a147bb-37ef-4a52-82e2-160a53323516",
    faces: &[face! {
        name: "Entreat the Dead",
        mana_cost: baylee_core::mana!("{X}{X}{B}{B}{B}"),
        types: TypeSet::SORCERY,
        miracle: Some(baylee_core::mana!("{X}{B}{B}")),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[spell!(&[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }], targets: Some(TargetReq::x_targets(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))))],
}
