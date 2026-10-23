#[allow(clippy::wildcard_imports)]
use super::*;

fn bravado() -> CardIndex {
    card_index("69b5af09-b3e6-4abf-b62e-9199c4f43ba7")
}

#[test]
fn bravado_grants_plus_one_plus_one_per_other_creature_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[bravado()])
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                grizzly_bears(),
                llanowar_elves(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    cast_from_hand(&mut engine, p0, bravado());
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

    assert_eq!(pt(&engine, bears), (4, 4));
    let ally = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    assert_eq!(pt(&engine, ally), (2, 2));
    kill(&mut engine, ally);
    assert_eq!(pt(&engine, bears), (3, 3));
}
