//! `cards/enchantments/auras/mv_3/feedback.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Feedback: "Enchant enchantment" / "At the beginning of the upkeep of
/// enchanted enchantment's controller, this Aura deals 1 damage to that
/// player." Attached to the opponent's Bad Moon, it hits only them, at
/// their own upkeep.
#[test]
fn feedback_deals_damage_at_the_enchanted_enchantments_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let feedback = feedback();
    let bad_moon = bad_moon();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[bad_moon])
        .hand(0, &[feedback])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let enchantment = on_battlefield(&engine, p1, bad_moon).expect("their enchantment");

    cast_from_hand(&mut engine, p0, feedback);
    aim_at(&mut engine, p0, enchantment);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "1 damage at its controller's upkeep"
    );
    assert_eq!(engine.state().players[0].life, 20);

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "not at Feedback's own controller's upkeep, only the enchanted enchantment's"
    );
    assert_eq!(engine.state().players[1].life, 19);
}
