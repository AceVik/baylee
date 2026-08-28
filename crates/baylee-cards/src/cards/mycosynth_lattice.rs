//! Mycosynth Lattice — {6} — Artifact
//! Oracle: All permanents are artifacts in addition to their other types.
//! Oracle: All cards that aren't on the battlefield, spells, and permanents are colorless.
//! Oracle: Players may spend mana as though it were mana of any color.
//! Set: BBD #241 — Battlebond | Scryfall ID: 94f89714-3b26-46a2-b9a8-3e664f391cd9 | Oracle ID: ae1f2ab5-c6a5-4d49-a746-3cb4668bf805
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet, PartnerKind};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
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
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
    }],
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Unimplemented,
    abilities: &[],
};

#[cfg(test)]
mod tests {
    // TODO(card): implement abilities + tests, see docs/card-dsl.md.
}
