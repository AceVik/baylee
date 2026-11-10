//! `cards/creatures/mv_1/frostling.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Frostling prints one line for one mana: a 1/1 Spirit, and "Sacrifice this
/// creature: It deals 1 damage to target creature." Both halves are the same
/// sentence, so the board plays the whole card: the target has to be picked
/// while the Spirit is still alive (CR 601.2c), the sacrifice is paid the
/// instant that answer lands (CR 601.2h), and the ability still deals its
/// damage with its own source already in a graveyard (CR 608.2b) — which is
/// the half a self-sacrificing cost can get wrong and nothing about the card
/// file would show. Two Elves stand across the table so "the creature it
/// named" is read off a board where the other one could have been the answer.
#[test]
fn frostling_sacrifices_itself_to_deal_one_damage_to_the_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[frostling()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, frostling());
    pass_until(&mut engine, stack_is_empty);
    let spirit = on_battlefield(&engine, p0, frostling()).expect("the Spirit resolved");
    assert_eq!(pt(&engine, spirit), (1, 1), "a printed 1/1 for {{R}}");
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, to tell the aimed one from the other"
    );
    let (prey, bystander) = (elves[0], elves[1]);

    // Ability 0 is the card's whole text: sacrifice the Spirit, then one damage.
    activate(&mut engine, p0, frostling(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated aims it");
    assert!(
        options.contains(&prey) && options.contains(&bystander),
        "\"target creature\" is any creature, either side of the table: {options:?}"
    );

    // CR 601.2c first, then CR 601.2h: answering the target pays the cost,
    // and the cost is the Spirit itself — before anything resolves.
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![prey],
            },
        )
        .expect("the creature the question offered");
    assert!(
        on_battlefield(&engine, p0, frostling()).is_none(),
        "the Spirit paid its own {{Sacrifice this creature}} cost"
    );
    assert!(
        in_graveyard(&engine, p0, frostling()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability is on the stack and has not resolved yet"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "one damage is lethal to the 1/1 it named, even though the Spirit \
         that dealt it is already in a graveyard (CR 608.2b)"
    );
    assert_eq!(
        all_on_battlefield(&engine, p1, llanowar_elves()),
        vec![bystander],
        "and only the creature it was aimed at: one target, one damage"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody pointed at is untouched"
    );
}
