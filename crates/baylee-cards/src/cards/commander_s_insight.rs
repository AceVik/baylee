//! Commander's Insight — {X}{U}{U}{U} — Instant
//! Oracle: Target player draws X cards plus an additional card for each time they've cast a commander from the command zone this game.
//! Set: SOC #113 — Secrets of Strixhaven Commander | Scryfall ID: 1a40e4da-a631-4423-b70f-701b27b09f79 | Oracle ID: 54d7d7f8-22cd-4859-b203-924d248b422b
// PARTIAL — X-draw with player targeting implemented. NOT SUPPORTED yet:
// +1 per commander cast from the command zone (command-zone cast tracking,
// format modifiers M2+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, Coverage, Effect, FaceDef, KeywordSet, PartnerKind,
    PlayerRel, TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(24),
    oracle_id: "54d7d7f8-22cd-4859-b203-924d248b422b",
    scryfall_id: "1a40e4da-a631-4423-b70f-701b27b09f79",
    faces: &[FaceDef {
        name: "Commander's Insight",
        mana_cost: baylee_core::mana!("{X}{U}{U}{U}"),
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
        cost_reduction: None,
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Partial("commander-cast count bonus (format modifiers M2+)"),
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::DrawCardsFor {
            amount: Amount::X,
            who: PlayerRel::Chosen,
        }],
        targets: Some(TargetReq::one(TargetSpec::AnyPlayer)),
    }],
};

#[cfg(test)]
mod tests {
    // X cards for the chosen player.
}
