//! `cards/creatures/artifacts/mv_7/darksteel_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Darksteel Gargoyle` prints `Flying` and `Indestructible` on a 3/3 artifact creature with `Coverage::Implemented`.
/// In this scenario, seat 0 controls `Darksteel Gargoyle` while seat 1 casts `vindicate()` targeting it.
/// Because `KeywordSet::INDESTRUCTIBLE` prevents destruction effects, the gargoyle survives on the battlefield upon resolution.
#[test]
fn darksteel_gargoyle_survives_destroy_effect_due_to_indestructible() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[darksteel_gargoyle()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);

    let gargoyle =
        on_battlefield(&engine, p0, darksteel_gargoyle()).expect("darksteel gargoyle seated");
    assert_eq!(pt(&engine, gargoyle), (3, 3));
    let kw = keywords(&engine, gargoyle);
    assert!(kw.contains(KeywordSet::FLYING), "has flying");
    assert!(
        kw.contains(KeywordSet::INDESTRUCTIBLE),
        "has indestructible"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![gargoyle],
            },
        )
        .expect("vindicate targets darksteel gargoyle");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, darksteel_gargoyle()).is_some(),
        "`Darksteel Gargoyle` remains on the battlefield despite destroy spell"
    );
    assert!(
        in_graveyard(&engine, p0, darksteel_gargoyle()).is_none(),
        "`Darksteel Gargoyle` was not put into the graveyard"
    );
}
