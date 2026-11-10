//! `cards/creatures/mv_1/shield_mate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shield Mate — {W}, 1/1 Human Soldier: "Sacrifice this creature: Target
/// creature gets +0/+4 until end of turn."
///
/// The ability's whole price is the creature printing it, and CR 601.2c puts
/// the target before CR 601.2h pays the cost — so the question is answered
/// while the Shield Mate still stands and is one of its own legal targets.
/// The pump is then read off two creatures at once: the one the ability named
/// and the printed 1/1 across the table it did not.
#[test]
fn shield_mate_trades_itself_for_four_toughness_on_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves()])
        .hand(0, &[shield_mate()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, shield_mate());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let mate = on_battlefield(&engine, p0, shield_mate()).expect("the Shield Mate resolved");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");

    // Ability 0 is the only line the card prints, and its price is no mana.
    activate(&mut engine, p0, shield_mate(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        options.contains(&mate),
        "and the Shield Mate is one of them: CR 601.2c picks the target before \
         CR 601.2h pays a cost that removes the source: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, shield_mate()).is_some(),
        "so the price has not been paid while the question is still open"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .unwrap();
    assert!(
        in_graveyard(&engine, p0, shield_mate()).is_some(),
        "CR 601.2h pays the cost as the target answer lands: the Shield Mate is \
         already gone while the ability is on the stack"
    );
    assert!(!stack_is_empty(&engine), "and the ability is waiting there");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (1, 5),
        "+0/+4 on the creature the ability named"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature it did not"
    );
    assert!(
        on_battlefield(&engine, p0, shield_mate()).is_none(),
        "the Shield Mate was the cost, so it left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, shield_mate()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
}
