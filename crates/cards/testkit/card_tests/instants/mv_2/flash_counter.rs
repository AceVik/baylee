//! `cards/instants/mv_2/flash_counter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flash Counter — {1}{U} instant: "Counter target instant spell."
///
/// The printed filter is a *type*, so the scenario puts exactly one spell on
/// the stack and reads the menu the cast publishes: p1's Giant Growth is on it
/// while the Elf that Growth was aimed at — a permanent, and the only other
/// object in the game — is not. The counter then has to do the whole of its
/// work rather than merely leave the stack alone: the Growth is in its owner's
/// graveyard, the +3/+3 never reached the creature, and the {1}{U} really left
/// the pool.
#[test]
fn flash_counter_counters_the_instant_spell_it_names_and_leaves_the_creature_alone() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[flash_counter()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(1, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    // The instant under test is cast on the *opponent's* turn, which is the
    // only turn an instant needs.
    assert!(
        walk_to_own_main(&mut engine, p1),
        "p1 reaches their own main"
    );

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, elf), (1, 1), "a printed 1/1 before the Growth");

    // p1's Forest pays {G}; the Elf is kept off the tap so that the creature
    // the pump is about to name is still the board this test reads back.
    tap_all_mana_but(&mut engine, p1, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        1,
        "one Forest, one green, and the Elf kept its own {{T}}"
    );
    cast_with_floating(&mut engine, p1, giant_growth());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Giant Growth targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it names the target");
    assert_eq!(options, vec![elf], "the only creature in the game");
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf came out of the list that offered it");

    // The Growth is on the stack and p0 holds priority with it there.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && !stack_is_empty(e)
    });
    let growth = on_stack(&engine, giant_growth()).expect("the Growth is on the stack");

    cast_from_hand(&mut engine, p0, flash_counter());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target instant spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it names the target");
    assert_eq!((min, max), (1, 1), "one spell, and the cast asks once");
    assert_eq!(
        options,
        vec![growth],
        "the instant on the stack is the whole menu: the creature the Growth \
         was aimed at is a permanent and no spell"
    );
    // CR 601.2c before CR 601.2h: the target is named while the {1}{U} the two
    // Islands made is still in the pool.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Islands are tapped and the {{1}}{{U}} is not yet paid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![growth],
            },
        )
        .expect("the spell the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, giant_growth()).is_some(),
        "a countered spell goes to its owner's graveyard"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "\"counter\": the +3/+3 never resolved, so the Elf is the 1/1 it was"
    );
    assert!(
        in_graveyard(&engine, p0, flash_counter()).is_some(),
        "and the counter itself resolved rather than being countered or fizzling"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{U}} it charged came out of the pool"
    );
}
