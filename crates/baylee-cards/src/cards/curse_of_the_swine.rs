//! Curse of the Swine — {X}{U}{U} — Sorcery
//! Oracle: Exile X target creatures. For each creature exiled this way, its controller creates a 2/2 green Boar creature token.
//! Set: SOC #192 — Secrets of Strixhaven Commander | Scryfall ID: 91eb9067-0bc7-4497-ba9c-c1ea41e5a379 | Oracle ID: 5669ea7c-c4fc-494c-896b-4bce9b494817
// IMPLEMENTED — X-cost mass exile with per-controller Boar tokens.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec, TokenDef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature;
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_F: Filter = Filter::HasType(TypeSet::CREATURE);

static BOAR: TokenDef = TokenDef {
    name: "Boar",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::BOAR],
    power: Some(2),
    toughness: Some(2),
    keywords: KeywordSet::EMPTY,
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(27),
    oracle_id: "5669ea7c-c4fc-494c-896b-4bce9b494817",
    scryfall_id: "91eb9067-0bc7-4497-ba9c-c1ea41e5a379",
    faces: &[FaceDef {
        name: "Curse of the Swine",
        mana_cost: baylee_core::mana!("{X}{U}{U}"),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::ExileTargetsCreateTokens { token: &BOAR }],
        targets: Some(TargetReq::x_targets(TargetSpec::Object(&CREATURE_F))),
    }],
};

#[cfg(test)]
mod tests {
    // X creatures exiled; each controller gets a Boar per exiled creature.
}
