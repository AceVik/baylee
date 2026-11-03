//! A table of three, played through the engine: turns go round in seat
//! order (CR 101.4, 500.1), an attacker chooses which opponent it attacks
//! (CR 506.2), only the attacked player may block it (CR 802.4a), and a
//! player who has left takes no more turns (CR 800.4a).

use super::testkit::{
    Duel, basic_forest, keep_mulligans, on_battlefield, pass_until, quiet_creature,
};
use super::*;
use baylee_core::ids::Defender;

const SEED: u64 = 0x7ab1_0003;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

type Game = Engine<super::testkit::RegistryLookup>;

fn table() -> Game {
    let mut engine = Duel::table(SEED, basic_forest(), 3)
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .battlefield(2, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn actives(engine: &mut Game, turns: usize) -> Vec<PlayerId> {
    let mut seen = vec![engine.state().turn.active];
    while seen.len() < turns {
        let number = engine.state().turn.number;
        pass_until(engine, |e| e.state().turn.number > number);
        seen.push(engine.state().turn.active);
    }
    seen
}

fn attack(engine: &mut Game, target: PlayerId) -> baylee_core::ids::ObjectId {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let elf = on_battlefield(engine, P0, quiet_creature()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(target))],
            },
        )
        .unwrap();
    elf
}

#[test]
fn turns_go_round_in_seat_order() {
    let mut engine = table();
    assert_eq!(actives(&mut engine, 6), vec![P0, P1, P2, P0, P1, P2]);
}

#[test]
fn an_attacker_may_choose_either_opponent() {
    for target in [P1, P2] {
        let mut engine = table();
        attack(&mut engine, target);
        pass_until(&mut engine, |e| {
            e.state().turn.phase == Phase::SecondMain
                || matches!(e.pending(), Pending::ChooseBlockers { .. })
        });
        if let Pending::ChooseBlockers { player, .. } = engine.pending().clone() {
            engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap();
        }
        pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
        for seat in [P1, P2] {
            let expected = if seat == target { 19 } else { 20 };
            assert_eq!(
                engine.state().players[usize::from(seat.get())].life,
                expected,
                "attacking {target:?}"
            );
        }
    }
}

#[test]
fn only_the_attacked_player_is_asked_to_block() {
    let mut engine = table();
    let elf = attack(&mut engine, P2);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, P2);
    let bystander = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    assert!(
        engine
            .apply(
                P2,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bystander, elf)],
                },
            )
            .is_err(),
        "seat 1's creature is not seat 2's to block with"
    );
}

#[test]
fn a_player_cannot_attack_themselves_at_a_table_of_three() {
    let mut engine = table();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(elf, Defender::Player(P0))],
                },
            )
            .is_err()
    );
}

#[test]
fn a_player_who_left_is_skipped() {
    let mut engine = table();
    engine.dev_state_mut(P0).expect("dev board").players[1].life = 0;
    let seen = actives(&mut engine, 5);
    assert!(!seen[1..].contains(&P1), "{seen:?}");
    assert_eq!(seen, vec![P0, P2, P0, P2, P0]);
}
