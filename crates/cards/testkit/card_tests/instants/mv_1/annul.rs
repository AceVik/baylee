//! `cards/instants/mv_1/annul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Annul costs `{U}` and prints a single line: "Counter target artifact
/// or enchantment spell". Both halves of this filter are played in *the same*
/// first main phase of the opponent — first a Sol Ring spell,
/// then an Exploration —, and because CR 500.5 empties the mana pool only at
/// the end of the step, two tapped Islands pay both `{U}`. The two
/// target lists are the actual proof: an ability without a matching
/// filter could not restrict the stack to exactly one spell, and
/// a countered spell that nevertheless stood on the battlefield would not be
/// a counter.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn annul_counters_an_artifact_spell_and_an_enchantment_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[annul(), annul()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[quiet_artifact(), exploration()])
        .start();
    keep_mulligans(&mut engine);

    // The opponent casts on their own Main: two Forests, two green mana.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "two Forests, two green mana"
    );
    cast_with_floating(&mut engine, p1, quiet_artifact());
    let ring = on_stack(&engine, quiet_artifact()).expect("der Sol Ring liegt auf dem Stapel");

    // p0 responds and taps both Islands at once: the pool outlasts
    // the resolution of the first Annul and also pays for the second.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands, two blue mana"
    );

    cast_with_floating(&mut engine, p0, annul());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("Annul targets a spell, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat chooses the target");
    assert_eq!(
        options,
        vec![ring],
        "ein Artefaktspruch und sonst nichts auf dem Stapel"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the Sol Ring spell was one of the offered options");

    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "a countered spell goes to its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and never onto the battlefield"
    );

    // Die andere Hälfte des gedruckten Filters: eine Verzauberung.
    cast_with_floating(&mut engine, p1, exploration());
    let enchantment = on_stack(&engine, exploration()).expect("Exploration liegt auf dem Stapel");
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_with_floating(&mut engine, p0, annul());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an enchantment is also a target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![enchantment],
        "ein Verzauberungsspruch und sonst nichts auf dem Stapel"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![enchantment],
            },
        )
        .expect("the Exploration spell was one of the offered options");

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, exploration()).is_some(),
        "auch die Verzauberung wurde gekontert"
    );
    assert!(
        on_battlefield(&engine, p1, exploration()).is_none(),
        "and never reached the battlefield"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "both Annuls resolved and landed in their graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the two Islands both paid {{U}}"
    );
}
