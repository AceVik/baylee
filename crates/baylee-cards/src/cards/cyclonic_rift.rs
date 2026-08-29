//! Cyclonic Rift — {1}{U} — Instant
//! Oracle: Return target nonland permanent you don't control to its owner's hand.
//! Oracle: Overload {6}{U} (You may cast this spell for its overload cost. If you do, change "target" in its text to "each.")
//! Set: RVR #40 — Ravnica Remastered | Scryfall ID: dfb7c4b9-f2f4-4d4e-baf2-86551c8150fe | Oracle ID: d75b9c82-1b49-4c3e-a1b5-aeef57d6644b
// IMPLEMENTED — modal spell: single-target bounce or overloaded mass bounce
// (choose cast mode in the wizard).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SpellMode, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static NONLAND_OPPONENT: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::LacksType(TypeSet::LAND),
]);
static NONLAND: Filter = Filter::LacksType(TypeSet::LAND);

static NORMAL_EFFECTS: &[Effect] = &[Effect::ReturnToHand {
    target: TargetSpec::Object(&NONLAND_OPPONENT),
}];
static OVERLOAD_EFFECTS: &[Effect] = &[Effect::ReturnAllToHand {
    filter: &NONLAND,
    opponents_only: true,
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(29),
    oracle_id: "d75b9c82-1b49-4c3e-a1b5-aeef57d6644b",
    scryfall_id: "dfb7c4b9-f2f4-4d4e-baf2-86551c8150fe",
    faces: &[FaceDef {
        name: "Cyclonic Rift",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::INSTANT,
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalSpell {
        modes: &[
            SpellMode {
                effects: NORMAL_EFFECTS,
                target: Some(TargetSpec::Object(&NONLAND_OPPONENT)),
                cost_override: None,
            },
            SpellMode {
                effects: OVERLOAD_EFFECTS,
                target: None,
                cost_override: Some(baylee_core::mana!("{6}{U}")),
            },
        ],
    }],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage in baylee-engine s7 tests: both modes resolve.
}
