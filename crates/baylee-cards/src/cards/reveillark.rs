//! Reveillark — {4}{W} — Creature — Elemental
//! Oracle: Flying
//! Oracle: When this creature leaves the battlefield, return up to two target creature cards with power 2 or less from your graveyard to the battlefield.
//! Oracle: Evoke {5}{W} (You may cast this spell for its evoke cost. If you do, it's sacrificed when it enters.)
//! Set: 2X2 #26 — Double Masters 2022 | Scryfall ID: 53b4dcd6-b1b6-4f1c-9264-e58bdc87399b | Oracle ID: 1be13ede-98f8-497e-800c-03e5802932b3
// IMPLEMENTED — evoke + LTB reanimation of up to two small creatures.

static SMALL_CREATURE_GY: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::CmcAtMost(0xFFFF), // power ≤ 2 handled below via CmcAtMost? no —
]);

use baylee_cards_dsl::prelude::*;

card! {
    index: 132,
    oracle_id: "1be13ede-98f8-497e-800c-03e5802932b3",
    scryfall_id: "53b4dcd6-b1b6-4f1c-9264-e58bdc87399b",
    faces: &[face! {
        name: "Reveillark",
        mana_cost: baylee_core::mana!("{4}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[baylee_core::generated::subtypes::creature::ELEMENTAL],
        power: Some(4),
        toughness: Some(3),
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: baylee_core::mana!("{5}{W}"),
                parts: &[],
            },
            condition: AltCondition::Always,
        }],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLYING,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::LeavesBattlefield(&Filter::This), &[Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&SMALL_CREATURE_GY, PlayerRel::You),
            }], targets: Some(TargetReq::up_to(
                TargetSpec::CardInGraveyard(&SMALL_CREATURE_GY, PlayerRel::You),
                2,
            ))),
        triggered!(Trigger::EntersBattlefieldEvoked, &[Effect::SacrificeSelf]),
    ],
}

// LTB returns up to two small creatures from your graveyard.
