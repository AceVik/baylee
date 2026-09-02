//! Padeem, Consul of Innovation — {3}{U} — Legendary Creature — Vedalken Artificer
//! Oracle: Artifacts you control have hexproof. (They can't be the targets of spells or abilities your opponents control.)
//! Oracle: At the beginning of your upkeep, if you control the artifact with the greatest mana value or tied for the greatest mana value, draw a card.
//! Set: CMM #109 — Commander Masters | Scryfall ID: 00a4aef8-64fc-4e9d-adac-ef4c85d40b4a | Oracle ID: 0c7ba712-6a99-4d2f-9242-a2163a11f69c
// IMPLEMENTED — hexproof grant + the greatest-cmc upkeep draw
// (IfControlGreatestCmc).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 108,
    oracle_id: "0c7ba712-6a99-4d2f-9242-a2163a11f69c",
    scryfall_id: "00a4aef8-64fc-4e9d-adac-ef4c85d40b4a",
    faces: &[face! {
        name: "Padeem, Consul of Innovation",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::VEDALKEN, creature::ARTIFICER],
        power: Some(1),
        toughness: Some(4),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            }, &[Effect::IfControlGreatestCmc {
                filter: &Filter::ARTIFACT,
                then: &[Effect::DrawCards {
                    amount: Amount::Fixed(1),
                }],
            }]),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou]),
            modifier: Modifier::AddKeyword(KeywordSet::HEXPROOF),
            cross_zone: false,
        }),
    ],
}
