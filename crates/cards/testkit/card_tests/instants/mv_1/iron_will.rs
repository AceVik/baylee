//! `cards/instants/mv_1/iron_will.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Iron Will is `{W}` for "Target creature gets +0/+4 until end of turn" and
/// cycles for `{2}`. One creature stands under each seat so the pump is read
/// as a target and not as a board buff — `(1, 5)` against `(1, 1)` — and the
/// second copy is cycled out of the same floating mana, which is the only way
/// to see that the other printed line discards *this* card and draws one: the
/// card ends in its owner's graveyard and the library is a card shorter.
#[test]
fn iron_will_pumps_one_creature_and_cycles_its_other_copy_for_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[iron_will(), iron_will()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the spell");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and so is the one across the table"
    );

    // The Elves are named as the printing kept back: `tap_all_mana` presses a
    // creature's own `{T}: Add {G}` too (#159), and the green it would float
    // has nothing to do with the `{W}` this spell costs.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        4,
        "four Plains, four white, and neither Elf tapped for it"
    );

    cast_with_floating(&mut engine, p0, iron_will());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    // CR 601.2c before CR 601.2h: the target is named while the {W} is still
    // in the pool, so the payment below really is the last step of the cast.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        4,
        "targets are chosen before costs are paid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered is a legal target");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        3,
        "the {{W}} came out of the pool"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (1, 5),
        "+0/+4 on the creature the spell named — a (1, 1) would mean the pump \
         never resolved, and a (5, 5) that the power was read too"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the pump reaches the creature it targeted and never across the table"
    );

    // The second printed line: Cycling {2} ({2}, Discard this card: Draw a
    // card). The card is a hand object, so the ability is offered on the card
    // itself and on nothing standing on the battlefield.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, iron_will(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, iron_will()).is_some(),
        "\"Discard this card\": the card goes to its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the discarded card left the hand and a drawn one replaced it"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "and the {{2}} came out of the same pool the {{W}} did"
    );
}
