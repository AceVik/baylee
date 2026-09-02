//! Werefox Bodyguard — {1}{W}{W} — Creature — Elf Fox Knight
//! Oracle: Flash
//! Oracle: When this creature enters, exile up to one other target non-Fox creature until this creature leaves the battlefield.
//! Oracle: {1}{W}, Sacrifice this creature: You gain 2 life.
//! Set: WOE #39 — Wilds of Eldraine | Scryfall ID: 4494dfa1-1343-417e-b0c5-2b096442dd0e | Oracle ID: d5ee2ced-29f4-430f-962e-2f930b92624c
// IMPLEMENTED — flash, linked-exile ETB, sacrifice-for-life outlet.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

static OTHER_NON_FOX_CREATURE: Filter = Filter::And(&[
    Filter::Another,
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(creature::FOX)),
]);

card! {
    index: 190,
    oracle_id: "d5ee2ced-29f4-430f-962e-2f930b92624c",
    scryfall_id: "4494dfa1-1343-417e-b0c5-2b096442dd0e",
    faces: &[face! {
        name: "Werefox Bodyguard",
        mana_cost: baylee_core::mana!("{1}{W}{W}"),
        types: TypeSet::CREATURE,
        subtypes: &[creature::ELF, creature::FOX, creature::KNIGHT],
        power: Some(2),
        toughness: Some(2),
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLASH,
    coverage: Coverage::Implemented,
    abilities: &[
        triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::ExileLinked {
                target: TargetSpec::Object(&OTHER_NON_FOX_CREATURE),
            }], targets: Some(TargetReq {
                spec: TargetSpec::Object(&OTHER_NON_FOX_CREATURE),
                min: 0,
                max: 1,
                count_is_x: false,
            })),
        activated!(Cost {
                mana: baylee_core::mana!("{1}{W}"),
                parts: &[CostPart::SacrificeSelf],
            }, &[Effect::GainLife {
                amount: Amount::Fixed(2),
            }]),
    ],
}
