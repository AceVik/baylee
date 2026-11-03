//! A cast trigger with a "may", with a real card: Kor Firewalker
//! ("Whenever a player casts a red spell, you may gain 1 life"). It
//! triggers on casting, whoever casts (CR 603.2), resolves before the spell
//! that caused it (CR 603.3), asks its controller (CR 603.5), and does not
//! trigger for a spell of another colour.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, pass_until, quiet_creature,
    stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::CardIndex;

const SEED: u64 = 0xfe11_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn firewalker() -> CardIndex {
    card_index("fb37ff1c-7461-4a16-b27f-6eef0db937fe")
}

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn duel(hand: &[CardIndex], lands: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, hand)
        .battlefield(0, lands)
        .battlefield(1, &[firewalker()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

/// Seat 0 shocks seat 0 (so seat 1's life moves only by the trigger) and
/// every question is answered, seat 1's "may" with `yes`. Returns how many
/// times seat 1 was asked.
fn shock_with_answer(engine: &mut Game, yes: bool) -> usize {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, shock()).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    let mut asked = 0;
    for _ in 0..30 {
        if stack_is_empty(engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                assert_eq!(player, P1, "the trigger's controller is asked");
                asked += 1;
                engine.apply(player, PlayerAction::YesNo(yes)).unwrap();
            }
            _ => {
                let (player, action) = super::testkit::answer_one(engine).expect("answerable");
                engine.apply(player, action).unwrap();
            }
        }
    }
    asked
}

#[test]
fn a_red_spell_triggers_and_yes_gains_a_life() {
    let mut engine = duel(&[shock()], &[mountain()]);
    assert_eq!(shock_with_answer(&mut engine, true), 1);
    assert_eq!(engine.state().players[1].life, 21);
    assert_eq!(engine.state().players[0].life, 18);
}

#[test]
fn no_gains_nothing() {
    let mut engine = duel(&[shock()], &[mountain()]);
    assert_eq!(shock_with_answer(&mut engine, false), 1);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn the_trigger_waits_above_the_spell_that_caused_it() {
    let mut engine = duel(&[shock()], &[mountain()]);
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, shock()).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len(),
        2,
        "the Shock and the trigger above it"
    );
}

#[test]
fn a_green_spell_triggers_nothing() {
    let mut engine = duel(&[quiet_creature()], &[basic_forest()]);
    super::testkit::cast_from_hand(&mut engine, P0, quiet_creature());
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len(),
        1
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 20);
}
