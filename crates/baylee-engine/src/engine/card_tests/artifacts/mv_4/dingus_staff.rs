//! `cards/artifacts/mv_4/dingus_staff.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dingus_staff() -> CardIndex {
    card_index("7127fc35-600c-4397-8c01-5327d3c4944b")
}

/// "Whenever a creature dies, this artifact deals 2 damage to that
/// creature's controller." Either side's creature, and the damage goes to
/// whoever controlled the one that died.
#[test]
fn a_dying_creature_costs_its_controller_two_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dingus_staff(), llanowar_elves()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let (l0, l1) = (life_of(&engine, p0), life_of(&engine, p1));

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    kill(&mut engine, ogre);
    assert_eq!(life_of(&engine, p1), l1 - 2, "the Ogre's controller is hit");
    assert_eq!(
        life_of(&engine, p0),
        l0,
        "and the Staff's controller is not"
    );

    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    kill(&mut engine, elf);
    assert_eq!(
        life_of(&engine, p0),
        l0 - 2,
        "a creature of its own controller too"
    );
    assert_eq!(
        life_of(&engine, p1),
        l1 - 2,
        "the other seat is not hit twice"
    );
}

/// A non-creature going to the graveyard is not a creature dying.
#[test]
fn a_dying_artifact_that_is_no_creature_does_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dingus_staff(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rock = on_battlefield(&engine, p0, quiet_artifact()).unwrap();
    kill(&mut engine, rock);
    assert_eq!(life_of(&engine, p0), 20);
    assert_eq!(life_of(&engine, p1), 20);
}
