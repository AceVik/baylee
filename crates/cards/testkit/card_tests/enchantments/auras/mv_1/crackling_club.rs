//! `cards/enchantments/auras/mv_1/crackling_club.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Crackling Club prints three sentences whose order is what makes them
/// readable: an Aura enchanting a creature (any creature — the Elf across the
/// table is offered as readily as my own), "+1/+0" on the creature it *holds*
/// and no other, and "Sacrifice this Aura: It deals 1 damage to target
/// creature." The pump is read on both sides of the sacrifice, so the static
/// is shown to hang on the Aura's presence rather than on the creature; and
/// the sacrifice is read as a cost, which is why the target question
/// (CR 601.2c) arrives with the Aura still on the battlefield and the payment
/// (CR 601.2h) not yet made.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn crackling_club_pumps_the_creature_it_enchants_then_trades_itself_for_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), llanowar_elves()])
        .hand(0, &[crackling_club()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // {R} off the Mountain alone: the Elf is kept untapped because it is the
    // creature the Aura is about to hold, not a mana source on the board.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, crackling_club());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "an Aura may enchant any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered");
    pass_until(&mut engine, stack_is_empty);

    let club = on_battlefield(&engine, p0, crackling_club()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(club).and_then(|o| o.attached_to),
        Some(host),
        "and landed on the creature it was cast at"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 1),
        "\"enchanted creature gets +1/+0\" — the point of power and no point of toughness"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the creature the Aura holds and never across the table"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    let mut offered = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == club);
    let (source, ability_index) = offered
        .next()
        .expect("with a creature on the table the sacrifice is offered");
    assert!(
        offered.next().is_none(),
        "and it is the Aura's only activated ability: the grant is a static"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the price is the Aura itself: nothing is floating to pay with"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("a cost of the Aura's own body is one it can always pay");

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&theirs) && options.contains(&host),
        "one damage may be aimed at any creature, its own host included: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, crackling_club()).is_some(),
        "CR 601.2c comes before CR 601.2h: the sacrifice is not paid while \
         the target is being chosen"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 1),
        "so the pump is still standing at the moment the question is asked"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the creature the question offered");

    assert!(
        on_battlefield(&engine, p0, crackling_club()).is_none(),
        "answering the target pays the cost, and the cost is the Aura"
    );
    assert!(
        in_graveyard(&engine, p0, crackling_club()).is_some(),
        "an Aura that sacrifices itself goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "and the damage is on the stack behind the price that bought it"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage to a 1/1 is CR 704.5g, and the Elf across the table is gone"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the Aura was holding is untouched"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "and it is a 1/1 again: the +1/+0 died with the Aura that granted it"
    );
}
