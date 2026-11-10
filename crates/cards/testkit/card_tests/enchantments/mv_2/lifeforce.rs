//! `cards/enchantments/mv_2/lifeforce.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lifeforce: "{G}{G}: Counter target black spell." The mirror of
/// Deathgrip, over black spells.
#[test]
fn lifeforce_counters_a_target_black_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let lifeforce = lifeforce();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lifeforce, forest(), forest()])
        .battlefield(1, &[swamp()])
        .hand(1, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, festering_goblin());

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, lifeforce, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"counter target black spell\" asks for one, got {:?}",
            engine.pending()
        )
    };
    let goblin_spell = on_stack(&engine, festering_goblin()).expect("the Goblin is on the stack");
    assert_eq!(options, vec![goblin_spell], "the only black spell up");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![goblin_spell],
            },
        )
        .expect("the target was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_none(),
        "the Goblin never resolved"
    );
    assert!(
        in_graveyard(&engine, p1, festering_goblin()).is_some(),
        "countered spells go to their owner's graveyard"
    );
}

/// Lifeforce's colour restriction: with only a white spell on the stack (no
/// black spell anywhere), "target black spell" has no legal target, so the
/// ability is not offered at all.
#[test]
fn lifeforce_does_not_counter_an_off_colour_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let lifeforce = lifeforce();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lifeforce, forest(), forest()])
        .battlefield(1, &[plains(), plains()])
        .hand(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let lifeforce_obj = on_battlefield(&engine, p0, lifeforce).expect("Lifeforce seated");
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, ondu_cleric());

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    assert!(
        !priority_offer(&engine)
            .abilities
            .contains(&(lifeforce_obj, 0)),
        "no black spell on the stack, so \"counter target black spell\" has no target"
    );
}
