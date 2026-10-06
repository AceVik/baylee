//! `cards/creatures/mv_5/ogre_arsonist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ogre Arsonist` is a 3/3 Ogre costing `{4}{R}` under `Coverage::Implemented`.
/// It prints "When this creature enters, destroy target land."
/// When cast from hand off five Mountains, it resolves onto the battlefield and puts its enters trigger
/// on the stack. Choosing an opponent's land as the target destroys that land upon resolution,
/// leaving `Ogre Arsonist` on the battlefield.
#[test]
fn ogre_arsonist_enters_and_destroys_target_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[ogre_arsonist()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let their_land = on_battlefield(&engine, p1, forest()).expect("opponent controls a Forest");

    cast_from_hand(&mut engine, p0, ogre_arsonist());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Ogre Arsonist ETB, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&their_land),
        "opponent's Forest is a legal land target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_land],
                players: Vec::new(),
            },
        )
        .expect("targeting opponent's Forest is legal");

    pass_until(&mut engine, stack_is_empty);

    let ogre = on_battlefield(&engine, p0, ogre_arsonist())
        .expect("Ogre Arsonist resolved onto the battlefield");
    assert_eq!(pt(&engine, ogre), (3, 3), "printed body is 3/3");
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "targeted Forest was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "destroyed Forest is in opponent's graveyard"
    );
}
