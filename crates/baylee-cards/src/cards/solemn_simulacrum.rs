//! Solemn Simulacrum — {4} — Artifact Creature — Golem
//! Oracle: When this creature enters, you may search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.
//! Oracle: When this creature dies, you may draw a card.
//! Set: MSC #215 — Marvel Super Heroes Commander | Scryfall ID: daafd816-f7c1-4630-9e5c-a1e5db570a35 | Oracle ID: 00c0543c-2a1f-4425-8283-4062d74a1637
// IMPLEMENTED — ETB ramp + dies cantrip.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, SearchDest, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static BASIC_LAND: Filter = Filter::And(&[
    Filter::HasSupertype(SupertypeSet::BASIC),
    Filter::HasType(TypeSet::LAND),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(151),
    oracle_id: "00c0543c-2a1f-4425-8283-4062d74a1637",
    scryfall_id: "daafd816-f7c1-4630-9e5c-a1e5db570a35",
    faces: &[FaceDef {
        name: "Solemn Simulacrum",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::GOLEM],
        power: Some(2),
        toughness: Some(2),
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
        disturb: false,
        adventure: false,
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Triggered {
            trigger: Trigger::EntersBattlefield(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::SearchLibrary {
                filter: &BASIC_LAND,
                dest: SearchDest::Battlefield,
                tapped: true,
                shuffle: true,
                optional: true,
            }],
            targets: None,
        },
        AbilityDef::Triggered {
            trigger: Trigger::Dies(&Filter::This),
            once_per_turn: false,
            effects: &[Effect::DrawCards {
                amount: Amount::Fixed(1),
            }],
            targets: None,
        },
    ],
};

#[cfg(test)]
mod tests {}
