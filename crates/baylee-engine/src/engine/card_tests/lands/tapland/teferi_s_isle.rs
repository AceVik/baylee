//! `cards/lands/tapland/teferi_s_isle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn teferis_isle_enters_tapped_and_taps_for_two_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[teferi_s_isle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, teferi_s_isle());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(land)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
    assert!(is_tapped(&engine, land), "phasing precedes untapping");
    let version = engine.state().object(land).unwrap().version;
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        engine
            .state()
            .object(land)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !engine
            .state()
            .object(land)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
    assert_eq!(engine.state().object(land).unwrap().version, version);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, teferi_s_isle(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 2);
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, land));
}
