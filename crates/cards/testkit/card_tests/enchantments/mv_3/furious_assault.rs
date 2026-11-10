//! Tests for Furious Assault.

#[allow(clippy::wildcard_imports)]
use super::*;

// oracle_id = "f5d1a004-5598-4201-ac07-bca6cd23008f"
fn furious_assault() -> CardIndex {
    card_index("f5d1a004-5598-4201-ac07-bca6cd23008f")
}

/// Whenever you cast a creature spell, Furious Assault deals 1 damage to target
/// player or planeswalker.
#[test]
fn furious_assault_deals_1_damage_to_target_player_when_you_cast_a_creature_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[furious_assault(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, furious_assault());
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, gray_ogre());
    let Pending::ChooseTargets { .. } = engine.pending() else {
        panic!(
            "expected target prompt from Furious Assault, got {:?}",
            engine.pending()
        );
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}
