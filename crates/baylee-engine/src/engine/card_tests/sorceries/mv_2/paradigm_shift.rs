//! `cards/sorceries/mv_2/paradigm_shift.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, PARADIGM_SHIFT};

/// `Paradigm Shift` ({1}{U}, Sorcery, Partial) shuffles your graveyard into
/// your library.
#[test]
fn paradigm_shift_shuffles_graveyard_into_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), GRIZZLY_BEARS])
        .hand(0, &[PARADIGM_SHIFT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let _ = engine.state.move_object(
        bears,
        ZoneLocation::Graveyard(p0),
        crate::zone::ZonePosition::Top,
        Cause::Effect,
    );
    engine.refresh_offer();

    let lib_before = library_size(&engine, p0);
    cast_from_hand(&mut engine, p0, PARADIGM_SHIFT);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, GRIZZLY_BEARS).is_none());
    assert_eq!(library_size(&engine, p0), lib_before + 1);
}
