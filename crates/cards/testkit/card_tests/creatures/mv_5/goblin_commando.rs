//! `cards/creatures/mv_5/goblin_commando.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "76c02534-35e3-4950-b4b3-90c679cdf6a7"

/// Goblin Commando — `{4}{R}` for a 2/2 Goblin whose whole text is "When this
/// creature enters, it deals 2 damage to target creature."
///
/// Nothing about that one sentence is answered by reading the card file, so the
/// scenario plays it: what "target creature" reaches is a board reading, and an
/// Elf across the table has to be on the menu beside my own while the Mountain
/// under my feet is not. Two damage on a printed 1/1 is lethal (CR 704.5g),
/// which is why the trigger is aimed at the opponent's Elf rather than at a
/// player, and the five tapped Mountains are exactly the `{4}{R}` — the Elves
/// are kept out of the payment, so the empty pool afterwards is a statement
/// about the price rather than about a board that never had the mana.
#[test]
#[allow(clippy::too_many_lines)]
fn goblin_commando_deals_two_damage_to_the_creature_its_trigger_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
            ],
        )
        // A creature across the table, so "target creature" is read as any
        // creature and the 2 damage has something of the opponent's to kill.
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[goblin_commando()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let land = on_battlefield(&engine, p0, mountain()).expect("a Mountain is out");
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "a printed 1/1 for two damage to kill"
    );

    // Five Mountains are exactly {4}{R}, and both Elves are named as the
    // printing kept back: one of them is the creature the trigger is about to
    // be aimed at, and a source tapped for mana is a source whose status has
    // already changed for a reason of its own.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Mountains, five red, and neither Elf contributed"
    );
    assert!(!is_tapped(&engine, mine), "my Elves never paid for it");

    cast_with_floating(&mut engine, p0, goblin_commando());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{R}} is the whole price, so the pool the Mountains filled is empty"
    );

    // The Goblin is on the battlefield before its trigger asks anything: an
    // enters-trigger is put on the stack once the permanent has arrived
    // (CR 603.3d).
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let commando = on_battlefield(&engine, p0, goblin_commando())
        .expect("the Commando resolved onto the battlefield");
    assert_eq!(pt(&engine, commando), (2, 2), "the body the card prints");
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the Commando's controller aims its own trigger");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the trigger asks once"
    );
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        options.contains(&commando),
        "the Commando is itself a creature on the battlefield by the time its \
         trigger is put on the stack, so it is a legal target too: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Mountain is a permanent and no creature, so the filter is read and \
         not skipped: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "two damage to a printed 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and the creature the trigger named left the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the creature the trigger did not name never moved"
    );
    assert_eq!(
        pt(&engine, mine),
        (1, 1),
        "and it still carries the body it was printed with"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the damage went to the creature and never to the player whose board it stood on"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_commando()).is_some(),
        "the Commando outlives the creature its trigger killed"
    );
}
