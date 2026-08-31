//! Damn — {B}{B} — Sorcery
//! Oracle: Destroy target creature. A creature destroyed this way can't be regenerated.
//! Oracle: Overload {2}{W}{W} (You may cast this spell for its overload cost. If you do, change "target" in its text to "each.")
//! Set: LCC #191 — The Lost Caverns of Ixalan Commander | Scryfall ID: 84056124-1a6f-4274-bee2-74cf0debddb5 | Oracle ID: b01d61cc-9844-4191-86a0-f2db6d42d6e5
// IMPLEMENTED — single-target destroy or overloaded wrath. ("A creature
// destroyed this way can't be regenerated" is vacuous: the engine has no
// regeneration mechanic yet; noted for the roadmap's regeneration family.)
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, PartnerKind,
    SpellMode, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static CREATURE_F: Filter = Filter::HasType(TypeSet::CREATURE);

static NORMAL_EFFECTS: &[Effect] = &[Effect::Destroy {
    target: TargetSpec::Object(&CREATURE_F),
}];
static OVERLOAD_EFFECTS: &[Effect] = &[Effect::DestroyAll {
    filter: &CREATURE_F,
}];

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(30),
    oracle_id: "b01d61cc-9844-4191-86a0-f2db6d42d6e5",
    scryfall_id: "84056124-1a6f-4274-bee2-74cf0debddb5",
    faces: &[FaceDef {
        name: "Damn",
        mana_cost: baylee_core::mana!("{B}{B}"),
        types: TypeSet::SORCERY,
        ..FaceDef::DEFAULT
    }],
    color_identity: ColorSet::from_slice(&[Color::Black, Color::White]),
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::ModalSpell {
        modes: &[
            SpellMode {
                effects: NORMAL_EFFECTS,
                target: Some(TargetSpec::Object(&CREATURE_F)),
                cost_override: None,
            },
            SpellMode {
                effects: OVERLOAD_EFFECTS,
                target: None,
                cost_override: Some(baylee_core::mana!("{2}{W}{W}")),
            },
        ],
    }],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {
    // Overload destroys everything; normal mode only the target.
}
