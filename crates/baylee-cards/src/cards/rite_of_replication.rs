//! Rite of Replication — {2}{U}{U} — Sorcery
//! Oracle: Kicker {5} (You may pay an additional {5} as you cast this spell.)
//! Oracle: Create a token that's a copy of target creature. If this spell was kicked, create five of those tokens instead.
//! Set: SOC #202 — Secrets of Strixhaven Commander | Scryfall ID: 5032d71d-d9f8-498c-97d1-271c2e9c1c47 | Oracle ID: fb60739e-1dc3-481d-a056-ad72e665c680
// IMPLEMENTED — kicker + 1 or 5 token copies.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Cost, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(135),
    oracle_id: "fb60739e-1dc3-481d-a056-ad72e665c680",
    scryfall_id: "5032d71d-d9f8-498c-97d1-271c2e9c1c47",
    faces: &[FaceDef {
        name: "Rite of Replication",
        mana_cost: baylee_core::mana!("{2}{U}{U}"),
        types: TypeSet::SORCERY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[Cost {
            mana: baylee_core::mana!("{5}"),
            parts: &[],
        }],
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
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::CreateTokenCopyOf {
            target: Some(TargetSpec::Object(&ANY_CREATURE)),
            kicked_bonus: 4,
        }],
        targets: Some(TargetReq::one(TargetSpec::Object(&ANY_CREATURE))),
    }],
};

#[cfg(test)]
mod tests {}
