#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn planar_collapse() -> CardIndex {
    card_index("82bccc9f-f700-4f1f-b89e-bea63d973161")
}

#[test]
fn planar_collapse_destroys_all_creatures_on_upkeep_with_four_or_more() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[planar_collapse(), gray_ogre(), earth_elemental()])
        .battlefield(1, &[gray_ogre(), earth_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(on_battlefield(&engine, p0, planar_collapse()).is_none());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_none());
}

#[test]
fn planar_collapse_does_not_trigger_below_four_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[planar_collapse(), gray_ogre()],
        &[gray_ogre(), earth_elemental()],
        &[],
        &[],
    );
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, planar_collapse()).is_some());
    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
    assert!(on_battlefield(&engine, p1, gray_ogre()).is_some());
}
