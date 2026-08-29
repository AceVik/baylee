//! Emeritus of Woe — {1}{B} — Creature — Vampire Warlock // Demonic Tutor (sorcery back face)
//! Oracle: Flying
//! Back face: Demonic Tutor — Search your library for a card, put that card into your hand, then shuffle.
//! Set: MH2 #92 — Modern Horizons 2 | Scryfall ID: 7eb9e83d-515d-4911-a06b-9982200277b2 | Oracle ID: 93056597-b964-421f-be2f-e92abef1c2a4
// IMPLEMENTED — flying 3/3 front, Demonic Tutor back face (cast either
// via the MDFC face choice).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SearchDest,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(41),
    oracle_id: "93056597-b964-421f-be2f-e92abef1c2a4",
    scryfall_id: "7eb9e83d-515d-4911-a06b-9982200277b2",
    faces: &[
        FaceDef {
            name: "Emeritus of Woe",
            mana_cost: baylee_core::mana!("{1}{B}"),
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::EMPTY,
            subtypes: &[creature::VAMPIRE, creature::WARLOCK],
            power: Some(3),
            toughness: Some(3),
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
        },
        FaceDef {
            name: "Demonic Tutor",
            mana_cost: baylee_core::mana!("{1}{B}"),
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
            abilities: BACK_ABILITIES,
            castable_from_hand: true,
            miracle: None,
            delve: false,
            convoke: false,
            cost_reduction: None,
        },
    ],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::FLYING,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[],
};

static BACK_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
    effects: &[Effect::SearchLibrary {
        filter: &Filter::Any,
        dest: SearchDest::Hand,
        tapped: false,
        shuffle: true,
        optional: false,
    }],
    targets: None,
}];

#[cfg(test)]
mod tests {}
