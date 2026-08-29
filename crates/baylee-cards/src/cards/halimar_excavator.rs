//! Halimar Excavator — {1}{U} — Creature — Human Wizard Ally
//! Oracle: Whenever this creature or another Ally you control enters, target player mills X cards, where X is the number of Allies you control.
//! Set: WWK #29 — Worldwake | Scryfall ID: d147dce7-b2dd-426a-9ff7-843d50bb8b01 | Oracle ID: fd3e37c9-93bf-4f3e-a279-22afbffd8d43
// IMPLEMENTED — rally mill per Ally (opponent heads-up; target choice M3).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, Trigger, ZoneSel,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ALLY_ETB: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::Or(&[Filter::This, Filter::HasSubtype(creature::ALLY)]),
]);
static ALLIES_YOU: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasSubtype(creature::ALLY)]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(64),
    oracle_id: "fd3e37c9-93bf-4f3e-a279-22afbffd8d43",
    scryfall_id: "d147dce7-b2dd-426a-9ff7-843d50bb8b01",
    faces: &[FaceDef {
        name: "Halimar Excavator",
        mana_cost: baylee_core::mana!("{1}{U}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::WIZARD, creature::ALLY],
        power: Some(1),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&ALLY_ETB),
        once_per_turn: false,
        effects: &[Effect::Mill {
            amount: Amount::CountOf {
                filter: &ALLIES_YOU,
                zone: ZoneSel::Battlefield,
            },
            target: PlayerRel::Opponent,
        }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
