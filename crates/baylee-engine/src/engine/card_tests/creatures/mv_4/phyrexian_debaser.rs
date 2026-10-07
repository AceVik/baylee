//! `cards/creatures/mv_4/phyrexian_debaser.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Phyrexian Debaser` is a 2/2 Phyrexian Carrier costing `{3}{B}` under `Coverage::Implemented` with flying.
/// It prints "{T}, Sacrifice this creature: Target creature gets -2/-2 until end of turn."
/// When its ability is activated targeting an opponent's 2/2 creature (such as `Desert Drake`),
/// paying the cost taps and sacrifices `Phyrexian Debaser`. The ability resolves, giving -2/-2 to the target
/// and causing it to die as a state-based action.
#[test]
fn phyrexian_debaser_taps_and_sacrifices_to_kill_two_toughness_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let drake_card = card_index("ce8f4eb4-08b8-404b-9147-1e28c1b14a65");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[phyrexian_debaser()])
        .battlefield(1, &[drake_card])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let debaser =
        on_battlefield(&engine, p0, phyrexian_debaser()).expect("Debaser is on battlefield");
    let their_drake =
        on_battlefield(&engine, p1, drake_card).expect("opponent controls Desert Drake");
    assert_eq!(pt(&engine, debaser), (2, 2), "printed body is 2/2");
    assert!(
        keywords(&engine, debaser).contains(KeywordSet::FLYING),
        "Phyrexian Debaser has flying"
    );

    activate(&mut engine, p0, phyrexian_debaser(), 0);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Phyrexian Debaser, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&their_drake),
        "opponent's creature is offered as target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_drake],
            },
        )
        .expect("targeting opponent's creature is legal");

    // Cost paid: Debaser was tapped and sacrificed.
    assert!(
        on_battlefield(&engine, p0, phyrexian_debaser()).is_none(),
        "Phyrexian Debaser was sacrificed to pay the activation cost"
    );
    assert!(
        in_graveyard(&engine, p0, phyrexian_debaser()).is_some(),
        "sacrificed Debaser is in its owner's graveyard"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, drake_card).is_none(),
        "target creature with 2 toughness died from -2/-2"
    );
    assert!(
        in_graveyard(&engine, p1, drake_card).is_some(),
        "dead creature is in opponent's graveyard"
    );
}
