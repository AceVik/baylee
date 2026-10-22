//! `cards/creatures/mv_2/nurturing_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn nurturing_licid() -> CardIndex {
    card_index("537a7af1-2b7e-4828-b645-7e34e86058d5")
}

/// Nurturing Licid (`537a7af1-2b7e-4828-b645-7e34e86058d5`) — {1}{G} Creature — Licid (Partial):
/// "{G}, {T}: ... {G}: Regenerate enchanted creature."
#[test]
fn nurturing_licid_regenerates_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                nurturing_licid(),
                forest(),
                mountain(),
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, nurturing_licid(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
