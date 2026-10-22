//! `cards/creatures/mv_2/ertai_s_familiar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ERTAI_S_FAMILIAR, LIGHTNING_BOLT};

/// `Ertai's Familiar` ({1}{U}, Creature — Illusion 2/2) has phasing, removes
/// phasing until your next upkeep for `{U}`, and mills three cards when it
/// phases out or leaves the battlefield.
#[test]
fn ertai_s_familiar_removes_phasing_until_next_upkeep_and_mills_3_on_leave() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), mountain()])
        .hand(0, &[ERTAI_S_FAMILIAR, LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ERTAI_S_FAMILIAR);
    pass_until(&mut engine, stack_is_empty);

    let familiar = on_battlefield(&engine, p0, ERTAI_S_FAMILIAR).unwrap();
    assert!(keywords(&engine, familiar).contains(KeywordSet::PHASING));

    activate(&mut engine, p0, ERTAI_S_FAMILIAR, 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(!keywords(&engine, familiar).contains(KeywordSet::PHASING));

    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    cast_with_floating(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, familiar);
    pass_until(&mut engine, stack_is_empty);

    let gy_after = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(gy_after, gy_before + 5);
}
