#[allow(clippy::wildcard_imports)]
use super::*;

fn thrull_surgeon() -> CardIndex {
    card_index("ead5a2bf-f48a-43a2-89bb-df3cc587c843")
}

#[test]
fn thrull_surgeon_sacrifices_to_choose_and_discard_from_target_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[thrull_surgeon()])
        .hand(1, &[grizzly_bears(), forest()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, thrull_surgeon());
    pass_until(&mut engine, stack_is_empty);
    let surgeon = on_battlefield(&engine, p0, thrull_surgeon()).expect("seated");
    let bears_in_p1_hand = in_hand(&engine, p1, grizzly_bears()).expect("in hand");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: surgeon,
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
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears_in_p1_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, thrull_surgeon()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
