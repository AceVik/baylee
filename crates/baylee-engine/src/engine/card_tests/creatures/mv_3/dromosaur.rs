#[allow(clippy::wildcard_imports)]
use super::*;

fn dromosaur() -> CardIndex {
    card_index("4576cc5b-7b3a-41a7-9b84-acadf2d4adf5")
}

#[test]
fn dromosaur_gets_plus_two_minus_two_when_blocked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[dromosaur()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dino = on_battlefield(&engine, p0, dromosaur()).expect("seated");
    let blocker = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dino, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, dino)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, dino), (4, 1));
}
