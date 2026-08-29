//! Entreat the Dead — {X}{X}{B}{B}{B} — Sorcery
//! Oracle: Return X target creature cards from your graveyard to the battlefield.
//! Oracle: Miracle {X}{B}{B} (You may cast this card for its miracle cost when you draw it if it's the first card you drew this turn.)
//! Set: C18 #15 — Commander 2018 | Scryfall ID: 31a147bb-37ef-4a52-82e2-160a53323516 | Oracle ID: 2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd
// IMPLEMENTED — X-target mass reanimation + miracle cast.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    PlayerRel, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_CARD: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(43),
    oracle_id: "2de6c3d9-1759-40a2-99c6-8cbe17b4bcdd",
    scryfall_id: "31a147bb-37ef-4a52-82e2-160a53323516",
    faces: &[FaceDef {
        name: "Entreat the Dead",
        mana_cost: baylee_core::mana!("{X}{X}{B}{B}{B}"),
        types: TypeSet::SORCERY,
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
        miracle: Some(baylee_core::mana!("{X}{B}{B}")),
        delve: false,
        convoke: false,
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&CREATURE_CARD, PlayerRel::You),
        }],
        targets: Some(TargetReq::x_targets(TargetSpec::CardInGraveyard(
            &CREATURE_CARD,
            PlayerRel::You,
        ))),
    }],
};

#[cfg(test)]
mod tests {}
