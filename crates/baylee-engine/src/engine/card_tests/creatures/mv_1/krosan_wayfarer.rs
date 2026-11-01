//! `cards/creatures/mv_1/krosan_wayfarer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn krosan_wayfarer_sacrifices_itself_to_put_a_land_card_from_hand_onto_battlefield_untapped() {
    let wayfarer = card_index("1729317d-ccfb-4f27-84c6-20b36b2826c9");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[wayfarer, mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wayfarer);
    pass_until(&mut engine, stack_is_empty);

    let wf = on_battlefield(&engine, p0, wayfarer).expect("Krosan Wayfarer entered");
    let land_in_hand = in_hand(&engine, p0, mountain()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wf,
                ability_index: 0,
            },
        )
        .expect("sacrifice cost activates immediately");
    assert!(in_graveyard(&engine, p0, wayfarer).is_some());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let m = on_battlefield(&engine, p0, mountain()).expect("Mountain entered battlefield");
    assert!(!is_tapped(&engine, m), "enters untapped");
}
