#[allow(clippy::wildcard_imports)]
use super::*;

fn wizard_mentor() -> CardIndex {
    card_index("a1fb1b77-a49f-4447-bbe0-156407f192e2")
}

#[test]
fn wizard_mentor_returns_self_and_target_creature_you_control_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[wizard_mentor(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    activate(&mut engine, p0, wizard_mentor(), 0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, wizard_mentor()).is_none());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(in_hand(&engine, p0, wizard_mentor()).is_some());
    assert!(in_hand(&engine, p0, gray_ogre()).is_some());
}
