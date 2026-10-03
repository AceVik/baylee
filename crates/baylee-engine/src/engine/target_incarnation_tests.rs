//! Exact announcement identity survives costs, partial resolution and replay.
use super::synthetic::{self, SyntheticLookup};
use super::*;
use baylee_cards_dsl::{AbilityDef, Amount, Cost, CostPart, Effect, Filter, TargetReq, TargetSpec};
use baylee_core::ids::CardIndex;

const PLAYER: PlayerId = PlayerId::new(0);
const PROBE: u32 = 98_161;
const VICTIM: u32 = 98_162;
static TARGET: TargetReq = TargetReq::one(TargetSpec::Object(&Filter::CREATURE));
static DAMAGE: &[Effect] = &[Effect::DealDamage {
    amount: Amount::Fixed(3),
    target: TargetSpec::Object(&Filter::CREATURE),
}];
static SACRIFICE: Cost = Cost {
    mana: baylee_core::mana::ManaCost::ZERO,
    parts: &[CostPart::SacrificeSelf],
};
static SPELL: &[AbilityDef] = &[baylee_cards_dsl::spell!(DAMAGE, targets = Some(TARGET))];
static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
    SACRIFICE,
    DAMAGE,
    targets = Some(TARGET)
)];

fn walk(engine: &mut Engine<SyntheticLookup>, done: impl Fn(&Engine<SyntheticLookup>) -> bool) {
    for _ in 0..200 {
        if done(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected {pending:?}"
        );
    }
    panic!("condition not reached: {:?}", engine.pending());
}
fn game(spell: bool) -> Engine<SyntheticLookup> {
    let probe = if spell {
        Box::leak(Box::new(baylee_cards_dsl::CardDef {
            faces: Box::leak(Box::new([baylee_cards_dsl::FaceDef {
                castable_from_hand: true,
                mana_cost: baylee_cards_dsl::mana!("{0}"),
                types: baylee_core::types::TypeSet::INSTANT,
                mandatory_additional_costs: &[CostPart::Sacrifice(&Filter::CREATURE)],
                ..synthetic::land_face("announcement probe")
            }])),
            abilities: SPELL,
            ..*synthetic::land(PROBE, "announcement probe", &[])
        }))
    } else {
        Box::leak(Box::new(baylee_cards_dsl::CardDef {
            abilities: ABILITIES,
            ..*synthetic::creature(PROBE, "announcement probe", 2, 4, &[])
        }))
    };
    let victim = synthetic::creature(VICTIM, "chosen cost object", 2, 4, &[]);
    let mut preset = synthetic::preset(501, if spell { &[VICTIM] } else { &[PROBE] });
    if spell {
        preset.seats[0].starting_hand = Some(vec![baylee_core::preset::DeckEntry {
            card: CardIndex::new(PROBE),
            print: baylee_core::ids::PrintRef::new(0),
        }]);
    }
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![probe, victim])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    walk(&mut engine, |e| {
        e.state.turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: PLAYER, .. })
    });
    engine
}

