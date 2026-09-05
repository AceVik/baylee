//! Akoum Warrior // Akoum Teeth — {5}{R} — Creature — Minotaur Warrior // Land
//! Set: ZNR #134 — Zendikar Rising | Scryfall ID: d8ed0335-daa6-4dbe-a94d-4d56c8cfd093 | Oracle ID: afedce7b-0e18-40ad-a26a-1933fddb560d
//! Face: Akoum Warrior — {5}{R} — Creature — Minotaur Warrior
//! Face: Akoum Teeth —  — Land
// IMPLEMENTED — trample on front, tapland back ({T}: Add {R}).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static TEETH_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card! {
    index: 215,
    oracle_id: "afedce7b-0e18-40ad-a26a-1933fddb560d",
    scryfall_id: "d8ed0335-daa6-4dbe-a94d-4d56c8cfd093",
    color_identity: ColorSet::from_slice(&[Color::Red]),
    keywords: KeywordSet::TRAMPLE,
    coverage: Coverage::Implemented,
    faces: &[
        face! {
            name: "Akoum Warrior",
            mana_cost: baylee_core::mana!("{5}{R}"),
            types: TypeSet::CREATURE,
            subtypes: &[subtypes::creature::MINOTAUR, subtypes::creature::WARRIOR],
            power: Some(4),
            toughness: Some(5),
        },
        face! {
            name: "Akoum Teeth",
            types: TypeSet::LAND,
            enter_modifiers: &[EnterModifier::Tapped],
            abilities: TEETH_MANA,
        },
    ],
}
