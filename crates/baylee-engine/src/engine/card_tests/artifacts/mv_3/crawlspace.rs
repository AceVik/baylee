#[allow(clippy::wildcard_imports)]
use super::*;

fn crawlspace() -> CardIndex {
    card_index("2296370c-fe34-4df6-92a5-260f1634bede")
}

#[test]
fn crawlspace_caps_attackers_at_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[crawlspace()])
        .battlefield(
            0,
            &[
                gray_ogre(),
                earth_elemental(),
                veteran_bodyguard(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, crawlspace());
    pass_until(&mut engine, stack_is_empty);

    let b1 = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    let b2 = on_battlefield(&engine, p0, earth_elemental()).unwrap();
    let b3 = on_battlefield(&engine, p0, veteran_bodyguard()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![
                        (b1, baylee_core::ids::Defender::Player(p1)),
                        (b2, baylee_core::ids::Defender::Player(p1)),
                        (b3, baylee_core::ids::Defender::Player(p1)),
                    ],
                },
            )
            .is_err()
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![
                        (b1, baylee_core::ids::Defender::Player(p1)),
                        (b2, baylee_core::ids::Defender::Player(p1)),
                    ],
                },
            )
            .is_ok()
    );
}
