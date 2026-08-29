//! Rhystic Study — {2}{U} — Enchantment
//! Oracle: Whenever an opponent casts a spell, you may have that player pay {1}. If they don't, you draw a card.
//! Set: J25 #587 — Foundations Jumpstart | Scryfall ID: 9f37c5b6-a59c-45cd-9a99-e9357fe9ea1b | Oracle ID: 53236dd7-845a-444c-96d5-f41ed7325d8e
// IMPLEMENTED — opponent-choice {1} tax on opponents' spells.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet,
    PartnerKind, PlayerRel, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static OPPONENT_SPELL: Filter = Filter::ControlledByOpponent;
static DRAW_ONE: Effect = Effect::DrawCards {
    amount: Amount::Fixed(1),
};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(133),
    oracle_id: "53236dd7-845a-444c-96d5-f41ed7325d8e",
    scryfall_id: "9f37c5b6-a59c-45cd-9a99-e9357fe9ea1b",
    faces: &[FaceDef {
        name: "Rhystic Study",
        mana_cost: baylee_core::mana!("{2}{U}"),
        types: TypeSet::ENCHANTMENT,
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
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Triggered {
        trigger: Trigger::SpellCast(&OPPONENT_SPELL),
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
