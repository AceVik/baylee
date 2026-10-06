//! `cards/sorceries/mv_4/vengeance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1d001145-5d14-43a9-bf3b-3ce5c20b2a46"

/// Vengeance — {3}{W} sorcery: "Destroy target tapped creature."
///
/// The printed filter has two halves and each needs a witness, so mine is a
/// Llanowar Elves that has already paid for itself (the only way this engine
/// taps a creature without a spell) while an *untapped* Llanowar Elves stands
/// across the table. The published menu is what reads the word "tapped" — the
/// Elf opposite is a creature and must not be on it — and the resolution reads
/// the verb: the named creature is in its owner's graveyard while the untapped
/// one never moved, and the {3}{W} came out of the pool the four Plains and the
/// Elf's own tap filled.
#[test]
fn vengeance_destroys_a_tapped_creature_and_declines_an_untapped_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[vengeance()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let tapped = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let idle = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        types(&engine, idle).contains(TypeSet::CREATURE),
        "the control is a creature, so its absence from the menu is about \
         \"tapped\" and not about its type: {:?}",
        types(&engine, idle)
    );

    // Four Plains and the Elves' own `{T}: Add {G}` — five mana, of which the
    // spell takes four. The same tap is what makes my Elf a legal target.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 5, "four Plains and one mana creature");
    assert!(
        is_tapped(&engine, tapped),
        "the Elf paid the price of being targetable with its own tap"
    );
    assert!(!is_tapped(&engine, idle), "and only my own was touched");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "four white off the Plains and one green off the Elf"
    );

    cast_with_floating(&mut engine, p0, vengeance());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the caster names the target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&tapped),
        "a tapped creature is exactly what the card prints: {options:?}"
    );
    assert!(
        !options.contains(&idle),
        "\"tapped\" is read and not skipped: an untapped creature is no legal \
         target, whichever side of the table it stands on: {options:?}"
    );
    assert_eq!(
        options.len(),
        1,
        "and it is the only tapped creature in the game: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![tapped],
            },
        )
        .expect("the tapped Elf came out of the menu the spell published");
    assert!(
        !stack_is_empty(&engine),
        "a sorcery that has paid its costs is on the stack, not resolved"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the targeted creature was destroyed (CR 701.8a)"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "and has left the battlefield, which is the destruction itself"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature the spell did not name never moved"
    );
    assert!(
        in_graveyard(&engine, p0, vengeance()).is_some(),
        "and the sorcery itself resolved into its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "four of the five mana paid the {{3}}{{W}} and the green is what is left"
    );
}
