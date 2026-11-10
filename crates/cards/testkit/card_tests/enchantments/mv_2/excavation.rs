//! `cards/enchantments/mv_2/excavation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn excavation() -> CardIndex {
    card_index("9f7b2ec6-a828-499b-a2ac-766bdaaf30c1")
}

#[test]
fn excavation_lets_any_player_pay_one_and_sacrifice_a_land_to_draw_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[excavation(), island()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let enchantment = on_battlefield(&engine, p0, excavation()).unwrap();
    let p0_island = on_battlefield(&engine, p0, island()).unwrap();
    let p0_hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, excavation(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![p0_island],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand_before + 1
    );
    assert!(in_graveyard(&engine, p0, island()).is_some());

    // Opponent activates Excavation on their turn by paying {1} and sacrificing their Forest.
    reach_their_main_phase(&mut engine, p1);
    let p1_forest = on_battlefield(&engine, p1, forest()).unwrap();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: enchantment,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![p1_forest],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before + 1
    );
    assert!(in_graveyard(&engine, p1, forest()).is_some());
}
