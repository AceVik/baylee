//! `cards/creatures/mv_1/agent_of_stromgald.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Agent of Stromgald` prints `{{R}}: Add {{B}}.` with `Coverage::Implemented`.
/// In this scenario, seat 0 controls the 1/1 creature and one Mountain.
/// Tapping the Mountain floats one red mana to activate the mana filter ability.
/// Because the cost does not include `{{T}}`, the creature remains untapped while filtering the red mana into black.
#[test]
fn agent_of_stromgald_filters_red_mana_into_black_without_tapping() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[agent_of_stromgald(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let agent = on_battlefield(&engine, p0, agent_of_stromgald()).expect("agent seated");
    assert_eq!(pt(&engine, agent), (1, 1));
    assert!(!is_tapped(&engine, agent));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0
    );

    activate(&mut engine, p0, agent_of_stromgald(), 0);
    assert!(
        stack_is_empty(&engine),
        "mana ability does not use the stack"
    );
    assert!(
        !is_tapped(&engine, agent),
        "`Agent of Stromgald` does not tap to activate"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0, "red mana was spent");
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "black mana was produced"
    );
    assert_eq!(pool.total(), 1, "exactly one mana floating");
}
