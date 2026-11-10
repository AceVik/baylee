//! `cards/creatures/mv_4/warmonger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "bae92332-1a0b-476b-8719-190e8d8cc03a"
fn warmonger() -> CardIndex {
    card_index("bae92332-1a0b-476b-8719-190e8d8cc03a")
}

/// `Warmonger` (`Coverage::Implemented`):
/// "{2}: This creature deals 1 damage to each creature without flying and each
/// player. Any player may activate this ability."
#[test]
fn warmonger_damages_nonflying_creatures_and_each_player_and_opponent_can_activate() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9718, mountain())
        .battlefield(0, &[warmonger(), index::STORM_CROW])
        .battlefield(1, &[index::GRIZZLY_BEARS, mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    let monger = on_battlefield(&engine, p0, warmonger()).expect("Warmonger");
    let crow = on_battlefield(&engine, p0, index::STORM_CROW).expect("Storm Crow");
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).expect("Grizzly Bears");

    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: monger,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Warmonger (3/3 without flying) and Grizzly Bears (2/2 without flying) take 1 damage;
    // Storm Crow (1/2 with flying) takes 0 damage; both players lose 1 life.
    assert_eq!(engine.state().object(monger).unwrap().damage, 1);
    assert_eq!(engine.state().object(bears).unwrap().damage, 1);
    assert_eq!(engine.state().object(crow).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}
