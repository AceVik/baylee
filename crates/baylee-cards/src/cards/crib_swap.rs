//! Crib Swap — {2}{W} — Kindred Instant — Shapeshifter
//! Oracle: Changeling (This card is every creature type.)
//! Oracle: Exile target creature. Its controller creates a 1/1 colorless Shapeshifter creature token with changeling.
//! Set: C18 #12 — Commander 2018 | Scryfall ID: 8f2fb3c6-af75-47a3-9f97-521872c32890 | Oracle ID: 2987c385-011a-4032-a516-a46d1e9dc9e8
// IMPLEMENTED — kindred/changeling + exile with shapeshifter token.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec, TokenDef,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_F: Filter = Filter::HasType(TypeSet::CREATURE);
static SHAPESHIFTER: TokenDef = TokenDef {
    name: "Shapeshifter",
    colors: ColorSet::EMPTY,
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::EMPTY,
    subtypes: &[creature::SHAPESHIFTER],
    power: Some(1),
    toughness: Some(1),
    keywords: KeywordSet::CHANGELING,
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(26),
    oracle_id: "2987c385-011a-4032-a516-a46d1e9dc9e8",
    scryfall_id: "8f2fb3c6-af75-47a3-9f97-521872c32890",
    faces: &[FaceDef {
        name: "Crib Swap",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::KINDRED.union(TypeSet::INSTANT),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::SHAPESHIFTER],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::CHANGELING,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::Exile {
                target: TargetSpec::Object(&CREATURE_F),
            },
            Effect::CreateTokenForTargetController {
                token: &SHAPESHIFTER,
            },
        ],
        targets: Some(TargetReq::one(TargetSpec::Object(&CREATURE_F))),
    }],
};

#[cfg(test)]
mod tests {}
