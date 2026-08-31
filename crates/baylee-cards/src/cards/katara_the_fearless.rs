//! Katara, the Fearless — {G}{W}{U} — Legendary Creature — Human Warrior Ally
//! Oracle: If a triggered ability of an Ally you control triggers, that ability triggers an additional time.
//! Set: TLA #230 — Avatar: The Last Airbender | Scryfall ID: b0a18f8b-7364-4375-b2e1-e2f15978517f | Oracle ID: 0972d46e-423b-454e-87c7-a2d40fb6fb6d
// IMPLEMENTED — Ally trigger multiplication for your permanents.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, PartnerKind,
    ReplacementRule, TriggerEventKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_ALLIES: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(82),
    oracle_id: "0972d46e-423b-454e-87c7-a2d40fb6fb6d",
    scryfall_id: "b0a18f8b-7364-4375-b2e1-e2f15978517f",
    faces: &[FaceDef {
        name: "Katara, the Fearless",
        mana_cost: baylee_core::mana!("{G}{W}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[creature::HUMAN, creature::WARRIOR, creature::ALLY],
        power: Some(2),
        toughness: Some(2),
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue, Color::Green]),
    commander: CommanderRule::Legendary,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplier {
            source_filter: &YOUR_ALLIES,
            event: TriggerEventKind::Any,
        },
    )],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
