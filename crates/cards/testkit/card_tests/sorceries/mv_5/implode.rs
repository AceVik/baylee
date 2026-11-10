//! `cards/sorceries/mv_5/implode.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Implode` is a sorcery costing `{4}{R}` under `Coverage::Implemented`.
/// It prints "Destroy target land. Draw a card."
/// When cast from hand off five Mountains targeting an opponent's land, it destroys
/// the target land and causes the caster to draw a card.
#[test]
fn implode_destroys_target_land_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[implode()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let initial_p0_cards = library_size(&engine, p0);
    let target_land = on_battlefield(&engine, p1, forest()).expect("opponent controls a Forest");

    cast_from_hand(&mut engine, p0, implode());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Implode, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&target_land),
        "opponent's Forest is an offered target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "target land was destroyed and put into opponent's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "target land is no longer on the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, implode()).is_some(),
        "Implode resolved and went to graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        initial_p0_cards - 1,
        "caster drew a card from Implode"
    );
}
