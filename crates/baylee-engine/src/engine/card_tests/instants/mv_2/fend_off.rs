#[allow(clippy::wildcard_imports)]
use super::*;

fn fend_off() -> CardIndex {
    card_index("fa8f3827-8cd9-4896-ab0c-26fecacceb40")
}

#[test]
fn fend_off_prevents_combat_damage_dealt_by_target_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[fend_off()])
        .battlefield(0, &[gray_ogre(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, fend_off());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let _ = attack_and_collect_blocks(&mut engine, ogre, p1);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });

    assert_eq!(engine.state().players[1].life, 20);
}
