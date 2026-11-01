//! `cards/creatures/mv_3/sabertooth_cobra.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sabertooth_cobra() -> CardIndex {
    card_index("115c7ca7-c27b-43ca-9281-be6913e90059")
}

/// Sabertooth Cobra (`{2}{G}`, `Coverage::Partial`): 2/2 Snake that gives a
/// player a poison counter whenever it deals damage to that player.
#[test]
fn sabertooth_cobra_gives_damaged_player_a_poison_counter() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[sabertooth_cobra()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sabertooth_cobra());
    pass_until(&mut engine, stack_is_empty);
    let cobra = on_battlefield(&engine, p0, sabertooth_cobra()).unwrap();

    // Advance to p0's next turn so Sabertooth Cobra can attack.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(cobra, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[1].poison, 1);
}

/// "Deals damage to a player": a Cobra blocked by Grizzly Bears deals its
/// damage to the Bears, so nobody gets a poison counter.
#[test]
fn sabertooth_cobra_blocked_gives_no_poison_counter() {
    let mut engine = lk::game(
        SEED,
        &[sabertooth_cobra()],
        &[index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let cobra = lk::obj(&engine, P0, sabertooth_cobra());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    let offer = lk::attack(&mut engine, &[cobra]);
    assert!(lk::may_block(&offer, bears, cobra));
    lk::block(&mut engine, &[(bears, cobra)]);
    lk::through_combat(&mut engine);

    assert_eq!(engine.state().players[1].poison, 0);
    assert_eq!(lk::life(&engine, P1), 20);
}

/// The second poison counter ("at the beginning of their next upkeep unless
/// they pay {2}") is not on the card: pinned, so the test moves when it is.
#[test]
fn sabertooth_cobra_gives_only_the_first_counter_for_now() {
    let mut engine = lk::game(SEED, &[sabertooth_cobra()], &[], &[], &[]);
    let cobra = lk::obj(&engine, P0, sabertooth_cobra());

    lk::attack(&mut engine, &[cobra]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(engine.state().players[1].poison, 1);

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        engine.state().players[1].poison,
        1,
        "NOT SUPPORTED: printed, a second counter arrives at this upkeep unless they pay {{2}}"
    );
}
