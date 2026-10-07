//! `cards/artifacts/mv_2/thopter_foundry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Thopter Foundry — {W/B}{U} artifact: "{1}, Sacrifice a nontoken artifact:
/// Create a 1/1 blue Thopter artifact creature token with flying. You gain 1 life."
///
/// Creating the 1/1 blue Thopter token is the `Coverage::Partial` gap because the
/// token pool lacks a definition for it. This scenario tests the implemented half:
/// paying {1} and sacrificing another nontoken artifact puts the activated
/// ability on the stack, and upon resolution the controller gains 1 life without
/// generating a token.
#[test]
fn thopter_foundry_sacrifices_an_artifact_for_one_mana_and_gains_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[thopter_foundry(), quiet_artifact(), forest()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let foundry = on_battlefield(&engine, p0, thopter_foundry()).expect("foundry is on the table");
    let fodder = on_battlefield(&engine, p0, quiet_artifact()).expect("the fodder artifact is out");
    let land = on_battlefield(&engine, p0, forest()).expect("forest is on the table");

    // Only the Forest: the Sol Ring is the fodder this ability sacrifices,
    // and a {1} paid out of its own {C}{C} would prove nothing about the {1}.
    tap_all_mana_but(&mut engine, p0, Some(quiet_artifact()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana floating from the forest"
    );

    activate(&mut engine, p0, thopter_foundry(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the activation cost asks which artifact to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "sacrifice prompt indicates cost payment"
    );
    assert_eq!((min, max), (1, 1), "sacrifice exactly one artifact");
    assert!(
        options.contains(&fodder),
        "the other artifact is a nontoken artifact"
    );
    assert!(
        options.contains(&foundry),
        "Thopter Foundry itself is a nontoken artifact"
    );
    assert!(!options.contains(&land), "a basic land is not an artifact");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("sacrificing the other artifact pays the cost");

    assert!(
        in_graveyard(&engine, p0, quiet_artifact()).is_some(),
        "the sacrificed artifact was moved to the graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}} mana cost was consumed from the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "the activated ability is now on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        21,
        "controller gained 1 life upon resolution"
    );
    // This read `is_empty()` and said "due to Coverage::Partial gap" — a test
    // pinning a limitation, which is the right thing to write while the
    // limitation is real and the wrong thing to leave behind once it is not.
    // The token ledger now holds a 1/1 blue Thopter with flying, read out of
    // this very card's own reference script.
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Thopter");
    let thopter = engine
        .state()
        .object(tokens[0])
        .expect("the Thopter is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(thopter.name, "Thopter");
    assert_eq!((thopter.power, thopter.toughness), (Some(1), Some(1)));
    assert!(
        thopter.colors.contains(baylee_core::color::Color::Blue),
        "a 1/1 *blue* Thopter"
    );
    assert!(
        thopter
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLYING),
        "with flying"
    );
    assert!(
        on_battlefield(&engine, p0, thopter_foundry()).is_some(),
        "Thopter Foundry remains on the battlefield"
    );
}
