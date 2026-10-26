//! `cards/creatures/mv_6/nesting_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn nesting_wurm_searches_library_for_up_to_three_copies_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, card_index("3160dada-d59f-4f23-8420-ba14f9df7514"))
        .hand(0, &[card_index("3160dada-d59f-4f23-8420-ba14f9df7514")])
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(
        &mut engine,
        p0,
        card_index("3160dada-d59f-4f23-8420-ba14f9df7514"),
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("choose to search");
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseCards");
    };
    assert_eq!((min, max), (0, 3));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options[..2].to_vec(),
            },
        )
        .expect("choose two Nesting Wurms");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1 + 2
    );
}
