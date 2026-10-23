#[allow(clippy::wildcard_imports)]
use super::*;

fn falter() -> CardIndex {
    card_index("d4b50749-a016-4aff-8d70-a1707cabf57b")
}

#[test]
fn falter_prevents_creatures_without_flying_from_blocking() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[falter()])
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[wild_elephant(), ornithopter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("seated");

    cast_from_hand(&mut engine, p0, falter());
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(elephant, bear)]
                }
            )
            .is_err()
    );
    let flyer = on_battlefield(&engine, p1, ornithopter()).unwrap();
    let Pending::ChooseBlockers { blockers, .. } = engine.pending() else {
        panic!("block offer");
    };
    assert!(crate::engine::card_tests::legends_kit::may_block(
        blockers, flyer, bear
    ));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(flyer, bear)],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let blocks = crate::engine::card_tests::legends_kit::attack(&mut engine, &[bear]);
    assert!(
        crate::engine::card_tests::legends_kit::may_block(&blocks, elephant, bear),
        "restriction expires after the turn"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elephant, bear)],
            },
        )
        .unwrap();
}
