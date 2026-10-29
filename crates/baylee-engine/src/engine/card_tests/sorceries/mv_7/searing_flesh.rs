//! `cards/sorceries/mv_7/searing_flesh.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Searing Flesh (`d9cb5ae8-5594-46c4-9b77-4f9c6402bcd2`) — {6}{R} Sorcery:
/// "Searing Flesh deals 7 damage to target opponent or planeswalker."
#[test]
fn searing_flesh_deals_seven_damage_to_target_opponent() {
    let searing_card = card_index("d9cb5ae8-5594-46c4-9b77-4f9c6402bcd2");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[searing_card])
        .battlefield(
            0,
            &[
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

    cast_from_hand(&mut engine, p0, searing_card);
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

    assert_eq!(engine.state().players[1].life, 13);
}
