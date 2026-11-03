//! Mana through the engine with basic lands: a Forest's ability adds one
//! green and taps it (CR 106.4, 305.6), the mana waits in the pool while
//! the step lasts and empties as it ends (CR 500.4), a tapped land adds
//! nothing more, and the other player's pool is never touched.

use super::testkit::{
    Duel, basic_forest, keep_mulligans, on_battlefield, pass_until, tap_all_mana,
};
use super::*;
use baylee_core::mana::ManaColor;

const SEED: u64 = 0x3a4a_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn duel(forests: usize) -> Game {
    let forest = basic_forest();
    let mut engine = Duel::new(SEED, forest)
        .battlefield(0, &vec![forest; forests])
        .battlefield(1, &vec![forest; forests])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn green(engine: &Game, seat: PlayerId) -> u32 {
    engine.state().players[usize::from(seat.get())]
        .mana_pool
        .available(ManaColor::Green)
}

#[test]
fn a_forest_adds_one_green_and_taps() {
    let mut engine = duel(1);
    let forest = on_battlefield(&engine, P0, basic_forest()).expect("a forest");
    assert_eq!(tap_all_mana(&mut engine, P0), 1);
    assert_eq!(green(&engine, P0), 1);
    assert!(
        engine
            .state()
            .object(forest)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert_eq!(green(&engine, P1), 0, "the other pool is untouched");
}

#[test]
fn three_forests_add_three_green() {
    let mut engine = duel(3);
    assert_eq!(tap_all_mana(&mut engine, P0), 3);
    assert_eq!(green(&engine, P0), 3);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and nothing but green"
    );
}

#[test]
fn a_tapped_forest_offers_no_more_mana() {
    let mut engine = duel(1);
    tap_all_mana(&mut engine, P0);
    assert_eq!(tap_all_mana(&mut engine, P0), 0);
    assert_eq!(green(&engine, P0), 1);
}

#[test]
fn unspent_mana_empties_when_the_step_ends() {
    let mut engine = duel(2);
    tap_all_mana(&mut engine, P0);
    assert_eq!(green(&engine, P0), 2);
    pass_until(&mut engine, |e| e.state().turn.phase != Phase::FirstMain);
    assert!(
        engine.state().players[0].mana_pool.is_empty(),
        "the pool empties between steps"
    );
}

#[test]
fn mana_stays_while_priority_goes_back_and_forth_in_one_step() {
    let mut engine = duel(1);
    tap_all_mana(&mut engine, P0);
    // An activated mana ability does not use the stack and passes no
    // priority; the pool is still there while seat 0 holds priority.
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0));
    assert_eq!(green(&engine, P0), 1);
}

#[test]
fn the_forest_untaps_for_its_controller_next_turn() {
    let mut engine = duel(1);
    let forest = on_battlefield(&engine, P0, basic_forest()).expect("a forest");
    tap_all_mana(&mut engine, P0);
    pass_until(&mut engine, |e| e.state().turn.active == P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !engine
            .state()
            .object(forest)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    assert_eq!(tap_all_mana(&mut engine, P0), 1);
}

/// Llanowar Elves taps for {G} (CR 605.1a): a mana ability, so nothing
/// goes on the stack and its controller keeps priority (CR 605.3a).
#[test]
fn a_mana_creature_adds_its_mana_without_the_stack() {
    let elf_card = super::testkit::quiet_creature();
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[elf_card])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert_eq!(tap_all_mana(&mut engine, P0), 1);
    assert_eq!(green(&engine, P0), 1);
    assert!(engine.state().zones.stack_is_empty());
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0));
}

/// A mana ability with {T} in its cost is still a {T} ability: an elf that
/// arrived this turn cannot tap for mana (CR 302.6).
#[test]
fn a_mana_creature_that_just_arrived_cannot_tap_for_mana() {
    let elf_card = super::testkit::quiet_creature();
    let mut engine = Duel::new(SEED, elf_card).start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        let card = state.draw_cards(P0, 1)[0];
        state
            .move_object(
                card,
                crate::zone::ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    // Let the game offer again now that the elf is there.
    let step = engine.state().turn.step;
    pass_until(&mut engine, |e| {
        e.state().turn.step != step
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert_eq!(engine.state().turn.active, P0);
    assert_eq!(tap_all_mana(&mut engine, P0), 0);
}
