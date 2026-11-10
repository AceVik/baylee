//! `cards/enchantments/mv_1/elven_palisade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Elven Palisade` (`Coverage::Implemented`):
/// "Sacrifice a Forest: Target attacking creature gets -3/-0 until end of turn."
///
/// Verifies that during combat, activating `Elven Palisade` targets an attacking creature,
/// requires sacrificing a Forest as an activation cost, and reduces the attacker's
/// power by 3 until end of turn.
#[test]
fn elven_palisade_sacrifices_forest_to_reduce_attacker_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1320, forest())
        .battlefield(0, &[forest(), elven_palisade(), rootbreaker_wurm()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, rootbreaker_wurm()).expect("wurm deployed");
    let my_forest = on_battlefield(&engine, p0, forest()).expect("forest deployed");
    assert_eq!(pt(&engine, wurm), (6, 6));

    // Advance to combat and declare attackers
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wurm, Defender::Player(p1))],
            },
        )
        .expect("wurm declares attack");

    // Activate Elven Palisade targeting the attacking wurm
    activate(&mut engine, p0, elven_palisade(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Elven Palisade, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&wurm), "attacking wurm is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm],
            },
        )
        .unwrap();

    // Cost choice: sacrifice a Forest
    let Pending::ChooseCards {
        prompt, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice cost choice, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::CostSacrifice);
    assert!(
        options.contains(&my_forest),
        "controlled Forest is offered to sacrifice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_forest],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, wurm), (3, 6), "wurm gets -3/-0 to become 3/6");
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "sacrificed Forest is in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, forest()).is_none(),
        "sacrificed Forest is no longer on the battlefield"
    );
}
