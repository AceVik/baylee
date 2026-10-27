//! `cards/creatures/mv_1/wall_of_hope.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Wall of Hope (`e43055c7-2245-4544-b0b4-ee30dc1ebf14`) — {W} Creature — Wall 0/3:
/// "Defender
/// Whenever this creature is dealt damage, you gain that much life."
#[test]
fn wall_of_hope_gains_life_equal_to_damage_dealt_to_it() {
    let wall_card = card_index("e43055c7-2245-4544-b0b4-ee30dc1ebf14");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), mountain()])
        .hand(0, &[wall_card, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let w_land = on_battlefield(&engine, p0, plains()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id == w_land);
    cast_with_floating(&mut engine, p0, wall_card);
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, wall_card).unwrap();
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wall],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, wall_card).is_some());
    assert_eq!(engine.state().players[0].life, 23);
}
