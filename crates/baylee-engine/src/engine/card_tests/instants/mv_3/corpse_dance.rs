//! `cards/instants/mv_3/corpse_dance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn corpse_dance() -> CardIndex {
    card_index("59b02ca7-e7bd-4820-ac32-87ea1a8c64c0")
}

/// Corpse Dance (59b02ca7-e7bd-4820-ac32-87ea1a8c64c0) — {2}{B} Instant (Partial):
/// "Return the top creature card of your graveyard to the battlefield."
#[test]
fn corpse_dance_returns_creature_from_graveyard_to_battlefield() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[index::GRIZZLY_BEARS, swamp(), swamp(), swamp()])
        .hand(0, &[corpse_dance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    bury(&mut engine, &[bears]);
    engine.refresh_offer();
    let gy_bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, corpse_dance());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![gy_bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_some());
}
