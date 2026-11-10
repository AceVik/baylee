//! `cards/sorceries/mv_5/extortion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "8841fdb9-0565-4ef5-814c-293c14242741"
fn extortion() -> CardIndex {
    card_index("8841fdb9-0565-4ef5-814c-293c14242741")
}

/// Extortion — {3}{B}{B} Sorcery: "Look at target player's hand and choose up
/// to two cards from it. That player discards those cards."
/// `Coverage::Partial` — looks at target player's hand privately without discarding.
#[test]
fn extortion_looks_at_target_players_hand() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[extortion()])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p1_forest = in_hand(&engine, P1, forest()).unwrap();
    let p1_island = in_hand(&engine, P1, island()).unwrap();

    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, extortion());
    let Pending::ChoosePlayer { .. } = engine.pending() else {
        panic!("expected player prompt, got {:?}", engine.pending());
    };
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
        "Extortion shows P0 the cards in target player P1's hand"
    );
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, extortion()).is_some());
}
