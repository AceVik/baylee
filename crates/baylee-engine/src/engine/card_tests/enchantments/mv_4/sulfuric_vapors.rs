#[allow(clippy::wildcard_imports)]
use super::*;

fn sulfuric_vapors() -> CardIndex {
    card_index("40b14aaa-f29e-4e84-9bdc-bc2ac9762ce9")
}

#[test]
fn sulfuric_vapors_boosts_red_creatures_on_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[sulfuric_vapors(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre_id = on_battlefield(&engine, p0, gray_ogre()).expect("ogre on bf");
    assert_eq!(pt(&engine, ogre_id), (3, 2));
}
