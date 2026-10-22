//! `cards/instants/mv_2/time_and_tide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn time_and_tide() -> CardIndex {
    card_index("b6fee94a-30b8-415e-8567-ad03ccb1d658")
}

fn shimmering_efreet() -> CardIndex {
    card_index("34f52d00-301d-4065-a86b-8393b4c6e99d")
}

/// `Time and Tide` ({U}{U}, Instant, `Coverage::Partial`) phases out all
/// phased-in creatures with phasing.
#[test]
fn time_and_tide_phases_out_creatures_with_phasing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[shimmering_efreet(), GRIZZLY_BEARS])
        .hand(0, &[time_and_tide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let efreet = on_battlefield(&engine, p1, shimmering_efreet()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, time_and_tide());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(efreet)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert!(
        !engine
            .state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
