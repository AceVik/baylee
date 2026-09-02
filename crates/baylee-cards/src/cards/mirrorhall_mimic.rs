//! Mirrorhall Mimic // Ghastly Mimicry — {3}{U} — Creature — Spirit // Enchantment — Aura
//! Oracle: You may have Mirrorhall Mimic enter the battlefield as a copy of any creature on the battlefield, except it's a Spirit in addition to its other types. Disturb {5}{U}. // Enchant creature. Enchanted creature is a copy of Mirrorhall Mimic, except it's a Spirit in addition to its other types.
//! Set: VOW #68 — Innistrad: Crimson Vow | Scryfall ID: 823ad188-bd56-476d-9853-bed90bfad582 | Oracle ID: 5768fe50-a134-492c-a725-5ed02610c39f
// IMPLEMENTED — clone front + disturb (cast Ghastly Mimicry from the
// graveyard, exile on resolution).
// NOTE: Ghastly Mimicry's aura effect ("enchanted creature is a copy of
// Mirrorhall Mimic") is an aura-attachment rules item (M3+).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::{creature, enchantment};

card! {
    index: 95,
    oracle_id: "5768fe50-a134-492c-a725-5ed02610c39f",
    scryfall_id: "823ad188-bd56-476d-9853-bed90bfad582",
    faces: &[
        face! {
            name: "Mirrorhall Mimic",
            mana_cost: baylee_core::mana!("{3}{U}"),
            types: TypeSet::CREATURE,
            subtypes: &[creature::SPIRIT],
            power: Some(2),
            toughness: Some(2),
        },
        face! {
            name: "Ghastly Mimicry",
            mana_cost: baylee_core::mana!("{5}{U}"),
            types: TypeSet::ENCHANTMENT,
            subtypes: &[enchantment::AURA],
            castable_from_hand: false, // disturb: cast from the graveyard
            disturb: true,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods: &[CopyMod::AddSubtype(creature::SPIRIT)],
    }],
}
