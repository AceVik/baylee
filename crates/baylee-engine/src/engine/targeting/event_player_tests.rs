//! Trigger choices retain the event seat independently of the ability controller.
use super::*;
use crate::engine::testkit::*;
use crate::event::{Cause, GameEvent};
use crate::object::AbilityList;
use crate::turn::{Phase, Step};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    AbilityDef, Effect, Filter, PlayerRel, StepKind, TargetReq, TargetSpec, Trigger,
};
use baylee_core::ids::CardIndex;

const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::StepBegin {
        step: StepKind::Upkeep,
        whose: PlayerRel::EachPlayer
    },
    &[Effect::TapTarget],
    targets = Some(
        TargetReq::one(TargetSpec::ObjectOfEventPlayer(&Filter::CREATURE)).chosen_by_event_player()
    )
)];
fn card(name: &str) -> CardIndex {
    baylee_cards::generated::ALL
        .iter()
        .find(|(_, c)| c.name() == name)
        .unwrap()
        .1
        .index
}
fn queued() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::table(941, card("Forest"), 2)
        .battlefield(0, &[card("Forest")])
        .battlefield(1, &[card("Hill Giant"), card("Slippery Bogle")])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, A);
    let source = on_battlefield(&engine, A, card("Forest")).unwrap();
    let victim = on_battlefield(&engine, B, card("Hill Giant")).unwrap();
    engine
        .state
        .object_mut(source)
        .unwrap()
        .take_abilities(AbilityList::from_static(ABILITIES, None, None));
    let from = engine.state.journal.last_seq();
    engine.state.journal.record(GameEvent::StepChanged {
        active: B,
        phase: Phase::Beginning,
        step: Step::Upkeep,
    });
    // Collection occurs after the turn cursor has moved: the event is authoritative.
    assert_eq!(engine.state.turn.active, A);
    let captured = crate::trigger::collect(&engine.state, &engine.lookup, from);
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].event_player, Some(B));
    engine.trigger_queue.extend(captured);
    engine.trigger_scan_seq = engine.state.journal.last_seq();
    (engine, source, victim)
}
#[test]
fn event_player_chooses_with_captured_controller_after_source_changes_or_leaves() {
    for leaves in [false, true] {
        let (mut engine, source, victim) = queued();
        if leaves {
            engine
                .state
                .move_object(
                    source,
                    ZoneLocation::Graveyard(A),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        } else {
            engine.state.object_mut(source).unwrap().set_controller(B);
        }
        engine.collect_triggers();
        let Pending::ChooseTargets {
            player, options, ..
        } = engine.pending()
        else {
            panic!("target question")
        };
        assert_eq!(*player, B);
        assert_eq!(
            options.as_slice(),
            &[victim],
            "B's hexproof creature remains illegal for A's ability"
        );
        let before = engine.snapshot_hash();
        assert!(
            engine
                .apply(
                    A,
                    PlayerAction::ChooseObjects {
                        objects: vec![victim]
                    }
                )
                .is_err()
        );
        assert_eq!(engine.snapshot_hash(), before);
        engine
            .apply(
                B,
                PlayerAction::ChooseObjects {
                    objects: vec![victim],
                },
            )
            .unwrap();
        let stack = engine.state.zones.list(ZoneLocation::Stack);
        let ability = engine.state.object(*stack.last().unwrap()).unwrap();
        assert_eq!(ability.controller, A);
        assert!(matches!(
            ability.target_req.unwrap().spec,
            TargetSpec::ObjectControlledBy(_, B)
        ));
        pass_until(&mut engine, |e| {
            e.state.zones.list(ZoneLocation::Stack).is_empty()
        });
        assert!(
            engine
                .state
                .object(victim)
                .unwrap()
                .status
                .contains(crate::object::Status::TAPPED)
        );
    }
}
#[test]
fn event_player_target_binding_is_rechecked_and_distinct_event_seats_do_not_batch() {
    let (mut engine, _, victim) = queued();
    let mut other = engine.trigger_queue.front().unwrap().clone();
    other.event_player = Some(A);
    engine.trigger_queue.push_back(other);
    engine.collect_triggers();
    assert_eq!(engine.target_batch_count(), 1);
    engine.trigger_queue.pop_back();
    engine
        .apply(
            B,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    engine.state.object_mut(victim).unwrap().set_controller(A);
    pass_until(&mut engine, |e| {
        e.state.zones.list(ZoneLocation::Stack).is_empty()
    });
    assert!(
        !engine
            .state
            .object(victim)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED),
        "the original event player's target restriction is rechecked"
    );
}
