//! Liquimetal Coating — {2} — Artifact
//! Oracle: {T}: Target permanent becomes an artifact in addition to its other types until end of turn.
//! Set: CM2 #197 — Commander Anthology Volume II | Scryfall ID: f631447c-36e3-4d82-a658-19c9767a216b | Oracle ID: f4bdc551-c2eb-4a34-a3e3-b4a017c925af
// IMPLEMENTED — timed type change (layer 4, until end of turn).
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, Coverage, Duration,
    Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_PERMANENT: Filter = Filter::Any;

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(85),
    oracle_id: "f4bdc551-c2eb-4a34-a3e3-b4a017c925af",
    scryfall_id: "f631447c-36e3-4d82-a658-19c9767a216b",
    faces: &[FaceDef {
        name: "Liquimetal Coating",
        mana_cost: baylee_core::mana!("{2}"),
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
    coverage: Coverage::Implemented,
    abilities: &[AbilityDef::Activated {
        cost: Cost::TAP,
        effects: &[Effect::CreateContinuousEffect {
            layer: Layer::Type,
            filter: &ANY_PERMANENT,
            modifier: Modifier::AddType(TypeSet::ARTIFACT),
            duration: Duration::UntilEndOfTurn,
        }],
        target: Some(TargetSpec::Object(&ANY_PERMANENT)),
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
    }],
};

#[cfg(test)]
mod tests {}
