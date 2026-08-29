//! Panharmonicon — {4} — Artifact
//! Oracle: If an artifact or creature entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.
//! Set: 2X2 #310 — Double Masters 2022 | Scryfall ID: 998d0cc8-ca2a-41c3-ab65-d05c26ab8278 | Oracle ID: 76678885-3674-443d-b9a2-2a460cf6aac0
// IMPLEMENTED — ETB-trigger multiplication for your permanents.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, PartnerKind,
    ReplacementRule, TriggerEventKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_PERMANENTS: Filter = Filter::ControlledByYou;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(110),
    oracle_id: "76678885-3674-443d-b9a2-2a460cf6aac0",
    scryfall_id: "998d0cc8-ca2a-41c3-ab65-d05c26ab8278",
    faces: &[FaceDef {
        name: "Panharmonicon",
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
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Replacement(
        ReplacementRule::TriggerMultiplier {
            source_filter: &YOUR_PERMANENTS,
            event: TriggerEventKind::EntersBattlefield,
        },
    )],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage in baylee-engine s6 tests: a rally trigger
    // fires twice with Panharmonicon on the battlefield.
}
