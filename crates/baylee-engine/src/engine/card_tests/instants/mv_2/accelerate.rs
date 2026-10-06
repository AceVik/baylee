//! `cards/instants/mv_2/accelerate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Accelerate — {1}{R} instant: "Target creature gains haste until end of
/// turn. Draw a card."
///
/// The two printed sentences are one card, and the board makes each of them
/// separately readable: "target creature" is any creature and not "you
/// control", so the Elf across the table is on the menu while the Elf that is
/// named is the only one of the three to end up with haste, and the draw is
/// read off the library where a card that only pumped could not show it. The
/// opponent's turn is walked to afterwards because the keyword is "until end
/// of turn" — reading haste in the turn it was granted cannot tell a duration
/// from a static.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn accelerate_grants_haste_to_the_creature_it_targets_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), llanowar_elves(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[accelerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    for id in [host, bystander, theirs] {
        assert!(
            !keywords(&engine, id).contains(KeywordSet::HASTE),
            "a printed Llanowar Elves has no haste of its own"
        );
    }

    // Both Mountains, and the Elves named as the printing kept back: they are
    // the creatures the spell is about, and a mana creature tapped for the mana
    // would leave a board this test no longer reads.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Mountains, and nothing off either Elf"
    );
    let card = in_hand(&engine, p0, accelerate()).expect("the spell is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with {{1}}{{R}} in the pool the spell is castable: {:?}",
        legal.castable
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_with_floating(&mut engine, p0, accelerate());

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "exactly one creature, no more and no fewer"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "no player is a creature: {player_options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "CR 601.2c before CR 601.2h: the target is chosen while the mana is \
         still floating"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, host).contains(KeywordSet::HASTE),
        "\"target creature gains haste until end of turn\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::HASTE),
        "the Elf nobody named is untouched: the pump reaches the target and \
         no other"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "nor across the table, where the same Elf was on the menu and was not \
         named"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool"
    );
    assert!(
        in_graveyard(&engine, p0, accelerate()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card left the hand with the spell and one came back with the draw"
    );

    // "Until end of turn" is a duration and not a static, which the turn it
    // was cast in cannot tell: the keyword is read again on the opponent's
    // turn, past the cleanup step that ends it.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, host).contains(KeywordSet::HASTE),
        "the granted haste expired with the turn it was granted in"
    );
}
