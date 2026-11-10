//! `cards/creatures/mv_3/temporal_adept.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Temporal Adept — {1}{U}{U} 1/1 Human Wizard: "{U}{U}{U}, {T}: Return
/// target permanent to its owner's hand."
///
/// "Target permanent" is the widest target any card prints, and the only way
/// to read it is to offer it a board with a permanent on each side and of
/// each kind: the opponent's Sol Ring and Forest are on the menu and so is
/// this seat's own Island, which is what tells `Filter::Any` from a filter
/// quietly narrowed to one side. The Sol Ring is the target, because the
/// destination is half the sentence — it has to reach the hand of the seat
/// that *owns* it and not the seat that aimed the bounce — and the Adept's
/// own {T} plus the three blue out of the pool are read after the answer,
/// since CR 601.2c names the target before CR 601.2h pays.
#[test]
fn temporal_adept_bounces_any_permanent_to_its_owners_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[temporal_adept(), island(), island(), island()])
        .battlefield(1, &[quiet_artifact(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let adept = on_battlefield(&engine, p0, temporal_adept()).expect("the Adept is out");
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let mine = on_battlefield(&engine, p0, island()).expect("my Island is out");
    assert!(!is_tapped(&engine, adept), "it starts untapped");

    // {U}{U}{U} is read off the pool and not off the untapped lands, so the
    // mana comes first: three Islands, three blue, and the Adept untouched.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, and the Adept prints no mana of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(adept, 0)),
        "with {{U}}{{U}}{{U}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, temporal_adept(), 0);
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
        )
    };
    assert_eq!(player, p0, "the activating seat chooses");
    assert_eq!((min, max), (1, 1), "exactly one permanent");
    assert!(
        options.contains(&ring) && options.contains(&their_land),
        "a permanent across the table is a permanent: {options:?}"
    );
    assert!(
        options.contains(&mine),
        "and so is one this seat controls — \"target permanent\" reads neither \
         side off the board: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the target is named before the cost is paid (CR 601.2c, then CR 601.2h)"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the artifact across the table was one of the options");

    assert!(
        is_tapped(&engine, adept),
        "{{T}} is half the cost, paid by the Adept itself"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{U}}{{U}}{{U}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "bouncing is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the targeted permanent left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, quiet_artifact()).is_some(),
        "\"to its owner's hand\": the Sol Ring goes back to the seat that owns \
         it, and not to the seat that aimed the bounce"
    );
    assert!(
        in_hand(&engine, p0, quiet_artifact()).is_none(),
        "the bouncing player's hand is where it did not go"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "and only the permanent that was named: the Forest never moved"
    );
    assert!(
        on_battlefield(&engine, p0, island()).is_some(),
        "nor the Island under the Adept's own controller"
    );
}
