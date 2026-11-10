//! `cards/artifacts/mv_3/honor_worn_shaku.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Honor-Worn Shaku prints two lines: "{T}: Add {C}." and "Tap an untapped
/// legendary permanent you control: Untap this artifact."
///
/// The board makes each word of the second line do work: tapping the Shaku for
/// {C} first is what leaves it down for the untap to be visible at all, the
/// legendary creature beside it is the only legal price while the Elf beside
/// that one is what a filter missing "legendary" would have offered, and the
/// legendary permanent across the table is what tells "you control" from "a
/// legendary permanent". The refusal of the opponent's creature is the probe
/// the offer cannot give on its own, because tapping is a cost: nothing has
/// moved while the question stands (CR 601.2h), and the untap is on the stack
/// once the answer is in.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn honor_worn_shaku_taps_a_legendary_permanent_of_yours_to_untap_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[honor_worn_shaku(), thrun_the_last_troll(), llanowar_elves()],
        )
        .battlefield(1, &[thrun_the_last_troll()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shaku = on_battlefield(&engine, p0, honor_worn_shaku()).expect("the Shaku is out");
    let mine = on_battlefield(&engine, p0, thrun_the_last_troll()).expect("my Troll is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, thrun_the_last_troll()).expect("their Troll is out");

    // Neither line costs mana, so an empty pool withholds nothing and both are
    // offered the moment the seat has priority.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(shaku, 0)),
        "the printed {{T}}: Add {{C}} is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(shaku, 1)),
        "and so is the untap, whose whole price is somebody else's tap: {:?}",
        legal.abilities
    );

    // Tap the Shaku on its own mana ability and by hand: it is the permanent
    // the second line is about, and it is the one route `tap_all_mana` would
    // have pressed anyway (#159).
    activate(&mut engine, p0, honor_worn_shaku(), 0);
    assert!(
        is_tapped(&engine, shaku),
        "{{T}} is what the mana ability costs"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "and it adds {{C}}, in the pool with no stack used (CR 605.3b)"
    );

    activate(&mut engine, p0, honor_worn_shaku(), 1);
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
            "the tap is a cost and the engine asks which permanent pays it, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(
        prompt,
        ChoicePrompt::CostTap,
        "the variant is what tells a client this is a cost and not an effect"
    );
    assert_eq!((min, max), (1, 1), "one permanent, and the cost asks once");
    assert!(
        options.contains(&mine),
        "a legendary permanent this seat controls is the whole of the answer: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and nothing else on the table is one: {options:?}"
    );
    assert!(
        !options.contains(&elf),
        "the Elf is a permanent you control and no legendary one: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "an opponent's legendary permanent is not yours to tap: {options:?}"
    );
    assert!(
        !options.contains(&shaku),
        "the Shaku is no legendary permanent, and it is already tapped: {options:?}"
    );

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "an answer the question did not enumerate is refused"
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the refusal costs the other seat nothing"
    );
    assert!(
        is_tapped(&engine, shaku),
        "CR 601.2h pays last: while the question stands the Shaku is still down"
    );
    assert!(
        !is_tapped(&engine, mine),
        "and the permanent that will pay is still standing"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the legendary permanent the question offered pays the cost");

    assert!(is_tapped(&engine, mine), "tapping it is the price");
    assert!(
        !stack_is_empty(&engine),
        "untapping is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        !is_tapped(&engine, shaku),
        "the ability untaps the Shaku it was paid for"
    );
    assert!(
        is_tapped(&engine, mine),
        "the permanent that paid stays tapped — the untap reaches the source and no other"
    );
    assert!(!is_tapped(&engine, elf), "nothing else on the board moved");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "the {{C}} the Shaku made is still floating, so the untap cost no mana"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(shaku, 0)),
        "and the standing Shaku has its {{T}} back: {:?}",
        legal.abilities
    );
}
