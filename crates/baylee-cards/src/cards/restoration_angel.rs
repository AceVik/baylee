//! Restoration Angel — {3}{W} — Creature — Angel
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When this creature enters, you may exile target non-Angel creature you control, then return that card to the battlefield under your control.
//! Set: INR #38 — Innistrad Remastered | Scryfall ID: f17f85d3-58e5-4128-90c5-98b524256af8 | Oracle ID: dfbd3afc-9905-4cff-a4f4-df08a4d0a7fa
// IMPLEMENTED — flash flying + immediate blink of a non-Angel creature.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static NON_ANGEL_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::Not(&Filter::HasSubtype(creature::ANGEL)),
    Filter::ControlledByYou,
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(131),
    oracle_id: "dfbd3afc-9905-4cff-a4f4-df08a4d0a7fa",
    scryfall_id: "f17f85d3-58e5-4128-90c5-98b524256af8",
    faces: &[FaceDef {
        name: "Restoration Angel",
        mana_cost: baylee_core::mana!("{3}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::ANGEL],
        power: Some(3),
        toughness: Some(4),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::Blink {
            target: TargetSpec::Object(&NON_ANGEL_CREATURE_YOU_CONTROL),
        }],
        targets: Some(baylee_cards_dsl::TargetReq {
            spec: TargetSpec::Object(&NON_ANGEL_CREATURE_YOU_CONTROL),
            min: 0,
            max: 1,
            count_is_x: false,
        }),
    }],
};

#[cfg(test)]
mod tests {}
