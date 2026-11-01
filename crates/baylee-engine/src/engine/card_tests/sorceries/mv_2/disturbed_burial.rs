//! `cards/sorceries/mv_2/disturbed_burial.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn disturbed_burial() -> CardIndex {
    card_index("cdbaa21d-5896-4b8b-9a08-f0839ccfddfc")
}

/// Disturbed Burial (`cdbaa21d-5896-4b8b-9a08-f0839ccfddfc`) — {1}{B} Sorcery (Partial):
/// "Buyback {3}. Return target creature card from your graveyard to your hand."
#[test]
fn disturbed_burial_returns_target_creature_card_from_graveyard_to_hand() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::GRIZZLY_BEARS, swamp(), swamp(), forest()])
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[disturbed_burial()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears_perm = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let land = on_battlefield(&engine, P0, forest()).unwrap();
    let opposing = on_battlefield(&engine, PlayerId::new(1), index::HILL_GIANT).unwrap();
    bury(&mut engine, &[bears_perm, land, opposing]);
    engine.refresh_offer();
    let bears = in_graveyard(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, disturbed_burial());
    let menu = aim_at(&mut engine, P0, bears);
    assert!(!menu.contains(&land));
    assert!(!menu.contains(&opposing));
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, P0, index::GRIZZLY_BEARS).is_some());
    assert!(in_graveyard(&engine, P0, disturbed_burial()).is_some());
}
