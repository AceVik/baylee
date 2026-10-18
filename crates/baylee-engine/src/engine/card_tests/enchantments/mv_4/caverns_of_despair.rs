//! `cards/enchantments/mv_4/caverns_of_despair.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit::hill_giant;

#[test]
fn caverns_of_despair_caps_attackers_and_blockers_at_two_each_combat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                baylee_core::generated::index::CAVERNS_OF_DESPAIR,
                grizzly_bears(),
                gray_ogre(),
                hill_giant(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), gray_ogre(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let a1 = on_battlefield(&engine, p0, grizzly_bears()).expect("attacker 1");
    let a2 = on_battlefield(&engine, p0, gray_ogre()).expect("attacker 2");
    let a3 = on_battlefield(&engine, p0, hill_giant()).expect("attacker 3");

    let b1 = on_battlefield(&engine, p1, grizzly_bears()).expect("blocker 1");
    let b2 = on_battlefield(&engine, p1, gray_ogre()).expect("blocker 2");
    let b3 = on_battlefield(&engine, p1, hill_giant()).expect("blocker 3");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![
                        (a1, Defender::Player(p1)),
                        (a2, Defender::Player(p1)),
                        (a3, Defender::Player(p1)),
                    ],
                },
            )
            .is_err(),
        "declaring three attackers under Caverns of Despair is illegal"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(a1, Defender::Player(p1)), (a2, Defender::Player(p1))],
            },
        )
        .expect("declaring two attackers is legal");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(b1, a1), (b2, a1), (b3, a2)],
                },
            )
            .is_err(),
        "declaring three distinct blockers under Caverns of Despair is illegal"
    );

    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(b1, a1), (b2, a2)],
            },
        )
        .expect("declaring two blockers is legal");
}
