//! `cards/artifacts/mv_3/bosium_strip.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn bosium_strip() -> CardIndex {
    card_index("2fa6f1b1-00af-433f-b8e1-36db99cd9bba")
}

/// `Bösium Strip` ({3}, Artifact, `Coverage::Partial`) activates for `{3}, {T}`.
#[test]
fn bosium_strip_activates_for_three_and_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), plains(), plains()],
        )
        .hand(0, &[bosium_strip()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bosium_strip());
    pass_until(&mut engine, stack_is_empty);
    let strip = on_battlefield(&engine, p0, bosium_strip()).unwrap();

    activate(&mut engine, p0, bosium_strip(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(strip)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
