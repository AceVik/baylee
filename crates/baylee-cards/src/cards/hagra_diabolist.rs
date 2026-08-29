//! Hagra Diabolist — {4}{B} — Creature — Ogre Shaman Ally
//! Oracle: Whenever this creature or another Ally you control enters, you may have target player lose life equal to the number of Allies you control.
//! Set: ZEN #95 — Zendikar | Scryfall ID: c303e7e2-cb22-4dea-889f-d03e2494ed0f | Oracle ID: 5e2c1e0e-0a10-416a-9b50-96ee0cbbc24e
// IMPLEMENTED — rally life loss per Ally (opponent heads-up; target player
// choice for multiplayer is a protocol M3 item).
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
    index: CardIndex::new(63),
    oracle_id: "5e2c1e0e-0a10-416a-9b50-96ee0cbbc24e",
    scryfall_id: "c303e7e2-cb22-4dea-889f-d03e2494ed0f",
    faces: &[FaceDef {
        name: "Hagra Diabolist",
        mana_cost: baylee_core::mana!("{4}{B}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::OGRE, creature::SHAMAN, creature::ALLY],
        power: Some(3),
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::EntersBattlefield(&ALLY_ETB),
        once_per_turn: false,
        effects: &[Effect::LoseLife {
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
