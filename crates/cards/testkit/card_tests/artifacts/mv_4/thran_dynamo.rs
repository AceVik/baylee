//! `cards/artifacts/mv_4/thran_dynamo.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "a699c663-8131-4045-9265-a83e86609374"

#[test]
fn thran_dynamo_taps_for_three_colorless_without_asking_a_colour() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[thran_dynamo()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Four Forests are exactly the {4}, so the pool is empty the moment the
    // artifact lands and whatever is in it below came off the Dynamo.
    cast_from_hand(&mut engine, p0, thran_dynamo());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let dynamo = on_battlefield(&engine, p0, thran_dynamo()).expect("the Dynamo resolved");
    assert!(
        types(&engine, dynamo).contains(TypeSet::ARTIFACT),
        "and what arrived is the artifact it prints"
    );
    assert!(!is_tapped(&engine, dynamo), "it enters untapped and ready");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Forests paid the {{4}} to the last mana"
    );

    // The whole price is the tap symbol, so the line is offered on an empty
    // pool even though `can_afford` reads the pool and not the untapped lands.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(dynamo, 0)),
        "an untapped artifact is a paid {{T}}, so the one line the card prints \
         is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, thran_dynamo(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the card names its mana, so nothing is asked on the way (CR 605.1), \
         got {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        3,
        "{{T}}: Add {{C}}{{C}}{{C}} — three, off one tap"
    );
    assert_eq!(pool.total(), 3, "and nothing else came with them");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "every Forest on this board is tapped and makes green besides, so the \
         colourless has no other source on it"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is here at once"
    );
    assert!(is_tapped(&engine, dynamo), "the Dynamo paid its own {{T}}");
}
