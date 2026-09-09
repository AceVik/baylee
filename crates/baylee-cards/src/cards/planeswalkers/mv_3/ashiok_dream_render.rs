//! Ashiok, Dream Render — {1}{U/B}{U/B} — Legendary Planeswalker — Ashiok
//! Oracle: Spells and abilities your opponents control can't cause their controller to search their library.
//! Oracle: −1: Target player mills four cards. Then exile each opponent's graveyard.
//! Set: WAR #228 — War of the Spark | Scryfall ID: f2df3258-c053-48a8-974f-d80899b2cd93 | Oracle ID: 93723b12-db34-4047-885e-8606415b1553
// IMPLEMENTED — search suppression (OpponentsCantSearch checked in the
// search ops) + the mill/exile-graveyards loyalty ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::planeswalker;

card! {
    index: 8,
    oracle_id: "93723b12-db34-4047-885e-8606415b1553",
    scryfall_id: "f2df3258-c053-48a8-974f-d80899b2cd93",
    faces: &[face! {
        name: "Ashiok, Dream Render",
        mana_cost: baylee_core::mana!("{1}{U/B}{U/B}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::ASHIOK],
        loyalty: Some(5),
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Black]),
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::OpponentsCantSearch,
            cross_zone: false,
        }),
        loyalty!(-1, &[
                Effect::Mill {
                    amount: Amount::Fixed(4),
                    target: PlayerRel::ControllerOfTarget,
                },
                Effect::ExileGraveyard {
                    player: PlayerRel::EachOpponent,
                },
            ], target: Some(TargetSpec::AnyPlayer)),
    ],
}
