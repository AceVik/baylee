//! `cards/creatures/mv_3/balloon_peddler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Balloon Peddler — {2}{U}, a 2/2 Human Spellshaper: "{U}, {T}, Discard a
/// card: Target creature gains flying until end of turn."
///
/// Three prices ride on one activation and each is read somewhere else: the
/// {U} out of a pool only the Islands paid into, the {T} off the Peddler's own
/// status, and "Discard a card" as a `CostDiscard` question whose answer ends
/// up in its owner's graveyard. A creature that arrived this turn cannot pay
/// its own {T} (CR 302.6), so the Peddler is cast on turn one and the ability
/// taken on the next — where the target question stands *before* any of the
/// three prices is asked (CR 601.2c, then 601.2h) and offers both Elves, one
/// on each side of the table, because "target creature" is not "target
/// creature you control".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn balloon_peddler_pays_a_blue_a_tap_and_a_card_to_grant_flying() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), llanowar_elves()])
        .hand(0, &[balloon_peddler(), counterspell()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{U} off the three Islands, with the Elf named as the printing kept
    // back: it is a creature this test reads again below, and a source tapped
    // for mana has already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        3,
        "three Islands, and the Elf beside them paid nothing"
    );
    cast_with_floating(&mut engine, p0, balloon_peddler());
    pass_until(&mut engine, stack_is_empty);
    let peddler = on_battlefield(&engine, p0, balloon_peddler()).expect("the Peddler resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, peddler), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and {{2}}{{U}} took the whole pool, so nothing is floating"
    );

    // CR 302.6: a creature that came down this turn cannot pay its own {T}.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, peddler),
        "a turn later the Peddler is standing, and its {{T}} is finally its own"
    );

    let fodder = in_hand(&engine, p0, counterspell()).expect("the card to pitch is still in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(peddler, 0)),
        "with {{U}} floating, a card in hand and the Peddler untapped, the one \
         line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, balloon_peddler(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        3,
        "CR 601.2c before CR 601.2h: nothing is paid while the target is chosen"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");

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
            "the cost asks which card to discard, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostDiscard,
        "a cost and not a cleanup, which is all a client has to tell the two apart"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "\"Discard a card\" — one card, no fewer"
    );
    assert!(
        options.contains(&fodder),
        "a card in hand is the price: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the card the question offered is the price");

    assert!(is_tapped(&engine, peddler), "{{T}} was the second price");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2,
        "and exactly one of the three blue paid the {{U}}"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "the discarded card left the hand"
    );
    assert!(
        in_graveyard(&engine, p0, counterspell()).is_some(),
        "and it is in its owner's graveyard, which is where a discarded card goes"
    );
    assert!(
        !stack_is_empty(&engine),
        "granting flying is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, mine).contains(KeywordSet::FLYING),
        "\"target creature gains flying until end of turn\""
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and nothing else about it changed: the pump is +0/+0 and the keyword is the card"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "the grant reaches the creature that was named and never across the table"
    );
    assert!(
        !keywords(&engine, peddler).contains(KeywordSet::FLYING),
        "nor the Spellshaper that paid for it"
    );
}
