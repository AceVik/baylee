//! `cards/enchantments/auras/mv_1/paralyze.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Paralyze: "When this Aura enters, tap enchanted creature." / "Enchanted
/// creature doesn't untap during its controller's untap step." / "At the
/// beginning of the upkeep of enchanted creature's controller, that player
/// may pay {4}. If the player does, untap the creature." Declined, the
/// creature stays tapped through its controller's own untap step.
#[test]
fn paralyze_taps_the_creature_on_etb_and_it_stays_tapped_if_the_controller_declines_to_pay() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let paralyze = paralyze();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[paralyze])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p1, quiet_creature()).expect("their creature");
    assert!(!is_tapped(&engine, creature));

    cast_from_hand(&mut engine, p0, paralyze);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, creature), "tapped as the Aura enters");

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    assert!(
        is_tapped(&engine, creature),
        "its controller's untap step passed without untapping it"
    );
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p1, "the enchanted creature's controller is asked");
    assert_eq!(prompt, YesNoPrompt::PayTax { mana: 4 });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, creature), "declined, so still tapped");
}

/// Paralyze's other answer: paying {4} at the upkeep untaps the creature.
#[test]
fn paralyze_untaps_the_creature_when_its_controller_pays_four() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let paralyze = paralyze();
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp()])
        .battlefield(
            1,
            &[quiet_creature(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[paralyze])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let creature = on_battlefield(&engine, p1, quiet_creature()).expect("their creature");

    cast_from_hand(&mut engine, p0, paralyze);
    aim_at(&mut engine, p0, creature);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, creature));

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p1);
    if engine.payment_window().is_some() {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, creature), "paid {{4}}, so it untapped");
}
