//! Public-zone lookup belongs to the resolving effect, not to an arena handle.
use super::synthetic::{SyntheticLookup, creature, keep_mulligans, preset, walk_past};
use super::*;
use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, Cost, CostPart, Duration,
    Effect, Filter, KeywordSet, Layer, Modifier, TargetSpec,
};
use baylee_core::types::TypeSet;

const SUBJECT: u32 = 990_117;
static PUMP: Effect = Effect::CreateContinuousEffect {
    layer: Layer::PtModify,
    filter: &Filter::This,
    modifier: Modifier::ModifyPT(2, 2),
    duration: Duration::UntilEndOfTurn,
};
static RETURN_AND_ASK: &[Effect] = &[
    Effect::reanimate(TargetSpec::ThisObject),
    Effect::MayDo { effects: &[PUMP] },
];
fn fixture(cost: Cost, effects: &'static [Effect]) -> (Engine<SyntheticLookup>, ObjectId) {
    let abilities = Box::leak(Box::new([AbilityDef::Activated {
        cost,
        effects,
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: false,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    }]));
    let lookup = SyntheticLookup::new(vec![Box::leak(Box::new(baylee_cards_dsl::CardDef {
        abilities,
        ..*creature(SUBJECT, "Subject", 2, 2, &[])
    }))]);
    let mut engine = Engine::new(&preset(8117, &[SUBJECT]), lookup).unwrap();
    keep_mulligans(&mut engine);
    while engine.state.turn.phase != Phase::FirstMain {
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending));
    }
    let source = engine
        .state
        .battlefield_seen()
        .find(|id| engine.state.object(*id).unwrap().card.unwrap().index.get() == SUBJECT)
        .unwrap();
    (engine, source)
}
fn activate(engine: &mut Engine<SyntheticLookup>, source: ObjectId) {
    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
}
fn settle(engine: &mut Engine<SyntheticLookup>) {
    while !engine.state.zones.list(ZoneLocation::Stack).is_empty() {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            break;
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
}
fn body(engine: &Engine<SyntheticLookup>, source: ObjectId) -> (i16, i16) {
    let c = engine.state.object(source).unwrap().characteristics();
    (c.power.unwrap(), c.toughness.unwrap())
}
#[test]
fn self_subject_cost_public_move_survives_nested_suspension_and_atomic_refusal() {
    let cost = Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::SacrificeSelf],
    };
    let (mut engine, source) = fixture(cost, RETURN_AND_ASK);
    let old = engine.state.source_identity(source).unwrap();
    activate(&mut engine, source);
    assert_eq!(engine.state.object(source).unwrap().zone, Zone::Graveyard);
    let paid_successor = engine.state.source_identity(source).unwrap();
    assert!(
        !engine.state.source_referenced_by(paid_successor).is_empty(),
        "the effect really refers to its public cost successor"
    );
    settle(&mut engine);
    assert!(matches!(engine.pending(), Pending::YesNo { .. }));
    assert_eq!(engine.state.object(source).unwrap().zone, Zone::Battlefield);
    assert_ne!(engine.state.source_identity(source), Some(old));
    let before = engine.snapshot_hash();
    assert!(
        engine
            .apply(PlayerId::new(1), PlayerAction::YesNo(true))
            .is_err()
    );
    assert_eq!(before, engine.snapshot_hash());
    let held = engine.resolution.as_ref().unwrap().clone();
    assert_eq!(
        held.subject.fingerprint(),
        engine.resolution.as_ref().unwrap().subject.fingerprint()
    );
    assert_ne!(
        held.subject.fingerprint(),
        crate::resolve::SubjectContext::default().fingerprint()
    );
    engine
        .apply(PlayerId::new(0), PlayerAction::YesNo(true))
        .unwrap();
    settle(&mut engine);
    assert_eq!(body(&engine, source), (4, 4));
}
#[test]
fn self_subject_effect_public_move_follows_in_nested_branch_but_damage_keeps_old_lki() {
    static OPS: &[Effect] = &[
        Effect::SacrificeSelf,
        Effect::MayDo {
            effects: &[
                Effect::reanimate(TargetSpec::ThisObject),
                PUMP,
                Effect::DealDamage {
                    amount: baylee_cards_dsl::Amount::Fixed(1),
                    target: TargetSpec::Player(baylee_cards_dsl::PlayerRel::EachOpponent),
                },
            ],
        },
    ];
    let (mut engine, source) = fixture(Cost::FREE, OPS);
    let filter = crate::effects::EffectFilter::object(&engine.state, source);
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: PlayerId::new(0),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: Layer::Ability,
            timestamp: 1,
            duration: Duration::UntilEndOfTurn,
            filter,
            modifier: Modifier::AddKeyword(KeywordSet::LIFELINK),
        });
    engine.state.refresh_characteristics();
    activate(&mut engine, source);
    settle(&mut engine);
    engine
        .apply(PlayerId::new(0), PlayerAction::YesNo(true))
        .unwrap();
    settle(&mut engine);
    assert_eq!(body(&engine, source), (4, 4));
    assert!(
        !engine
            .state
            .object(source)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::LIFELINK)
    );
    assert_eq!(
        engine.state.players[0].life, 21,
        "damage belongs to the old lifelink source"
    );
    assert_eq!(engine.state.players[1].life, 19);
}
#[test]
fn self_subject_failed_return_cannot_modify_the_unrelated_returned_permanent() {
    static OPS: &[Effect] = &[
        Effect::SacrificeSelf,
        Effect::MayDo {
            effects: RETURN_AND_ASK,
        },
    ];
    let (mut engine, source) = fixture(Cost::FREE, OPS);
    activate(&mut engine, source);
    settle(&mut engine);
    engine
        .state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .apply(PlayerId::new(0), PlayerAction::YesNo(true))
        .unwrap();
    // The nested prompt is permitted, but its This still names the departed
    // graveyard object, not this independently returned permanent.
    engine
        .apply(PlayerId::new(0), PlayerAction::YesNo(true))
        .unwrap();
    settle(&mut engine);
    assert_eq!(body(&engine, source), (2, 2));
    assert!(
        engine
            .state
            .object(source)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}

#[test]
fn self_subject_cost_successor_is_not_referenced_by_an_unrelated_effect() {
    let cost = Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::SacrificeSelf],
    };
    let (mut engine, source) = fixture(
        cost,
        &[Effect::GainLife {
            amount: baylee_cards_dsl::Amount::Fixed(1),
        }],
    );
    activate(&mut engine, source);
    let successor = engine.state.source_identity(source).unwrap();
    assert!(engine.state.source_referenced_by(successor).is_empty());
    settle(&mut engine);
    assert_eq!(engine.state.players[0].life, 21);
}
#[test]
fn self_subject_public_lookup_requires_a_real_move_and_never_crosses_hidden_zones() {
    for hidden in [false, true] {
        let (mut engine, source) = fixture(Cost::FREE, &[]);
        let original = engine.state.source_identity(source).unwrap();
        let since = engine.state.journal.last_seq();
        assert_eq!(
            resolve::subjects::public_successor(&engine.state, original, since, Cause::Effect),
            None
        );
        let zone = if hidden {
            ZoneLocation::Hand(PlayerId::new(0))
        } else {
            ZoneLocation::Exile(PlayerId::new(0))
        };
        engine
            .state
            .move_object(source, zone, ZonePosition::Top, Cause::Effect)
            .unwrap();
        engine
            .state
            .move_object(
                source,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let successor =
            resolve::subjects::public_successor(&engine.state, original, since, Cause::Effect);
        assert_eq!(
            successor,
            (!hidden).then(|| engine.state.source_identity(source).unwrap())
        );
    }
}
