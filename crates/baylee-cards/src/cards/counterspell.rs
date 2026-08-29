//! Counterspell — {U}{U} — Instant
//! Oracle: Counter target spell.
//! Set: DSC #114 — Duskmourn: House of Horror Commander | Scryfall ID: 4f616706-ec97-4923-bb1e-11a69fbaa1f8 | Oracle ID: cc187110-1148-4090-bbb8-e205694a39f5
// IMPLEMENTED — hard counter (target selection on the stack).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_SPELL: Filter = Filter::Any;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(25),
    oracle_id: "cc187110-1148-4090-bbb8-e205694a39f5",
    scryfall_id: "4f616706-ec97-4923-bb1e-11a69fbaa1f8",
    faces: &[FaceDef {
        name: "Counterspell",
        mana_cost: baylee_core::mana!("{U}{U}"),
        types: TypeSet::INSTANT,
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
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::CounterTargetSpell],
        targets: Some(TargetReq::one(TargetSpec::Spell(&ANY_SPELL))),
    }],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage via s4 scenario tests: countering a creature
    // spell moves it to the graveyard instead of the battlefield.
}
