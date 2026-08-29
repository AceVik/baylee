//! Toxic Deluge — {2}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, pay X life.
//! Oracle: All creatures get -X/-X until end of turn.
//! Set: MSC #161 — Marvel Super Heroes Commander | Scryfall ID: de5afccc-8d42-4bd6-b068-b9ea2361655e | Oracle ID: afaef788-34d1-460b-b884-9d7ae6ddeb18
// IMPLEMENTED — mandatory pay-life-X additional cost + X-driven global
// debuff until end of turn.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CostPart, Coverage, Duration, Effect, FaceDef,
    Filter, KeywordSet, PartnerKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURES: Filter = Filter::HasType(TypeSet::CREATURE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(172),
    oracle_id: "afaef788-34d1-460b-b884-9d7ae6ddeb18",
    scryfall_id: "de5afccc-8d42-4bd6-b068-b9ea2361655e",
    faces: &[FaceDef {
        name: "Toxic Deluge",
        mana_cost: baylee_core::mana!("{2}{B}"),
        types: TypeSet::SORCERY,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[CostPart::PayLifeX],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Black]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::PumpFilter {
            filter: &CREATURES,
            power: Amount::NegX,
            toughness: Amount::NegX,
            duration: Duration::UntilEndOfTurn,
        }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {
    // X life paid at cast; all creatures get -X/-X until end of turn.
}
