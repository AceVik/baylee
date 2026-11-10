//! `cards/sorceries/mv_1/fireball.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fireball, its one-target half: "Fireball deals X damage … among any
/// number of targets" with one target is X damage to it. X = 3 at p1.
#[test]
fn fireball_deals_x_damage_to_one_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let fireball = card_index("aa7714b0-2bfb-458a-8ebf-37ec2c53383e");
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[fireball])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = engine.state().players[1].life;
    cast_from_hand(&mut engine, p0, fireball);
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 is any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, life - 3);
}
