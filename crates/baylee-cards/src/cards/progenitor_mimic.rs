//! Progenitor Mimic — {4}{G}{U} — Creature — Shapeshifter
//! Oracle: You may have this creature enter as a copy of any creature on the battlefield, except it has "At the beginning of your upkeep, if this creature isn't a token, create a token that's a copy of this creature."
//! Set: 2XM #212 — Double Masters | Scryfall ID: acba72e1-3f7f-4e5c-af3f-dfe37b5d61f9 | Oracle ID: 88929ea9-900f-4dbb-b16c-cf3bad4e410c
// IMPLEMENTED — clone + upkeep token-copy-of-self.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, CopyMod, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, StepKind, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(121),
    oracle_id: "88929ea9-900f-4dbb-b16c-cf3bad4e410c",
    scryfall_id: "acba72e1-3f7f-4e5c-af3f-dfe37b5d61f9",
    faces: &[FaceDef {
        name: "Progenitor Mimic",
        mana_cost: baylee_core::mana!("{4}{G}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::SHAPESHIFTER],
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
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Green]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&ANY_CREATURE),
            mods: &[],
        },
        AbilityDef::Triggered {
            trigger: Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            once_per_turn: false,
            effects: &[Effect::CreateTokenCopyOf {
                target: None,
                kicked_bonus: 0,
            }],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {}
