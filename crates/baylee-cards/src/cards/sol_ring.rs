//! Sol Ring — {1} — Artifact
//! Oracle: {T}: Add {C}{C}.
//! Set: MSC #211 — Marvel Super Heroes Commander | Scryfall ID: 91fdb56b-54d5-4272-8319-505ff987fe9b | Oracle ID: 6ad8011d-3471-4369-9d68-b264cc027487
// IMPLEMENTED — mana rock (mana ability, resolves without the stack).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, CardDef, CommanderRule, Cost, Coverage, Effect, FaceDef,
    KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(150),
    oracle_id: "6ad8011d-3471-4369-9d68-b264cc027487",
    scryfall_id: "91fdb56b-54d5-4272-8319-505ff987fe9b",
    faces: &[FaceDef {
        name: "Sol Ring",
        mana_cost: baylee_core::mana!("{1}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::AddMana {
            color: ManaColor::Colorless,
            amount: 2,
        }],
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
    }],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage via s4 scenario tests: tapping adds {C}{C}
    // immediately (no stack object created).
}
