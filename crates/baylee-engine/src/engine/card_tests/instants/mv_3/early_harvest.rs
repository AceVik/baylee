//! `cards/instants/mv_3/early_harvest.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CITY_OF_BRASS, EARLY_HARVEST};

const P0: PlayerId = PlayerId::new(0);

/// Early Harvest (32001382-3079-4bbe-8d86-05da654c0d1a) — {1}{G}{G} Instant:
/// "Target player untaps all basic lands they control."
#[test]
fn early_harvest_untaps_all_basic_lands_controlled_by_caster() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), plains(), CITY_OF_BRASS])
        .hand(0, &[EARLY_HARVEST])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let brass = on_battlefield(&engine, P0, CITY_OF_BRASS).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(brass)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, EARLY_HARVEST);
    pass_until(&mut engine, stack_is_empty);

    for &id in engine.state().zones.list(ZoneLocation::Battlefield) {
        let obj = engine.state().object(id).unwrap();
        if obj.controller == P0
            && obj
                .characteristics()
                .supertypes
                .contains(SupertypeSet::BASIC)
        {
            assert!(!obj.status.contains(Status::TAPPED));
        }
    }
    assert!(
        engine
            .state()
            .object(brass)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
