//! The monarch's two inherent triggered abilities (CR 724.2).
//!
//! "At the beginning of the monarch's end step, that player draws a card"
//! and "Whenever a creature deals combat damage to the monarch, its
//! controller becomes the monarch". Both have no source and are controlled
//! by whoever was the monarch when they triggered. The designation is
//! planted on the state rather than made by a card: the cards that make a
//! monarch are tested with the cards (`card_tests`), and what is tested
//! here is what the designation does afterwards.

use super::testkit::*;
use super::*;
use crate::zone::ZoneLocation;

fn duel(seed: u64, board: &[baylee_core::ids::CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, basic_forest())
        .battlefield(0, board)
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn crown(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness trusts itself")
        .set_monarch(player);
}

fn hand(engine: &Engine<RegistryLookup>, player: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(player)).len()
}

fn stack(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Stack).clone()
}

/// What a stack object is called, controlled by and sourced from.
fn stack_entry(engine: &Engine<RegistryLookup>, id: ObjectId) -> (&str, PlayerId, ObjectId) {
    let obj = engine.state().object(id).expect("on the stack");
    (
        engine.state().names.get(obj.base.name),
        obj.controller,
        obj.ability.expect("an ability").source,
    )
}

/// The draw is a triggered ability on the stack, not a draw that happens
/// as the step begins: the monarch has priority with it waiting and the
/// hand still unchanged, and draws only when it resolves.
#[test]
fn the_monarch_draws_at_their_own_end_step_through_the_stack() {
    let p0 = PlayerId::new(0);
    let mut engine = duel(724, &[]);
    crown(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && !e.state().zones.stack_is_empty()
    });
    assert_eq!(engine.state().turn.active, p0);
    let before = hand(&engine, p0);
    let [trigger] = stack(&engine)[..] else {
        panic!("one ability on the stack, got {:?}", stack(&engine));
    };
    assert_eq!(
        stack_entry(&engine, trigger),
        ("Monarch", p0, ObjectId::NO_SOURCE),
        "sourceless, named for the designation, controlled by the monarch"
    );
    pass_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(hand(&engine, p0), before + 1, "the monarch drew one card");
}

/// Only the monarch's own end step. The draw used to happen at the
/// beginning of every end step, the opponent's too.
#[test]
fn the_monarch_does_not_draw_at_an_opponents_end_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel(725, &[]);
    crown(&mut engine, p1);
    let before = hand(&engine, p1);
    for _ in 0..100 {
        if engine.state().turn.active != p0 {
            break;
        }
        if engine.state().turn.step == crate::turn::Step::End {
            assert!(
                engine.state().zones.stack_is_empty(),
                "a monarch trigger in the opponent's end step: {:?}",
                stack(&engine)
                    .iter()
                    .map(|&id| stack_entry(&engine, id))
                    .collect::<Vec<_>>()
            );
        }
        let (player, action) = match engine.pending().clone() {
            Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
            Pending::ChooseAttackers { player, .. } => {
                (player, PlayerAction::DeclareAttackers { attackers: vec![] })
            }
            other => panic!("unexpected: {other:?}"),
        };
        engine.apply(player, action).unwrap();
    }
    assert_eq!(engine.state().turn.active, p1, "seat 0's turn is over");
    assert_eq!(
        hand(&engine, p1),
        before,
        "the monarch drew during the opponent's turn"
    );
}

/// Seat 0 attacks the monarch with its creature once it may, and the
/// monarch blocks with whatever it can when `block` says so. Returns with
/// the takeover on the stack, once the damage has landed.
fn attack_the_monarch(engine: &mut Engine<RegistryLookup>, block: bool) -> ObjectId {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for _ in 0..200 {
        match engine.pending().clone() {
            Pending::ChooseAttackers {
                player, attackers, ..
            } => {
                let attack = if player == p0 && !attackers.is_empty() {
                    vec![(attackers[0], baylee_core::ids::Defender::Player(p1))]
                } else {
                    Vec::new()
                };
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: attack })
                    .unwrap();
            }
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                let blockers = blockers
                    .iter()
                    .filter(|_| block)
                    .filter_map(|o| Some((o.blocker, *o.attackers.first()?)))
                    .take(1)
                    .collect();
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers })
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                let damaged = matches!(
                    engine.state().turn.step,
                    crate::turn::Step::CombatDamage | crate::turn::Step::CombatDamageFirst
                );
                if damaged && !engine.state().zones.stack_is_empty() {
                    let [trigger] = stack(engine)[..] else {
                        panic!("one ability on the stack, got {:?}", stack(engine));
                    };
                    return trigger;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
    panic!("seat 0 never dealt the monarch combat damage");
}

/// The takeover goes on the stack under the monarch who is losing the
/// title, and gives it to the attacking creature's controller.
#[test]
fn combat_damage_to_the_monarch_takes_the_crown() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = duel(726, &[quiet_creature()]);
    crown(&mut engine, p1);
    let trigger = attack_the_monarch(&mut engine, false);
    assert_eq!(
        stack_entry(&engine, trigger),
        ("Monarch", p1, ObjectId::NO_SOURCE),
        "controlled by the monarch it triggered against (CR 724.2)"
    );
    assert_eq!(engine.state().monarch, Some(p1), "not before it resolves");
    pass_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(engine.state().monarch, Some(p0));
}

fn fangren_hunter() -> baylee_core::ids::CardIndex {
    card_index("c5dc5546-e9e5-4b5b-b812-5716d4bdee0e")
}

fn baleful_strix() -> baylee_core::ids::CardIndex {
    card_index("37688720-03de-4eca-a82d-a0afe8d58adc")
}

/// The creature need not survive to take the crown. Fangren Hunter
/// tramples over a deathtouch Strix, and the Strix's damage kills it in
/// the same state-based check that the takeover waits behind. "Its
/// controller" is then read from what the creature last was.
#[test]
fn a_creature_that_died_dealing_the_damage_still_takes_the_crown() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(727, basic_forest())
        .battlefield(0, &[fangren_hunter()])
        .battlefield(1, &[baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    crown(&mut engine, p1);
    let trigger = attack_the_monarch(&mut engine, true);
    let hunter = engine
        .state()
        .object(trigger)
        .and_then(|o| o.event_object)
        .expect("the creature is the event object");
    assert_ne!(
        engine.state().object(hunter).map(|o| o.zone),
        Some(crate::zone::Zone::Battlefield),
        "the Hunter died to the deathtouch block before the takeover stacked"
    );
    pass_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(engine.state().monarch, Some(p0));
}
