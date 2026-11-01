//! `cards/creatures/mv_2/mogg_maniac.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mogg_maniac() -> CardIndex {
    card_index("53b86704-4f16-498e-8c2c-e661cb85eb9c")
}

/// Mogg Maniac (`53b86704-4f16-498e-8c2c-e661cb85eb9c`) — {1}{R} 1/1 Creature — Goblin (Implemented):
/// "Whenever this creature is dealt damage, it deals that much damage to target opponent or planeswalker."
#[test]
fn mogg_maniac_deals_damage_to_target_opponent_when_dealt_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mogg_maniac(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let maniac = on_battlefield(&engine, P0, mogg_maniac()).unwrap();
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, maniac);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn reflection_cannot_target_its_own_controller() {
    let mut e = lk::game(SEED, &[mogg_maniac()], &[], &[lightning_bolt()], &[]);
    let maniac = lk::obj(&e, P0, mogg_maniac());
    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    lk::aim(&mut e, &[maniac], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0]
            }
        )
        .is_err()
    );
    lk::aim(&mut e, &[], &[P1]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, P1), 17);
    assert_eq!(lk::life(&e, P0), 20);
}
