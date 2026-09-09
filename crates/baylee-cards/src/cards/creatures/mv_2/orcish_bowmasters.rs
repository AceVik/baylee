//! Orcish Bowmasters — {1}{B} — Creature — Orc Archer
//! Oracle: Flash
//! Oracle: When this creature enters, it deals 1 damage to target opponent. Amass Orcs 1.
//! Oracle: Whenever an opponent draws a card except the first one they draw each turn, amass Orcs 1.
//! Set: LTR #103 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 7c024bae-5631-4e20-ac69-df392ac9e109 | Oracle ID: ea5103f5-27e0-4eb1-902c-7f34652d6bf3
// IMPLEMENTED — flash + ping + amass on opponents' extra draws.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card! {
    index: 106,
    oracle_id: "ea5103f5-27e0-4eb1-902c-7f34652d6bf3",
    scryfall_id: "7c024bae-5631-4e20-ac69-df392ac9e109",
    faces: &[face! {
        name: "Orcish Bowmasters",
        mana_cost: baylee_core::mana!("{1}{B}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::ORC, creature::ARCHER],
        power: Some(1),
        toughness: Some(1),
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::FLASH,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[
                Effect::DealDamage {
                    amount: Amount::Fixed(1),
                    target: TargetSpec::Player(PlayerRel::Opponent),
                },
                Effect::Amass {
                    token: &crate::tokens::ARMY_0_0_BLACK,
                    subtype: creature::ORC,
                    amount: 1,
                },
            ]),
        triggered!(Trigger::DrawsExceptFirst(PlayerRel::Opponent), &[Effect::Amass {
                token: &crate::tokens::ARMY_0_0_BLACK,
                subtype: creature::ORC,
                amount: 1,
            }]),
    ],
}
