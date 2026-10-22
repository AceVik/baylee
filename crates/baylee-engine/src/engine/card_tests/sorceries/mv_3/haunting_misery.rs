//! `cards/sorceries/mv_3/haunting_misery.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HAUNTING_MISERY;

/// `Haunting Misery` ({1}{B}{B}, Sorcery, Partial) deals 1 damage to target
/// player or planeswalker.
#[test]
fn haunting_misery_deals_damage_to_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[HAUNTING_MISERY])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, HAUNTING_MISERY);
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

    assert_eq!(engine.state().players[1].life, 19);
}
