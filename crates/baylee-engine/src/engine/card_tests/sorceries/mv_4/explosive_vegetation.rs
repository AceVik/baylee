//! `cards/sorceries/mv_4/explosive_vegetation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn explosive_vegetation_searches_library_for_two_basic_lands_onto_battlefield_tapped() {
    let p0 = PlayerId::new(0);
    let veg = card_index("a0abd957-01a0-4aa9-8fc8-0e6840d21606");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[veg])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, veg);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected search choice");
    };
    let picked = vec![options[0], options[1]];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: picked.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(all_on_battlefield(&engine, p0, forest()).len(), 6);
    for f in picked {
        assert!(is_tapped(&engine, f));
    }
}
