#[allow(clippy::wildcard_imports)]
use super::*;

fn soul_sculptor() -> CardIndex {
    card_index("d407abd0-69a9-405d-ab69-75b379a45780")
}

#[test]
fn soul_sculptor_turns_target_creature_into_enchantment() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[soul_sculptor(), plains(), plains()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("ogre on bf");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, soul_sculptor(), 0);
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(ogre).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::ENCHANTMENT));
    assert!(!chars.types.contains(TypeSet::CREATURE));
}
