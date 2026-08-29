//! Recruiter of the Guard — {2}{W} — Creature — Human Soldier
//! Oracle: When this creature enters, you may search your library for a creature card with toughness 2 or less, reveal it, put it into your hand, then shuffle.
//! Set: CN2 #90 — Conspiracy: Take the Crown | Scryfall ID: 8e4c6ba1-1abc-478f-9b7c-97e9e3c92fb0 | Oracle ID: d521a329-a53a-4962-810a-2abed80df260
// IMPLEMENTED — ETB tutor with the real toughness filter
// (Filter::ToughnessAtMost).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SearchDest, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, creature};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static TOUGH_CREATURE: Filter = Filter::And(&[
    Filter::HasType(TypeSet::CREATURE),
    Filter::ToughnessAtMost(2),
]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(126),
    oracle_id: "d521a329-a53a-4962-810a-2abed80df260",
    scryfall_id: "8e4c6ba1-1abc-478f-9b7c-97e9e3c92fb0",
    faces: &[FaceDef {
        name: "Recruiter of the Guard",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::CREATURE,
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
        trigger: Trigger::EntersBattlefield(&Filter::This),
        once_per_turn: false,
        effects: &[Effect::SearchLibrary {
            filter: &TOUGH_CREATURE,
            dest: SearchDest::Hand,
            tapped: false,
            shuffle: true,
            optional: true,
        }],
        targets: None,
    }],
};

#[cfg(test)]
mod tests {}
