//! `cards/creatures/mv_1/soldier_of_fortune.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldier_of_fortune() -> CardIndex {
    card_index("8b2367b3-3a8d-442f-8d58-6e4c03fa5a46")
}

/// Soldier of Fortune ({R} 1/1 Creature — Human Mercenary):
/// - "{R}, {T}: Target player shuffles their library."
#[test]
fn soldier_of_fortune_taps_to_shuffle_target_player_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[soldier_of_fortune(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let soldier = on_battlefield(&engine, p0, soldier_of_fortune()).unwrap();

    tap_all_mana_but(&mut engine, p0, Some(soldier_of_fortune()));
    activate(&mut engine, p0, soldier_of_fortune(), 0);
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

    assert!(is_tapped(&engine, soldier));
    assert!(
        engine.state().journal.entries().iter().any(|e| {
            matches!(
                e.event,
                crate::event::GameEvent::Shuffled { player, .. } if player == p1
            )
        }),
        "target player's library must be shuffled"
    );
}
