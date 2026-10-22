//! `cards/creatures/mv_2/sylvan_hierophant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, LIGHTNING_BOLT, SYLVAN_HIEROPHANT};

/// `Sylvan Hierophant` ({1}{G}, Creature — Human Cleric 1/2) exiles itself when
/// it dies and returns another target creature card from your graveyard to your
/// hand.
#[test]
fn sylvan_hierophant_exiles_self_and_returns_other_creature_from_graveyard_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[SYLVAN_HIEROPHANT, GRIZZLY_BEARS, mountain()])
        .hand(0, &[LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hierophant = on_battlefield(&engine, p0, SYLVAN_HIEROPHANT).unwrap();
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let _ = engine.state.move_object(
        bears,
        ZoneLocation::Graveyard(p0),
        crate::zone::ZonePosition::Top,
        Cause::Effect,
    );
    engine.refresh_offer();
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, hierophant);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(hierophant).unwrap().zone, Zone::Exile);
    assert!(in_hand(&engine, p0, GRIZZLY_BEARS).is_some());
}
