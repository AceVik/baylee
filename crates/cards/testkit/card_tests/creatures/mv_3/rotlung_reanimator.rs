//! `cards/creatures/mv_3/rotlung_reanimator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rotlung Reanimator — {2}{B} — Creature — Zombie Cleric, 2/2, printing
/// "Whenever this creature or another Cleric dies, create a 2/2 black Zombie
/// creature token."
///
/// The trigger is a subtype filter with an `or` in it, so the board has to
/// carry both sides of that word: an Ondu Cleric (Kor Cleric Ally) is the
/// Cleric, and a Llanowar Elves (Elf Druid) is the body that must not count.
/// Three Vindicates kill the Elf, the Cleric and finally the Reanimator
/// itself, and the token ledger is read off the battlefield after each one —
/// so neither the subtype nor the second arm of the printed `or` can be
/// missing without a count going wrong.
#[test]
#[allow(clippy::too_many_lines)] // three deaths, one printed trigger, and every arm of it read off the board
fn rotlung_reanimator_makes_a_zombie_for_a_cleric_and_not_for_a_body() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), ondu_cleric(), llanowar_elves()],
        )
        .hand(0, &[rotlung_reanimator()])
        // Six Swamps and six Plains: three Vindicates are three white and
        // three black pips plus three generic, so the pool has to leave room
        // for the auto-payer to spend a colour on the generic half.
        .battlefield(
            1,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(1, &[vindicate(), vindicate(), vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The card arrives the way it arrives: cast for three mana off the Swamps.
    cast_from_hand(&mut engine, p0, rotlung_reanimator());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, rotlung_reanimator()).is_some()
    });
    let reanimator =
        on_battlefield(&engine, p0, rotlung_reanimator()).expect("the Reanimator resolved");
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("the Cleric is out");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, reanimator), (2, 2), "a printed 2/2 body");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has died yet, so the trigger has had nothing to look at"
    );

    reach_their_main_phase(&mut engine, p1);

    // The control: a death that is not a Cleric. The Elf really does die,
    // which is what makes the token count a claim about the subtype filter
    // rather than about a destroy that never resolved.
    cast_from_hand(&mut engine, p1, vindicate());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it does the choosing");
    assert!(
        options.contains(&elf),
        "a creature is a permanent, on either side of the table: {options:?}"
    );
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the Elf died, so the count below is read after a real death"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "an Elf Druid is no Cleric, so its death creates nothing at all"
    );

    // The card's own word: another Cleric dies.
    cast_from_hand(&mut engine, p1, vindicate());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it does the choosing");
    assert!(
        options.contains(&cleric),
        "the Ondu Cleric is a permanent and a Cleric: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .expect("the Cleric was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, ondu_cleric()).is_some(),
        "the Cleric died"
    );
    let zombies = tokens_of(&engine, p0);
    assert_eq!(
        zombies.len(),
        1,
        "another Cleric died, so exactly one Zombie: {zombies:?}"
    );
    let zombie = engine
        .state()
        .object(zombies[0])
        .expect("the token is on the battlefield")
        .token
        .expect("and it knows which token it is");
    assert_eq!(zombie.name, "Zombie");
    assert_eq!((zombie.power, zombie.toughness), (Some(2), Some(2)));
    assert!(
        zombie.colors.contains(baylee_core::color::Color::Black),
        "a black Zombie, not a colourless one"
    );
    assert_eq!(
        pt(&engine, zombies[0]),
        (2, 2),
        "and the body the battlefield projects is the body the token prints"
    );

    // The other arm of the printed `or`: "whenever this creature ... dies".
    // The first arm cannot answer for it — it carries `Another`, which is
    // exactly what the source is not — so a card missing `Filter::This`
    // would leave this death silent.
    cast_from_hand(&mut engine, p1, vindicate());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"destroy target permanent\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the seat that cast it does the choosing");
    assert!(
        options.contains(&reanimator),
        "the Reanimator is itself a Zombie Cleric and a permanent: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![reanimator],
            },
        )
        .expect("the Reanimator was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, rotlung_reanimator()).is_some(),
        "the Reanimator died"
    );
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "\"this creature or another Cleric\": its own death is the second arm, \
         and the ability looks back to the game before it left (CR 603.10a)"
    );
}
