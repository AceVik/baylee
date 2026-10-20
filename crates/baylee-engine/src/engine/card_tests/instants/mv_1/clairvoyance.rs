//! `cards/instants/mv_1/clairvoyance.rs`, played.
//!
//! Oracle:
//! - "Look at target player's hand."
//! - "Draw a card at the beginning of the next turn's upkeep."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "851333be-d7d5-472a-95cb-0a28da09ce24"
fn clairvoyance() -> CardIndex {
    card_index("851333be-d7d5-472a-95cb-0a28da09ce24")
}

#[test]
fn clairvoyance_looks_at_target_players_hand_and_draws_at_next_turns_upkeep() {
    let mut engine = lk::game(SEED, &[], &[], &[clairvoyance()], &[index::GRIZZLY_BEARS]);

    let their_bears = in_hand(&engine, P1, index::GRIZZLY_BEARS).expect("in P1's hand");

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, clairvoyance());
    engine
        .apply(P0, PlayerAction::ChoosePlayer(P1))
        .expect("target P1 with Clairvoyance");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected LookAtHand prompt");
    };
    assert_eq!(options, vec![their_bears], "looks at target player's hand");
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![] })
        .expect("dismiss LookAtHand");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "Clairvoyance does not draw immediately on resolution"
    );

    // Advance to P1's upkeep (the next turn's upkeep) and resolve the delayed draw trigger.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        1,
        "Clairvoyance draws a card at the beginning of the next turn's upkeep"
    );
}
