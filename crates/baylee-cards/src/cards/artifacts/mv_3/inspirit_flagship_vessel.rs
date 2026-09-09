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

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;

static ANOTHER_CREATURE: Filter =
    Filter::And(&[Filter::Another, Filter::CREATURE, Filter::ControlledByYou]);
static OTHER_ARTIFACT: Filter = Filter::And(&[Filter::Another, Filter::ARTIFACT]);
static HEXPROOF_INDESTRUCTIBLE: KeywordSet = KeywordSet::HEXPROOF.union(KeywordSet::INDESTRUCTIBLE);

card! {
    index: 73,
    oracle_id: "554df866-3dbb-4811-8573-6033481591aa",
    scryfall_id: "46900ec7-eb18-45c4-8e90-a48b665cfdee",
    faces: &[face! {
        name: "Inspirit, Flagship Vessel",
        mana_cost: baylee_core::mana!("{4}"),
        types: TypeSet::ARTIFACT,
        supertypes: SupertypeSet::LEGENDARY,
        subtypes: &[artifact::SPACECRAFT],
    }],
    coverage: Coverage::Implemented,
    abilities: &[
        // Station: tap another creature → its power in charge counters,
        // sorcery speed. It's an artifact creature at 8+.
        activated!(Cost::FREE, &[
                Effect::TapTarget,
                Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: CounterKind::Charge,
                    amount: Amount::TargetPower,
                },
            ], target: Some(TargetSpec::Object(&ANOTHER_CREATURE)), timing: ActivationTiming::SorcerySpeed),
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
                Filter::ARTIFACT,
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
                whose: PlayerRel::You,
            },
            modes: &[
                mode!(&[Effect::AddCounter {
                        kind: CounterKind::P1P1,
                        amount: Amount::Fixed(1),
                    }], target: Some(TargetSpec::Object(&OTHER_ARTIFACT))),
                mode!(&[Effect::AddCounter {
                        kind: CounterKind::Charge,
                        amount: Amount::Fixed(2),
                    }], target: Some(TargetSpec::Object(&OTHER_ARTIFACT))),
            ],
            once_per_turn: false,
        },
    ],
}
