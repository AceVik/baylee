#[allow(clippy::wildcard_imports)]
use super::*;

fn spike_rogue() -> CardIndex {
    card_index("90ec956f-b6de-4214-9b5a-895280e542c1")
}

#[test]
fn spike_rogue_enters_with_two_counters_and_moves_counter_to_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[spike_rogue()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, spike_rogue());
    pass_until(&mut engine, stack_is_empty);
    let rogue = on_battlefield(&engine, p0, spike_rogue()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, rogue), (2, 2));
    assert_eq!(pt(&engine, bears), (2, 2));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: rogue,
                ability_index: 0,
            },
        )
        .unwrap();
    let land = on_battlefield(&engine, p0, forest()).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("targets")
    };
    assert!(!options.contains(&land));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, rogue), (1, 1));
    assert_eq!(pt(&engine, bears), (3, 3));
}
