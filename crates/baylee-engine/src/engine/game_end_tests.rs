//! How a game ends through the engine: a player who draws from an empty
//! library loses at the next check (CR 704.5b) and the last player
//! standing wins (CR 104.2a); two players losing at once is a draw
//! (CR 104.4a); a finished game takes no more answers; and at a table of
//! three, one loss does not end the game.

use super::testkit::{Duel, basic_forest, keep_mulligans, pass_until};
use super::*;
use crate::win::{EndReason, Victor};
use crate::zone::ZoneLocation;

const SEED: u64 = 0xe4d0_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn duel() -> Game {
    let mut engine = Duel::new(SEED, basic_forest()).start();
    keep_mulligans(&mut engine);
    engine
}

fn over(engine: &Game) -> bool {
    matches!(engine.pending(), Pending::GameOver(_))
}

fn run_to_the_end(engine: &mut Game) {
    pass_until(engine, over);
}

fn empty_library(engine: &mut Game, seat: PlayerId) {
    let state = engine.dev_state_mut(P0).expect("dev board");
    let cards = state.zones.list(ZoneLocation::Library(seat)).clone();
    for card in cards {
        state
            .move_object(
                card,
                ZoneLocation::Exile(seat),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
}

#[test]
fn drawing_from_an_empty_library_loses_the_game() {
    let mut engine = duel();
    empty_library(&mut engine, P1);
    run_to_the_end(&mut engine);
    let Pending::GameOver(result) = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(result.winner, Some(Victor::Player(P0)));
    assert_eq!(result.reason, EndReason::LastPlayerStanding);
    assert_eq!(
        engine.state().players[1].loss,
        Some(crate::event::LossReason::EmptyDraw)
    );
    assert_eq!(
        engine.state().turn.active,
        P1,
        "it happened in seat 1's turn"
    );
}

#[test]
fn an_empty_library_is_no_loss_until_a_draw_is_tried() {
    let mut engine = duel();
    empty_library(&mut engine, P1);
    // Seat 0 starts and does not draw on its first turn; seat 1 has not
    // drawn yet either, so nobody has lost while seat 0's turn lasts.
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().turn.active, P0);
    assert!(!over(&engine));
    assert!(!engine.state().players[1].has_lost());
}

#[test]
fn a_player_at_zero_life_loses_and_the_other_wins() {
    let mut engine = duel();
    engine.dev_state_mut(P0).expect("dev board").players[0].life = 0;
    run_to_the_end(&mut engine);
    let Pending::GameOver(result) = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(result.winner, Some(Victor::Player(P1)));
}

#[test]
fn both_players_losing_at_once_is_a_draw() {
    let mut engine = duel();
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        state.players[0].life = 0;
        state.players[1].life = 0;
    }
    run_to_the_end(&mut engine);
    let Pending::GameOver(result) = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(result.winner, None);
    assert_eq!(result.reason, EndReason::Draw);
}

#[test]
fn a_finished_game_takes_no_more_answers() {
    let mut engine = duel();
    engine.dev_state_mut(P0).expect("dev board").players[1].poison = 10;
    run_to_the_end(&mut engine);
    let before = engine.state().snapshot_hash();
    for seat in [P0, P1] {
        assert!(matches!(
            engine.apply(seat, PlayerAction::PassPriority),
            Err(EngineError::GameOver)
        ));
    }
    assert_eq!(engine.state().snapshot_hash(), before);
    assert!(over(&engine));
}

#[test]
fn at_a_table_of_three_one_loss_does_not_end_the_game() {
    let mut engine = Duel::table(SEED, basic_forest(), 3).start();
    keep_mulligans(&mut engine);
    engine.dev_state_mut(P0).expect("dev board").players[1].life = 0;
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| over(e) || e.state().turn.number > turn);
    assert!(!over(&engine), "two players are still in the game");
    assert!(engine.state().players[1].has_lost());
    assert!(!engine.state().players[0].has_lost());
    assert!(!engine.state().players[2].has_lost());
}

#[test]
fn at_a_table_of_three_the_last_one_standing_wins() {
    let mut engine = Duel::table(SEED, basic_forest(), 3).start();
    keep_mulligans(&mut engine);
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        state.players[0].life = 0;
        state.players[1].life = 0;
    }
    run_to_the_end(&mut engine);
    let Pending::GameOver(result) = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(result.winner, Some(Victor::Player(PlayerId::new(2))));
}

fn plague_spitter() -> baylee_core::ids::CardIndex {
    super::testkit::card_index("5feedfb0-30e6-400d-9e28-d541ea1aa14e")
}

fn nether_shadow() -> baylee_core::ids::CardIndex {
    super::testkit::card_index("c358b9e2-524c-434b-b3fa-74d2aa6d1df7")
}

/// Seat 0's Plague Spitter at its first upkeep, seat 1 at 1 life: its
/// damage kills seat 0's two Elves and seat 1 in one state-based check
/// (CR 704.3), in a game where Nether Shadow reads the order of a graveyard,
/// so simultaneous arrivals are put in order by their owner (CR 404.3).
/// Played until the game is over or a graveyard order is asked.
fn spitter_finishes_it() -> Game {
    let elf = super::testkit::quiet_creature();
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[plague_spitter(), elf, elf])
        .hand(0, &[nether_shadow()])
        .life(1, 1)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        over(e) || matches!(e.pending(), Pending::Arrange { .. })
    });
    engine
}

/// A game ends immediately when a player wins (CR 104.1), and in a duel
/// the last opponent losing is the win (CR 104.2a), so nothing is asked
/// once the check that decided it is done: where the owner of the cards
/// that died beside the loss would order them, the game is over instead.
///
/// The record: Carbon's ladder game hc-expert-b/game-0023, turn 36. Seat 1
/// was at -5 and had lost; seat 0 was still asked to put the five creatures
/// that died in the same combat damage step in order.
///
/// The question was the whole difference: the engine as it was hashed the
/// board it asked on as the board this one ends on.
#[test]
fn a_decided_game_asks_no_graveyard_order() {
    let engine = spitter_finishes_it();
    let Pending::GameOver(result) = engine.pending().clone() else {
        panic!("asked {:?} after seat 1 lost", engine.pending())
    };
    assert_eq!(result.winner, Some(Victor::Player(P0)));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        2,
        "both Elves died with seat 1's loss"
    );
    let then = super::progress::ASK_IN_A_DECIDED_GAME.with(|ask| {
        ask.set(true);
        let then = spitter_finishes_it();
        ask.set(false);
        then
    });
    assert!(
        matches!(then.pending(), Pending::Arrange { player: P0, .. }),
        "the control: the engine as it was asked the order, got {:?}",
        then.pending()
    );
    assert_eq!(then.snapshot_hash(), engine.snapshot_hash());
}
