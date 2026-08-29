//! Banishing Stroke — {5}{W} — Instant
//! Oracle: Put target artifact, creature, or enchantment on the bottom of its owner's library.
//! Oracle: Miracle {W} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)
//! Set: C18 #63 — Commander 2018 | Scryfall ID: aad93570-b50a-405a-ad73-03f97594061f | Oracle ID: a6898364-c29e-4b97-a500-344efa3ec24a
// IMPLEMENTED — bottom-of-library removal + miracle cast.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ARTIFACT_CREATURE_ENCHANTMENT: Filter = Filter::Or(&[
    Filter::HasType(TypeSet::ARTIFACT),
    Filter::HasType(TypeSet::CREATURE),
    Filter::HasType(TypeSet::ENCHANTMENT),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(11),
    oracle_id: "a6898364-c29e-4b97-a500-344efa3ec24a",
    scryfall_id: "aad93570-b50a-405a-ad73-03f97594061f",
    faces: &[FaceDef {
        name: "Banishing Stroke",
        mana_cost: baylee_core::mana!("{5}{W}"),
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
        miracle: Some(baylee_core::mana!("{W}")),
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
        effects: &[Effect::PutTargetOnBottomOfLibrary],
        targets: Some(TargetReq::one(TargetSpec::Object(
            &ARTIFACT_CREATURE_ENCHANTMENT,
        ))),
    }],
};

#[cfg(test)]
mod tests {}
