//! `cards/sorceries/mv_5/rout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "d69aa624-23d1-406a-b7e0-9140a6f57929"
fn rout() -> CardIndex {
    card_index("d69aa624-23d1-406a-b7e0-9140a6f57929")
}

/// A regeneration shield really bought from Lotleth Troll does not save it or
/// any other creature from Rout. Seven Plains and one Swamp pay both the {B}
/// for that shield and the {5}{W}{W} needed for the optional flash route.
#[test]
fn rout_with_its_optional_flash_cost_sweeps_even_a_regenerating_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board = vec![lotleth_troll(), llanowar_elves(), swamp()];
    board.extend([plains(); 7]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[rout()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1), "p1 reaches its own main");
    engine
        .apply(p1, PlayerAction::PassPriority)
        .expect("p1 gives p0 priority");

    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll is out");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);
    cast_sorcery_with_optional_flash(&mut engine, p0, rout());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, lotleth_troll()).is_some());
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
}
