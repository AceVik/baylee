//! `cards/artifacts/mv_1/despotic_scepter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Despotic Scepter — {1} artifact: "{T}: Destroy target permanent you own.
/// It can't be regenerated." The words that need a witness on both sides of
/// the table are "you own", so a second Sol Ring — the same card as the one
/// being destroyed — stands across the table and must stay off the menu: a
/// filter that had widened to any permanent would have offered it. The
/// victim is the Sol Ring beside the Scepter rather than the Scepter itself,
/// which leaves the artifact standing so its own {T} can be read as the cost
/// that was actually paid.
#[test]
fn despotic_scepter_destroys_a_permanent_its_controller_owns_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), quiet_artifact()])
        .hand(0, &[despotic_scepter()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {1} out of the two Forests; `tap_all_mana` also takes the Sol Ring's own
    // printed `{T}: Add {C}`, which is a mana ability whose whole price is its
    // own tap (#159), so what pays for the Scepter is real mana in the pool.
    cast_from_hand(&mut engine, p0, despotic_scepter());
    pass_until(&mut engine, stack_is_empty);
    let scepter = on_battlefield(&engine, p0, despotic_scepter()).expect("the Scepter resolved");
    let mine = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let theirs = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // The price is `{T}` and no mana at all, so the offer turns on nothing the
    // pool could have supplied.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(scepter, 0)),
        "the Scepter's only line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, despotic_scepter(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target permanent\" is a target choice, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&mine),
        "a permanent this seat owns is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"you own\" is not \"any permanent\": the Sol Ring across the table is \
         the same card, and it is not owned by the activating seat: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the permanent the question offered is the one that dies");

    // CR 601.2h: the {T} is the last step of the activation, so it is read
    // after the target question has been answered.
    assert!(is_tapped(&engine, scepter), "{{T}} was the cost");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_none(),
        "the named permanent left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "and is in its owner's graveyard, which is the seat that owns it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the ability did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p0, despotic_scepter()).is_some(),
        "the Scepter was not the target and is still standing"
    );
}
