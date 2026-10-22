//! `cards/creatures/mv_3/llanowar_sentinel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LLANOWAR_SENTINEL;

/// `Llanowar Sentinel` ({2}{G}, Creature — Elf 2/3) lets you pay `{1}{G}` when
/// it enters to search your library for a card named Llanowar Sentinel and put
/// it onto the battlefield.
#[test]
fn llanowar_sentinel_searches_library_for_another_copy_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[LLANOWAR_SENTINEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, LLANOWAR_SENTINEL);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, LLANOWAR_SENTINEL).is_some());
}
