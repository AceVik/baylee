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

#[test]
fn null_brooch_cannot_counter_a_creature_spell() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut e = lk::game(SEED, &[null_brooch()], &[], &[gray_ogre(), forest()], &[]);
    lk::float(&mut e, p0, &[(ManaColor::Red, 10)]);
    cast_with_floating(&mut e, p0, gray_ogre());
    let source = lk::obj(&e, p0, null_brooch());
    assert!(
        kit::offered(&e, source).is_empty(),
        "plenty of mana and cards, but no noncreature spell"
    );
}
