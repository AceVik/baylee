//! `cards/sorceries/mv_3/fallow_earth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::FALLOW_EARTH;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Fallow Earth (05a4acfd-3bf3-4ec2-a28a-bc43ba7c6fbf) — {2}{G} Sorcery:
/// "Put target land on top of its owner's library."
#[test]
fn fallow_earth_puts_target_land_on_top_of_owners_library() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .battlefield(1, &[mountain()])
        .hand(0, &[FALLOW_EARTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let mtn = on_battlefield(&engine, P1, mountain()).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, FALLOW_EARTH);
    aim_at(&mut engine, P0, mtn);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P1),
    );
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, mountain()).is_none());
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P1))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.map(|c| c.index),
        Some(mountain())
    );
}
