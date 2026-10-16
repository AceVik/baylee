//! `cards/artifacts/mv_4/grafted_skullcap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn grafted_skullcap() -> CardIndex {
    card_index("7694887a-2f3b-4ef1-b598-f3323f018ef4")
}

/// "At the beginning of your end step, discard your hand." The hand goes at
/// the controller's end step, and only there: nothing is lost at the other
/// seat's.
#[test]
fn the_hand_is_discarded_at_my_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grafted_skullcap()])
        .hand(0, &[plains(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 2);

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && e.state().turn.active == p0
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        0,
        "the whole hand goes at my end step"
    );
    assert!(in_graveyard(&engine, p0, plains()).is_some());
    assert!(in_graveyard(&engine, p0, swamp()).is_some());

    // The other seat's end step has nothing of mine to discard, but the
    // trigger is "your": give me a card and it must survive their turn.
    reach_their_main_phase(&mut engine, p1);
    let card = in_graveyard(&engine, p0, plains()).unwrap();
    engine
        .state
        .move_object(
            card,
            ZoneLocation::Hand(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End && e.state().turn.active == p1
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        1,
        "their end step does not empty my hand"
    );
}

/// "At the beginning of your draw step, draw an additional card." Two cards
/// leave the library on my turn, one extra, and none on the other seat's.
#[test]
fn the_controller_draws_an_extra_card_in_their_own_draw_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[grafted_skullcap()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lib = library_size(&engine, p0);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(library_size(&engine, p0), lib, "their draw step is not mine");
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(library_size(&engine, p0), lib - 2, "one normal, one extra");
}
