//! Esper Sentinel — {W} — Artifact Creature — Human Soldier
//! Oracle: Whenever an opponent casts their first noncreature spell of the turn, you may have that player pay {1}. If they don't, you draw a card.
//! Set: MH2 #12 — Modern Horizons 2 | Scryfall ID: f3537373-ef54-4578-9d05-6216420ee349 | Oracle ID: 5def9f38-0a0b-4e8d-9f9d-29dcb46520b4
// IMPLEMENTED — first-noncreature-spell-per-turn tax (per-turn tracking).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, KeywordSet, PartnerKind,
    PlayerRel, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static DRAW_ONE: Effect = Effect::DrawCards {
    amount: Amount::Fixed(1),
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(46),
    oracle_id: "5def9f38-0a0b-4e8d-9f9d-29dcb46520b4",
    scryfall_id: "f3537373-ef54-4578-9d05-6216420ee349",
    faces: &[FaceDef {
        name: "Esper Sentinel",
        mana_cost: baylee_core::mana!("{W}"),
        types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[creature::HUMAN, creature::SOLDIER],
        power: Some(1),
        toughness: Some(1),
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
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::FirstNoncreatureSpellCast(PlayerRel::Opponent),
        once_per_turn: false,
        effects: &[Effect::PlayerMayPayOr {
            player: PlayerRel::Opponent,
            mana: 1,
            effect: &DRAW_ONE,
        }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
