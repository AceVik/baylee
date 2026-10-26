//! `cards/creatures/mv_6/sliptide_serpent.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn sliptide_serpent_returns_itself_to_owners_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[index::SLIPTIDE_SERPENT])
        .battlefield(0, &[island(); 10])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, index::SLIPTIDE_SERPENT);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, index::SLIPTIDE_SERPENT).is_some());

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, index::SLIPTIDE_SERPENT, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, index::SLIPTIDE_SERPENT).is_none());
    assert!(in_hand(&engine, p0, index::SLIPTIDE_SERPENT).is_some());
}
