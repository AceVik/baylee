//! Commander's Insight — {X}{U}{U}{U} — Instant
//! Oracle: Target player draws X cards plus an additional card for each time they've cast a commander from the command zone this game.
//! Set: SOC #113 — Secrets of Strixhaven Commander | Scryfall ID: 1a40e4da-a631-4423-b70f-701b27b09f79 | Oracle ID: 54d7d7f8-22cd-4859-b203-924d248b422b
// IMPLEMENTED — X plus the command-zone cast count (the count is the
// caster's; targeting another player using THEIR count is a protocol-v2
// corner case).
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
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::DrawCardsFor {
            amount: Amount::XPlusCommanderCasts,
            who: PlayerRel::Chosen,
        }],
        targets: Some(TargetReq::one(TargetSpec::AnyPlayer)),
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {
    // X cards for the chosen player.
}
