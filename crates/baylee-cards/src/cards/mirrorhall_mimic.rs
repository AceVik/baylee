//! Mirrorhall Mimic // Ghastly Mimicry — {3}{U} — Creature — Spirit // Enchantment — Aura
//! Oracle: You may have Mirrorhall Mimic enter the battlefield as a copy of any creature on the battlefield, except it's a Spirit in addition to its other types. Disturb {5}{U}. // Enchant creature. Enchanted creature is a copy of Mirrorhall Mimic, except it's a Spirit in addition to its other types.
//! Set: VOW #68 — Innistrad: Crimson Vow | Scryfall ID: 823ad188-bd56-476d-9853-bed90bfad582 | Oracle ID: 5768fe50-a134-492c-a725-5ed02610c39f
// PARTIAL — the clone front is implemented. Disturb (casting the back
// face from the graveyard) needs graveyard face-casting (own milestone).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, CopyMod, Coverage, FaceDef, Filter, KeywordSet,
    PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature, enchantment};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(95),
    oracle_id: "5768fe50-a134-492c-a725-5ed02610c39f",
    scryfall_id: "823ad188-bd56-476d-9853-bed90bfad582",
    faces: &[
        FaceDef {
            name: "Mirrorhall Mimic",
            mana_cost: baylee_core::mana!("{3}{U}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[creature::SPIRIT],
            power: Some(2),
            toughness: Some(2),
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
            enter_modifiers: &[],
            abilities: &[],
            castable_from_hand: true,
            miracle: None,
            delve: false,
            convoke: false,
        },
        FaceDef {
            name: "Ghastly Mimicry",
            mana_cost: baylee_core::mana!("{5}{U}"),
            types: TypeSet::ENCHANTMENT,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[enchantment::AURA],
            power: None,
            toughness: None,
            loyalty: None,
            alternative_costs: &[],
            additional_costs: &[],
            mandatory_additional_costs: &[],
            enter_modifiers: &[],
            abilities: &[],
            castable_from_hand: false, // disturb: cast from the graveyard
            miracle: None,
            delve: false,
            convoke: false,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("disturb (graveyard back-face casting, own milestone)"),
    abilities: &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&ANY_CREATURE),
        mods: &[CopyMod::AddSubtype(creature::SPIRIT)],
    }],
};

#[cfg(test)]
mod tests {}
