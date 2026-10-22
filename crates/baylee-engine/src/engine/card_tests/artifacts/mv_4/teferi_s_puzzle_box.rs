//! `cards/artifacts/mv_4/teferi_s_puzzle_box.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn teferi_s_puzzle_box() -> CardIndex {
    card_index("37abcc92-9466-47ea-9e0b-5eda2eb62c8e")
}

/// `Teferi's Puzzle Box` ({4}, Artifact, `Coverage::Partial`) triggers at the
/// beginning of your draw step to let you put cards from your hand on the
/// bottom of your library and draw that many cards.
#[test]
fn teferi_s_puzzle_box_triggers_on_draw_step_to_bottom_hand_cards_and_draw_that_many() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[teferi_s_puzzle_box(), GRIZZLY_BEARS, HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let h1 = in_hand(&engine, p0, GRIZZLY_BEARS).unwrap();
    let h2 = in_hand(&engine, p0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, teferi_s_puzzle_box());
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's main phase, then to P0's next draw step where Puzzle Box asks ChooseCards
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![h1, h2],
            },
        )
        .expect("put both cards on bottom of library");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(h1).unwrap().zone, Zone::Library);
    assert_eq!(engine.state().object(h2).unwrap().zone, Zone::Library);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}