#[test]
fn casting_and_activation_bind_targets_before_sacrifice_costs() {
    for spell in [false, true] {
        let mut engine = game(spell);
        let victim = synthetic::permanents(&engine, if spell { VICTIM } else { PROBE })[0];
        let original = engine.state.source_identity(victim).unwrap();
        let action = if spell {
            let card = *engine
                .state
                .zones
                .list(ZoneLocation::Hand(PLAYER))
                .iter()
                .find(|&&id| {
                    engine.state.object(id).unwrap().card.unwrap().index == CardIndex::new(PROBE)
                })
                .unwrap();
            PlayerAction::CastSpell { card }
        } else {
            PlayerAction::ActivateAbility {
                source: victim,
                ability_index: 0,
            }
        };
        engine.apply(PLAYER, action).unwrap();
        engine
            .apply(
                PLAYER,
                PlayerAction::ChooseTargets {
                    objects: vec![victim],
                    players: vec![],
                },
            )
            .unwrap();
        if spell {
            assert!(matches!(engine.pending(), Pending::ChooseCards { .. }));
            engine
                .apply(
                    PLAYER,
                    PlayerAction::ChooseObjects {
                        objects: vec![victim],
                    },
                )
                .unwrap();
        }
        assert_eq!(
            engine.state.object(victim).unwrap().zone,
            crate::zone::Zone::Graveyard
        );
        let stack = *engine.state.zones.list(ZoneLocation::Stack).last().unwrap();
        assert_eq!(
            engine.state.recorded_target_reference(stack, false, 0),
            Some(original)
        );
        engine
            .state
            .move_object(
                victim,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        assert_ne!(engine.state.source_identity(victim), Some(original));
        walk(&mut engine, |e| {
            e.state.zones.list(ZoneLocation::Stack).is_empty()
        });
        assert_eq!(engine.state.object(victim).unwrap().damage, 0);
        assert!(engine.state.journal.entries().iter().any(|entry| matches!(entry.event, GameEvent::StackObjectDidNotResolve { object } if object == stack)));
    }
}

#[test]
fn announced_versions_affect_resume_hash_and_survive_atomic_refusal() {
    let mut engine = game(true);
    let victim = synthetic::permanents(&engine, VICTIM)[0];
    let card = *engine
        .state
        .zones
        .list(ZoneLocation::Hand(PLAYER))
        .iter()
        .find(|&&id| engine.state.object(id).unwrap().card.unwrap().index == CardIndex::new(PROBE))
        .unwrap();
    engine
        .apply(PLAYER, PlayerAction::CastSpell { card })
        .unwrap();
    engine
        .apply(
            PLAYER,
            PlayerAction::ChooseTargets {
                objects: vec![victim],
                players: vec![],
            },
        )
        .unwrap();
    let hash = engine.snapshot_hash();
    let announced = engine.cast_wizard.clone();
    engine.cast_wizard.as_mut().unwrap().target_references.first[0].version += 1;
    assert_ne!(engine.snapshot_hash(), hash);
    engine.cast_wizard = announced;
    assert_eq!(engine.snapshot_hash(), hash);
    assert!(
        engine
            .apply(PLAYER, PlayerAction::ChooseObjects { objects: vec![] })
            .is_err()
    );
    assert_eq!(engine.snapshot_hash(), hash);
    assert_eq!(
        engine.cast_wizard.as_ref().unwrap().target_references.first[0],
        engine.state.source_identity(victim).unwrap()
    );
}

#[test]
fn a_missing_announcement_reference_never_binds_to_the_current_object_at_resolution() {
    let mut engine = game(false);
    let target = synthetic::permanents(&engine, PROBE)[0];
    let name = engine.state.names.intern("unbound target probe");
    let ability = engine.state.create_bare(
        PLAYER,
        crate::object::ObjectKind::AbilityOnStack,
        name,
        ZoneLocation::Stack,
    );
    let obj = engine.state.object_mut(ability).unwrap();
    obj.targets.push(target);
    obj.target_req = Some(TARGET);
    obj.ability = Some(crate::object::AbilityLoc {
        source: target,
        card: Some(CardIndex::new(PROBE)),
        index: 0,
    });
    assert_eq!(
        engine.state.recorded_target_reference(ability, false, 0),
        None
    );
    engine.resolve_stack_top();
    assert_eq!(engine.state.object(target).unwrap().damage, 0);
    assert!(engine.state.journal.entries().iter().any(|entry| matches!(entry.event, GameEvent::StackObjectDidNotResolve { object } if object == ability)));
}

#[test]
fn action_replay_preserves_announced_versions_through_payment_and_fizzle() {
    let mut original = game(true);
    let mut replay = game(true);
    let victim = synthetic::permanents(&original, VICTIM)[0];
    let card = *original
        .state
        .zones
        .list(ZoneLocation::Hand(PLAYER))
        .iter()
        .find(|&&id| {
            original.state.object(id).unwrap().card.unwrap().index == CardIndex::new(PROBE)
        })
        .unwrap();
    let actions = [
        PlayerAction::CastSpell { card },
        PlayerAction::ChooseTargets {
            objects: vec![victim],
            players: vec![],
        },
        PlayerAction::ChooseObjects {
            objects: vec![victim],
        },
    ];
    for action in actions {
        original.apply(PLAYER, action.clone()).unwrap();
        replay.apply(PLAYER, action).unwrap();
        assert_eq!(original.snapshot_hash(), replay.snapshot_hash());
        assert_eq!(original.fingerprint(), replay.fingerprint());
    }
    for _ in 0..4 {
        if original.state.zones.list(ZoneLocation::Stack).is_empty() {
            break;
        }
        let Pending::Priority { player, .. } = *original.pending() else {
            panic!("priority expected")
        };
        original.apply(player, PlayerAction::PassPriority).unwrap();
        replay.apply(player, PlayerAction::PassPriority).unwrap();
        assert_eq!(original.fingerprint(), replay.fingerprint());
    }
    assert!(original.state.zones.list(ZoneLocation::Stack).is_empty());
}
