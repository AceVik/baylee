//! `cards/creatures/mv_2/llanowar_druid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LLANOWAR_DRUID;

/// `Llanowar Druid` ({1}{G}, Creature — Elf Druid 1/2) — `{T}`, Sacrifice this
/// creature: Untap all Forests.
#[test]
fn llanowar_druid_sacrifices_to_untap_all_forests() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[LLANOWAR_DRUID, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, LLANOWAR_DRUID, 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, LLANOWAR_DRUID).is_some());
    let all_untapped = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .all(|&id| !is_tapped(&engine, id));
    assert!(all_untapped);
}
