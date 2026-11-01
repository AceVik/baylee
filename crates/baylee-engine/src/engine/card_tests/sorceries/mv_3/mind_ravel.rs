//! `cards/sorceries/mv_3/mind_ravel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_ravel() -> CardIndex {
    card_index("8cd0fcac-800a-431c-aca9-804998704606")
}

/// Mind Ravel (8cd0fcac-800a-431c-aca9-804998704606) — {2}{B} Sorcery:
/// - "Target player discards a card."
/// - "Draw a card at the beginning of the next turn's upkeep."
#[test]
fn mind_ravel_makes_target_player_discard_and_draws_on_next_turns_upkeep() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[mind_ravel()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p1_card = in_hand(&engine, P1, forest()).unwrap();
    let lib_before = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, mind_ravel());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P1, forest()).is_some(),
        "target player P1 discarded the card"
    );
    assert_eq!(
        library_size(&engine, P0),
        lib_before,
        "Mind Ravel does not draw immediately upon resolution"
    );

    // Advance to the next turn's upkeep (P1's upkeep) and let the delayed trigger resolve.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        library_size(&engine, P0),
        lib_before - 1,
        "delayed upkeep trigger drew a card for P0 at the beginning of the next turn's upkeep"
    );
}
