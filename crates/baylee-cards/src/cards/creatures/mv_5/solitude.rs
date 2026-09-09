//! Solitude — {3}{W}{W} — Creature — Elemental Incarnation
//! Oracle: Flash
//! Oracle: Lifelink
//! Oracle: When this creature enters, exile up to one other target creature. That creature's controller gains life equal to its power.
//! Oracle: Evoke—Exile a white card from your hand.
//! Set: MSC #37 — Marvel Super Heroes Commander | Scryfall ID: 47a6234f-309f-4e03-9263-66da48b57153 | Oracle ID: dcb9c2a7-ae54-4ddc-a567-640bf4bf4366
// IMPLEMENTED — flash/lifelink, exile ETB with life, pitch-evoke.

static WHITE_CARD: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::White]));

use baylee_cards_dsl::prelude::*;

card! {
    index: 152,
    oracle_id: "dcb9c2a7-ae54-4ddc-a567-640bf4bf4366",
    scryfall_id: "47a6234f-309f-4e03-9263-66da48b57153",
    faces: &[face! {
        name: "Solitude",
        mana_cost: baylee_core::mana!("{3}{W}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[
            baylee_core::generated::subtypes::creature::ELEMENTAL,
            baylee_core::generated::subtypes::creature::INCARNATION,
        ],
        power: Some(3),
        toughness: Some(2),
        alternative_costs: &[AlternativeCost {
            cost: Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::ExileFromHand(&WHITE_CARD)],
            },
            condition: AltCondition::Always,
        }],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLASH.union(KeywordSet::LIFELINK),
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[
                Effect::Exile {
                    target: TargetSpec::Object(&Filter::ANOTHER_CREATURE),
                },
                Effect::GainLifeFor {
                    amount: Amount::TargetPower,
                    who: PlayerRel::ControllerOfTarget,
                },
            ], targets: Some(TargetReq::up_to_one(TargetSpec::Object(&Filter::ANOTHER_CREATURE)))),
        triggered!(Trigger::EntersBattlefieldEvoked, &[Effect::SacrificeSelf]),
    ],
}

// Pitch path: exile a white card from hand, no mana spent; creature is
// sacrificed after its ETB.
