//! Agreed draws (CR 104.4a): unanimous or nothing.

use super::testkit::*;
use super::*;

fn forest() -> baylee_core::ids::CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// Both players agree: the game ends with no winner, while both are alive.
#[test]
fn an_accepted_offer_ends_the_game_in_a_draw() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest()).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.apply(p0, PlayerAction::OfferDraw).unwrap();
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!("expected the offer to reach p1, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    assert!(matches!(
        prompt,
        crate::choice::YesNoPrompt::DrawOffer { proposer } if proposer == p0
    ));

    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    let Pending::GameOver(result) = engine.pending().clone() else {
        panic!("expected the game to end, got {:?}", engine.pending())
    };
    assert_eq!(result.winner, None);
    assert_eq!(result.reason, crate::win::EndReason::Draw);
    // A draw is not an elimination: nobody lost.
    assert!(engine.state().players.iter().all(|p| !p.has_lost));
}

/// One refusal and the game carries on from exactly where it was.
#[test]
fn a_refused_offer_hands_priority_straight_back() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(32, forest()).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let before = engine.pending().clone();

    engine.apply(p0, PlayerAction::OfferDraw).unwrap();
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();

    assert_eq!(
        format!("{:?}", engine.pending()),
        format!("{before:?}"),
        "the interrupted decision comes back untouched"
    );
}

/// The offer is a priority action: it cannot be used to interrupt someone
/// else's decision.
#[test]
fn an_offer_without_priority_is_refused() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(33, forest()).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(engine.apply(p1, PlayerAction::OfferDraw).is_err());
}
