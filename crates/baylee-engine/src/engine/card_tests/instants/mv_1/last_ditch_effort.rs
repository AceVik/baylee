#[allow(clippy::wildcard_imports)]
use super::*;

fn last_ditch_effort() -> CardIndex {
    card_index("cdd8d7cf-28f2-4a4d-a373-40d48bff615b")
}

#[test]
fn last_ditch_effort_sacrifices_your_creatures_and_deals_that_much_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), gray_ogre(), earth_elemental()])
        .hand(0, &[last_ditch_effort()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, last_ditch_effort());
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

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, p0, earth_elemental()).is_none());
    assert_eq!(engine.state().players[1].life, 18);
}
