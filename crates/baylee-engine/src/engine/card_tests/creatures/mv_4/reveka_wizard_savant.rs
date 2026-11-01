//! Reveka, Wizard Savant — `{2}{U}{U}` 0/1 Legendary Creature — Dwarf Wizard (Homelands)
//!
//! `{T}`: Reveka deals 2 damage to any target and doesn't untap during your
//! next untap step.

#[allow(clippy::wildcard_imports)]
use super::*;

fn reveka_wizard_savant() -> CardIndex {
    card_index("7085d5c6-da96-465e-90cf-8469fcacb474")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn reveka_wizard_savant_deals_two_damage_and_suppresses_next_untap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[reveka_wizard_savant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let reveka = on_battlefield(&engine, p0, reveka_wizard_savant()).unwrap();
    let p1_life_before = engine.state().players[1].life;

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: reveka,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, p1_life_before - 2);
    assert!(is_tapped(&engine, reveka));

    // Advance through p1's turn to p0's next FirstMain: Reveka did NOT untap!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        is_tapped(&engine, reveka),
        "Reveka, Wizard Savant did not untap during your next untap step"
    );

    // Only that one untap step: the next one untaps her.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        !is_tapped(&engine, reveka),
        "she untaps in the untap step after the skipped one"
    );
}

/// "Any target": a creature takes the 2 damage too, and the untap is
/// withheld just the same.
#[test]
fn reveka_wizard_savant_deals_two_damage_to_a_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[reveka_wizard_savant()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let reveka = on_battlefield(&engine, p0, reveka_wizard_savant()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: reveka,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the player is untouched"
    );
    assert!(is_tapped(&engine, reveka));
}

#[test]
fn reveka_wizard_savant_untaps_for_a_new_controller() {
    crate::engine::mechanics_tests::untap_scope::assert_untaps_after_control_change(
        reveka_wizard_savant(),
    );
}
