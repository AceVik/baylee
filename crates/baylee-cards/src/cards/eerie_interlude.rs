//! Eerie Interlude — {2}{W} — Instant
//! Oracle: Exile any number of target creatures you control. Return those cards to the battlefield under their owner's control at the beginning of the next end step.
//! Set: KHC #22 — Kaldheim Commander | Scryfall ID: 4ba9f15f-00d2-4797-9228-91b320e85705 | Oracle ID: 0634091a-a74c-4cea-b6d1-7324a725554a
// IMPLEMENTED — mass end-step blink of your creatures.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_CREATURES: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::ControlledByYou]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(38),
    oracle_id: "0634091a-a74c-4cea-b6d1-7324a725554a",
    scryfall_id: "4ba9f15f-00d2-4797-9228-91b320e85705",
    faces: &[FaceDef {
        name: "Eerie Interlude",
        mana_cost: baylee_core::mana!("{2}{W}"),
        types: TypeSet::INSTANT,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::ExileAndReturnAtEndStep],
        targets: Some(TargetReq {
            spec: TargetSpec::Object(&YOUR_CREATURES),
            min: 0,
            max: 255,
            count_is_x: false,
        }),
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
