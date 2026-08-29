//! Machine God's Effigy — {4} — Artifact
//! Oracle: You may have this artifact enter as a copy of any creature on the battlefield, except it's an artifact and it has "{T}: Add {U}." (It's not a creature.)
//! Oracle: {T}: Add {U}.
//! Set: BRC #16 — The Brothers' War Commander | Scryfall ID: 637f69c2-ba24-42d1-9345-8ebdb04b6904 | Oracle ID: 64ebdd6f-acde-4aab-a86b-2798bad5f70c
// IMPLEMENTED — clone as noncreature artifact + blue mana tap.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, CopyMod, Cost, Coverage,
    Effect, FaceDef, Filter, KeywordSet, PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(89),
    oracle_id: "64ebdd6f-acde-4aab-a86b-2798bad5f70c",
    scryfall_id: "637f69c2-ba24-42d1-9345-8ebdb04b6904",
    faces: &[FaceDef {
        name: "Machine God's Effigy",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&ANY_CREATURE),
            mods: &[
                CopyMod::AddType(TypeSet::ARTIFACT),
                CopyMod::RemoveType(TypeSet::CREATURE),
            ],
        },
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Blue,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
