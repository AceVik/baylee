//! `cards/creatures/mv_3/flamewave_invoker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Flamewave Invoker (`ee17706a-edc0-4301-8330-1da539679c7a`) — {2}{R} Creature — Goblin Mutant 2/2:
/// "{7}{R}: This creature deals 5 damage to target player or planeswalker."
#[test]
fn flamewave_invoker_deals_five_damage_to_target_player() {
    let invoker_card = card_index("ee17706a-edc0-4301-8330-1da539679c7a");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                invoker_card,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, None);
    activate(&mut engine, p0, invoker_card, 0);
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

    assert_eq!(engine.state().players[1].life, 15);
}
