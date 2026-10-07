//! `cards/instants/mv_1/unsummon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unsummon — {U} instant: "Return target creature to its owner's hand."
///
/// Two printed words carry the card, and each gets its own witness. "target
/// creature" is read off what the engine offers: three creatures stand on the
/// table — mine and both of the opponent's — and a filter that had narrowed to
/// one side, or to something that is no creature, could not list three. "its
/// owner's hand" is read by aiming it at the *opponent's* Elf, so the card has
/// to arrive in p1's hand and nowhere else, which a test that only watched the
/// creature leave the battlefield could never tell.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn unsummon_returns_the_creature_it_names_to_its_owners_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(917, island())
        .battlefield(0, &[island(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .hand(0, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(theirs.len(), 2, "two Elves stand across the table");
    let (target, bystander) = (theirs[0], theirs[1]);

    // `legal.castable` is filtered through `can_afford`, which reads the mana
    // *pool* and not the untapped lands, so the mana is made first. The Island
    // taps for {U} and the Elf beside it prints its own `{T}: Add {G}`, which
    // is a route whose whole price is its own tap (#159) — so two routes is
    // every source on this board and the pool has to say two.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 2,
        "the Island and the Elf, and nothing else makes mana"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one blue, which is exactly what {{U}} costs"
    );
    assert_eq!(
        pool.total(),
        2,
        "and the Elves' own {{G}} is beside it, so both sources are counted"
    );

    cast_with_floating(&mut engine, p0, unsummon());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the seat that cast the spell aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        options.len(),
        3,
        "\"target creature\" is every creature in the game: {options:?}"
    );
    assert!(
        options.contains(&mine) && options.contains(&target) && options.contains(&bystander),
        "both sides of the table are on the menu: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "a creature is no player, so there is no face to choose: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the Elf the question enumerated is a legal answer");

    // CR 601.2c chose the target and CR 601.2h paid afterwards, so the mana is
    // gone now and the spell is what is waiting.
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the {{U}} came out of the pool to pay for the spell"
    );
    assert_eq!(
        pool.total(),
        1,
        "and the green the Elves made is all that is left of the two"
    );
    assert!(
        !stack_is_empty(&engine),
        "returning a creature is no mana ability, so the spell waits on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p1, llanowar_elves()),
        vec![bystander],
        "only the creature that was named left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, llanowar_elves()).is_some(),
        "\"to its owner's hand\": the Elf is in p1's hand, the seat that owns it"
    );
    assert_eq!(
        engine
            .state()
            .object(target)
            .expect("the bounced Elf is still an object")
            .zone,
        crate::zone::Zone::Hand,
        "and the very card that was aimed at is the one that moved to a hand"
    );
    assert!(
        in_hand(&engine, p0, llanowar_elves()).is_none(),
        "and never to the hand of the seat that cast the spell"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature nobody named never moved"
    );
    assert!(
        in_graveyard(&engine, p0, unsummon()).is_some(),
        "the spell itself went to its owner's graveyard once it resolved"
    );
}
