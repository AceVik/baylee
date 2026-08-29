//! Tower of the Magistrate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Target creature gains protection from artifacts until end of turn.
//! Set: MMQ #330 — Mercadian Masques | Scryfall ID: ee0481db-15ae-46b4-89a3-01c95a9626c7 | Oracle ID: ac08fae8-208c-4602-8d39-9bfd29b53a5e
// IMPLEMENTED — {C} mana + protection-from-artifacts grant until EOT.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost, CostPart, Coverage,
    Duration, Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind, TargetSpec,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::CardIndex;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_core::types::{SupertypeSet, TypeSet};

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);
static ARTIFACT_F: Filter = Filter::HasType(TypeSet::ARTIFACT);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(171),
    oracle_id: "ac08fae8-208c-4602-8d39-9bfd29b53a5e",
    scryfall_id: "ee0481db-15ae-46b4-89a3-01c95a9626c7",
    faces: &[FaceDef {
        name: "Tower of the Magistrate",
        mana_cost: ManaCost::ZERO,
        types: TypeSet::LAND,
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
    color_identity: ColorSet::EMPTY,
    keywords: KeywordSet::EMPTY,
    commander: CommanderRule::NotEligible,
    partner: PartnerKind::None,
    coverage: Coverage::Implemented,
    abilities: &[
        AbilityDef::Activated {
            cost: Cost::TAP,
            effects: &[Effect::AddMana {
                color: ManaColor::Colorless,
                amount: 1,
            }],
            target: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: true,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Activated {
            cost: Cost {
                mana: baylee_core::mana!("{1}"),
                parts: &[CostPart::TapSelf],
            },
            effects: &[Effect::CreateContinuousEffect {
                layer: Layer::Text,
                filter: &Filter::This,
                modifier: Modifier::ProtectionFrom(&ARTIFACT_F),
                duration: Duration::UntilEndOfTurn,
            }],
            target: Some(TargetSpec::Object(&ANY_CREATURE)),
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
    ],
};

#[cfg(test)]
mod tests {}
