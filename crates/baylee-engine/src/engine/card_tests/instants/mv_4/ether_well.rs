//! `cards/instants/mv_4/ether_well.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ETHER_WELL, GRIZZLY_BEARS};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Ether Well (a2b6faca-3321-45d2-a027-ff3a8ae61f10) — {3}{U} Instant:
/// "Put target creature on top of its owner's library."
#[test]
fn ether_well_puts_target_creature_on_top_of_owners_library() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[ETHER_WELL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ETHER_WELL);
    aim_at(&mut engine, P0, bears);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P1),
    );
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, GRIZZLY_BEARS).is_none());
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.map(|c| c.index),
        Some(GRIZZLY_BEARS)
    );
}
