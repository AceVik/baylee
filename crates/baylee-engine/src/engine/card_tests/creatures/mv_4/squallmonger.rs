//! `cards/creatures/mv_4/squallmonger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

// oracle_id = "50bf5af6-50d7-4dea-936e-508e24db0a03"
fn squallmonger() -> CardIndex {
    card_index("50bf5af6-50d7-4dea-936e-508e24db0a03")
}

/// `Squallmonger` (`Coverage::Implemented`):
/// "{2}: This creature deals 1 damage to each creature with flying and each
/// player. Any player may activate this ability."
#[test]
fn squallmonger_damages_flying_creatures_and_each_player_and_opponent_can_activate() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9706, forest())
        .battlefield(0, &[squallmonger(), index::STORM_CROW])
        .battlefield(1, &[index::GRIZZLY_BEARS, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p1));

    let monger = on_battlefield(&engine, p0, squallmonger()).expect("Squallmonger");
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

    assert_eq!(engine.state().object(crow).unwrap().damage, 1);
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}
