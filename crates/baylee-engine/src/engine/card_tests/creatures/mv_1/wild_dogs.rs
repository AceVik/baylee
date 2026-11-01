#[allow(clippy::wildcard_imports)]
use super::*;

fn wild_dogs() -> CardIndex {
    card_index("711cda97-bf57-4b22-b9c1-854ef448d27a")
}

#[test]
fn wild_dogs_cycles_to_draw_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[wild_dogs()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dogs_id = in_hand(&engine, p0, wild_dogs()).expect("wild dogs in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: dogs_id,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, wild_dogs()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
}

#[test]
fn wild_dogs_changes_controller_only_for_a_unique_life_leader_on_its_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    for (mine, theirs, owner) in [(10, 20, p1), (20, 20, p0), (25, 20, p0)] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[wild_dogs()])
            .life(0, mine)
            .life(1, theirs)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        assert!(on_battlefield(&engine, owner, wild_dogs()).is_some());
        if owner == p0 {
            engine.dev_state_mut(p0).unwrap().players[1].life = 30;
            reach_their_main_phase(&mut engine, p1);
            assert!(
                on_battlefield(&engine, p0, wild_dogs()).is_some(),
                "opponent's upkeep does not trigger"
            );
        }
    }
}
