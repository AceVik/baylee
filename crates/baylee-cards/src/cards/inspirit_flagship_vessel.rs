//! Inspirit, Flagship Vessel — {4} — Legendary Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 1+ | At the beginning of combat on your turn, put your choice of a +1/+1 counter or two charge counters on up to one other target artifact.
//! Oracle: 8+ | Flying
//! Oracle: Other artifacts you control have hexproof and indestructible.
//! Set: EOC #39 — Edge of Eternities Commander | Scryfall ID: 46900ec7-eb18-45c4-8e90-a48b665cfdee | Oracle ID: 554df866-3dbb-4811-8573-6033481591aa
// IMPLEMENTED — station (tap another creature for power-many charge
// counters, sorcery speed), artifact-creature at 8+, 8+ flying, the
// artifact hexproof/indestructible grant, and the 1+ modal counter
// trigger.
#![allow(unused_imports, missing_docs)]

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, ActivationZone, Amount, CardDef, CommanderRule, Cost,
    CounterKind, Coverage, Effect, FaceDef, Filter, KeywordSet, Layer, Modifier, PartnerKind,
    SpellMode, StaticAbility, StepKind, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{self, artifact};
use baylee_core::ids::CardIndex;
use baylee_core::mana::ManaCost;
use baylee_core::types::{SupertypeSet, TypeSet};

static ANOTHER_CREATURE: Filter = Filter::And(&[
    Filter::Another,
    Filter::HasType(TypeSet::CREATURE),
    Filter::ControlledByYou,
]);
static OTHER_ARTIFACT: Filter = Filter::And(&[Filter::Another, Filter::HasType(TypeSet::ARTIFACT)]);
static HEXPROOF_INDESTRUCTIBLE: KeywordSet = KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE);

pub static CARD: CardDef = CardDef {
    index: CardIndex::new(73),
    oracle_id: "554df866-3dbb-4811-8573-6033481591aa",
    scryfall_id: "46900ec7-eb18-45c4-8e90-a48b665cfdee",
    faces: &[FaceDef {
        name: "Inspirit, Flagship Vessel",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[artifact::SPACECRAFT],
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        // Station: tap another creature → its power in charge counters,
        // sorcery speed. It's an artifact creature at 8+.
        AbilityDef::Activated {
            cost: Cost::FREE,
            effects: &[
                Effect::TapTarget,
                Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: CounterKind::Charge,
                    amount: Amount::TargetPower,
                },
            ],
            target: Some(TargetSpec::Object(&ANOTHER_CREATURE)),
            timing: ActivationTiming::SorcerySpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
        },
        AbilityDef::Static(StaticAbility {
            layer: Layer::Type,
            filter: Filter::This,
            modifier: Modifier::AddTypeIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 8,
                types: TypeSet::CREATURE,
            },
            cross_zone: false,
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::This,
            modifier: Modifier::AddKeywordIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 8,
                keywords: KeywordSet::FLYING,
            },
            cross_zone: false,
        }),
        // Other artifacts you control have hexproof and indestructible.
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::And(&[
                Filter::HasType(TypeSet::ARTIFACT),
                Filter::ControlledByYou,
                Filter::Another,
            ]),
            modifier: Modifier::AddKeyword(HEXPROOF_INDESTRUCTIBLE),
            cross_zone: false,
        }),
        // 1+: modal combat trigger (a +1/+1 counter or two charge
        // counters on up to one other artifact).
        AbilityDef::ModalTriggered {
            trigger: Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: baylee_cards_dsl::PlayerRel::You,
            },
            modes: &[
                SpellMode {
                    effects: &[Effect::AddCounter {
                        kind: CounterKind::P1P1,
                        amount: Amount::Fixed(1),
                    }],
                    target: Some(TargetSpec::Object(&OTHER_ARTIFACT)),
                    cost_override: None,
                },
                SpellMode {
                    effects: &[Effect::AddCounter {
                        kind: CounterKind::Charge,
                        amount: Amount::Fixed(2),
                    }],
                    target: Some(TargetSpec::Object(&OTHER_ARTIFACT)),
                    cost_override: None,
                },
            ],
            once_per_turn: false,
        },
    ],
    ..CardDef::DEFAULT
};

#[cfg(test)]
mod tests {}
