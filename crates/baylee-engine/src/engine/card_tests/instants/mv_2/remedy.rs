//! `cards/instants/mv_2/remedy.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LIGHTNING_BOLT;

fn remedy() -> CardIndex {
    card_index("c644f190-63e5-4496-8612-bbe7be91afc9")
}

/// `Remedy` ({1}{W}, Instant, `Coverage::Partial`) prevents the next 5 damage
/// that would be dealt to any target this turn.
#[test]
fn remedy_prevents_next_5_damage_to_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), mountain()])
        .hand(0, &[remedy(), LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, remedy());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 20);
}
