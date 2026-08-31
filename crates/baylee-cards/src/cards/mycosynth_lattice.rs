//! Mycosynth Lattice — {6} — Artifact
//! Oracle: All permanents are artifacts in addition to their other types.
//! Oracle: All cards that aren't on the battlefield, spells, and permanents are colorless.
//! Oracle: Players may spend mana as though it were mana of any color.
//! Set: BBD #241 — Battlebond | Scryfall ID: 94f89714-3b26-46a2-b9a8-3e664f391cd9 | Oracle ID: ae1f2ab5-c6a5-4d49-a746-3cb4668bf805
// PARTIAL — type (layer 4) and color (layer 5) implemented; NOT SUPPORTED
// yet: "spend mana as though it were mana of any color" (player mana-convert
// rule, M2.S7+).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, Layer, Modifier,
    PartnerKind, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(100),
    oracle_id: "ae1f2ab5-c6a5-4d49-a746-3cb4668bf805",
    scryfall_id: "94f89714-3b26-46a2-b9a8-3e664f391cd9",
    faces: &[FaceDef {
        name: "Mycosynth Lattice",
        mana_cost: baylee_core::mana!("{6}"),
        types: TypeSet::ARTIFACT,
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Type,
            filter: Filter::Any,
            modifier: Modifier::AddType(TypeSet::ARTIFACT),
            cross_zone: true,
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Color,
            filter: Filter::Any,
            modifier: Modifier::SetColor(ColorSet::EMPTY),
            cross_zone: true,
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Text,
            filter: Filter::Any,
            modifier: Modifier::ManaIsAnyColor,
            cross_zone: false,
        }),
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
