//! `cards/creatures/mv_4/phantasmal_fiend.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn phantasmal_fiend() -> CardIndex {
    card_index("821ce349-5379-4415-89e2-224e21dda24b")
}

/// Phantasmal Fiend ({3}{B} 1/5 Creature — Illusion):
/// - "{B}: This creature gets +1/-1 until end of turn."
/// - "{1}{U}: Switch this creature's power and toughness until end of turn."
#[test]
fn phantasmal_fiend_pumps_and_switches_power_and_toughness_in_layer_order() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[phantasmal_fiend(), swamp(), swamp(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let fiend = on_battlefield(&engine, p0, phantasmal_fiend()).unwrap();
    assert_eq!(pt(&engine, fiend), (1, 5));

    tap_all_mana(&mut engine, p0);

    // Activate {B}: +1/-1 -> 2/4.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: fiend,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fiend), (2, 4));

    // Activate {1}{U}: switch P/T -> 4/2 (layer 7d applies after layer 7c).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: fiend,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, fiend), (4, 2));

    // "Until end of turn": on the opponent's turn it is a 1/5 again.
    pass_until(&mut engine, |e| {
        e.state().turn.active == PlayerId::new(1)
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, fiend), (1, 5));
}
