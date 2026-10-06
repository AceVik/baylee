//! `cards/instants/mv_2/aura_blast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aura Blast costs `{1}{W}` and prints two sentences: "Destroy target
/// enchantment" and "Draw a card."
///
/// Two Plains pay the cost, and the only enchantment in the game is across the
/// table, so the spell has to reach over and take it — while the Sol Ring
/// standing beside it is an artifact and the whole reason the target filter is
/// read rather than skipped. The draw is asserted as a *move* rather than as a
/// question that was asked: the card on top of the library before the cast is
/// the card in hand afterwards, which is the half a destroyed enchantment
/// alone could never prove.
#[test]
fn aura_blast_destroys_an_enchantment_across_the_table_and_draws_a_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[their_enchantment(), quiet_artifact()])
        .hand(0, &[aura_blast()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let theirs = on_battlefield(&engine, p1, their_enchantment()).expect("the enchantment is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("the Sol Ring is out");
    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");

    // Two Plains, tapped before the cast: the card leaves the hand as a
    // payment, and nothing about how it is paid is what this test reads.
    cast_from_hand(&mut engine, p0, aura_blast());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "exactly one enchantment");
    assert!(
        options.contains(&theirs),
        "\"target enchantment\" reaches across the table: {options:?}"
    );
    assert!(
        !options.contains(&rock),
        "an artifact is not an enchantment: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and that one is the whole menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the enchantment the question offered is the one it named");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment was destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and it went to its owner's graveyard, which is the seat that had it"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, aura_blast()).is_some(),
        "and the instant itself is spent: a resolved instant goes to its \
         owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\": one card left the top of the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top),
        "and it is the very card that was on top before the cast, not merely \
         one card fewer in the library"
    );
}
