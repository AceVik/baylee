//! `cards/artifacts/mv_4/null_brooch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn null_brooch() -> CardIndex {
    card_index("6f885041-3e57-4a69-84f2-fd207ff9f31b")
}

#[test]
fn null_brooch_discards_hand_and_counters_target_noncreature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[null_brooch(), island(), island()])
        .battlefield(1, &[mountain()])
        .hand(0, &[forest(), island()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p0);
    let brooch = on_battlefield(&engine, p0, null_brooch()).unwrap();
    let stack_bolt = on_stack(&engine, lightning_bolt()).unwrap();
    kit::activate_object(&mut engine, p0, brooch, 0);
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[stack_bolt], &[])
    });

    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(p0)).len(), 0);
    assert_eq!(engine.state().players[0].life, 20);
}
