//! `cards/creatures/artifacts/mv_4/complex_automaton.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Complex Automaton ({4} 4/4 Artifact Creature — Golem):
/// "At the beginning of your upkeep, if you control seven or more permanents, return this creature to its owner's hand."
#[test]
fn complex_automaton_returns_to_hand_on_upkeep_only_with_seven_or_more_permanents() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("e4951b4e-ba68-495c-8062-e89250d02cf3"),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    // Turn 1: P0 controls 6 permanents; pass to P1 and back to P0's upkeep.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        on_battlefield(
            &engine,
            P0,
            card_index("e4951b4e-ba68-495c-8062-e89250d02cf3")
        )
        .is_some(),
        "with only 6 permanents, Complex Automaton stays on the battlefield"
    );

    // Play a 7th permanent (Forest), then advance to P0's next upkeep.
    let land = in_hand(&engine, P0, forest()).unwrap();
    engine
        .apply(P0, PlayerAction::PlayLand { card: land })
        .unwrap();
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    assert!(
        on_battlefield(
            &engine,
            P0,
            card_index("e4951b4e-ba68-495c-8062-e89250d02cf3")
        )
        .is_none(),
        "with 7 permanents on upkeep, Complex Automaton returned to hand"
    );
    assert!(
        in_hand(
            &engine,
            P0,
            card_index("e4951b4e-ba68-495c-8062-e89250d02cf3")
        )
        .is_some()
    );
}
