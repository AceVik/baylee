//! Spark Double — {3}{U} — Creature — Illusion
//! Oracle: You may have this creature enter as a copy of a creature or planeswalker you control, except it enters with an additional +1/+1 counter on it if it's a creature, it enters with an additional loyalty counter on it if it's a planeswalker, and it isn't legendary.
//! Set: RVR #62 — Ravnica Remastered | Scryfall ID: c41b9ba2-0006-4d8e-b600-efe81ff5e0cc | Oracle ID: 8dcb35e5-ae44-455f-86e3-4a77d496ff34
// IMPLEMENTED — clone of your creatures/walkers with counter + not legendary.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, CopyMod, CounterKind, Coverage, FaceDef, Filter,
    KeywordSet, PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_CREATURE: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[
        Filter::HasType(TypeSet::CREATURE),
        Filter::HasType(TypeSet::PLANESWALKER),
    ]),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(154),
    oracle_id: "8dcb35e5-ae44-455f-86e3-4a77d496ff34",
    scryfall_id: "c41b9ba2-0006-4d8e-b600-efe81ff5e0cc",
    faces: &[FaceDef {
        name: "Spark Double",
        mana_cost: baylee_core::mana!("{3}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::ILLUSION],
        power: Some(0),
        toughness: Some(0),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("loyalty counter on walker-copy needs walker support (M2.S7c+)"),
    abilities: &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&YOUR_CREATURE),
        mods: &[
            CopyMod::RemoveSupertype(SupertypeSet::LEGENDARY),
            CopyMod::AddCounter(CounterKind::P1P1, 1),
        ],
    }],
};

#[cfg(test)]
mod tests {}
