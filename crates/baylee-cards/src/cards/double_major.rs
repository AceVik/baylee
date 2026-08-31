//! Double Major — {G}{U} — Instant
//! Oracle: Copy target creature spell you control, except it isn't legendary if the spell is legendary. (A copy of a creature spell becomes a token.)
//! Set: STX #179 — Strixhaven: School of Mages | Scryfall ID: c3d35413-8742-4443-8859-93c91112978d | Oracle ID: ece44a82-dcf0-4439-bdd9-a09c99a6f159
// PARTIAL — spell copy on the stack implemented; NOT SUPPORTED yet: the copy
// isn't legendary (copy-time modification on spell copies, M2.S7c+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    TargetReq, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static YOUR_CREATURE_SPELL: Filter =
    Filter::And(&[Filter::ControlledByYou, Filter::HasType(TypeSet::CREATURE)]);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(34),
    oracle_id: "ece44a82-dcf0-4439-bdd9-a09c99a6f159",
    scryfall_id: "c3d35413-8742-4443-8859-93c91112978d",
    faces: &[FaceDef {
        name: "Double Major",
        mana_cost: baylee_core::mana!("{G}{U}"),
        types: TypeSet::INSTANT,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Blue, Color::Green]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Spell {
        effects: &[Effect::CopyTargetSpell {
            mods: &[baylee_cards_dsl::CopyMod::RemoveSupertype(
                SupertypeSet::LEGENDARY,
            )],
        }],
        targets: Some(TargetReq::one(TargetSpec::Spell(&YOUR_CREATURE_SPELL))),
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
