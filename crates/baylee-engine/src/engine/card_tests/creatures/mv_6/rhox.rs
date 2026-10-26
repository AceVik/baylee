//! `cards/creatures/mv_6/rhox.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn rhox_regenerates_for_two_and_a_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("f6f4c684-2915-4a58-9c4b-487387f7cf73"),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let rhox = on_battlefield(
        &engine,
        p0,
        card_index("f6f4c684-2915-4a58-9c4b-487387f7cf73"),
    )
    .expect("Rhox");
    assert_eq!(pt(&engine, rhox), (5, 5));

    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, rhox, 0);
    kill(&mut engine, rhox);

    assert!(
        on_battlefield(
            &engine,
            p0,
            card_index("f6f4c684-2915-4a58-9c4b-487387f7cf73")
        )
        .is_some(),
        "Rhox survives destruction via regeneration shield"
    );
    assert!(
        engine
            .state()
            .object(rhox)
            .unwrap()
            .status
            .contains(Status::TAPPED),
        "regenerated Rhox is tapped"
    );
}
