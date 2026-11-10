//! `cards/artifacts/mv_3/nuisance_engine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nuisance Engine — {3} artifact: "{2}, {T}: Create a 0/1 colorless Pest
/// artifact creature token." Both halves of the price have to be read in one
/// activation, so five Forests pay the {3} and leave exactly the {2} the
/// ability charges — read off the pool, because `legal.abilities` is filtered
/// through `can_afford` and that reads the pool rather than the untapped
/// lands. The token itself is asserted only after the stack has emptied,
/// since making a token is no mana ability: the artifact tapped, the two mana
/// gone and no Pest anywhere while the question is up is what says the token
/// is the resolution and not the cost.
#[test]
fn nuisance_engine_taps_and_two_mana_for_a_zero_one_pest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(413, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[nuisance_engine()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Forests: {3} brings the artifact to the table and the {2} the ability
    // charges is what the same pool has left beside it — a pool survives until
    // the step ends (CR 500.5) and this all happens inside one main phase.
    cast_from_hand(&mut engine, p0, nuisance_engine());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let machine = on_battlefield(&engine, p0, nuisance_engine()).expect("the Engine resolved");
    assert!(!is_tapped(&engine, machine), "it enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{3}} is spent and exactly the {{2}} the ability charges is left"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so nothing has been made"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(machine, 0)),
        "with {{2}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, nuisance_engine(), 0);
    assert!(
        is_tapped(&engine, machine),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the two mana that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Pest arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Pest");
    let pest = tokens[0];
    let kinds = types(&engine, pest);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "\"artifact creature token\": {kinds:?}"
    );
    assert_eq!(pt(&engine, pest), (0, 1), "the printed 0/1 body");
    let printed = engine
        .state()
        .object(pest)
        .expect("the Pest is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Pest");
    assert!(
        on_battlefield(&engine, p0, nuisance_engine()).is_some(),
        "the {{2}} and the tap were the whole price, so the Engine stays to make \
         another one"
    );
}
