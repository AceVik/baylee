//! `cards/creatures/mv_4/aven_trooper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aven Trooper — {3}{W} — a 1/1 Bird Soldier with flying and
/// "{2}{W}, Discard a card: This creature gets +1/+2 until end of turn."
///
/// Both printed lines are played off one board: seven tapped Plains pay the
/// four for the body and leave exactly the three the ability charges, so the
/// pump is pressed on real mana and not on a label, and the discard arrives
/// as a `CostDiscard` question over the hand. The `(2, 3)` is read on the
/// Trooper alone — an Elf beside it and the Elf across the table stay
/// printed 1/1s — and the discarded card and the spent mana are read in the
/// zones they went to.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn aven_trooper_discards_a_card_to_grow_itself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[aven_trooper(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Every source but the two Elves is tapped for the body: four Plains pay
    // {3}{W} and the other three leave precisely the {2}{W} the ability
    // charges. The mana is read before the claim, because `can_afford` reads
    // the pool and not the untapped lands.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven Plains, and neither Elf paid into the pool"
    );
    cast_with_floating(&mut engine, p0, aven_trooper());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, aven_trooper()).is_some()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let trooper = on_battlefield(&engine, p0, aven_trooper()).expect("the Trooper resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(
        pt(&engine, trooper),
        (1, 1),
        "a printed 1/1 before the pump"
    );
    assert!(
        keywords(&engine, trooper).contains(KeywordSet::FLYING),
        "the card's other line is flying"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "seven Plains less the four the body cost: exactly what the ability charges"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(trooper, 0)),
        "with {{2}}{{W}} floating the ability is offered: {:?}",
        legal.abilities
    );

    let hand_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();

    activate(&mut engine, p0, aven_trooper(), 0);
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
            "the discard is paid as the activation's last step: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat pays its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostDiscard,
        "a cost and not a search, which is all a client has to tell them apart"
    );
    assert_eq!((min, max), (1, 1), "one card, no more and no fewer");
    assert!(!options.is_empty(), "there are cards in hand to discard");
    for id in &options {
        assert!(
            hand_before.contains(id),
            "`Discard a card` is asked out of the hand: {options:?}"
        );
    }
    assert!(
        !options.contains(&trooper),
        "the creature paying the cost is on the battlefield, not in hand: {options:?}"
    );

    let discarded = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![discarded],
            },
        )
        .expect("a card the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, trooper),
        (2, 3),
        "{{2}}{{W}} and a card for +1/+2 until end of turn"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "\"This creature\" is the Trooper, not the board it stands on"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the pump reaches nothing across the table"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&discarded),
        "the discarded card is in its owner's graveyard"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&discarded),
        "and it left the hand, so it was paid and not merely shown"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{W}} came out of the pool"
    );
    assert!(
        keywords(&engine, trooper).contains(KeywordSet::FLYING),
        "the pump is until end of turn and takes nothing else away"
    );
}
