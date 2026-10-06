//! `cards/instants/mv_2/remove_soul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Remove Soul — {1}{U} instant: "Counter target creature spell."
///
/// The whole card is one question about a zone no other test in this file
/// reads from: the target is a *spell*, so the scenario has to stop while the
/// Elf is a stack object and in neither of the two zones a creature card
/// lives in, and the offer that names it can only have come off the stack. The
/// counter is then read as a move rather than as a question — a countered
/// creature spell was never a permanent, so the Elf can only end up in its
/// owner's graveyard — and the two Islands that paid for the counterspell are
/// spent with it.
#[test]
fn remove_soul_counters_the_creature_spell_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(307, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[remove_soul()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // p1 puts a creature *spell* on the stack — the only thing Remove Soul can
    // name — and the Elf is nowhere a permanent yet.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, llanowar_elves());
    let elf_spell =
        on_stack(&engine, llanowar_elves()).expect("the Elves are a spell on the stack");
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none()
            && in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "an unresolved creature spell is in neither of the two zones that hold \
         a creature card"
    );

    // p0's window: the creature spell is still waiting and p0 holds priority.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // Mana into the pool before anything is claimed about the offer: the
    // engine reads `castable` off the pool and not off untapped lands.
    let soul = in_hand(&engine, p0, remove_soul()).expect("the counterspell is in hand");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Islands pay {{1}}{{U}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&soul),
        "with the mana floating and a creature spell on the stack, the card is \
         castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, remove_soul());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it aims it");
    assert_eq!(
        options,
        vec![elf_spell],
        "the creature spell on the stack is the whole menu: this card counters \
         a spell, so nothing that is not on the stack can be named"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elf_spell],
            },
        )
        .expect("the creature spell the offer named is a legal target");

    assert!(
        on_stack(&engine, remove_soul()).is_some() && on_stack(&engine, llanowar_elves()).is_some(),
        "choosing the target does not counter anything: the counterspell is on \
         the stack above the spell it named, and both are still there"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_stack(&engine, llanowar_elves()).is_none(),
        "the creature spell left the stack"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "and it never resolved, so it never became a permanent"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "a countered spell goes to its owner's graveyard (CR 701.6a)"
    );
    assert!(
        in_graveyard(&engine, p0, remove_soul()).is_some(),
        "the counterspell resolved and was put into its caster's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{U}} came out of the pool"
    );
}
