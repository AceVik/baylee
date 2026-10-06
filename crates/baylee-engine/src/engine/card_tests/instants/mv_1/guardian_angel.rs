//! `cards/instants/mv_1/guardian_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Guardian Angel — "Prevent the next X damage that would be dealt to any
/// target this turn." (Its "pay {1} any time" half is not written.) X = 2
/// on p0 itself, then a Lightning Bolt at p0: 2 of the 3 are prevented.
#[test]
fn guardian_angel_prevents_the_next_x_damage_to_its_target() {
    let p0 = PlayerId::new(0);
    let angel = card_index("1a91ca69-e890-41dc-866b-3aabf10c9a9c");
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), mountain()])
        .hand(0, &[angel, lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let red = on_battlefield(&engine, p0, mountain()).expect("the Mountain is out");
    tap_mana_except(&mut engine, p0, red);
    cast_with_floating(&mut engine, p0, angel);
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pass_until(&mut engine, stack_is_empty);
    let life = engine.state().players[0].life;

    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("the Bolt at p0");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        life - 1,
        "3 dealt, 2 prevented"
    );
}
