//! Venser, the Sojourner — {3}{W}{U} — Legendary Planeswalker — Venser
//! Oracle: +2: Exile target permanent you own. Return it to the battlefield under your control at the beginning of the next end step.
//! Oracle: −1: Creatures can't be blocked this turn.
//! Oracle: −8: You get an emblem with "Whenever you cast a spell, exile target permanent."
//! Set: DDI #1 — Duel Decks: Venser vs. Koth | Scryfall ID: 8f61a0ea-c2e8-4571-9669-19abd8bbc874 | Oracle ID: a8bf8ff8-d924-4fd2-b5ed-05b38f55325a
// PARTIAL — −1 (unblockable team) implemented; +2 needs end-step delayed
// return (M2.S7b+); −8 needs triggered emblems (M2+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Duration, Effect, FaceDef, Filter, KeywordSet,
    Layer, Modifier, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, planeswalker};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURES: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(183),
    oracle_id: "a8bf8ff8-d924-4fd2-b5ed-05b38f55325a",
    scryfall_id: "8f61a0ea-c2e8-4571-9669-19abd8bbc874",
    faces: &[FaceDef {
        name: "Venser, the Sojourner",
        mana_cost: baylee_core::mana!("{3}{W}{U}"),
        types: TypeSet::PLANESWALKER,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[planeswalker::VENSER],
        power: None,
        toughness: None,
        loyalty: Some(3),
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White, Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("+2 delayed end-step blink (M2.S7b+), −8 triggered emblem (M2+)"),
    abilities: &[AbilityDef::Loyalty {
        cost: -1,
        effects: &[Effect::CreateContinuousEffect {
            layer: Layer::Ability,
            filter: &CREATURES,
            modifier: Modifier::AddKeyword(KeywordSet::UNBLOCKABLE),
            duration: Duration::UntilEndOfTurn,
        }],
        target: None,
    }],
};

#[cfg(test)]
mod tests {}
