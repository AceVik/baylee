//! A combat trick with real cards: Giant Growth cast in the declare
//! blockers step, after blocks are fixed. The pumped creature deals and
//! takes damage as it now is, and the block itself is not undone by the
//! change (CR 509.1h).

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, on_battlefield, pass_until,
    quiet_creature, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, Defender, ObjectId};

const SEED: u64 = 0x7e1c_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn giant_growth() -> CardIndex {
    card_index("5748ebf1-24e3-499d-ab7c-c2cebd462a24")
}

fn troll() -> CardIndex {
    card_index("c2f86723-4765-4dac-a9f6-9cfa543613b2")
}

/// Seat 0 attacks with an elf; seat 1 blocks it with `blocker`. Returns
/// the game in the declare blockers step with seat 0 holding priority.
fn blocked_by(blocker: CardIndex, growth_for: usize) -> (Game, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(growth_for, &[giant_growth()])
        .battlefield(0, &[quiet_creature(), basic_forest()])
        .battlefield(1, &[blocker, basic_forest()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    let wall = on_battlefield(&engine, P1, blocker).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(P1))],
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
                blockers: vec![(wall, elf)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { .. })
    });
    (engine, elf, wall)
}

fn grow(engine: &mut Game, seat: PlayerId, target: ObjectId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    assert_eq!(engine.state().turn.step, crate::turn::Step::DeclareBlockers);
    tap_all_mana(engine, seat);
    let card = in_hand(engine, seat, giant_growth()).expect("in hand");
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .unwrap();
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
}

#[test]
fn a_pumped_attacker_kills_a_blocker_it_could_not_kill_before() {
    let (mut engine, elf, _) = blocked_by(troll(), 0);
    grow(&mut engine, P0, elf);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    // A 4/4 elf and a 4/3 troll deal each other lethal damage.
    assert!(on_battlefield(&engine, P1, troll()).is_none());
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_none());
}

#[test]
fn a_pumped_blocker_survives_and_kills_the_attacker() {
    let (mut engine, elf, blocker) = blocked_by(quiet_creature(), 1);
    grow(&mut engine, P1, blocker);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        engine
            .state()
            .object(elf)
            .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield)
    );
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_some());
}

#[test]
fn a_pump_does_not_undo_the_block() {
    let (mut engine, elf, _) = blocked_by(troll(), 0);
    grow(&mut engine, P0, elf);
    assert!(engine.state().combat.is_blocked(elf));
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "a blocked creature without trample"
    );
}

/// Seat 0 casts Pyroclasm; seat 1 answers with Giant Growth on its elf.
/// The pump resolves first (CR 405.5), so the elf is a 4/4 when the two
/// damage arrive and survives; seat 0's unpumped elf does not.
#[test]
fn a_pump_in_response_saves_a_creature_from_a_sweeper() {
    let pyroclasm = card_index("e4bcd4ea-e7cd-4471-8f3b-18bb51d3d70c");
    let mountain = card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685");
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[pyroclasm])
        .hand(1, &[giant_growth()])
        .battlefield(0, &[mountain, mountain, quiet_creature()])
        .battlefield(1, &[basic_forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    super::testkit::cast_from_hand(&mut engine, P0, pyroclasm);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    let theirs = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    tap_all_mana(&mut engine, P1);
    let card = in_hand(&engine, P1, giant_growth()).unwrap();
    engine.apply(P1, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .unwrap();
    pass_until(&mut engine, super::testkit::stack_is_empty);
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_some());
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_none());
}
