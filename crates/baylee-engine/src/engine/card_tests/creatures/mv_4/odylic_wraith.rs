//! `cards/creatures/mv_4/odylic_wraith.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::ODYLIC_WRAITH;

/// `Odylic Wraith` ({3}{B}, Creature — Wraith 2/2) has swampwalk and makes a
/// player it deals damage to discard a card.
#[test]
fn odylic_wraith_has_swampwalk_and_forces_discard_on_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[ODYLIC_WRAITH])
        .battlefield(1, &[swamp(), llanowar_elves()])
        .hand(1, &[swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand1_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    let wraith = on_battlefield(&engine, p0, ODYLIC_WRAITH).unwrap();
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wraith, Defender::Player(p1))],
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
                    blockers: vec![(elves, wraith)],
                },
            )
            .is_err(),
        "Swampwalk prevents blocking while defending player controls a Swamp"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseCards for p1 discard");
    };
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == crate::turn::Phase::SecondMain
    });

    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand1_before - 1
    );
}
