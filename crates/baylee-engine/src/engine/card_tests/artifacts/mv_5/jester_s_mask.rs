//! `cards/artifacts/mv_5/jester_s_mask.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn jester_s_mask() -> CardIndex {
    card_index("bcc797b9-d2e5-4fde-8d80-c890c871f344")
}

/// Jester's Mask (bcc797b9-d2e5-4fde-8d80-c890c871f344) — {5} Artifact:
/// - "This artifact enters tapped."
/// - "{1}, {T}, Sacrifice this artifact: Target opponent puts the cards from their hand on top of their library. Search that player's library for that many cards. That player puts those cards into their hand, then shuffles."
///   (Partial: enters tapped; {1}, {T}, Sacrifice shuffles target opponent's hand into their library.)
#[test]
fn jesters_mask_enters_tapped_and_shuffles_target_opponents_hand_into_their_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[jester_s_mask()])
        .hand(1, &[island(), plains(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    cast_from_hand(&mut engine, P0, jester_s_mask());
    pass_until(&mut engine, stack_is_empty);

    let mask = on_battlefield(&engine, P0, jester_s_mask()).expect("Mask entered");
    assert!(
        is_tapped(&engine, mask),
        "Jester's Mask enters the battlefield tapped"
    );
    assert!(
        !ability_offered(&engine, mask, 0),
        "cannot activate {{T}} ability while Jester's Mask is tapped"
    );

    // Advance to P0's next main phase so Jester's Mask untaps naturally.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(!is_tapped(&engine, mask));

    let p1_hand = engine.state().zones.list(ZoneLocation::Hand(P1)).len();
    let p1_lib = engine.state().zones.list(ZoneLocation::Library(P1)).len();
    assert!(p1_hand >= 3);

    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, jester_s_mask(), 0);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets target prompt, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        player_options,
        vec![P1],
        "TargetSpec::AnyOpponent offers only P1 and excludes P0"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, jester_s_mask()).is_some(),
        "Jester's Mask was sacrificed as part of the activation cost"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P1)).len(),
        0,
        "target opponent's hand was shuffled into their library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(P1)).len(),
        p1_lib + p1_hand,
        "target opponent's library grew by the cards from their hand"
    );
}
