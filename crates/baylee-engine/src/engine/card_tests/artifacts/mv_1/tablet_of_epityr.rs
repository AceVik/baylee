//! `cards/artifacts/mv_1/tablet_of_epityr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Tablet of Epityr: "Whenever an artifact you control is put into a
/// graveyard from the battlefield, you may pay {1}. If you do, you gain 1
/// life."
///
/// The Ring dies through the destruction door, which is exactly the "put
/// into a graveyard from the battlefield" the sentence names (CR 700.4's
/// "dies"; CR 603.6c's leaves-the-battlefield look-back) — a bounced or
/// exiled artifact would never be one. The {1} is paid out of the green the
/// Forest filled first, so the pool empties with the payment.
#[test]
fn tablet_of_epityr_pays_one_to_gain_a_life_when_your_artifact_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tablet_of_epityr(), quiet_artifact(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, quiet_artifact()).expect("the Ring is out");
    // The Forest pays; the Ring's own {C}{C} would only confuse the pool.
    tap_mana_except(&mut engine, p0, ring);
    let before = life_of(&engine, p0);
    bury(&mut engine, &[ring]);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "the Tablet's controller is the one asked");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(life_of(&engine, p0), before + 1, "the {{1}} bought 1 life");

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the Ring died"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the floating {{G}} paid the {{1}}"
    );
}

/// The declined half and the word "you control" in one: p0's own Ring dying
/// is declined (no life, no mana spent), and p1's Ring dying asks nothing at
/// all — if it did, the walk would reach a `PayTax` question it cannot
/// answer and fail there.
#[test]
fn tablet_of_epityr_gains_nothing_when_declined_or_when_the_artifact_is_not_yours() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tablet_of_epityr(), quiet_artifact(), forest()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Ring");
    // The Forest pays; the Ring's own {C}{C} would only confuse the pool.
    tap_mana_except(&mut engine, p0, mine);
    let before = life_of(&engine, p0);
    bury(&mut engine, &[mine]);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(
        life_of(&engine, p0),
        before,
        "declining spends nothing and gains nothing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{G}} is still floating"
    );
    pass_until(&mut engine, stack_is_empty);

    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Ring");
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::sba::destroy(state, theirs);
    }
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p0),
        before,
        "\"an artifact you control\" was not met"
    );
}
