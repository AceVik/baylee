//! Elesh Norn, Mother of Machines — {4}{W} — Legendary Creature — Phyrexian Praetor
//! Oracle: Vigilance
//! Oracle: If a permanent entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.
//! Oracle: Permanents entering don't cause abilities of permanents your opponents control to trigger.
//! Set: ONE #10 — Phyrexia: All Will Be One | Scryfall ID: 44dcab01-1d13-4dfc-ae2f-fbaa3dd35087 | Oracle ID: 5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f
// IMPLEMENTED — vigilance + ETB-trigger multiplication (yours) and
// suppression (opponents').
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, PartnerKind,
    ReplacementRule, TriggerEventKind,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOURS: Filter = Filter::ControlledByYou;
static OPPONENTS: Filter = Filter::ControlledByOpponent;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(39),
    oracle_id: "5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f",
    scryfall_id: "44dcab01-1d13-4dfc-ae2f-fbaa3dd35087",
    faces: &[FaceDef {
        name: "Elesh Norn, Mother of Machines",
        mana_cost: baylee_core::mana!("{4}{W}"),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[subtypes::creature::PHYREXIAN, subtypes::creature::PRAETOR],
        power: Some(4),
        toughness: Some(7),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    keywords: KeywordSet::VIGILANCE,
    commander: CommanderRule::Legendary,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Replacement(ReplacementRule::TriggerMultiplier {
            source_filter: &YOURS,
            event: TriggerEventKind::EntersBattlefield,
        }),
        AbilityDef::Replacement(ReplacementRule::TriggerSuppress {
            source_filter: &OPPONENTS,
            event: TriggerEventKind::EntersBattlefield,
        }),
    ],
};

#[cfg(test)]
mod tests {
    // Engine-level coverage in baylee-engine s6 tests: your rally fires
    // twice, the opponent's rally is fully suppressed.
}
