//! `cards/instants/mv_2/mage_s_guile.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mage's Guile prints two lines: "{1}{U}: Target creature gains shroud until
/// end of turn" and "Cycling {U}". The shroud half is read twice, because a
/// projected keyword is only half the sentence — "it can't be the target of
/// spells or abilities" is a targeting restriction, so the same creature has
/// to be missing from the menu of an ability that could otherwise equip it,
/// while the Elf beside it stays offered. The cycling half is then paid out of
/// hand, which is a cost of its own: the card discards itself and the draw
/// replaces it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn mages_guile_shrouds_the_creature_it_names_and_cycles_itself_away() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(311, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                lightning_greaves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[mage_s_guile(), mage_s_guile()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which is the control");
    let (host, bystander) = (elves[0], elves[1]);
    let greaves = on_battlefield(&engine, p0, lightning_greaves()).expect("the Greaves are out");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::SHROUD),
        "a printed 1/1 has no shroud before the spell"
    );

    // Three Islands and both Elves are five mana, and a pool survives until the
    // step ends (CR 500.5) — one main phase pays `{1}{U}` now and the cycling
    // `{U}` afterwards.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "three Islands and both Elves tapped for it"
    );

    cast_with_floating(&mut engine, p0, mage_s_guile());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat aims it");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both Elves are creatures it may name: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::SHROUD),
        "\"target creature gains shroud\" — the creature it named"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::SHROUD),
        "and the Elf standing beside it gains nothing"
    );

    // The grant is a restriction as well as a keyword, and `Equip {0}` is the
    // cheapest question that can see it: it costs no mana at all, so the offer
    // below turns on the shroud and on nothing else.
    activate(&mut engine, p0, lightning_greaves(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !options.contains(&host),
        "\"it can't be the target of spells or abilities\": {options:?}"
    );
    assert!(
        options.contains(&bystander),
        "while the creature without shroud is still offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bystander],
            },
        )
        .expect("the creature the equip question offered");
    pass_until(&mut engine, |e| {
        e.state()
            .object(greaves)
            .is_some_and(|o| o.attached_to == Some(bystander))
    });

    // Cycling {U} (CR 702.29a): a second copy is still in hand, and both the
    // mana and the card itself are the price.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let cycler = in_hand(&engine, p0, mage_s_guile()).expect("the second copy is still in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cycler, 0)),
        "cycling is an activated ability of the card in hand: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, mage_s_guile(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        mine(&engine, p0, mage_s_guile(), Zone::Graveyard).len(),
        2,
        "the resolved copy and the cycled copy are both in the graveyard — \
         discarding the card is the cost, not a rider"
    );
    assert!(
        in_hand(&engine, p0, mage_s_guile()).is_none(),
        "and no copy of it is left in hand"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"draw a card\": one card left the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one discarded and one drawn leave the hand the size it was, so an \
         empty library would not satisfy the count above"
    );
}
