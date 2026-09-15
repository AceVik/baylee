//! Inspirit, Flagship Vessel — {U}{R}{W} — Legendary Artifact — Spacecraft
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 1+ | At the beginning of combat on your turn, put your choice of a +1/+1 counter or two charge counters on up to one other target artifact.
//! Oracle: 8+ | Flying
//! Oracle: Other artifacts you control have hexproof and indestructible.
//! Set: EOC #2 — Edge of Eternities Commander | Scryfall ID: 46900ec7-eb18-45c4-8e90-a48b665cfdee | Oracle ID: 554df866-3dbb-4811-8573-6033481591aa
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

card!(
    index = 73,
    oracle_id = "554df866-3dbb-4811-8573-6033481591aa",
    scryfall_id = "46900ec7-eb18-45c4-8e90-a48b665cfdee",
    faces = &[face!(
        name = "Inspirit, Flagship Vessel",
        mana_cost = mana!("{U}{R}{W}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[artifact::SPACECRAFT],
        // Printed on the card and used only at 8+, exactly as a Vehicle's
        // numbers are used only once it crews.
        power = Some(5),
        toughness = Some(5),
    )],
    color_identity = ColorSet::from_slice(&[Color::White, Color::Blue, Color::Red]),
    coverage = Coverage::Implemented,
    abilities = &[
        // Station: tap another creature → its power in charge counters,
        // sorcery speed. It's an artifact creature at 8+.
        activated!(
            Cost::FREE,
            &[
                Effect::TapTarget,
                Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: CounterKind::Charge,
                    amount: Amount::TargetPower,
                },
            ],
            target = Some(TargetSpec::Object(&ANOTHER_CREATURE)),
            timing = ActivationTiming::SorcerySpeed
        ),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Type,
            filter: Filter::This,
            modifier: Modifier::AddTypeIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 8,
                types: TypeSet::CREATURE,
            },
        }),
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::This,
            modifier: Modifier::AddKeywordIfCountersAtLeast {
                kind: CounterKind::Charge,
                at_least: 8,
                keywords: KeywordSet::FLYING,
            },
        }),
        // Other artifacts you control have hexproof and indestructible.
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou, Filter::Another,]),
            modifier: Modifier::AddKeyword(HEXPROOF_INDESTRUCTIBLE),
        }),
        // 1+: modal combat trigger (a +1/+1 counter or two charge
        // counters on up to one other artifact).
        modal_triggered!(
            Trigger::StepBegin {
                step: StepKind::CombatBegin,
                whose: PlayerRel::You,
            },
            &[
                // "up to one other target artifact": with nothing else on
                // the board this trigger still goes on the stack and does
                // nothing, where a count of exactly one would take it off.
                mode!(
                    &[Effect::AddCounter {
                        kind: CounterKind::P1P1,
                        amount: Amount::Fixed(1),
                    }],
                    targets = Some(TargetReq::up_to_one(TargetSpec::Object(&OTHER_ARTIFACT)))
                ),
                mode!(
                    &[Effect::AddCounter {
                        kind: CounterKind::Charge,
                        amount: Amount::Fixed(2),
                    }],
                    targets = Some(TargetReq::up_to_one(TargetSpec::Object(&OTHER_ARTIFACT)))
                ),
            ]
        ),
    ],
);
