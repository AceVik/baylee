//! A combat played through the engine's own questions with real cards
//! (Llanowar Elves, a 1/1): attacking taps the attacker (CR 508.1f), an
//! unblocked attacker's damage reaches the player (CR 510.1a), a block
//! trades the two creatures (CR 510.1c, 704.5g), an attack against the
//! wrong seat, with a creature that cannot attack or with a blocker the
//! defending player does not control is refused.

use super::testkit::{
    Duel, basic_forest, keep_mulligans, on_battlefield, pass_until, quiet_creature,
};
use super::*;
use baylee_core::ids::Defender;

const SEED: u64 = 0xc0b4_7001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn duel(theirs: &[baylee_core::ids::CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    engine
}

/// Passes priority until seat 0 is asked to declare attackers.
fn to_attackers(engine: &mut Game) {
    for _ in 0..60 {
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == P0 => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected question {other:?}"),
        }
    }
    panic!("seat 0 was never asked to attack");
}

fn life(engine: &Game, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn after_combat(engine: &mut Game) {
    pass_until(engine, |e| e.state().turn.phase == Phase::SecondMain);
}

#[test]
fn an_unblocked_attacker_taps_and_hits_the_defending_player() {
    let mut engine = duel(&[]);
    to_attackers(&mut engine);
    let elf = on_battlefield(&engine, P0, quiet_creature()).expect("an elf");
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(P1))],
            },
        )
        .unwrap();
    assert!(
        engine
            .state()
            .object(elf)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED),
        "attacking taps a creature without vigilance"
    );
    after_combat(&mut engine);
    assert_eq!(life(&engine, P1), 19);
    assert_eq!(life(&engine, P0), 20);
}

#[test]
fn a_blocked_attacker_and_its_blocker_trade() {
    let mut engine = duel(&[quiet_creature()]);
    to_attackers(&mut engine);
    let mine = on_battlefield(&engine, P0, quiet_creature()).expect("my elf");
    let theirs = on_battlefield(&engine, P1, quiet_creature()).expect("their elf");
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mine, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(theirs, mine)],
            },
        )
        .unwrap();
    after_combat(&mut engine);
    assert_eq!(life(&engine, P1), 20, "the block kept the damage off");
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_none());
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_none());
}

#[test]
fn declaring_no_attackers_deals_no_damage() {
    let mut engine = duel(&[]);
    to_attackers(&mut engine);
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    after_combat(&mut engine);
    assert_eq!(life(&engine, P1), 20);
    let elf = on_battlefield(&engine, P0, quiet_creature()).expect("an elf");
    assert!(
        !engine
            .state()
            .object(elf)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
}

#[test]
fn a_player_cannot_attack_themselves() {
    let mut engine = duel(&[]);
    to_attackers(&mut engine);
    let elf = on_battlefield(&engine, P0, quiet_creature()).expect("an elf");
    let before = engine.state().snapshot_hash();
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
    assert_eq!(engine.state().snapshot_hash(), before);
}

#[test]
fn a_player_cannot_attack_with_an_opponents_creature() {
    let mut engine = duel(&[quiet_creature()]);
    to_attackers(&mut engine);
    let theirs = on_battlefield(&engine, P1, quiet_creature()).expect("their elf");
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(theirs, Defender::Player(P1))],
                },
            )
            .is_err()
    );
}

#[test]
fn a_tapped_creature_is_refused_as_an_attacker() {
    let mut engine = duel(&[]);
    to_attackers(&mut engine);
    let elf = on_battlefield(&engine, P0, quiet_creature()).expect("an elf");
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .object_mut(elf)
        .unwrap()
        .status
        .insert(crate::object::Status::TAPPED);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(elf, Defender::Player(P1))],
                },
            )
            .is_err()
    );
}

#[test]
fn a_player_cannot_block_with_a_creature_they_do_not_control() {
    let mut engine = duel(&[quiet_creature()]);
    to_attackers(&mut engine);
    let mine = on_battlefield(&engine, P0, quiet_creature()).expect("my elf");
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mine, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(mine, mine)],
                },
            )
            .is_err()
    );
}

/// Lionheart Maverick has vigilance: attacking does not tap it
/// (CR 702.20b), and its damage reaches the player all the same.
#[test]
fn a_vigilant_attacker_stays_untapped() {
    let maverick = super::testkit::card_index("5058b9f1-9941-495d-9eed-554ae2f6d500");
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[maverick])
        .start();
    keep_mulligans(&mut engine);
    to_attackers(&mut engine);
    let knight = on_battlefield(&engine, P0, maverick).expect("the knight");
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(knight, Defender::Player(P1))],
            },
        )
        .unwrap();
    assert!(
        !engine
            .state()
            .object(knight)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    after_combat(&mut engine);
    assert_eq!(life(&engine, P1), 19);
}

/// Shivan Dragon flies: a ground creature's block on it is refused
/// (CR 702.9b), and its five damage reach the defending player.
#[test]
fn a_flier_cannot_be_blocked_by_a_ground_creature() {
    let dragon = super::testkit::card_index("711eea87-0fa3-46e0-a42b-fa5a86455f04");
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[dragon])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    to_attackers(&mut engine);
    let shivan = on_battlefield(&engine, P0, dragon).expect("the dragon");
    let elf = on_battlefield(&engine, P1, quiet_creature()).expect("their elf");
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(shivan, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
            || e.state().turn.phase == Phase::SecondMain
    });
    if matches!(engine.pending(), Pending::ChooseBlockers { .. }) {
        assert!(
            engine
                .apply(
                    P1,
                    PlayerAction::DeclareBlockers {
                        blockers: vec![(elf, shivan)],
                    },
                )
                .is_err()
        );
    }
    after_combat(&mut engine);
    assert_eq!(life(&engine, P1), 15);
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_some());
}
