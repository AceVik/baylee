#[allow(clippy::wildcard_imports)]
use super::*;

fn pyromancy() -> CardIndex {
    card_index("71a176ce-cbe7-4cec-a2ca-58abf59ad964")
}

#[test]
fn pyromancy_discards_a_card_at_random_and_deals_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), pyromancy()])
        .hand(0, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pyro = on_battlefield(&engine, p0, pyromancy()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pyro,
                ability_index: 0,
            },
        )
        .unwrap();
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

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1
    );
    assert_eq!(engine.state().players[1].life, 19);
}
