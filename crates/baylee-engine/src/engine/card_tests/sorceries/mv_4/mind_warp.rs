//! `cards/sorceries/mv_4/mind_warp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mind_warp() -> CardIndex {
    card_index("5109aa76-ca40-44d1-abb4-5e7a23a2c274")
}

/// Mind Warp (5109aa76-ca40-44d1-abb4-5e7a23a2c274) — {X}{3}{B} Sorcery:
/// "Look at target player's hand and choose X cards from it. That player discards those cards."
/// `Coverage::Partial` — looks at target player's hand without discarding.
#[test]
fn mind_warp_looks_at_target_players_hand() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[mind_warp()])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p1_forest = in_hand(&engine, P1, forest()).unwrap();
    let p1_island = in_hand(&engine, P1, island()).unwrap();

    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, mind_warp());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    engine.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LookAtHand,
                ..
            }
        )
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        options.contains(&p1_forest) && options.contains(&p1_island),
        "Mind Warp shows P0 the cards in target player P1's hand"
    );
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, mind_warp()).is_some());
}
