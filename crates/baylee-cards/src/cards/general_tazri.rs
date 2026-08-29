//! General Tazri — {4}{W} — Legendary Creature — Human Ally
//! Oracle: When General Tazri enters, you may search your library for an Ally creature card, reveal it, put it into your hand, then shuffle.
//! Oracle: {W}{U}{B}{R}{G}: Ally creatures you control get +X/+X until end of turn, where X is the number of colors among those creatures.
//! Set: OGW #19 — Oath of the Gatewatch | Scryfall ID: 34e9aa86-1a31-4c0f-928d-923f066286b6 | Oracle ID: b0f19cba-1339-4518-8320-d7b1dcaf2eb0
// PARTIAL — ETB ally tutor implemented. NOT SUPPORTED yet: the {WUBRG}
// activated pump needs Amount-driven modifiers (M2.S7). Reveal is
// presentation-only and lands with the protocol (M3).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SearchDest, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ALLY_CARD: Filter = Filter::And(&[
    Filter::HasSubtype(creature::ALLY),
    Filter::HasType(TypeSet::CREATURE),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(57),
    oracle_id: "b0f19cba-1339-4518-8320-d7b1dcaf2eb0",
    scryfall_id: "34e9aa86-1a31-4c0f-928d-923f066286b6",
    faces: &[FaceDef {
        name: "General Tazri",
        mana_cost: baylee_core::mana!("{4}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::HUMAN, subtypes::creature::ALLY],
        power: Some(3),
        toughness: Some(4),
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
    color_identity: ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::White,
        Color::Blue,
    ]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("activated {WUBRG} pump needs Amount-driven modifiers (M2.S7)"),
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::SearchLibrary {
            filter: &ALLY_CARD,
            dest: SearchDest::Hand,
            tapped: false,
            shuffle: true,
            optional: true,
        }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage lives in baylee-engine m2 tests: with Maskwood
    // Nexus on the battlefield, a non-Ally creature card in the library is
    // a legal tutor option (cross-zone projection).
}
