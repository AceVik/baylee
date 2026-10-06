//! `cards/enchantments/mv_2/sustenance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sustenance is `{1}{G}` and prints exactly one line: "{1}, Sacrifice a
/// land: Target creature gets +1/+1 until end of turn." Three things in that
/// sentence are the engine's answer rather than the card's, so the board is
/// built to read each of them. The `{1}` is filtered out of the offer until
/// the pool holds it (`can_afford` reads the pool, not the untapped lands);
/// the second price asks *which* land and may offer only this seat's own,
/// with the Forest across the table as the control; and the pump has to land
/// on the creature that was named and on no other, which is why an Elf of
/// mine and an Elf of theirs are both standing when the target question is
/// asked and their Elf is still a printed 1/1 afterwards.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn sustenance_trades_a_land_for_one_more_power_on_the_creature_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(421, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), sustenance(), llanowar_elves()],
        )
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let enchantment =
        on_battlefield(&engine, p0, sustenance()).expect("the enchantment is on the table");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // `{1}` is read off the pool and not off the untapped lands, so with
    // nothing floating the whole price is unpayable and the line is not there
    // at all — the half a test that only ever taps first would never see.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the enchantment holds it");
    assert!(
        !legal.abilities.contains(&(enchantment, 0)),
        "{{1}} is not one mana, so nothing is offered: {:?}",
        legal.abilities
    );

    // Three Forests, and the Elves named as the printing kept back: they are
    // the creature the pump is about, and a host tapped for its own mana reads
    // wrong afterwards. Rule 17 in one line — the card's own price is not its
    // own {{T}}, so `tap_all_mana_but` is what keeps the creature standing.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests, and no Elf of mine contributed"
    );
    let lands = all_on_battlefield(&engine, p0, forest());
    assert_eq!(lands.len(), 3, "three Forests of my own to give up");
    let doomed = lands[0];

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(enchantment, 0)),
        "with the {{1}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sustenance(), 0);

    // CR 601.2c before CR 601.2h: the target is named while the mana is still
    // in the pool and every Forest still standing, so both prices are read
    // after this answer.
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&enchantment),
        "the enchantment is no creature: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");

    assert!(
        on_battlefield(&engine, p0, forest()).is_some(),
        "targets are chosen before costs are paid (CR 601.2c, then CR 601.2h), \
         so no land has been given up yet"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "and the {{1}} is still in the pool for the same reason"
    );

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    assert!(
        options.contains(&doomed),
        "a Forest this seat controls is on the menu: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "the three Forests of mine and nothing else: {options:?}"
    );
    assert!(
        !options.contains(&enchantment),
        "the enchantment is no land, so it cannot pay its own price: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "and the Elves are a creature: \"a land\" is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&their_land),
        "`CR 701.21a`: an opponent's land is not yours to sacrifice, whatever \
         the filter says: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the land the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}} came out of the pool the three Forests filled"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        2,
        "exactly one Forest was given up: the other two are still standing"
    );
    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "a sacrificed land goes to its owner's graveyard"
    );
    assert_eq!(
        pt(&engine, mine),
        (2, 2),
        "+1/+1 until end of turn on the creature that was named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for the creature the ability did not target"
    );
    assert!(
        on_battlefield(&engine, p0, sustenance()).is_some(),
        "an activated ability costs the enchantment nothing but what it prints"
    );
}
