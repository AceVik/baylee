#[allow(clippy::wildcard_imports)]
use super::*;

fn aura_thief() -> CardIndex {
    card_index("cf0936ea-2741-43a4-ad6a-a1258b54945b")
}

#[test]
fn aura_thief_gains_control_of_all_enchantments_when_it_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[aura_thief()])
        .battlefield(1, &[sylvan_library()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let thief = on_battlefield(&engine, p0, aura_thief()).unwrap();
    assert!(on_battlefield(&engine, p1, sylvan_library()).is_some());

    kill(&mut engine, thief);

    assert!(on_battlefield(&engine, p0, sylvan_library()).is_some());
}
