//! `cards/artifacts/mv_4/magnetic_mine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn magnetic_mine() -> CardIndex {
    card_index("bbb50cf5-cd07-4ae8-bfa8-d747e44fb1ef")
}

/// "Whenever another artifact is put into a graveyard from the battlefield,
/// this artifact deals 2 damage to that artifact's controller." Either seat's
/// artifact counts, the damage goes to its own controller, and the Mine
/// itself dying is not "another".
#[test]
fn another_artifact_dying_hurts_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[magnetic_mine(), quiet_artifact()])
        .battlefield(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let theirs = on_battlefield(&engine, p1, quiet_artifact()).unwrap();
    kill(&mut engine, theirs);
    assert_eq!(life_of(&engine, p1), 18, "the dead artifact's controller");
    assert_eq!(life_of(&engine, p0), 20);

    let mine = on_battlefield(&engine, p0, quiet_artifact()).unwrap();
    kill(&mut engine, mine);
    assert_eq!(life_of(&engine, p0), 18, "my own artifact too");
    assert_eq!(life_of(&engine, p1), 18, "and nobody is hit twice");

    let mine_itself = on_battlefield(&engine, p0, magnetic_mine()).unwrap();
    kill(&mut engine, mine_itself);
    assert_eq!(life_of(&engine, p0), 18, "\"another\": the Mine itself is free");
}

/// A creature that is not an artifact does not set it off.
#[test]
fn a_dying_non_artifact_does_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[magnetic_mine()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    kill(&mut engine, ogre);
    assert_eq!(life_of(&engine, p1), 20);
}
