//! `cards/sorceries/mv_4/demolish.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Demolish — {3}{R} sorcery: "Destroy target artifact or land."
///
/// The disjunction is the whole card, so the board carries both halves of it
/// plus the one permanent that is neither: an artifact and a land stand across
/// the table beside an Elf, and the menu has to name the first two while
/// declining the Elf — "target artifact or land" is not "target permanent".
/// Each half is then actually played rather than merely offered, one Demolish
/// eating the Sol Ring and a second eating the Forest, which is what tells a
/// filter that reads `or` from a filter that happens to list both.
///
/// Eight Mountains pay both casts out of one pool, and the target question
/// arrives before the price (CR 601.2c, then CR 601.2h): the four mana the
/// first cast is about to spend is still floating while that question stands.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, and both halves of its disjunction played
fn demolish_destroys_the_artifact_or_land_it_names_and_never_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 8])
        .hand(0, &[demolish(), demolish()])
        .battlefield(1, &[quiet_artifact(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let mine = on_battlefield(&engine, p0, mountain()).expect("my Mountains are out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Mountains are tapped"
    );

    // (1) The artifact half of "target artifact or land".
    cast_from_hand(&mut engine, p0, demolish());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact or land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&rock),
        "an artifact is on the menu, whichever seat controls it: {options:?}"
    );
    assert!(
        options.contains(&land),
        "and a land is too, so both words of the disjunction are read: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "\"artifact or land\" is not \"target permanent\": the Elf across the \
         table is neither and must not be offered: {options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays the {3}{R}, so the eight
    // the Mountains filled are still in the pool while the question stands.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cost is the last step of the cast, not the first"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![rock],
            },
        )
        .expect("the artifact the question offered was chosen");

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "{{3}}{{R}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying a permanent is no mana ability, so the spell is on the stack"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the artifact the spell named left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's graveyard, which is where a destroyed \
         permanent goes"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the land the spell did not name never moved"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "and neither did the creature it could never name"
    );

    // (2) The land half, off the four mana the first cast left floating.
    cast_with_floating(&mut engine, p0, demolish());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target artifact or land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&land),
        "the Forest is still a legal target for the second copy: {options:?}"
    );
    assert!(
        options.contains(&mine),
        "\"a land\" names no side of the table, so my own tapped Mountains are \
         offered beside it: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the second copy reads the same disjunction as the first: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("the land the question offered was chosen");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "the second Demolish destroyed the land it named"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "and the Forest is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "while the Elf watched both casts resolve without ever being offered \
         as a target"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two casts at {{3}}{{R}} apiece out of the eight the Mountains made"
    );
    assert!(
        in_graveyard(&engine, p0, demolish()).is_some(),
        "and both sorceries are where a resolved spell goes"
    );
}
