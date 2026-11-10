//! `cards/creatures/mv_3/arms_dealer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arms Dealer — {2}{R}, a 1/1 Goblin Rogue: "{1}{R}, Sacrifice a Goblin:
/// This creature deals 4 damage to target creature."
///
/// Both halves of the price are the engine's answer rather than the card's,
/// so the board is built to read both. The sacrifice menu has to name the
/// Dealer itself (a Goblin Rogue *is* a Goblin) and the Goblin beside it, and
/// has to decline the Elf ("a Goblin" is read, not skipped) and the Goblin of
/// the same printing across the table (CR 701.21a: a seat sacrifices only what
/// it controls). The {1}{R} is read off the *pool*, which is why the line is
/// not offered at one mana and is there at two — and the four damage is read
/// off the board, where a printed 1/1 dies and the player beside it keeps
/// every point of life.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn arms_dealer_eats_a_goblin_and_deals_four_damage_to_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
                festering_goblin(),
            ],
        )
        .battlefield(1, &[festering_goblin()])
        .hand(0, &[arms_dealer(), mountain()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fodder = on_battlefield(&engine, p0, festering_goblin()).expect("my Goblin is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, festering_goblin()).expect("their Goblin is out");

    // Three Mountains and the Elf are four mana and the Dealer costs three, so
    // one is left — one short of the {1}{R} the ability charges. That is the
    // price read where the engine reads it (`can_afford` looks at the pool and
    // not at the untapped lands), with the Goblin it would eat already there.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "three Mountains and the Elf's own {{G}}"
    );
    cast_with_floating(&mut engine, p0, arms_dealer());
    pass_until(&mut engine, stack_is_empty);
    let dealer = on_battlefield(&engine, p0, arms_dealer()).expect("the Dealer resolved");
    assert_eq!(pt(&engine, dealer), (1, 1), "a printed 1/1 Goblin Rogue");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{R}} came out of the pool"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(dealer, 0)),
        "one mana is not {{1}}{{R}}: the line is not offered at all while a \
         Goblin to eat stands beside it: {:?}",
        legal.abilities
    );

    // The land drop is still open, so the missing mana is one land away — and
    // with it the whole cost, read off the pool again.
    play_land(&mut engine, p0, mountain());
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the Mountain just played, and {{1}}{{R}} is now payable"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(dealer, 0)),
        "with {{1}}{{R}} floating the whole price is payable: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, arms_dealer(), 0);
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
        options.contains(&theirs) && options.contains(&elf),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Goblin across the table was one of the options");

    // CR 601.2c picked the target and CR 601.2h pays afterwards, so the
    // creature is still on the battlefield and the mana still in the pool
    // while the cost question is open.
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "the cost is paid after the target, not before it"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{1}}{{R}} is still floating"
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
        panic!(
            "the sacrifice asks which Goblin, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one Goblin, no more and no fewer");
    assert!(
        options.contains(&dealer),
        "the Dealer is a Goblin Rogue, so it is on its own menu: {options:?}"
    );
    assert!(
        options.contains(&fodder),
        "and so is the Goblin beside it: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "an Elf is no Goblin: \"a Goblin\" is read, not skipped: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "CR 701.21a: an opponent's Goblin is not yours to sacrifice: {options:?}"
    );
    assert_eq!(options.len(), 2, "and those two are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the Goblin the question offered pays the cost");

    // The sacrificed Goblin's own dies trigger goes on the stack above the
    // ability (CR 603.3b) and asks for a target of its own; it is aimed at the
    // Elf so that nothing but the four damage can account for the Goblin
    // across the table.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the controller of the dead Goblin chooses");
    assert!(
        options.contains(&elf),
        "any creature is a target for `-1/-1`: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options it enumerated");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, arms_dealer()).is_some(),
        "the Dealer paid with a Goblin and not with itself"
    );
    assert!(
        in_graveyard(&engine, p0, festering_goblin()).is_some(),
        "the Goblin it ate is in its owner's graveyard, not merely gone"
    );
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_none(),
        "four damage on a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        in_graveyard(&engine, p1, festering_goblin()).is_some(),
        "and the card is in the graveyard of the seat that owned it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"target creature\": the damage went to the creature and never to the player"
    );
}
