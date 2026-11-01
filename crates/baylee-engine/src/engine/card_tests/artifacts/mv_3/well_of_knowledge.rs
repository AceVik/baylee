//! `cards/artifacts/mv_3/well_of_knowledge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn well_of_knowledge() -> CardIndex {
    card_index("99e2b36b-1fab-4b09-b371-d97a6854dfbc")
}

fn hand_size(e: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    e.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Well of Knowledge (99e2b36b-1fab-4b09-b371-d97a6854dfbc) — "{2}: Draw a
/// card. Any player may activate this ability but only during their draw
/// step." The Well belongs to player 0; in main phase 1 of turn 1 it is not
/// offered to anyone. In player 1's draw step player 1, not its controller,
/// may pay {2} and draw (two Forests' mana, and nothing left over); the
/// controller is not offered it then.
#[test]
fn well_of_knowledge_lets_the_opponent_draw_in_their_own_draw_step() {
    let mut e = lk::game(
        SEED,
        &[well_of_knowledge(), forest(), forest()],
        &[forest(), forest()],
        &[],
        &[],
    );
    tap_all_mana(&mut e, P0);
    assert!(
        !lk::offered(&e, well_of_knowledge(), P0, 0),
        "not in a main phase"
    );

    pass_until(&mut e, |e| {
        e.state().turn.step == Step::Draw
            && e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    tap_all_mana(&mut e, P1);
    assert!(
        lk::offered(&e, well_of_knowledge(), P1, 0),
        "any player: the opponent too"
    );

    let hand_before = hand_size(&e, P1);
    activate(&mut e, P1, well_of_knowledge(), 0);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        hand_size(&e, P1),
        hand_before + 1,
        "the activating player draws"
    );
    assert_eq!(hand_size(&e, P0), 0, "its controller draws nothing");
    assert_eq!(lk::pool_total(&e, P1), 0, "the ability costs {{2}}");
}

/// Well of Knowledge — "only during their draw step": in its controller's own
/// draw step (turn 3; the first player skips the turn-1 draw) the controller
/// is offered it, and when priority passes to the opponent in that same step
/// the opponent is not.
#[test]
fn well_of_knowledge_is_only_offered_to_the_player_whose_draw_step_it_is() {
    let mut e = lk::game(
        SEED,
        &[well_of_knowledge(), forest(), forest()],
        &[forest(), forest()],
        &[],
        &[],
    );

    pass_until(&mut e, |e| {
        e.state().turn.number == 3
            && e.state().turn.step == Step::Draw
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut e, P0);
    assert!(
        lk::offered(&e, well_of_knowledge(), P0, 0),
        "the active player may in their draw step"
    );

    lk::pass_once(&mut e);
    assert!(
        matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
        "priority moved to the opponent in the same step: {:?}",
        e.pending()
    );
    tap_all_mana(&mut e, P1);
    assert!(
        !lk::offered(&e, well_of_knowledge(), P1, 0),
        "it is not the opponent's draw step"
    );
}
