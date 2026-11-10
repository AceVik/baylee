//! `cards/enchantments/mv_2/deathgrip.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Deathgrip: "{B}{B}: Counter target green spell." Cast on the active
/// player's own turn (creature spells are sorcery-speed), it is countered
/// in response and never reaches the battlefield.
#[test]
fn deathgrip_counters_a_target_green_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let deathgrip = deathgrip();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[deathgrip, swamp(), swamp()])
        .battlefield(1, &[forest()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, llanowar_elves());

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, deathgrip, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"counter target green spell\" asks for one, got {:?}",
            engine.pending()
        )
    };
    let elf_spell = on_stack(&engine, llanowar_elves()).expect("the Elves are on the stack");
    assert_eq!(options, vec![elf_spell], "the only green spell up");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elf_spell],
            },
        )
        .expect("the target was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "the Elves never resolved"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "countered spells go to their owner's graveyard"
    );
}

/// Deathgrip's colour restriction: with only a black spell on the stack (no
/// green spell anywhere), "target green spell" has no legal target, so the
/// ability is not offered at all.
#[test]
fn deathgrip_does_not_counter_an_off_colour_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let deathgrip = deathgrip();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[deathgrip, swamp(), swamp()])
        .battlefield(1, &[swamp()])
        .hand(1, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let deathgrip_obj = on_battlefield(&engine, p0, deathgrip).expect("Deathgrip seated");
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, festering_goblin());

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    assert!(
        !priority_offer(&engine)
            .abilities
            .contains(&(deathgrip_obj, 0)),
        "no green spell on the stack, so \"counter target green spell\" has no target"
    );
}
