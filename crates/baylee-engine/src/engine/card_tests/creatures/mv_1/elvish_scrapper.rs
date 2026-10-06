//! `cards/creatures/mv_1/elvish_scrapper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elvish Scrapper — {G}, a 1/1 Elf — prints one line: "{G}, {T}, Sacrifice
/// this creature: Destroy target artifact." Every part of that price is
/// named, so one activation is the whole card: the green is read out of the
/// pool, the creature out of its owner's graveyard, and the artifact off the
/// battlefield.
///
/// The second Sol Ring across the table is the counter-half. "Target
/// artifact" is one artifact on either side of the table — an effect that
/// had lost its targeting while keeping the filter would have taken both,
/// which is the shape Liquimetal Coating shipped in.
///
/// The target is asked for before the price is paid (CR 601.2c, then
/// CR 601.2h), so while the question stands the Scrapper is still standing
/// untapped and the {G} is still in the pool; the cost assertions belong
/// behind the answer.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn elvish_scrapper_trades_a_green_a_tap_and_itself_for_one_artifact() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), elvish_scrapper()])
        // Two artifacts, so the ability has one to destroy and one to leave
        // alone.
        .battlefield(1, &[quiet_artifact(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    // A turn cycle first: the Scrapper pays its own {T}, and a creature may
    // not tap for an ability unless it has been under its controller's
    // control since that player's turn began (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    let scrapper = on_battlefield(&engine, p0, elvish_scrapper()).expect("the Scrapper is out");
    let artifacts = all_on_battlefield(&engine, p1, quiet_artifact());
    assert_eq!(artifacts.len(), 2, "two artifacts across the table");
    let (victim, bystander) = (artifacts[0], artifacts[1]);

    // Mana into the pool before the offer is read (#159): the Forest is the
    // only source on this board.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one green, off the Forest"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(scrapper, 0)),
        "a green in the pool, an untapped Scrapper and something to destroy is \
         the whole price of the one line the card prints: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, elvish_scrapper(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "destroying an artifact is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat chooses the target");
    assert_eq!((min, max), (1, 1), "one artifact, no more and no fewer");
    assert!(
        options.contains(&victim) && options.contains(&bystander),
        "\"target artifact\" is any artifact, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and the two across the table are the whole menu: {options:?}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "CR 601.2c before CR 601.2h: the green is not spent while the target is \
         still being chosen"
    );
    assert!(
        on_battlefield(&engine, p0, elvish_scrapper()).is_some(),
        "and the creature the price will sacrifice is still on the battlefield"
    );
    assert!(
        !is_tapped(&engine, scrapper),
        "still untapped, too: the {{T}} is part of the same last step"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the artifact the question offered is a legal answer");

    // The price lands with the answer.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} came out of the pool, and nothing else was in it"
    );
    assert!(
        in_graveyard(&engine, p0, elvish_scrapper()).is_some(),
        "\"sacrifice this creature\" is the last part of the cost, so the \
         Scrapper is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, elvish_scrapper()).is_none(),
        "and off the battlefield"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "the artifact that was named is destroyed"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&bystander),
        "the artifact the ability did not name is untouched: one target, one \
         destruction"
    );
}
