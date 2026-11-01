use super::*;

fn shackles() -> CardIndex {
    card_index("05861ff2-ec85-43d1-b641-c5d2da819c03")
}

#[test]
fn shackles_prevents_untap_and_returns_to_hand_for_w() {
    use crate::engine::card_tests::legends_kit as lk;
    use crate::engine::card_tests::partials_w1b_kit as kit;

    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[gray_ogre(), plains(), plains(), plains(), plains()])
        .hand(0, &[shackles()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, shackles());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, shackles()).is_some());
    lk::attack(&mut engine, &[ogre]);
    kit::walk_to_next_own_main(&mut engine, p0);
    assert!(is_tapped(&engine, ogre), "host skips its actual untap step");
    lk::float(&mut engine, p0, &[(ManaColor::White, 1)]);
    activate(&mut engine, p0, shackles(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, shackles()).is_none());
    assert!(in_hand(&engine, p0, shackles()).is_some());
    kit::walk_to_next_own_main(&mut engine, p0);
    assert!(!is_tapped(&engine, ogre));
}
