#[allow(clippy::wildcard_imports)]
use super::*;

fn blanchwood_armor() -> CardIndex {
    card_index("80ea56ad-e741-4a85-b4e8-ce62e7d593d5")
}

#[test]
fn blanchwood_armor_grants_plus_one_plus_one_per_forest_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[blanchwood_armor()])
        .battlefield(0, &[forest(), forest(), forest(), grizzly_bears()])
        .battlefield(1, &[forest(), forest(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, blanchwood_armor());
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (5, 5));
    let their_bear = on_battlefield(&engine, PlayerId::new(1), grizzly_bears()).unwrap();
    assert_eq!(pt(&engine, their_bear), (2, 2));
}
