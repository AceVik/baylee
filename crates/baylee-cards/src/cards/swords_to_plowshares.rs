//! Swords to Plowshares — {W} — Instant
//! Oracle: Exile target creature. Its controller gains life equal to its power.
//! Set: MSC #143 — Marvel Super Heroes Commander | Scryfall ID: b4e9c870-23c0-413a-ae39-265f09da16d1 | Oracle ID: b1544f21-7e98-461b-aed5-e748b0168c52
// IMPLEMENTED — exile removal + controller gains power as life.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(164),
    oracle_id: "b1544f21-7e98-461b-aed5-e748b0168c52",
    scryfall_id: "b4e9c870-23c0-413a-ae39-265f09da16d1",
    faces: &[FaceDef {
        name: "Swords to Plowshares",
        mana_cost: baylee_core::mana!("{W}"),
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
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::Exile {
                target: TargetSpec::Object(&CREATURE),
            },
            Effect::GainLifeFor {
                amount: Amount::TargetPower,
                who: PlayerRel::ControllerOfTarget,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::Object(&CREATURE))),
    }],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage via s4 scenario tests: the creature is exiled
    // (not destroyed) and its controller gains life equal to its power.
}
