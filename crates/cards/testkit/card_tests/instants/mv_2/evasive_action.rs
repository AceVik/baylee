//! `cards/instants/mv_2/evasive_action.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Evasive Action is {1}{U} and counters a spell unless its controller pays
/// {1} for each **basic land type among lands you control** — the domain
/// count belongs to the instant's own controller, not to the spell's. The
/// three lands under p0 hold only two basic land types (Island and Forest),
/// and that number is only readable against the alternatives: one per land
/// would ask for three, one per land of the spell's controller would ask for
/// one off p1's three Swamps, and only "basic land type" asks for two. The
/// declined tax is the other half — the Ritual never adds its three black and
/// its card ends up in its owner's graveyard.
#[test]
fn evasive_action_taxes_by_the_basic_land_types_of_its_own_controllers_lands() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[evasive_action()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    // p1 has to be the active player to cast in a main phase, and its spell
    // has to still be on the stack when the counter is aimed at it.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, dark_ritual());
    let ritual = on_stack(&engine, dark_ritual()).expect("the Ritual is on the stack");

    // Priority comes back around to p0 while the Ritual waits to resolve.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, evasive_action());

    // Targets are chosen as the spell is cast (CR 601.2c); with one legal
    // spell on the stack the engine may name it itself.
    if let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    {
        assert_eq!(player, p0, "the caster names their own target");
        assert!(
            options.contains(&ritual),
            "the Ritual is the spell on the stack: {options:?}"
        );
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![ritual],
                },
            )
            .unwrap();
    }

    // The tax is asked when Evasive Action *resolves*, not when it is cast:
    // a counterspell goes on the stack above the Ritual and both seats get
    // priority first (CR 117.3c). Reading the pending straight after the
    // target choice reads that priority window and nothing else.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });

    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Ritual's controller is asked for the domain tax, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1, "the controller of the target spell pays");
    assert_eq!(
        mana, 2,
        "Island and Forest are two basic land types among p0's three lands — \
         three would be one per land and one would be the target's own Swamps"
    );

    // Declining is a real counter: the Ritual's three black never arrive.
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, dark_ritual()).is_some(),
        "an unpaid tax counters the spell"
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        2,
        "the three Swamps paid for the Ritual and nothing else, so a Ritual \
         that had resolved would have added {{B}}{{B}}{{B}} on top"
    );
    assert!(
        in_graveyard(&engine, p0, evasive_action()).is_some(),
        "and the instant itself resolved and went to its owner's graveyard"
    );
}
