//! Spirit Water Revival — {4}{U} — Sorcery
//! Oracle: As an additional cost to cast this spell, you may waterbend {6}. (While paying a waterbend cost, you can tap your artifacts and creatures to help. Each one pays for {1}.)
//! Oracle: Draw two cards. If this spell's additional cost was paid, instead shuffle your graveyard into your library, draw seven cards, and you have no maximum hand size for the rest of the game.
//! Oracle: Exile Spirit Water Revival.
//! Set: TLA #74 — Avatar: The Last Airbender | Scryfall ID: 0c019e76-c88e-4d1b-a546-0f4e462ef44a | Oracle ID: 68979160-b5ce-4787-8a1e-1f40e614c3b0
// PARTIAL — waterbend (convoke-style payment assists) and the kicked
// outcome need payment assists (M2.S7+). Base draw-2 + exile implemented.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, KeywordSet, PartnerKind,
    TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(156),
    oracle_id: "68979160-b5ce-4787-8a1e-1f40e614c3b0",
    scryfall_id: "0c019e76-c88e-4d1b-a546-0f4e462ef44a",
    faces: &[FaceDef {
        name: "Spirit Water Revival",
        mana_cost: baylee_core::mana!("{4}{U}"),
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
        miracle: None,
        delve: false,
        convoke: false,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("waterbend assist + kicked outcome (M2.S7+)"),
    abilities: &[AbilityDef::Spell {
        effects: &[
            Effect::DrawCards {
                amount: Amount::Fixed(2),
            },
            Effect::Exile {
                target: TargetSpec::ThisObject,
            },
        ],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
