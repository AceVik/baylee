//! Primaris Eliminator — {4}{B} — Creature — Astartes Warrior
//! Oracle: When this creature enters, choose one —
//! Oracle: • Executioner Round — Destroy target creature.
//! Oracle: • Hyperfrag Round — Creatures target player controls get -2/-2 until end of turn.
//! Set: 40K #50 — Warhammer 40,000 Commander | Scryfall ID: db7ab081-d6cd-4323-98bf-536e4df95115 | Oracle ID: 7d679591-f8ea-4c4c-ab98-7b9e3438cf57
// PARTIAL — ETB destruction works; NOT SUPPORTED yet: "choose one" modal
// triggers and the -2/-2 mode (modal trigger choices, M2.S8).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_F: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(118),
    oracle_id: "7d679591-f8ea-4c4c-ab98-7b9e3438cf57",
    scryfall_id: "db7ab081-d6cd-4323-98bf-536e4df95115",
    faces: &[FaceDef {
        name: "Primaris Eliminator",
        mana_cost: baylee_core::mana!("{4}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::ASTARTES, creature::WARRIOR],
        power: Some(3),
        toughness: Some(3),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("modal trigger choice + -2/-2 mode (M2.S8)"),
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        effects: &[Effect::Destroy {
            target: TargetSpec::Object(&CREATURE_F),
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&CREATURE_F))),
    }],
};

#[cfg(test)]
mod tests {}
